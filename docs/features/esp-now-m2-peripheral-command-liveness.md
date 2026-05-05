# Feature: ESP-NOW M2 — Peripheral Command and Peer Liveness Primitives

*Status: Design / Scouting*
*Created: 2026-05-05.*

## Motivation

Chromatic Clash M1 (`hal_c6_multitask_async`) demonstrated coordinated Embassy tasks on a single
board using an in-process `Signal`.
M2 extends this across the radio: one ESP32-C6 acts as a **controller** (sends commands) and one
or more act as **peripherals** (receive commands and drive the LED ring).

Two primitive concerns must be solved before application logic can be added:

1. **Peripheral Command** — how to encode and decode effect-control messages that fit within
   ESP-NOW's 250-byte payload and decode unambiguously on the peripheral side.
2. **Peer Liveness** — how to detect that a controller peer has gone silent so the peripheral
   can fall back to a safe default (e.g. slow pulse) instead of freezing on the last received
   effect indefinitely.

This document scouts both primitives, records design alternatives and their trade-offs, and lists
open questions that need resolution before implementation begins.

---

## ESP-NOW Constraints

| Property              | Value                          |
|:----------------------|:-------------------------------|
| Max payload           | 250 bytes                      |
| Addressing            | 6-byte MAC address             |
| Delivery confirmation | optional `send_cb` per frame   |
| Reliability           | unreliable datagram (no ACK by default unless unicast + send_cb) |
| Encryption            | optional PMK/LMK (16-byte key) |
| Max peer list         | 20 peers                       |
| Crate to use          | `esp-now` (esp-rs ecosystem)   |

<!-- SUGGESTION: confirm the exact `esp-now` crate version and API surface before
     committing to any of the encoding choices below. The crate is still in active
     development and its `EspNow::receive` and `send` signatures changed between
     0.1 and 0.2. Pin the version in `Cargo.toml` with `"=x.y.z"` matching the
     rest of the workspace's exact-pin strategy. -->

---

## 1. Peripheral Command Primitives

### 1.1 Command Vocabulary

The minimum set of commands needed for M2:

```
SetEffect(effect_id: u8)            — switch to the named animation
SetColor(r: u8, g: u8, b: u8)      — change the primary color in the current effect
SetBrightness(value: u8)            — global brightness scale (0 = off, 255 = full)
Ping                                — controller→peripheral heartbeat probe
Pong                                — peripheral→controller liveness reply (optional in M2)
Reset                               — return to default effect + color + brightness
```

<!-- IMPROVEMENT: keep the command set small for M2. Additional commands
     (SetSpeed, SetPalette, SetNumLeds, etc.) can be added later without
     breaking the wire format if the first byte is treated as an opcode with
     the remaining bytes as variable-length arguments. Do NOT over-engineer
     the command vocabulary now — every extra variant that is never sent still
     needs a match arm on the peripheral and inflate the ROM. -->

<!-- SUGGESTION: assign opcode 0x00 to `Ping` deliberately so that a zero-length
     packet or a zero-filled buffer is treated as a no-op heartbeat rather than
     a malformed command. This makes the liveness mechanism tolerant of
     accidental padding. -->

### 1.2 Wire Encoding

A fixed-size, tag–value layout fits inside 4 bytes:

```
Byte 0: opcode (u8)
Byte 1: arg0   (u8, meaning depends on opcode)
Byte 2: arg1   (u8, meaning depends on opcode)
Byte 3: arg2   (u8, meaning depends on opcode)
```

Encoding table:

| Command           | Byte 0 | Byte 1 | Byte 2 | Byte 3 |
|:------------------|-------:|-------:|-------:|-------:|
| `Ping`            | `0x00` |   `0`  |   `0`  |   `0`  |
| `Pong`            | `0x01` |   `0`  |   `0`  |   `0`  |
| `SetEffect`       | `0x10` | effect |   `0`  |   `0`  |
| `SetColor`        | `0x11` |  `r`   |  `g`   |  `b`   |
| `SetBrightness`   | `0x12` | value  |   `0`  |   `0`  |
| `Reset`           | `0xFF` |   `0`  |   `0`  |   `0`  |

<!-- IMPROVEMENT: grouping opcodes into ranges (0x00–0x0F = control, 0x10–0x1F
     = effect, 0xFF = special) keeps future extensions predictable. The range
     0x20–0xFE is reserved for future commands without conflicting with the
     liveness opcodes. Document this in the crate's type-level docs so that
     downstream contributors don't accidentally reuse opcode values. -->

<!-- COMMENT: four bytes is intentionally minimal. ESP-NOW's 250-byte limit is
     generous, but small frames are cheaper over the air and trivial to
     allocate on the stack. If a future command genuinely needs more than
     3 argument bytes (e.g. a palette upload), introduce a separate `PayloadV2`
     type rather than growing the fixed-size layout — this keeps the common
     case allocation-free. -->

Rust type sketch:

```rust
/// A command sent from a controller to a peripheral LED node.
///
/// The wire representation is exactly 4 bytes (see `to_bytes` / `from_bytes`).
/// Commands are transported as ESP-NOW unicast datagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Controller heartbeat probe.
    ///
    /// A peripheral that does not receive a `Ping` within the liveness timeout
    /// should fall back to the default effect. No reply is required in M2.
    Ping = 0x00,

    /// Peripheral liveness reply (sent if the controller requests it).
    ///
    /// Not required for M2 unidirectional flow, but included so the opcode
    /// space is reserved and the peripheral can optionally send it.
    Pong = 0x01,

    /// Switch the active animation effect.
    ///
    /// `effect_id` maps to the same index as in `hal_c6_multitask_async`'s
    /// `NUM_EFFECTS` constant (0 = RainbowComet, 1 = Meteor, 2 = Breathe,
    /// 3 = Spinner). Unknown IDs should be silently ignored — do NOT panic.
    SetEffect { effect_id: u8 },

    /// Change the primary color of the current effect.
    ///
    /// Not all effects use a configurable primary color (e.g. RainbowComet
    /// ignores it). The peripheral is responsible for routing the call to
    /// the correct setter; effects that do not support `set_color` should
    /// silently discard this command.
    SetColor { r: u8, g: u8, b: u8 },

    /// Set the global brightness scale.
    ///
    /// This is applied as a post-render scale on the color buffer,
    /// not via the individual effect's brightness parameter.
    /// A value of 0 turns all LEDs off; 255 is full brightness.
    SetBrightness { value: u8 },

    /// Return to the default effect, color, and brightness.
    ///
    /// Equivalent to a power-cycle from the user's perspective without
    /// actually rebooting the peripheral.
    Reset = 0xFF,
}

impl Command {
    /// Encode this command into 4 bytes suitable for an ESP-NOW payload.
    pub fn to_bytes(self) -> [u8; 4] {
        match self {
            Command::Ping                       => [0x00, 0, 0, 0],
            Command::Pong                       => [0x01, 0, 0, 0],
            Command::SetEffect { effect_id }    => [0x10, effect_id, 0, 0],
            Command::SetColor { r, g, b }       => [0x11, r, g, b],
            Command::SetBrightness { value }    => [0x12, value, 0, 0],
            Command::Reset                      => [0xFF, 0, 0, 0],
        }
    }

    /// Decode a command from the first 4 bytes of an ESP-NOW payload.
    ///
    /// Returns `None` for unknown opcodes rather than an error — this makes
    /// the peripheral forward-compatible with future command extensions
    /// sent by a newer controller.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // COMMENT: require exactly 4 bytes. Shorter payloads indicate a
        // protocol mismatch or transmission truncation; discard silently.
        if bytes.len() < 4 {
            return None;
        }
        match bytes[0] {
            0x00 => Some(Command::Ping),
            0x01 => Some(Command::Pong),
            0x10 => Some(Command::SetEffect { effect_id: bytes[1] }),
            0x11 => Some(Command::SetColor { r: bytes[1], g: bytes[2], b: bytes[3] }),
            0x12 => Some(Command::SetBrightness { value: bytes[1] }),
            0xFF => Some(Command::Reset),
            _    => None, // unknown opcode: silently ignore
        }
    }
}
```

<!-- SUGGESTION: derive `PartialEq` and add unit tests for every (Command,
     round-trip) pair in the `ws2812-pure` crate or a new `esp-now-protocol`
     crate that has zero hardware dependencies. This follows the sans-io
     pattern already established by `ferriswheel`. ESP-NOW encoding logic
     has no reason to touch a radio peripheral — keep it testable on the host. -->

<!-- IMPROVEMENT: consider placing `Command` in a new `no_std` crate
     (e.g. `rustyfarian-esp-now-protocol`) that both the controller and the
     peripheral binary can depend on. This avoids duplicating the encoding
     logic, prevents the two sides from drifting, and is consistent with how
     `ws2812-pure` isolates pure logic. The crate would have zero mandatory
     dependencies — just `core`. -->

### 1.3 Command Dispatch on the Peripheral

The peripheral receives ESP-NOW frames in a callback or via an async channel.
The recommended pattern for Embassy is to push received frames into a
`embassy_sync::channel::Channel` (bounded, capacity = 4 is sufficient for
the M2 demo) and dequeue them in the render task:

```rust
// In peripheral firmware (sketch, not final):

static CMD_CHANNEL: Channel<CriticalSectionRawMutex, Command, 4> = Channel::new();

// ESP-NOW receive callback pushes into the channel:
fn on_receive(data: &[u8]) {
    if let Some(cmd) = Command::from_bytes(data) {
        // try_send: if the channel is full, drop the oldest frame.
        // For LED control, losing a command is better than blocking the ISR.
        let _ = CMD_CHANNEL.try_send(cmd);
    }
}

// Render task dequeues commands before each frame:
async fn render_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    loop {
        // Non-blocking check: drain all pending commands before rendering.
        while let Ok(cmd) = CMD_CHANNEL.try_receive() {
            dispatch_command(cmd, &mut state);
        }
        // ... render frame, await timer ...
    }
}
```

<!-- COMMENT: `try_send` from an ISR context is safe with `CriticalSectionRawMutex`
     because the channel's push path only briefly disables interrupts to update
     the write index. Do not use `send().await` from a callback — callbacks are
     not async contexts. -->

<!-- SUGGESTION: for M2 the callback approach is fine, but if `esp-now` exposes
     an async receive API in a future version, switch to that and eliminate the
     static channel in favour of direct `.await` in a dedicated receive task.
     Flag this with a `// TODO(m3): switch to async receive when esp-now supports it`
     comment so it is not forgotten. -->

---

## 2. Peer Liveness Primitives

### 2.1 The Liveness Problem

ESP-NOW is an unreliable datagram protocol. The controller may:
- Go out of range silently
- Reboot without notifying the peripheral
- Crash

Without a liveness mechanism, the peripheral would freeze on the last received
effect forever. A safe fallback (e.g. slow blue pulse indicating "no signal")
is essential for a polished demo.

<!-- COMMENT: the same problem exists in the opposite direction — the controller
     does not know if the peripheral is still alive. For M2 (unidirectional
     demo) only the peripheral's liveness detection is in scope. Controller-side
     awareness of peripheral death can be deferred to M3. -->

### 2.2 Heartbeat Design

The simplest viable liveness mechanism is a **periodic Ping** from the controller
and a **timeout watchdog** on the peripheral:

```
Controller                       Peripheral
    │──── Ping (every T_ping) ──────►│  heartbeat_at = now()
    │──── Ping ──────────────────────►│  heartbeat_at = now()
    │   (controller reboots / goes out of range)
    │                                │  now() - heartbeat_at > T_timeout
    │                                │  → enter FALLBACK state
```

Recommended intervals for M2:

| Parameter      | Value      | Rationale                                                    |
|:---------------|:----------:|:-------------------------------------------------------------|
| `T_ping`       | 500 ms     | Low radio duty cycle; 2 Hz pings are barely noticeable       |
| `T_timeout`    | 2000 ms    | Misses 4 consecutive pings before triggering; tolerant of    |
|                |            | occasional packet loss without false positives               |
| Fallback effect| slow pulse | Visually distinct from all M2 effects; obviously "offline"   |

<!-- IMPROVEMENT: express T_timeout as a multiple of T_ping (e.g.
     `T_timeout = 4 * T_ping`) rather than as independent constants.
     This makes the relationship explicit and prevents accidental
     misconfiguration where T_timeout < T_ping. -->

### 2.3 Liveness State Machine

```
              ┌──────────┐
   power-on   │          │  Ping received
  ──────────► │  WAITING │ ─────────────────────────────────────────► ┐
              │          │                                             │
              └──────────┘                                             ▼
                   │                                            ┌────────────┐
                   │ T_timeout elapsed                          │   LIVE     │◄──┐
                   │ (no Ping ever received)                    └────────────┘   │
                   ▼                                                  │          │ Ping
              ┌──────────┐      Ping received                        │ T_timeout│ received
              │ FALLBACK │◄────────────────────────────────────────── ┘ elapsed  │
              │          │                                             ▲          │
              └──────────┘ ────────────────────────────────────────────┘          │
                   │        Ping received                                         │
                   └─────────────────────────────────────────────────────────────┘
```

<!-- COMMENT: the WAITING state is entered at power-on and covers the period
     before the first Ping is received. It is separate from FALLBACK so that
     the power-on animation (e.g. a brief boot sequence) can be distinct from
     the "lost signal" animation. For M2 both states can show the same fallback
     effect, but keeping them separate in the state machine costs nothing and
     makes future refinement easier. -->

Rust sketch:

```rust
use embassy_time::{Duration, Instant};

/// Tracks whether the controller peer is reachable.
pub struct PeerLiveness {
    last_ping: Option<Instant>,
    timeout: Duration,
}

impl PeerLiveness {
    /// `timeout` — duration after the last Ping before the peer is considered dead.
    pub const fn new(timeout: Duration) -> Self {
        Self { last_ping: None, timeout }
    }

    /// Record that a Ping (or any command) was received from the controller.
    ///
    /// Call this every time a valid frame arrives, not only on `Command::Ping`.
    /// Any activity from the controller proves liveness; requiring a dedicated
    /// Ping opcode is stricter than necessary and drops useful signal.
    pub fn record_activity(&mut self) {
        self.last_ping = Some(Instant::now());
    }

    /// Returns `true` if the controller is considered alive.
    pub fn is_alive(&self) -> bool {
        match self.last_ping {
            None => false, // never heard from the controller
            Some(t) => t.elapsed() < self.timeout,
        }
    }

    /// Returns `true` if the controller has been silent long enough to trigger fallback.
    pub fn timed_out(&self) -> bool {
        !self.is_alive()
    }
}
```

<!-- SUGGESTION: call `record_activity` on *every* successfully decoded frame,
     not just on `Command::Ping`. This means `SetEffect`, `SetColor`, etc. all
     implicitly extend the liveness window. The dedicated `Ping` command is then
     only needed when the controller has no other commands to send. This halves
     the radio traffic in practice (the controller skips the Ping whenever it
     has just sent a real command). -->

<!-- IMPROVEMENT: `PeerLiveness` has no hardware dependencies and can live in a
     `no_std` module that is unit-testable on the host using `std::time` in
     `#[cfg(test)]` (or by injecting a clock via a trait). Write tests that
     exercise:
     - is_alive() returns false before any activity
     - is_alive() returns true immediately after record_activity()
     - is_alive() returns false after timeout elapses
     Avoids needing real hardware to validate the timeout logic. -->

### 2.4 Integration in the Render Task

```rust
const LIVENESS_TIMEOUT: Duration = Duration::from_millis(2000);

async fn render_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    let mut liveness = PeerLiveness::new(LIVENESS_TIMEOUT);
    let mut state = EffectState::default();

    loop {
        // Drain pending commands; update liveness on each valid command.
        while let Ok(cmd) = CMD_CHANNEL.try_receive() {
            liveness.record_activity();
            dispatch_command(cmd, &mut state);
        }

        // Select the animation source based on liveness.
        if liveness.timed_out() {
            // FALLBACK: slow blue pulse to indicate "no controller signal".
            state.apply_fallback();
        }

        // Render and transmit.
        state.update_frame(&mut colors);
        ws.set_pixels_slice(&colors).await.unwrap();
        Timer::after_millis(FRAME_DELAY_MS).await;
    }
}
```

<!-- COMMENT: `apply_fallback` should be idempotent — calling it every frame
     when timed_out() is true must not cause visible flicker or restart the
     animation from scratch. The simplest implementation is: if the current
     effect is already the fallback, do nothing; otherwise switch to it.
     Use a boolean `in_fallback` flag to avoid repeated resets. -->

<!-- SUGGESTION: log the liveness transition (LIVE → FALLBACK and FALLBACK → LIVE)
     via `esp_println::println!` in debug builds so that demo operators can
     see in serial output exactly when the controller went silent. Wrap in
     `#[cfg(feature = "debug-log")]` so that release builds have zero logging
     overhead. -->

---

## 3. Open Questions

1. **`esp-now` crate API** — The `esp-now` crate version to use with `esp-hal 1.1.0`
   must be confirmed. The receive callback model (synchronous callback) vs. an async
   receive future model differ significantly in how `CMD_CHANNEL` is populated.
   *Action: spike a bare `esp-now::EspNow::receive` call on the C6 before writing
   the full command layer.*

2. **Broadcast vs. unicast** — For M2 (single controller, single peripheral), unicast
   using the MAC address of the controller is simplest. For M3 (one controller,
   multiple peripherals), broadcast or multicast should be considered.
   *Decision deferred to M3; M2 hardcodes the peer MAC in a `const`.*

3. **Encryption** — ESP-NOW supports LMK (link master key) encryption per peer.
   For the demo context no encryption is needed, but the `PeerConfig` type in
   `esp-now` requires a decision. For M2: no encryption; document the choice
   explicitly in the example source with a `// SECURITY NOTE:` comment.

4. **Channel conflict with Wi-Fi** — ESP-NOW and Wi-Fi cannot use different channels
   simultaneously on a single radio. If both are needed in M3 (e.g. OTA update +
   ESP-NOW control), they must share a channel. For M2 (ESP-NOW only): no conflict.

5. **Peripheral MAC address discovery** — The controller needs to know the peripheral's
   MAC address to add it as a peer. Options: (a) hardcode in both firmwares, (b) use
   broadcast discovery, (c) print it at boot via serial and paste it in.
   *For M2: option (c) — print MAC at boot, paste into controller const. Document
   the procedure in the example's `//!` header.*

6. **Frame loss handling** — A `SetEffect` command sent over a noisy channel may be
   dropped. The peripheral should apply the command if received, and silently ignore
   any gaps. The controller may optionally send critical commands twice with a 10 ms
   gap. *For M2: no retransmission. Log frame loss via send_cb if available.*

7. **`embassy_time::Instant` in `no_std` without `std`** — `PeerLiveness::new` uses
   `const fn` but `Instant::now()` is not `const`. The struct can be initialized with
   `None` as shown; confirm that `embassy-time`'s `Instant` is available in the
   `no_std + embassy` build and that `t.elapsed()` compiles under `esp-rtos 0.3`.

---

## 4. Non-Goals for M2

- Multiple peripherals (M3)
- Controller-side liveness tracking of peripheral death
- Encrypted ESP-NOW
- OTA firmware update over ESP-NOW
- Dynamic peer discovery (broadcast beacon / mDNS equivalent)
- Per-effect configuration parameters (speed, tail length, etc.) in commands

---

## 5. Implementation Order

1. **Spike** — Add `esp-now` to `rustyfarian-esp-hal-ws2812` as an optional feature;
   confirm a raw `Ping`/`Pong` exchange compiles and runs on two C6 boards.
2. **Protocol crate** — Extract `Command` + `PeerLiveness` into a zero-dependency
   `no_std` crate (e.g. `rustyfarian-esp-now-protocol`) with host-runnable tests.
3. **Peripheral firmware** — `hal_c6_espnow_peripheral` example: receives commands,
   drives LED ring, falls back on timeout.
4. **Controller firmware** — `hal_c6_espnow_controller` example: button cycles effects,
   sends `SetEffect` + periodic `Ping`.
5. **Integration test** — Two-board smoke test: press button on controller, confirm
   effect change on peripheral; power-cycle controller, confirm peripheral falls back
   within 2 s.
6. **ADR** — Record the wire-format decision, crate layout decision, and liveness
   parameter choices in a new `docs/adr/009-esp-now-m2.md`.

---

## 6. Alternatives Considered

| Alternative                                         | Why not (for M2)                                                                  |
|:----------------------------------------------------|:----------------------------------------------------------------------------------|
| MQTT over Wi-Fi                                     | Requires broker, TLS, much larger firmware; ESP-NOW is the stated goal            |
| BLE GATT                                            | Higher overhead, pairing ceremony; ESP-NOW is simpler for a demo                  |
| Variable-length command encoding (e.g. `postcard`)  | Overkill for ≤6 commands; fixed 4-byte layout is trivial to decode and test       |
| `serde` + JSON                                      | `no_std` JSON is non-trivial; too large for a 4-byte opcode space                 |
| Symmetric heartbeat (both sides ping each other)    | Doubles radio traffic; controller-side liveness is not needed for M2              |
| T_timeout = T_ping (1 missed ping = fallback)       | Too sensitive to single-packet loss; 4× multiple is the conventional minimum      |

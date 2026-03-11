# ADR 006: Async Support Strategy

## Status

Accepted

## Context

With the dual-HAL strategy in place (ADR 005), we now have two fully implemented driver crates:

- `rustyfarian-esp-idf-ws2812` — std, blocking, using `esp-idf-hal 0.46`
- `rustyfarian-esp-hal-ws2812` — no_std, blocking, using `esp-hal 1.0.0`

The ESP Rust ecosystem increasingly uses async for embedded concurrency.
A typical downstream application (e.g., an RGB clock) runs LED animations alongside Wi-Fi, sensors, and timers.
Async lets these tasks share a single thread cooperatively instead of requiring an RTOS or busy-waiting.

### Async runtimes in the ESP ecosystem

| Stack               | Executor               | Timer/delay                    | RMT async?                          |
|:--------------------|:-----------------------|:-------------------------------|:------------------------------------|
| `esp-hal` (no_std)  | Embassy via `esp-rtos` | `embassy-time`                 | Yes — `Rmt::new().into_async()`     |
| `esp-idf-hal` (std) | `edge-executor`, tokio | `esp-idf-svc::EspTimerService` | No — `TxRmtDriver` is blocking only |

The asymmetry is important: `esp-hal` has native async RMT support, while `esp-idf-hal` does not.

### Where async adds value for WS2812

There are three levels where async could be introduced:

**Driver transmission** — An async `set_pixels_slice().await` yields to the executor while the RMT peripheral transmits autonomously.
In practice, WS2812 transmission is fast (~30 µs per LED, ~360 µs for a 12-LED ring), so the yield window is small.
Still, in a bare-metal system with no threads, even small yields prevent blocking other tasks.

**Animation loop delays** — This is where async has the most impact.
A typical animation loop sleeps 16–50 ms between frames.
With blocking sleep, nothing else runs.
With `Timer::after_millis(16).await`, the executor can service Wi-Fi, button handling, or sensor reads during the delay.

**The `Effect` trait** — `update()` and `current()` are pure computation (no I/O, no delay).
Making them async would add complexity with no benefit.

### Existing async WS2812 crates

**`esp-hal-smartled2`** — Wraps esp-hal RMT, supports both blocking (`SmartLedsWrite`) and async (`SmartLedsWriteAsync`) depending on the channel mode.
Uses the `smart-leds` trait ecosystem.

**`ws2812-async`** — Uses async SPI (via `embedded-hal-async::spi`) to bit-bang WS2812 timing.
Portable across HALs but ties up an SPI bus.

Neither of these follows this project's philosophy of separating pure logic from hardware I/O.
They embed color conversion and encoding logic in the driver.

## Options

### Option A: No async support

Keep all crates synchronous.
Users write their own async wrappers:

```rust
loop {
    effect.update(&mut buffer)?;
    driver.set_pixels_slice(&buffer)?;
    Timer::after_millis(16).await;
}
```

**Pros:**

- Zero additional complexity
- No async runtime dependency in any crate
- Users already need an executor for their application; wrapping is trivial

**Cons:**

- `rustyfarian-esp-hal-ws2812` cannot yield during RMT transmission (blocks the executor)
- Users must import and manage `embassy-time` or equivalent themselves

### Option B: Async driver for `rustyfarian-esp-hal-ws2812` only

Add an `async` feature flag to `rustyfarian-esp-hal-ws2812` that enables async variants of `set_pixel` and `set_pixels_slice`.
The `rustyfarian-esp-idf-ws2812` crate stays blocking (no async RMT available in `esp-idf-hal`).

The driver currently uses `Channel<'d, Blocking, Tx>`.
To support async, the struct would become generic over `esp-hal`'s driver mode:

```rust
pub struct Ws2812Rmt<'d, Dm: esp_hal::DriverMode, const N: usize> {
    channel: Option<Channel<'d, Dm, Tx>>,
    buffer: [PulseCode; N],
}
```

This is a **breaking change** to the current public API (`Ws2812Rmt<'d, const N: usize>`).
Existing blocking users would need to write `Ws2812Rmt<'d, Blocking, N>` instead of `Ws2812Rmt<'d, N>`.
A type alias can reduce the migration cost:

```rust
pub type Ws2812RmtBlocking<'d, const N: usize> = Ws2812Rmt<'d, Blocking, N>;
```

The blocking `set_pixels_slice` stays on `Ws2812Rmt<'d, Blocking, N>` as today.
The async variant lives on `Ws2812Rmt<'d, Async, N>`:

```rust
impl<'d, const N: usize> Ws2812Rmt<'d, Async, N> {
    pub async fn set_pixels_slice(&mut self, rgbs: &[RGB8]) -> Result<(), Error> {
        let num_leds = rgbs.len();
        let needed = num_leds * 24 + 1;
        if needed > N {
            return Err(Error::BufferTooSmall);
        }
        for (i, &rgb) in rgbs.iter().enumerate() {
            Self::encode_color(rgb, &mut self.buffer[i * 24..(i + 1) * 24]);
        }
        self.buffer[num_leds * 24] = PulseCode::end_marker();
        self.do_transmit_async(needed).await
    }
}
```

Pure logic crates (`ws2812-pure`, `ferriswheel`, `led-effects`) remain unchanged.

**Pros:**

- Yields during RMT transmission on bare-metal
- Follows `esp-hal`'s own `Blocking`/`Async` driver mode pattern
- No impact on pure logic crates or `rustyfarian-esp-idf-ws2812`
- Feature-gated: no cost for users who don't need async

**Cons:**

- Breaking change to the existing `Ws2812Rmt` API (type parameter order changes)
- Adds `esp-rtos` as an optional dependency
- Only benefits the `esp-hal` path (asymmetric — reflects HAL reality)

### Option C: Async animation runner in `ferriswheel`

Add an optional async `run_effect` function to `ferriswheel` that drives an `Effect` + driver in a loop with async delays:

```rust
pub async fn run_effect<D, E>(
    effect: &mut dyn Effect,
    driver: &mut D,
    buffer: &mut [RGB8],
    frame_delay: Duration,
) -> Result<!, E>
where
    D: StatusLed<Error = E>,
{
    loop {
        effect.update(buffer)?;
        driver.set_pixels_slice(buffer)?;
        Timer::after(frame_delay).await;
    }
}
```

**Pros:**

- Convenient high-level API for the common "animate forever" pattern
- Centralizes the animation loop logic

**Cons:**

- **Couples a pure logic crate to an async runtime** (`embassy-time` dependency behind feature flag)
- Violates the project's core principle: `ferriswheel` is currently pure computation with zero I/O dependencies
- The animation runner is trivial to write in application code (3–5 lines)
- Becomes the wrong abstraction when users need cancellation, dynamic effect switching, or frame-rate adaptation

### Option D: Both B and C

Async driver (Option B) plus async animation runner (Option C).

Inherits the pros and cons of both.
The coupling concern from Option C applies regardless of whether Option B is also adopted.

## Decision

**Option B: Async driver for `rustyfarian-esp-hal-ws2812` only.**

### Rationale

The async RMT channel is a hardware capability that belongs in the driver crate.
Adding it there follows `esp-hal`'s own `Blocking`/`Async` pattern and has zero impact on pure logic crates.

Option C is rejected because it violates the foundational principle of this project: pure logic crates should have no I/O or runtime dependencies.
The animation loop is application-level glue code (3 lines) that belongs in the downstream application, not in a library crate.
Adding `embassy-time` to `ferriswheel` would make it the first pure crate with an optional runtime dependency — a precedent that erodes the clean separation.

Option A is viable but leaves value on the table for `esp-hal` users.
The RMT transmission yield is small but meaningful in bare-metal systems where every microsecond of blocking is a missed interrupt opportunity.

`rustyfarian-esp-idf-ws2812` does not gain async support because `esp-idf-hal`'s `TxRmtDriver` is blocking-only.
If ESP-IDF gains async RMT support in the future, async can be added then under a separate feature flag.

### Implementation sketch

The `async` feature flag enables `esp-hal-embassy` and the `Async` driver mode.

```toml
[features]
default = ["esp32c6", "unstable", "led-effects"]
esp32c6    = ["esp-hal/esp32c6",  "esp-bootloader-esp-idf/esp32c6"]
esp32c3    = ["esp-hal/esp32c3",  "esp-bootloader-esp-idf/esp32c3"]
esp32      = ["esp-hal/esp32",    "esp-bootloader-esp-idf/esp32"]
unstable   = ["esp-hal/unstable"]
led-effects = ["dep:led-effects"]
smart-leds  = ["dep:smart-leds"]
rt         = ["esp-hal/rt"]
async      = ["dep:esp-rtos", "esp-hal/unstable"]
```

The struct gains a `Dm: DriverMode` type parameter alongside the existing `const N: usize`:

```rust
pub struct Ws2812Rmt<'d, Dm: esp_hal::DriverMode, const N: usize> {
    channel: Option<Channel<'d, Dm, Tx>>,
    buffer: [PulseCode; N],
}
```

Blocking construction (pre-configured channel, as today):

```rust
let channel = rmt.channel0.configure_tx(pin, config).unwrap();
let mut led = Ws2812Rmt::<Blocking, N>::new(channel);
led.set_pixels_slice(&buffer).unwrap();
```

Async construction:

```rust
let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
    .unwrap()
    .into_async();
let channel = rmt.channel0.configure_tx(pin, config).unwrap();
let mut led = Ws2812Rmt::<Async, N>::new(channel);

let mut rainbow = RainbowEffect::new(12)?;
let mut buffer = [RGB8::default(); 12];

loop {
    rainbow.update(&mut buffer)?;
    led.set_pixels_slice(&buffer).await?;
    Timer::after_millis(16).await;
}
```

Both call `ws2812_pure::rgb_to_grb()` for color conversion — the pure logic stays shared.

## Consequences

### Positive

- **Pure crates stay pure** — `ws2812-pure`, `ferriswheel`, and `led-effects` gain no async dependencies
- **Ecosystem alignment** — follows `esp-hal`'s own `Blocking`/`Async` driver mode pattern
- **Incremental** — async is feature-gated, no cost for blocking-only users
- **Bare-metal friendly** — enables cooperative multitasking on the no_std path where it matters most

### Negative

- **Asymmetric** — only `rustyfarian-esp-hal-ws2812` gets async; `rustyfarian-esp-idf-ws2812` stays blocking (reflects the underlying HAL reality)
- **Breaking change** — adding the `Dm` type parameter to `Ws2812Rmt` changes the existing public API; a `Ws2812RmtBlocking` type alias can ease migration
- **API surface** — two code paths (blocking + async) in the driver to maintain
- **Embassy version** — deferred to implementation time

### What this ADR does NOT cover

- **Which `esp-rtos` version to target** — deferred to implementation time; verify `esp-rtos` compatibility with the pinned `esp-hal 1.0.0` (note: `esp-hal-embassy` was deprecated in early 2025 and its functionality merged into `esp-rtos` v0.2.0)
- **Async `StatusLed` trait** — `led-effects` currently defines a sync `StatusLed` trait; an async variant (`AsyncStatusLed`) may be warranted but is a separate decision
- **`rustyfarian-esp-idf-ws2812` async** — revisit if `esp-idf-hal` gains async RMT support
- **`SmartLedsWriteAsync`** — the `smart-leds` ecosystem defines an async write trait; implementing it on `Ws2812Rmt<'d, Async, N>` is a natural follow-on

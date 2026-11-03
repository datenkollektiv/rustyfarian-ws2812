# Embassy Task Spawning Patterns for esp-hal 1.2.2 + esp-rtos 0.4.0

This document answers five concrete questions about spawning Embassy tasks on ESP32
with `esp-hal 1.2.2`, `esp-rtos 0.4.0` and `embassy-executor 0.10.0`.
All answers are grounded in the actual versions pinned in this workspace's `Cargo.toml`
and verified against the official docs and release notes.

> **2026-09-21:** the workspace moved to `esp-hal 1.2.2` + `esp-rtos 0.4.0`.
> The one change that touches this document is the scheduler entry point: `esp-hal 1.2.0` removed `SoftwareInterruptControl` and the `SW_INTERRUPT` singleton, and `esp_rtos::start` now takes `peripherals.FROM_CPU_INTR0` directly (see the example below and `docs/esp-hal-version-matrix.md`).
> The five answers themselves were written against 1.1.0 / 0.3 and have **not** been re-verified against 0.4 beyond compiling the workspace examples.

## `make_static!` — Where Does it Live?

`make_static!` is a macro in the `static_cell` crate (not in `esp-hal` or `esp-rtos`).
It requires a **nightly** compiler and the `#![feature(type_alias_impl_trait)]` attribute.
`esp-rtos 0.4.0` exports no `mk_static` or `make_static` macro of its own —
its public API includes the attribute macro `#[esp_rtos::main]`, the `embassy` module,
and the functions `start` and `start_with_idle_hook`.

For projects on stable Rust, the community pattern is a local `mk_static!` macro built on
`static_cell::StaticCell`:

```rust
macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        STATIC_CELL.uninit().write($val)
    }};
}
```

**Critical distinction**: `mk_static!` / `StaticCell` are needed when you create a value at
runtime that must be given a `'static` lifetime — most commonly for spawning an additional
`Executor` on a second core.
They are **not needed** for single-core Embassy setups where `#[esp_rtos::main]` is used,
because `esp_hal::init()` already returns peripheral singletons with `'static` lifetime,
and `Ws2812Rmt<'static, Async, N>` satisfies the task `'static` bound directly.

## Passing an Async RMT Driver to a Spawned Task

### Why the driver is `'static`

`esp_hal::init()` returns peripheral singletons that live for the duration of the program.
Peripherals obtained from it, and drivers constructed from those peripherals, carry
a `'static` lifetime.
`Ws2812Rmt<'d, Dm, N>` holds its `Channel<'d, Dm, Tx>`, so its lifetime comes from the `Rmt` it was built from.
Because that `Rmt` is built from a `'static` peripheral singleton, `Ws2812Rmt<'static, Async, N>` can be passed directly to a spawned task with no `StaticCell` involved.

### Build the async driver in `main`

The pattern is to build the async driver in `main` and hand the driver, not the RMT channel, to the task.
`#[embassy_executor::task]` requires all arguments to be `Send + 'static`, and a `Ws2812Rmt<'static, Async, N>` meets both bounds.

`into_async()` is called on the **`Rmt`** struct, not on the channel, so the whole peripheral handle becomes async before its channel is taken.
`Ws2812Rmt::from_channel_creator` then configures the TX channel and connects the output pin in one call.
`from_channel_creator` assumes the 80 MHz RMT source rate that `Rmt::new(.., Rate::from_mhz(80))` sets, and `tx_channel_config_for(rate)` with `Ws2812Rmt::new` covers other source rates.

Build the driver in `main` and spawn the task with it:

```rust
let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
    .unwrap()
    .into_async();
let ws: Ws2812Rmt<'static, Async, N> =
    Ws2812Rmt::from_channel_creator(rmt.channel0, peripherals.GPIO18).unwrap();

spawner.spawn(led_task(ws).expect("led_task spawn token"));
```

The task takes the driver by value and sends frames from its own loop, with `N` set to `buffer_size(NUM_LEDS)` as in the complete example below:

```rust
#[embassy_executor::task]
async fn led_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    let mut colors = [RGB8::default(); NUM_LEDS];
    loop {
        ws.set_pixels_slice(&colors).await.unwrap();
        Timer::after_millis(30).await;
    }
}
```

There is no public `Channel::into_async()` in esp-hal 1.2.2, so the `Rmt` must be async before `from_channel_creator` takes `rmt.channel0`.

### Concrete complete example

The existing `hal_c6_rainbow_comet_async.rs` in this workspace demonstrates the same driver setup without task spawning (single-task, runs animation in `main`).
Below is how the same driver is handed to a spawned task.
The panic handler is omitted here; `hal_c6_multitask_async.rs` shows a full multitask setup with a button task and a panic handler.

```rust
#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{rmt::Rmt, time::Rate, timer::timg::TimerGroup, Async};
use ferriswheel::RainbowCometEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

#[embassy_executor::task]
async fn led_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    let mut effect = RainbowCometEffect::new(NUM_LEDS).unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];
    loop {
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors).await.unwrap();
        Timer::after_millis(30).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();
    let ws: Ws2812Rmt<'static, Async, N> =
        Ws2812Rmt::from_channel_creator(rmt.channel0, peripherals.GPIO18).unwrap();

    spawner.spawn(led_task(ws).expect("led_task spawn token"));

    loop {
        Timer::after_secs(60).await;
    }
}
```

## `embassy_sync::Signal` as a Static Global

`Signal<M, T>` is designed to be declared as a `static` global.
It implements `const fn new()`, making it usable in `static` initializers without any
`StaticCell` or runtime initialization step.
Multiple tasks can reference the same `static Signal` by shared reference (`&'static Signal<...>`).

```rust
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;

static EFFECT_SIGNAL: Signal<NoopRawMutex, u8> = Signal::new();
```

This is valid `no_std` code and needs no `#[link_section]`, `unsafe`, or nightly features.

### Which mutex type to use

| Mutex type                | When to use                                                                          |
|:--------------------------|:-------------------------------------------------------------------------------------|
| `CriticalSectionRawMutex` | Signal shared between tasks **and** interrupt handlers, or across multiple executors |
| `NoopRawMutex`            | Signal used only within a single executor, never from an ISR                         |
| `ThreadModeRawMutex`      | Same as `NoopRawMutex` but also enforces single-executor singleton semantics         |

For two tasks in the same `#[esp_rtos::main]` executor — the most common Embassy
pattern on `esp-rtos` — `NoopRawMutex` is the right choice: it's zero-cost and
correctly captures the single-executor invariant.
Reach for `CriticalSectionRawMutex` only if the signal is touched from an ISR or
shared across multiple executors (e.g., a separate interrupt-mode executor).

### Task sharing pattern

Multiple tasks can hold a `&'static Signal<...>` simultaneously.
Because `Signal<M, T>: Sync`, this is safe.
One task signals, the other waits — the API is:

```rust
// Sender task (e.g., button handler)
EFFECT_SIGNAL.signal(new_effect_id);

// Receiver task (e.g., LED animation loop)
let effect_id = EFFECT_SIGNAL.wait().await;
```

`Signal` stores only the **latest** value and wakes exactly **one** waiter.
If multiple tasks call `.wait()` concurrently, only one is woken.
Use `embassy_sync::watch::Watch` if you need all watchers notified.

## GPIO Button Input: Async API in esp-hal 1.2.2

`Input<'d>` carries a `'d` lifetime — it does **not** need to be `'static` in itself.
However, if it is passed to a spawned `#[embassy_executor::task]`, the task argument must be
`'static`, so an `Input<'static>` is required (which it will be if constructed from a
`'static` peripheral singleton via `esp_hal::init()`).

### Async edge-wait methods (requires `unstable` feature)

```rust
use esp_hal::gpio::{Input, InputConfig, Pull};

let config = InputConfig::default().with_pull(Pull::Up);
let mut button = Input::new(peripherals.GPIO9, config);

// These methods are all available:
button.wait_for_high().await;
button.wait_for_low().await;
button.wait_for_rising_edge().await;
button.wait_for_falling_edge().await;   // high → low transition
button.wait_for_any_edge().await;
```

All methods return `()` (not a `Result`) in the current API.
They implement the `embedded-hal-async` `Wait` trait.

### Passing to a spawned task

```rust
#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        EFFECT_SIGNAL.signal(next_effect());
    }
}

// In main:
let config = InputConfig::default().with_pull(Pull::Up);
let button = Input::new(peripherals.GPIO9, config);
// button is Input<'static> because peripherals.GPIO9 is 'static
spawner.spawn(button_task(button).expect("button_task spawn token"));
```

The `button` variable does not need to be stored in a `StaticCell` — the `'static` bound
is satisfied because `peripherals.GPIO9` itself is `'static`.

### Cancellation note

Dropping the future returned by any `wait_for_*` method cancels the wait.
If the event fires after the future is dropped, a subsequent wait will **miss** it.
For button debounce, add a small `Timer::after_millis(20).await` after the edge wait
before re-arming.

## Multiple Tasks Sharing a Static Signal

Yes.
`Signal<M, T>` is `Sync`, so any number of tasks can hold `&'static Signal<M, T>` without
any ownership transfer.
Tasks access it by name (as a module-level `static`) or via a shared reference:

```rust
static EFFECT_SIGNAL: Signal<NoopRawMutex, u8> = Signal::new();

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    loop {
        button.wait_for_falling_edge().await;
        EFFECT_SIGNAL.signal(1_u8);
        Timer::after_millis(20).await;   // debounce
    }
}

#[embassy_executor::task]
async fn led_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    loop {
        let effect_id = EFFECT_SIGNAL.wait().await;
        run_effect(&mut ws, effect_id).await;
    }
}
```

No `Arc`, no `Mutex`, no heap allocation.
The `static` declaration is sufficient.
Both tasks can be spawned from `main`'s `Spawner` without any coordination:

```rust
spawner.spawn(button_task(button).expect("button_task spawn token"));
spawner.spawn(led_task(ws).expect("led_task spawn token"));
```

## Summary Table

| Question                           | Answer                                                                                                                     |
|:-----------------------------------|:---------------------------------------------------------------------------------------------------------------------------|
| `make_static!` in esp-hal 1.2.2?   | No — lives in `static_cell` crate (nightly) or define `mk_static!` locally (stable)                                        |
| `make_static!` in esp-rtos 0.4?    | No — `esp-rtos` exports no such macro                                                                                      |
| Needed for single-core Embassy?    | Usually **no** — `esp_hal::init()` returns `'static` peripherals; `Ws2812Rmt<'static, ...>` satisfies task bounds directly |
| Pass RMT driver to task            | Build `Ws2812Rmt<'static, Async, N>` in `main` via `from_channel_creator`, then spawn                                      |
| `Channel::into_async()`?           | **Does not exist** — `into_async()` is on `Rmt`, not `Channel`                                                             |
| `Signal` as `static`?              | Yes — `Signal::new()` is `const fn`, no runtime init needed                                                                |
| Multiple tasks on one `Signal`?    | Yes — `Signal: Sync`, reference as `&'static Signal<M, T>`                                                                 |
| GPIO async API                     | `Input::wait_for_falling_edge().await` — requires `unstable` feature                                                       |
| `Input` needs `'static` for tasks? | Only if passed to a spawned task; `peripherals.GPIOx` is already `'static`                                                 |

## Key Insight: When is `StaticCell` Actually Needed?

`StaticCell` / `mk_static!` are needed when a **runtime-created value** (not sourced from
`esp_hal::init()`) must be given a `'static` lifetime.
Common cases:

- Spawning an Embassy `Executor` on a second core (ESP32/S3 dual-core).
- Initializing `embassy_net::StackResources` with a runtime-chosen capacity.
- Sharing a runtime-initialized buffer (e.g., a large array allocated on first call) across tasks.

For single-core LED animation with button input — the primary use case for this workspace —
`StaticCell` is not needed at all.
All required types (`Ws2812Rmt<'static, Async, N>`, `Input<'static>`, `Signal`) are either
directly `'static` from `esp_hal::init()` or are valid `static` globals by construction.

---

*Research date: 2026-03-27. Updated 2026-05-20: esp-rtos 0.2 → 0.3, esp-hal 1.0.0 → 1.1.0. Updated 2026-11-03: esp-hal 1.1.0 → 1.2.2, esp-rtos 0.3 → 0.4.0, and the RMT snippets now build the driver with `Ws2812Rmt::from_channel_creator`.*

Sources:
- [esp-hal 1.0.0 release announcement — Espressif Developer Portal](https://developer.espressif.com/blog/2025/10/esp-hal-1/)
- [esp-hal 1.0.0 beta announcement — Espressif Developer Portal](https://developer.espressif.com/blog/2025/02/rust-esp-hal-beta/)
- [esp-rs/esp-hal Releases — GitHub](https://github.com/esp-rs/esp-hal/releases)
- [esp_hal::rmt::Channel — docs.espressif.com](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32c6/esp_hal/rmt/struct.Channel.html)
- [esp_hal::rmt::Rmt — docs.espressif.com](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32c6/esp_hal/rmt/struct.Rmt.html)
- [esp_hal::gpio::Input — docs.espressif.com](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32c6/esp_hal/gpio/struct.Input.html)
- [esp_rtos — docs.espressif.com](https://docs.espressif.com/projects/rust/esp-rtos/0.3.0/esp32c6/esp_rtos/index.html)
- [static_cell::make_static! — docs.rs](https://docs.rs/static_cell/latest/static_cell/macro.make_static.html)
- [static_cell 1.1.0 source — docs.rs](https://docs.rs/crate/static_cell/1.1.0/source/src/lib.rs)
- [embassy_sync — docs.embassy.dev](https://docs.embassy.dev/embassy-sync)
- [Sharing Data Among Tasks in Rust Embassy — The Embedded Rustacean](https://blog.theembeddedrustacean.com/sharing-data-among-tasks-in-rust-embassy-synchronization-primitives)
- [impl Rust for ESP32 — Connecting Wi-Fi (mk_static! pattern)](https://esp32.implrust.com/wifi/embassy/connecting-wifi.html)

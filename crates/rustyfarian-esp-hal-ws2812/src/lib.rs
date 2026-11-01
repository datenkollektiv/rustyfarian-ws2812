#![no_std]
//! WS2812 (NeoPixel) LED driver using `esp-hal` RMT peripheral (bare-metal, `no_std`).
//!
//! This crate provides a bare-metal driver for WS2812/NeoPixel addressable LEDs
//! using the `esp-hal` RMT peripheral.
//! It is the `no_std` counterpart to `rustyfarian-esp-idf-ws2812`.
//!
//! Pure colour utilities are available in the `bunting` crate for testing.
//!
//! # Buffer Sizing
//!
//! The driver uses a const-generic buffer `[PulseCode; N]` where `N = num_leds * 24 + 1`.
//! Use [`buffer_size`] to compute `N` at compile time:
//!
//! ```ignore
//! use rustyfarian_esp_hal_ws2812::buffer_size;
//! const N: usize = buffer_size(8); // 8-LED ring
//! ```
//!
//! # RMT Clock Configuration
//!
//! The driver needs a 10 MHz RMT tick.
//! Using a different tick will produce incorrect LED timing.
//! [`tx_channel_config`] assumes an 80 MHz RMT source rate, the rate passed to `Rmt::new`, and sets [`RMT_CLK_DIV`] for it.
//! For any other source rate use [`tx_channel_config_for`], which derives the divider or returns [`Error::RmtConfig`] when no integer divider gives exactly 10 MHz.
//! [`Ws2812Rmt::from_channel_creator`] does the whole channel setup with [`tx_channel_config`], so it carries the same 80 MHz precondition.
//!
//! # Blocking Example
//!
//! ```ignore
//! use esp_hal::{
//!     rmt::{Rmt, TxChannelCreator},
//!     time::Rate,
//! };
//! use rgb::RGB8;
//! use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, tx_channel_config};
//!
//! const N: usize = buffer_size(1);
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
//! let channel = rmt
//!     .channel0
//!     .configure_tx(&tx_channel_config())
//!     .unwrap()
//!     .with_pin(peripherals.GPIO8);
//!
//! let mut led = Ws2812Rmt::<_, N>::new(channel);
//! led.set_pixel(RGB8::new(255, 0, 0)).unwrap();
//!
//! let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
//! led.set_pixels_slice(&colors).unwrap();
//! ```
//!
//! # Async Example (feature `embassy`)
//!
//! ```ignore
//! use embassy_time::Timer;
//! use esp_hal::{
//!     rmt::{Rmt, TxChannelCreator},
//!     time::Rate,
//! };
//! use rgb::RGB8;
//! use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, tx_channel_config};
//!
//! const NUM_LEDS: usize = 12;
//! const N: usize = buffer_size(NUM_LEDS);
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
//!     .unwrap()
//!     .into_async();
//! let channel = rmt
//!     .channel0
//!     .configure_tx(&tx_channel_config())
//!     .unwrap()
//!     .with_pin(peripherals.GPIO18);
//!
//! let mut ws = Ws2812Rmt::<_, N>::new(channel);
//! let colors = [RGB8::new(255, 0, 0); NUM_LEDS];
//!
//! loop {
//!     ws.set_pixels_slice(&colors).await.unwrap();
//!     Timer::after_millis(30).await;
//! }
//! ```
//!
//! # When does async help?
//!
//! WS2812 transmission is fast: approximately 30 µs per LED, or ~360 µs for a 12-LED ring.
//! In a bare-metal system with no RTOS threads, even that small window matters —
//! a blocking transmit prevents the executor from servicing any other tasks during that time.
//! Frames longer than one RMT channel's RAM are the exception; see [Long frames](crate#long-frames).
//!
//! The larger gain comes from inter-frame delays.
//! A typical animation loop waits 16–50 ms between frames.
//! With a blocking `delay_ms()`, the CPU is spinning the whole time.
//! With `Timer::after_millis(16).await`, the executor is free to handle Wi-Fi events,
//! button presses, sensor reads, or any other spawned task during that delay.
//!
//! # Long frames
//!
//! A frame of `num_leds * 24 + 1` pulse codes needs no refill only while it fits in one RMT channel's RAM.
//! In async mode such a frame is also truly async.
//! Longer frames follow the `LongFramePolicy` set with `with_long_frame_policy`, in blocking and async mode alike.
//! The default, `LongFramePolicy::CriticalSection`, drives them to completion inside a critical section.
//! In async mode the executor cannot run other tasks while they transmit.
//! The estimated nominal masked time, at 1.3 µs per pulse code, is capped at [`DEFAULT_MAX_NOMINAL_MASKED_US`] (1 ms, about 32 LEDs) by default, set with `with_max_nominal_masked_us`.
//! This limits the nominal estimate, not a hard interrupts-off bound.
//! A stalled refill can keep interrupts masked up to the timeout below, about 2.5 ms at the default limit, and the stopped-clock fallback can take longer.
//! A longer frame returns [`Error::FrameTooLong`] in both modes instead of masking, and `with_max_nominal_masked_us(None)` lifts the ceiling.
//! The estimate assumes the slowest pulse code (1.3 µs) for every code.
//! `LongFramePolicy::Poll`, `LongFramePolicy::Reject` and `set_pixels_slice_masked` ignore the ceiling.
//! `LongFramePolicy::Poll` polls the refill with interrupts live instead.
//! That keeps interrupts running but lets them delay the refill, and in async mode it blocks the executor too.
//! `LongFramePolicy::Reject` returns [`Error::FrameTooLong`] instead and sends nothing.
//! `set_pixels_slice_masked` is the blocking method that always masks interrupts for the frame, whatever the length, policy and ceiling.
//!
//! | Chip                         | Channel RAM | Largest frame without a refill |
//! |:-----------------------------|:------------|:-------------------------------|
//! | ESP32-C3, ESP32-C6, ESP32-S3 | 48 codes    | 1 LED                          |
//! | ESP32                        | 64 codes    | 2 LEDs                         |
//!
//! The rule uses one RAM block, so it holds even for a channel configured with a larger `memsize`.
//!
//! esp-hal 1.2.2 refills the channel RAM from the transmit's `poll`, not from the interrupt, and the refill is due about 30 µs after the RAM half-empties.
//! The critical-section path removes interrupt and executor latency from that refill path by polling the transmit with interrupts masked.
//! It does not guarantee the waveform: flash-cache misses and other stalls can still delay a refill, as listed under residual risks below.
//! A timeout of about 2 µs per pulse code plus 1 ms bounds the refill polling on every chip, masked or not.
//! It bounds the polling and so the interrupts-off window of a frame that goes wrong.
//! When it expires, [`Error::Transmit`] is returned.
//! In async mode the transmit future is dropped and the channel is stopped, so the frame is cut short.
//! In blocking mode the driver then keeps polling the refill with interrupts live for a second budget of the same size.
//! If the frame ends there, which is likely garbled, [`Error::Transmit`] is returned, the channel is kept and the driver stays usable.
//! If it does not, the transaction is dropped and esp-hal stops the channel, which also drops the channel.
//! That call and every later call return [`Error::ChannelLost`], and the driver must be recreated.
//! An async driver never loses its channel and never returns `ChannelLost`.
//! To rebuild, create the `Rmt` from `peripherals.RMT.reborrow()` and pass the pin with `reborrow()`, so a fresh `Rmt::new` and channel are possible; otherwise only a reset recovers.
//! Rebuilding is not hardware-tested.
//! The one unbounded case is a peripheral that is dead on the classic ESP32, where esp-hal's stop spins until the hardware reaches an end marker.
//! A stopped clock ends the polling only after `LATCH_WAIT_MAX_STALLED_READS` unchanged clock reads, which are taken every 16 polls: about 16000 polls, that is milliseconds, not microseconds.
//!
//! Cost of the critical-section path:
//!
//! - Interrupts are masked for the whole frame, 1.2–1.3 µs per pulse code: roughly 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.
//! - On radio firmware keep the default [`DEFAULT_MAX_NOMINAL_MASKED_US`] ceiling, or lower it, rather than lifting it.
//!   Frames above the ceiling return [`Error::FrameTooLong`]; use `LongFramePolicy::Reject` to refuse every long frame.
//! - Such interrupt-off windows can disturb Wi-Fi and ESP-NOW and overflow UART FIFOs, so test your radio and serial traffic with the real strip length.
//! - On dual-core chips (ESP32 and ESP32-S3), any `critical_section::with` on the other core waits for the whole frame; its interrupts stay live.
//! - The critical-section implementation comes from esp-hal's `rt` feature, so firmware without `rt` fails to link.
//! - In async mode the executor does not run other tasks during a long frame.
//!
//! The blocking methods `set_pixel`, `set_pixels_slice` and `SmartLedsWrite::write` follow the same policy, so firmware can switch between this driver and the ESP-IDF driver, or between blocking and async mode, without changing what a method name means.
//!
//! ## Radio firmware
//!
//! Async firmware with a radio that uses `CriticalSection` should build with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true`.
//! The evidence is an earlier async bench, not commit f53c5e8: on an ESP32-C3 with Wi-Fi active, a downstream bench garbled async 12-LED frames until this setting was enabled.
//! With the radio not started the frames stayed clean; modem sleep did not help while Wi-Fi was active.
//! With the setting enabled the frames stayed clean with the radio on.
//! The blocking masked path (`set_pixels_slice` under the default policy, `SmartLedsWrite::write` and `set_pixels_slice_masked`) benched clean in three runs at commit f53c5e8 without the setting on a downstream ESP32-C3, with 12 LEDs and Wi-Fi active.
//! The setting is still recommended for firmware with a radio as margin.
//! The crate cannot enforce or detect this setting, because it is an esp-hal-internal cfg.
//! `poll_to_completion` and the blocking polling loops are placed in RAM, but the esp-hal poll they call is not, unless the setting is on.
//! RAM placement adds margin; it is not a proven guarantee.
//!
//! ## Blocking driver
//!
//! The blocking methods route frames through the same policy as async mode.
//! With the default `CriticalSection`, `set_pixels_slice` and `SmartLedsWrite::write` wrap a long frame's transmit and bounded refill polling in `critical_section::with`, with the interrupts-off cost above (about 0.36 ms for 12 LEDs).
//! The bounded blocking paths at commit f53c5e8 benched clean downstream on one ESP32-C3 with Wi-Fi active and 12 LEDs.
//! Async, other chips, longer strips, waveform capture and the drop and rebuild path remain unvalidated.
//! `LongFramePolicy::Poll` runs the refill with interrupts live; radio or other interrupts can delay it and corrupt the frame.
//! `set_pixels_slice_masked` masks regardless of the policy.
//! A single LED (25 codes) always needs no refill and never masks interrupts.
//!
//! ## No option is proven under radio load
//!
//! On esp-hal 1.2.2 no option is proven to keep long frames correct under radio load.
//! `CriticalSection` and `set_pixels_slice_masked` mask interrupts for the whole frame, and masking is not a waveform guarantee.
//! `Poll` leaves interrupts live and is expected to garble long frames under radio load.
//! `Reject` keeps every frame within channel RAM, so it never masks interrupts and never needs a refill.
//! The real fix is an ISR-side refill upstream.
//!
//! Residual risks, not yet proven on hardware:
//!
//! - Flash-cache misses can still stall the refill path.
//!   The `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM` setting above is the mitigation.
//! - Shared-resource contention from the radio (bus, SRAM and cache refills) is the working hypothesis for the Wi-Fi failure; it is not measured yet.
//! - On ESP32 and ESP32-S3, the other core can stall the refill path through its flash accesses.
//! - High-priority interrupt levels on Xtensa are not verified to be masked by `critical_section`.
//! - esp-hal's `Instant::now` and its tick conversion are RAM-resident only if they are inlined into the `#[esp_hal::ram]` polling loops; this is not verified.
//! - The poll-gap margin is not measured on hardware yet, and the waveform on the data pin has not been captured.
//!   The `hal_c3_frame_sweep_async` example is the planned capture target.
//!
//! This is a workaround.
//! It can be removed once esp-hal refills the channel RAM from the interrupt.
//! See ADR 009 (`docs/adr/009-long-frame-policy-and-latch.md`) and ADR 010 (`docs/adr/010-interchangeable-driver-semantics.md`) for the policy decision.
//!
//! # Frame spacing (latch)
//!
//! WS2812 LEDs need a low period of at least 280 µs between frames to latch the previous one; the driver enforces this as [`WS2812_RESET_US`].
//! How it is enforced depends on the path:
//!
//! | Path | How the latch is enforced | Cost |
//! |:-----|:--------------------------|:-----|
//! | Async, frame fits in channel RAM | Encoded in the signal: the last slot is a 280 µs low code | The `await` takes about 280 µs longer; no CPU wait |
//! | Async, long frame (`CriticalSection` or `Poll`) | Busy-wait before the next frame | Up to 280 µs, only when frames come faster than that |
//! | Blocking | Busy-wait before the next frame | Up to 280 µs, only when frames come faster than that |
//!
//! Encoding the latch on the long and blocking paths would add 280 µs of interrupts-off or CPU time to every frame, so they keep the busy-wait.
//! On the masked paths the busy-wait runs before the critical section, never inside it.
//! The busy-wait runs with the caller's interrupt state and never opens a critical section of its own.
//! It reads esp-hal's timer, which `esp_hal::init` starts, and that requires the `rt` feature.
//! Safe code cannot build the driver without that initialisation.
//! Only an `unsafe` setup that skips `init` leaves the timer stopped; the wait then gives up after about 1000 unchanged reads instead of hanging.
//! Short async frames are unaffected, because their latch is in the signal.
//!
//! Cancelling an async transmit mid-frame, for example by dropping its future, still schedules the latch wait for the next frame.
//! The next valid frame is therefore never read as a continuation of the cut-short one.
//!
//! # Migration from the pre-0.4 API
//!
//! Before `async` support was added, `Ws2812Rmt` had two type parameters: `<'d, N>`.
//! It now has three: `<'d, Dm, N>` where `Dm` is the driver mode (`Blocking` or `Async`).
//!
//! | Before | After |
//! |:-------|:------|
//! | `Ws2812Rmt<'d, N>` | `Ws2812Rmt<'d, Blocking, N>` or [`Ws2812RmtBlocking<'d, N>`](Ws2812RmtBlocking) |
//! | `Ws2812Rmt::<N>::new(channel)` | `Ws2812Rmt::<_, N>::new(channel)` (infers `Blocking` from channel type) |
//!
//! The simplest migration is to use the [`Ws2812RmtBlocking`] type alias — no other code changes
//! are required:
//!
//! ```ignore
//! use rustyfarian_esp_hal_ws2812::{Ws2812RmtBlocking, buffer_size, RMT_CLK_DIV};
//!
//! const N: usize = buffer_size(12);
//! let mut led: Ws2812RmtBlocking<N> = Ws2812RmtBlocking::new(channel);
//! ```
//!
//! Alternatively, let the compiler infer the driver mode:
//!
//! ```ignore
//! let mut led = Ws2812Rmt::<_, N>::new(channel); // Dm inferred from channel type
//! ```
//!
//! # Future: `SmartLedsWriteAsync`
//!
//! The `smart-leds-trait` ecosystem defines a `SmartLedsWriteAsync` trait for async LED writers.
//! Implementing it on `Ws2812Rmt<'d, Async, N>` is a planned follow-on once the trait
//! stabilises in the ecosystem.
//! See ADR 006 for details.
//!
//! # Runtime Stack
//!
//! The `async` feature gates the async `Ws2812Rmt<'d, Async, N>` API and pulls in no runtime.
//! `critical-section` is a regular dependency of this crate, used by the masked transmit paths (see [Long frames](crate#long-frames)); its implementation comes from esp-hal's `rt` feature.
//! The `embassy` feature implies `async` and pulls in the runtime stack:
//! the `esp-rtos` executor, `embassy-time`, `embassy-executor` and `embassy-sync`.
//! The `*_async` examples require `--features embassy`.
//!
//! The async API is built on `esp-hal`'s native async RMT channel and the `esp-rtos` Embassy executor,
//! which is the standard async runtime for `esp-hal 1.0+`.
//! Other Embassy-compatible executors (e.g., `embassy-executor` with a custom time driver)
//! are theoretically possible but untested; the `RmtTxFuture` in `esp-hal` is executor-agnostic
//! (it uses `core::task::Waker`), but the `embassy-time` timer support requires the
//! `esp-rtos` time driver to be initialised via `esp_rtos::start()`.
//!
//! `esp-idf-hal` (the std path) does not have async RMT support as of `esp-idf-hal 0.46`.
//! `rustyfarian-esp-idf-ws2812` therefore remains blocking-only.
//! If `esp-idf-hal` gains async RMT in a future release, async support can be added there
//! under a separate feature flag without affecting this crate.

use bunting::rgb_to_grb;
#[cfg(feature = "async")]
use core::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll, Waker},
};
use esp_hal::time::{Duration, Instant, Rate};
#[cfg(feature = "async")]
use esp_hal::Async;
use esp_hal::{
    gpio::{interconnect::PeripheralOutput, Level},
    rmt::{Channel, PulseCode, Tx, TxChannelConfig, TxChannelCreator, TxTransaction},
    Blocking,
};
use rgb::RGB8;
use smart_leds_trait::SmartLedsWrite;

/// Clock divider for the RMT peripheral to achieve the required 10 MHz timing clock.
///
/// Only valid for an 80 MHz RMT source rate, the rate passed to `Rmt::new`: divider 8 then yields 10 MHz (100 ns per tick).
/// For another source rate use [`tx_channel_config_for`].
/// Pass this constant to [`TxChannelConfig::with_clk_divider`] when constructing the channel.
pub const RMT_CLK_DIV: u8 = 8;

/// Returns the RMT TX channel configuration the driver requires, for an 80 MHz RMT source rate.
///
/// **Precondition:** the `Rmt` was created with `Rate::from_mhz(80)`.
/// It sets [`RMT_CLK_DIV`] for the 10 MHz timing clock, an idle output held low, and no carrier modulation.
/// With any other source rate the LED timing is wrong; use [`tx_channel_config_for`] instead.
/// Pass it to `TxChannelCreator::configure_tx`, or use [`Ws2812Rmt::from_channel_creator`] to do the whole channel setup.
pub fn tx_channel_config() -> TxChannelConfig {
    channel_config_with_divider(RMT_CLK_DIV)
}

/// Returns the RMT TX channel configuration the driver requires, with the divider derived from the RMT source rate.
///
/// `source` is the rate passed to `Rmt::new`.
/// The divider gives exactly the 10 MHz timing clock, so 80 MHz yields [`RMT_CLK_DIV`] and 40 MHz yields 4.
/// The other settings are those of [`tx_channel_config`].
///
/// # Errors
///
/// - [`Error::RmtConfig`] if no integer divider from 1 to 255 gives exactly 10 MHz, for example for 85 MHz.
///   esp-hal stores the divider in a `u8` without validating it (`rmt.rs:657`, applied at `rmt.rs:1216`),
///   and this function never produces 0.
pub fn tx_channel_config_for(source: Rate) -> Result<TxChannelConfig, Error> {
    let divider = clk_divider_for(source.as_hz()).ok_or(Error::RmtConfig)?;
    Ok(channel_config_with_divider(divider))
}

/// The channel configuration shared by [`tx_channel_config`] and [`tx_channel_config_for`].
fn channel_config_with_divider(divider: u8) -> TxChannelConfig {
    TxChannelConfig::default()
        .with_clk_divider(divider)
        .with_idle_output_level(Level::Low)
        .with_idle_output(true)
        .with_carrier_modulation(false)
}

/// The RMT timing clock the WS2812 constants assume, in hertz.
const TIMING_CLOCK_HZ: u32 = 10_000_000;

/// The divider that turns `source_hz` into exactly [`TIMING_CLOCK_HZ`], if one exists in `1..=255`.
const fn clk_divider_for(source_hz: u32) -> Option<u8> {
    if !source_hz.is_multiple_of(TIMING_CLOCK_HZ) {
        return None;
    }
    let divider = source_hz / TIMING_CLOCK_HZ;
    if divider == 0 || divider > u8::MAX as u32 {
        None
    } else {
        Some(divider as u8)
    }
}

const _: () = {
    assert!(matches!(clk_divider_for(80_000_000), Some(RMT_CLK_DIV)));
    assert!(matches!(clk_divider_for(40_000_000), Some(4)));
    assert!(matches!(clk_divider_for(10_000_000), Some(1)));
    assert!(matches!(clk_divider_for(2_550_000_000), Some(255)));
    assert!(clk_divider_for(85_000_000).is_none());
    assert!(clk_divider_for(0).is_none());
    assert!(clk_divider_for(9_999_999).is_none());
    assert!(clk_divider_for(2_560_000_000).is_none());
};

/// Minimum low time in microseconds between two frames, so the LEDs latch the previous one.
///
/// The WS2812B datasheet asks for at least 280 µs; older WS2812 parts need only 50 µs.
/// The driver waits for the remainder of this interval before it starts a frame
/// that follows the previous one too closely.
/// Async frames that fit in channel RAM carry this interval in the signal.
/// Other frames are followed by a busy-wait, which needs esp-hal's `rt` feature;
/// see [Frame spacing (latch)](crate#frame-spacing-latch).
pub const WS2812_RESET_US: u32 = 280;

/// Default ceiling in microseconds on the estimated nominal masked time of one `LongFramePolicy::CriticalSection` frame.
///
/// The estimate takes 1.3 µs per pulse code.
/// A long frame whose estimate exceeds the ceiling is refused with [`Error::FrameTooLong`].
/// This limits the nominal estimate, not a hard interrupts-off bound:
/// a stalled refill can keep interrupts masked up to the timeout of 2 µs per code plus 1 ms, about 2.5 ms at this default,
/// and the stopped-clock fallback can take longer.
/// 1 ms is about 32 LEDs.
/// Change it with [`Ws2812Rmt::with_max_nominal_masked_us`]; `None` lifts it.
/// See [Long frames](crate#long-frames).
pub const DEFAULT_MAX_NOMINAL_MASKED_US: u32 = 1000;

/// Consecutive unchanged timer reads after which the latch wait treats the timer as stopped.
///
/// A running timer advances by 1 µs every few reads (one read costs roughly 0.2–1 µs),
/// so 1000 identical reads only happen when the time source is not running
/// (firmware built without the esp-hal `rt` feature never starts it).
/// The wait is then skipped after well under a millisecond instead of hanging.
const LATCH_WAIT_MAX_STALLED_READS: u32 = 1000;

/// Ticks of the encoded latch: [`WS2812_RESET_US`] at the 10 MHz RMT tick (10 ticks per µs).
#[cfg(feature = "async")]
const LATCH_TICKS: u16 = (WS2812_RESET_US * 10) as u16;

/// End marker that carries the latch: a low level held for [`LATCH_TICKS`] before the end.
///
/// It replaces the plain end marker in the last slot of frames sent by the async await path.
/// A zero second length makes it an end marker, so the transmit still ends there.
/// The latch is then already on the wire when the frame completes.
/// Only the await path uses it: on the critical-section path it would add 280 µs of interrupts-off time,
/// and on the blocking path it would add 280 µs of CPU time per frame even for spaced frames.
/// Those paths keep a plain [`PulseCode::end_marker`] and wait before the next frame instead.
#[cfg(feature = "async")]
const LATCH_END_CODE: PulseCode = PulseCode::new(Level::Low, LATCH_TICKS, Level::Low, 0);

// WS2812 timing constants at 10 MHz RMT clock (100 ns per tick).
// Based on WS2812B datasheet typical values.
const T0H: u16 = 4; // ~400 ns  (spec: 350 ns ± 150 ns)
const T0L: u16 = 8; // ~800 ns  (spec: 800 ns ± 150 ns)
const T1H: u16 = 7; // ~700 ns  (spec: 700 ns ± 150 ns)
const T1L: u16 = 6; // ~600 ns  (spec: 600 ns ± 150 ns)

/// Returns the required buffer size (in [`PulseCode`]s) for `num_leds` WS2812 LEDs.
///
/// Formula: `num_leds * 24 + 1` — 24 bits of colour data per LED, plus one end-of-stream marker.
///
/// Use this as the const generic `N` for [`Ws2812Rmt`]:
///
/// ```
/// use rustyfarian_esp_hal_ws2812::buffer_size;
/// const N: usize = buffer_size(8); // 8-LED ring → 193
/// assert_eq!(N, 193);
/// ```
pub const fn buffer_size(num_leds: usize) -> usize {
    num_leds * 24 + 1
}

/// How the driver sends a frame that does not fit in one RMT channel's RAM.
///
/// The policy applies to the blocking and the async driver alike, and defaults to [`LongFramePolicy::CriticalSection`].
/// Frames that fit are never refilled and are not affected by the policy.
/// See [Long frames](crate#long-frames).
///
/// # How the driver sends a long frame
///
/// esp-hal refills the channel RAM from a polling loop, not from an interrupt.
/// The refill is due about 30 µs after the RAM half-empties, so every variant that sends the frame busy-polls.
///
/// | Policy | Blocking driver | Async driver |
/// |:-------|:----------------|:-------------|
/// | `CriticalSection` | Latch wait, then a bounded transmit inside a critical section, or [`Error::FrameTooLong`] above the masked-time ceiling | The same; the executor is blocked |
/// | `Poll` | Latch wait, then a bounded transmit with interrupts live | Latch wait, then a bounded poll loop with interrupts live; the executor is blocked |
/// | `Reject` | [`Error::FrameTooLong`] before the latch wait | [`Error::FrameTooLong`] before the latch wait |
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LongFramePolicy {
    /// Busy-poll the transmit inside a critical section, bounded by a timeout.
    ///
    /// All interrupts are blocked for the whole frame, and in async mode the executor is blocked too.
    /// A frame whose estimated nominal masked time exceeds the ceiling ([`DEFAULT_MAX_NOMINAL_MASKED_US`] by default, see [`Ws2812Rmt::with_max_nominal_masked_us`]) returns [`Error::FrameTooLong`] instead.
    /// If the caller already runs with interrupts masked, they stay masked afterwards.
    /// Masking removes interrupt latency from the refill; it does not guarantee the waveform.
    #[default]
    CriticalSection,
    /// Busy-poll the transmit with interrupts live, bounded by a timeout.
    ///
    /// Interrupts, such as radio traffic, can delay the refill past its deadline and garble the frame.
    /// Choose it only when the firmware cannot tolerate interrupts-off windows,
    /// and accept that long frames can be corrupted under radio load.
    /// In async mode the executor is blocked for the whole frame, although interrupts still run.
    Poll,
    /// Return [`Error::FrameTooLong`] without transmitting.
    ///
    /// Choose this when the firmware must never mask interrupts for a frame,
    /// for example because of radio traffic.
    Reject,
}

/// Errors that can occur during WS2812 RMT operations.
///
/// # Error recovery
///
/// [`Error::Transmit`] means this frame failed and the driver is still usable, in both modes.
/// Only [`Error::ChannelLost`] means the driver must be recreated.
///
/// **Blocking** ([`Ws2812Rmt<'d, Blocking, N>`](Ws2812Rmt)):
/// - `Channel::transmit()` hands the channel back when it fails, and the transaction's `wait()` hands it back on every outcome, so the driver is reusable after a failed transmit.
/// - A long frame that times out and then does not end during the drain is dropped, which stops the channel and loses it: that call and every later call return [`Error::ChannelLost`].
///
/// **Async** ([`Ws2812Rmt<'d, Async, N>`](Ws2812Rmt)) — `Channel::transmit()` takes `&mut self`
/// (does not consume the channel), so the driver never loses its channel and remains fully usable after [`Error::Transmit`].
///
/// [`Error::BufferTooSmall`] is always recoverable in both modes: the buffer is never written
/// and no transmission is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// RMT peripheral configuration failed.
    ///
    /// Returned by [`Ws2812Rmt::from_channel_creator`] when esp-hal rejects the channel configuration.
    RmtConfig,
    /// RMT transmission failed, or a long frame timed out.
    ///
    /// Returned in blocking and async mode alike when the polling of a long frame
    /// exceeds its timeout or the clock is found stopped.
    /// This frame failed and the driver is still usable.
    ///
    /// **Blocking mode**: `esp-hal` hands the channel back when `transmit()` fails and on every `wait()` outcome.
    /// After a timeout the driver drains the frame with interrupts live, which is likely garbled.
    /// If the frame still does not end, the driver returns [`Error::ChannelLost`] instead.
    ///
    /// **Async mode**: the channel is never consumed, so the driver is immediately reusable
    /// after this error.
    /// A hardware-level RMT error is very rare.
    Transmit,
    /// The driver lost its RMT channel and must be recreated.
    ///
    /// Blocking mode only: returned by the call whose long frame timed out and did not end during the drain,
    /// because esp-hal then stops the channel and drops it with the transaction.
    /// Every later call on the same driver returns it too.
    /// An async driver never loses its channel and never returns this error.
    ///
    /// To rebuild, create the `Rmt` from `peripherals.RMT.reborrow()` and pass the pin with `reborrow()`, so a fresh `Rmt::new` and channel are possible; otherwise only a reset recovers.
    /// Rebuilding is not hardware-tested.
    ChannelLost,
    /// The pixel count exceeds the buffer capacity `N`.
    ///
    /// Returned synchronously, before any transmission begins (and before any `.await`
    /// in async mode).
    /// The buffer is not modified and the channel remains fully operational.
    ///
    /// Ensure `N >= num_leds * 24 + 1`.
    /// Use [`buffer_size`] to compute the correct `N` at compile time.
    BufferTooSmall,
    /// The frame exceeds one RMT channel's RAM and `LongFramePolicy::Reject` is set.
    ///
    /// Also returned for a `CriticalSection` frame whose estimated nominal masked time exceeds the ceiling
    /// ([`DEFAULT_MAX_NOMINAL_MASKED_US`], see [`Ws2812Rmt::with_max_nominal_masked_us`]).
    ///
    /// Returned before any transmission and before the latch wait, in blocking and async mode alike.
    /// The channel remains fully operational.
    ///
    /// A refused frame sends nothing and is visible only through this error, so log it once rather than on every frame.
    FrameTooLong,
}

impl core::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::RmtConfig => write!(f, "RMT peripheral configuration failed"),
            Error::Transmit => write!(f, "RMT transmission failed"),
            Error::ChannelLost => write!(f, "RMT channel lost; recreate the driver"),
            Error::BufferTooSmall => write!(f, "pixel count exceeds buffer capacity"),
            Error::FrameTooLong => write!(
                f,
                "frame exceeds RMT channel RAM and long frames are rejected by policy"
            ),
        }
    }
}

/// WS2812 LED driver using the `esp-hal` RMT peripheral (bare-metal, `no_std`).
///
/// `N` is the pulse-code buffer size in [`PulseCode`] entries.
/// Compute it with [`buffer_size`]: `N = num_leds * 24 + 1`.
///
/// # Type Parameters
///
/// - `'d` — lifetime of the underlying RMT channel.
/// - `Dm` — driver mode: [`esp_hal::Blocking`] or (with feature `async`) [`esp_hal::Async`].
/// - `N` — pulse-code buffer size (`num_leds * 24 + 1`).
///
/// Use [`Ws2812RmtBlocking`] as a convenience alias for the blocking variant.
///
/// # Timing
///
/// The driver expects the RMT channel to be configured at 10 MHz
/// (80 MHz source rate ÷ [`RMT_CLK_DIV`] = 8, or see [`tx_channel_config_for`] for other rates).
pub struct Ws2812Rmt<'d, Dm: esp_hal::DriverMode, const N: usize> {
    /// The RMT TX channel, wrapped in `Option` to support esp-hal's type-state transmit API
    /// (blocking transmit consumes the channel; wait, or a failed transmit, returns it).
    channel: Option<Channel<'d, Dm, Tx>>,
    /// Pre-allocated pulse-code buffer to avoid runtime allocation.
    buffer: [PulseCode; N],
    /// Earliest instant the next frame may start, used to enforce [`WS2812_RESET_US`].
    next_frame_at: Option<Instant>,
    /// What to do with frames that do not fit in one channel's RAM.
    long_frame_policy: LongFramePolicy,
    /// Ceiling on the estimated nominal masked time of a `CriticalSection` frame; `None` is unlimited.
    max_nominal_masked_us: Option<u32>,
}

/// Type alias for the blocking variant of [`Ws2812Rmt`].
///
/// Existing code that used `Ws2812Rmt<'d, N>` can migrate to this alias
/// without further changes.
pub type Ws2812RmtBlocking<'d, const N: usize> = Ws2812Rmt<'d, Blocking, N>;

/// Shared methods available in both blocking and async modes.
impl<'d, Dm: esp_hal::DriverMode, const N: usize> Ws2812Rmt<'d, Dm, N> {
    /// Creates a new WS2812 driver from a pre-configured RMT TX channel.
    ///
    /// The channel **must** give a 10 MHz tick: [`RMT_CLK_DIV`] (8) on an 80 MHz RMT source rate.
    /// Using a different tick will produce incorrect WS2812 timing.
    /// [`tx_channel_config`] returns a matching configuration for 80 MHz, and [`tx_channel_config_for`] derives one for another source rate.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use esp_hal::{
    ///     rmt::{Rmt, TxChannelCreator},
    ///     time::Rate,
    /// };
    /// use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, tx_channel_config};
    ///
    /// const N: usize = buffer_size(1);
    ///
    /// let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    /// let channel = rmt
    ///     .channel0
    ///     .configure_tx(&tx_channel_config())
    ///     .unwrap()
    ///     .with_pin(peripherals.GPIO8);
    ///
    /// let mut led = Ws2812Rmt::<_, N>::new(channel);
    /// ```
    pub fn new(channel: Channel<'d, Dm, Tx>) -> Self {
        Self {
            channel: Some(channel),
            buffer: [PulseCode::end_marker(); N],
            next_frame_at: None,
            long_frame_policy: LongFramePolicy::default(),
            max_nominal_masked_us: Some(DEFAULT_MAX_NOMINAL_MASKED_US),
        }
    }

    /// Creates a new WS2812 driver from an unconfigured RMT TX channel and an output pin.
    ///
    /// Configures the channel with [`tx_channel_config`] and connects `pin` to it.
    /// **Precondition:** the `Rmt` was created with an 80 MHz source rate, `Rate::from_mhz(80)`; otherwise the LED timing is wrong.
    /// For another source rate configure the channel with [`tx_channel_config_for`] and use [`Ws2812Rmt::new`].
    /// This replaces the channel setup every consumer otherwise copies.
    ///
    /// # Errors
    ///
    /// - [`Error::RmtConfig`] if esp-hal rejects the channel configuration,
    ///   for example because the channel's RAM block is in use by another channel.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use esp_hal::{rmt::Rmt, time::Rate};
    /// use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size};
    ///
    /// const N: usize = buffer_size(1);
    ///
    /// let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    /// let mut led = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO8).unwrap();
    /// ```
    pub fn from_channel_creator<C>(
        creator: C,
        pin: impl PeripheralOutput<'d>,
    ) -> Result<Self, Error>
    where
        C: TxChannelCreator<'d, Dm>,
    {
        let channel = creator
            .configure_tx(&tx_channel_config())
            .map_err(|_| Error::RmtConfig)?
            .with_pin(pin);
        Ok(Self::new(channel))
    }

    /// Sets how frames longer than one RMT channel's RAM are sent.
    ///
    /// The default is [`LongFramePolicy::CriticalSection`].
    /// It applies to blocking and async mode alike; see [Long frames](crate#long-frames).
    #[must_use]
    pub fn with_long_frame_policy(mut self, policy: LongFramePolicy) -> Self {
        self.long_frame_policy = policy;
        self
    }

    /// Returns the current [`LongFramePolicy`].
    pub fn long_frame_policy(&self) -> LongFramePolicy {
        self.long_frame_policy
    }

    /// Sets the ceiling in microseconds on the estimated nominal masked time of one `CriticalSection` frame.
    ///
    /// The default is `Some(`[`DEFAULT_MAX_NOMINAL_MASKED_US`]`)`, about 32 LEDs.
    /// Under [`LongFramePolicy::CriticalSection`], a long frame whose estimated nominal masked time exceeds the ceiling returns [`Error::FrameTooLong`] instead.
    /// The estimate takes the slowest pulse code, `T1H + T1L` = 1.3 µs, for every code of the frame.
    /// `None` lifts the ceiling.
    /// The ceiling limits the nominal estimate, not a hard interrupts-off bound.
    /// A stalled refill can keep interrupts masked up to the timeout of 2 µs per code plus 1 ms, about 2.5 ms at the default,
    /// and the stopped-clock fallback can take longer.
    /// [`LongFramePolicy::Poll`], [`LongFramePolicy::Reject`], frames that fit in channel RAM and [`Ws2812Rmt::set_pixels_slice_masked`] ignore it.
    /// It applies to blocking and async mode alike.
    /// A refused frame sends nothing and is visible only through [`Error::FrameTooLong`], so log it once rather than on every frame.
    #[must_use]
    pub fn with_max_nominal_masked_us(mut self, max_nominal_masked_us: Option<u32>) -> Self {
        self.max_nominal_masked_us = max_nominal_masked_us;
        self
    }

    /// Returns the current ceiling on the estimated nominal masked time of a `CriticalSection` frame, `None` if unlimited.
    pub fn max_nominal_masked_us(&self) -> Option<u32> {
        self.max_nominal_masked_us
    }

    /// Busy-waits until `next_frame_at`, the earliest instant the next frame may start.
    ///
    /// Returns immediately when no frame was sent yet or the instant has passed.
    /// Never runs inside a critical section it opens itself; it restores the caller's interrupt state.
    /// If the caller already runs with interrupts masked, the wait runs masked too.
    /// A stopped timer is detected after [`LATCH_WAIT_MAX_STALLED_READS`] unchanged reads,
    /// and the wait is then skipped.
    fn wait_until_next_frame(&self) {
        let Some(target) = self.next_frame_at else {
            return;
        };
        let target_us = target.duration_since_epoch().as_micros();
        let mut previous_us = Instant::now().duration_since_epoch().as_micros();
        let mut stalled_reads = 0;
        loop {
            let now_us = Instant::now().duration_since_epoch().as_micros();
            if remaining_us(now_us, target_us) == 0 {
                return;
            }
            if now_us == previous_us {
                stalled_reads += 1;
                if stalled_reads >= LATCH_WAIT_MAX_STALLED_READS {
                    return;
                }
            } else {
                stalled_reads = 0;
                previous_us = now_us;
            }
            core::hint::spin_loop();
        }
    }

    /// Encodes one RGB pixel into 24 consecutive [`PulseCode`] slots (GRB bit order, MSB first).
    fn encode_color(rgb: RGB8, buf: &mut [PulseCode]) {
        let grb = rgb_to_grb(rgb);
        debug_assert_eq!(buf.len(), 24);
        for (i, slot) in buf.iter_mut().enumerate() {
            let bit = (grb >> (23 - i)) & 1 != 0;
            *slot = if bit {
                PulseCode::new(Level::High, T1H, Level::Low, T1L)
            } else {
                PulseCode::new(Level::High, T0H, Level::Low, T0L)
            };
        }
    }
}

/// Blocking methods.
impl<'d, const N: usize> Ws2812Rmt<'d, Blocking, N> {
    /// Sets a single LED to the given colour.
    ///
    /// The buffer size `N` must be at least 25 (`buffer_size(1)`).
    ///
    /// A single LED is 25 pulse codes, which fits in one channel's RAM on every supported chip.
    /// It needs no refill, so the [`LongFramePolicy`] does not apply and interrupts stay live.
    ///
    /// Colour is transmitted in WS2812 GRB order.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] if `N < 25`.
    /// - [`Error::Transmit`] if the RMT transmission fails.
    /// - [`Error::ChannelLost`] if the driver lost its channel earlier.
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<(), Error> {
        if N < 25 {
            return Err(Error::BufferTooSmall);
        }
        Self::encode_color(rgb, &mut self.buffer[..24]);
        self.buffer[24] = PulseCode::end_marker();
        self.send(25)
    }

    /// Sets multiple LEDs from a colour slice.
    ///
    /// Colours are transmitted in WS2812 GRB order.
    /// The buffer size `N` must be at least `rgbs.len() * 24 + 1`.
    ///
    /// Frames longer than one RMT channel's RAM (1 LED on ESP32-C3, ESP32-C6 and ESP32-S3, 2 LEDs on ESP32) need a refill while they transmit.
    /// The [`LongFramePolicy`] decides how it runs, as in async mode:
    /// the default [`LongFramePolicy::CriticalSection`] polls the refill with interrupts masked, about 1.2–1.3 µs per pulse code, so a 12-LED frame holds the CPU for about 0.36 ms and a 64-LED frame for about 1.9 ms.
    /// A frame above the masked-time ceiling ([`DEFAULT_MAX_NOMINAL_MASKED_US`], see [`Ws2812Rmt::with_max_nominal_masked_us`]) returns [`Error::FrameTooLong`] instead of masking.
    /// [`LongFramePolicy::Poll`] polls with interrupts live, and [`LongFramePolicy::Reject`] returns [`Error::FrameTooLong`].
    /// Masking interrupts is not a waveform guarantee; see [Long frames](crate#long-frames).
    /// Firmware with a radio should build with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true` as margin; see [Radio firmware](crate#radio-firmware).
    /// [`set_pixels_slice_masked`](Self::set_pixels_slice_masked) masks regardless of the policy and the ceiling.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] if `N < rgbs.len() * 24 + 1`.
    /// - [`Error::FrameTooLong`] if the frame exceeds one channel's RAM and the policy is [`LongFramePolicy::Reject`], or is [`LongFramePolicy::CriticalSection`] with a frame above the masked-time ceiling.
    /// - [`Error::Transmit`] if the RMT transmission fails or a long frame times out and then ends.
    /// - [`Error::ChannelLost`] if a long frame times out and does not end during the drain, or the driver lost its channel earlier.
    pub fn set_pixels_slice(&mut self, rgbs: &[RGB8]) -> Result<(), Error> {
        let needed = self.encode_slice(rgbs)?;
        self.send(needed)
    }

    /// Sets multiple LEDs from a colour slice, always with interrupts masked for the whole frame.
    ///
    /// Same validation, encoding and errors as [`set_pixels_slice`](Self::set_pixels_slice),
    /// except that it masks regardless of the frame length, the [`LongFramePolicy`] and the masked-time ceiling, and never returns [`Error::FrameTooLong`].
    /// Calling it is an explicit request to mask, so the ceiling does not cap it.
    /// The latch wait runs first, outside the critical section.
    /// The transmit and its bounded refill polling then run inside `critical_section::with`,
    /// so interrupts cannot delay the refill: about 1.2–1.3 µs per pulse code,
    /// roughly 0.36 ms for 12 LEDs.
    /// Afterwards the next frame is held back by [`WS2812_RESET_US`].
    ///
    /// The bounded implementation at commit f53c5e8 benched clean downstream on one ESP32-C3 with Wi-Fi active and 12 LEDs.
    /// Async, other chips, longer strips, waveform capture and the drop and rebuild path remain unvalidated.
    /// The masked window can disturb radio and serial traffic on long strips, so test with the real strip length.
    /// Still open: waveform capture, other chips and longer strips.
    /// The critical section needs esp-hal's `rt` feature to link.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] if `N < rgbs.len() * 24 + 1`.
    /// - [`Error::Transmit`] if the RMT transmission fails or the frame times out and then ends.
    /// - [`Error::ChannelLost`] if the frame times out and does not end during the drain, or the driver lost its channel earlier.
    pub fn set_pixels_slice_masked(&mut self, rgbs: &[RGB8]) -> Result<(), Error> {
        let needed = self.encode_slice(rgbs)?;
        self.wait_until_next_frame();
        self.transmit_bounded(needed, true)
    }

    /// Validates and encodes `rgbs` into the buffer and returns the frame length in pulse codes.
    fn encode_slice(&mut self, rgbs: &[RGB8]) -> Result<usize, Error> {
        let num_leds = rgbs.len();
        let needed = num_leds * 24 + 1;
        if needed > N {
            return Err(Error::BufferTooSmall);
        }
        for (i, &rgb) in rgbs.iter().enumerate() {
            Self::encode_color(rgb, &mut self.buffer[i * 24..(i + 1) * 24]);
        }
        self.buffer[num_leds * 24] = PulseCode::end_marker();
        Ok(needed)
    }

    /// Sends the encoded `buffer[..len]` as the [`LongFramePolicy`] prescribes.
    ///
    /// Shared by [`Self::set_pixel`], [`Self::set_pixels_slice`] and the `SmartLedsWrite` impl.
    /// A frame that fits in channel RAM goes through [`Self::do_transmit`] and needs no timeout.
    /// [`LongFramePolicy::Reject`] returns [`Error::FrameTooLong`] before the latch wait.
    /// [`LongFramePolicy::Poll`] and [`LongFramePolicy::CriticalSection`] wait for the latch,
    /// then run [`Self::transmit_bounded`], unmasked and masked respectively.
    fn send(&mut self, len: usize) -> Result<(), Error> {
        match long_frame_action(
            len,
            esp_hal::rmt::CHANNEL_RAM_SIZE,
            self.long_frame_policy,
            self.max_nominal_masked_us,
        ) {
            Action::Direct => self.do_transmit(len),
            Action::Reject => Err(Error::FrameTooLong),
            Action::Poll => {
                self.wait_until_next_frame();
                self.transmit_bounded(len, false)
            }
            Action::CriticalSection => {
                self.wait_until_next_frame();
                self.transmit_bounded(len, true)
            }
        }
    }

    /// Sends a frame that fits in channel RAM via the RMT channel and waits for completion.
    ///
    /// Such a frame needs no refill, so interrupts cannot make it late and no timeout applies.
    ///
    /// Waits until the previous frame's latch is complete first, see [`WS2812_RESET_US`].
    /// The frame ends with a plain end marker, so the latch is enforced after it instead:
    /// `next_frame_at` is set to the end of this attempt plus [`WS2812_RESET_US`].
    /// Encoding the latch into the frame would add 280 µs of CPU time per frame, even for spaced frames.
    ///
    /// Uses `Option<Channel>` to handle esp-hal's ownership-based transmit API:
    /// `transmit()` consumes the channel and either `wait()` or a `transmit()` error returns it.
    fn do_transmit(&mut self, len: usize) -> Result<(), Error> {
        self.wait_until_next_frame();
        let ch = self.channel.take().ok_or(Error::ChannelLost)?;
        self.terminate_frame(len);
        let result = match ch.transmit(&self.buffer[..len]) {
            Ok(txn) => match txn.wait() {
                Ok(ch_back) => {
                    self.channel = Some(ch_back);
                    Ok(())
                }
                Err((_, ch_back)) => {
                    self.channel = Some(ch_back);
                    Err(Error::Transmit)
                }
            },
            Err((_, ch_back)) => {
                self.channel = Some(ch_back);
                Err(Error::Transmit)
            }
        };
        self.next_frame_at = Some(latch_deadline());
        result
    }

    /// Writes the plain end marker into the last slot of the frame.
    fn terminate_frame(&mut self, len: usize) {
        if let Some(last) = len.checked_sub(1).and_then(|i| self.buffer.get_mut(i)) {
            *last = PulseCode::end_marker();
        }
    }

    /// Transmits a long `buffer[..len]` with a bounded refill poll, without the latch wait.
    ///
    /// The refill polling is bounded by [`tx_timeout_us`], see [`start_and_poll`].
    /// With `masked` set, the transmit start and the polling run inside `critical_section::with`.
    ///
    /// If the first budget is spent before the frame ends, the section is left and [`drain`] polls the refill with interrupts live for a second budget.
    /// A slow but working frame still ends there, and `wait()` then returns immediately and hands the channel back.
    /// A frame that still has not ended is dropped: esp-hal's `TxGuard` stops the channel
    /// (immediately on ESP32-C3, ESP32-C6 and ESP32-S3; on the classic ESP32 it fills in an end marker and spins until the hardware reaches it).
    /// Dropping the transaction also drops the RMT channel with its pin connection and RAM block,
    /// so the driver keeps no channel: this call and every later call return [`Error::ChannelLost`]; see [`Error::ChannelLost`] for how to rebuild.
    /// A peripheral that is dead on the classic ESP32 can still spin in that `TxGuard::drop`.
    /// The critical section restores the caller's interrupt state:
    /// if the caller already runs with interrupts masked, the drain and the drop run masked too.
    ///
    /// A timeout or a stopped timer returns [`Error::Transmit`] when the frame ends during the drain, and [`Error::ChannelLost`] otherwise.
    /// `next_frame_at` is set on every exit.
    fn transmit_bounded(&mut self, len: usize, masked: bool) -> Result<(), Error> {
        let ch = self.channel.take().ok_or(Error::ChannelLost)?;
        self.terminate_frame(len);
        let data = &self.buffer[..len];
        let started = if masked {
            critical_section::with(|_| start_and_poll(ch, data))
        } else {
            start_and_poll(ch, data)
        };
        let (mut txn, completed) = match started {
            Ok(started) => started,
            Err(ch_back) => {
                self.channel = Some(ch_back);
                self.next_frame_at = Some(latch_deadline());
                return Err(Error::Transmit);
            }
        };
        let ended = completed || drain(&mut txn, len);
        let result = if ended {
            // wait() consumes `txn`, releasing the borrow on `self.buffer`; it returns immediately
            let outcome = match txn.wait() {
                Ok(ch_back) => (ch_back, completed),
                Err((_, ch_back)) => (ch_back, false),
            };
            self.channel = Some(outcome.0);
            if outcome.1 {
                Ok(())
            } else {
                Err(Error::Transmit)
            }
        } else {
            // Dropping `txn` stops the channel; the channel inside it is lost with it.
            drop(txn);
            Err(Error::ChannelLost)
        };
        self.next_frame_at = Some(latch_deadline());
        result
    }
}

/// Async methods (requires feature `async`).
///
/// Unlike the blocking variant, the async `Channel::transmit()` takes `&mut self` and returns
/// a `Future` directly — no channel ownership transfer occurs.
#[cfg(feature = "async")]
impl<'d, const N: usize> Ws2812Rmt<'d, Async, N> {
    /// Sets a single LED to the given colour, yielding to the executor during transmission.
    ///
    /// The buffer size `N` must be at least 25 (`buffer_size(1)`).
    ///
    /// A single LED is 25 pulse codes, which fits in one channel's RAM on every supported chip, so this method stays truly async.
    /// The latch after this LED is carried in the signal, so the call completes only once the LEDs have latched.
    /// A call that follows a blocking or long-frame transmit may first busy-wait the rest of the latch interval; see [Frame spacing (latch)](crate#frame-spacing-latch).
    ///
    /// Colour is transmitted in WS2812 GRB order.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] — returned immediately, **before** any `.await`, if `N < 25`.
    ///   The buffer is not modified and the driver remains fully operational.
    /// - [`Error::Transmit`] — returned after `.await` completes if the RMT hardware signals
    ///   an error.
    ///   Because the async channel is never consumed, the driver is reusable after this error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// match ws.set_pixel(RGB8::new(255, 0, 0)).await {
    ///     Ok(()) => {}
    ///     Err(Error::BufferTooSmall) => {
    ///         // N is too small for even one LED — fix the const N at compile time.
    ///         panic!("buffer too small");
    ///     }
    ///     Err(Error::Transmit) => {
    ///         // Hardware error — driver is still usable; retry or log.
    ///         log_error();
    ///     }
    ///     Err(_) => {
    ///         // `Error` is non-exhaustive; a single LED never yields `FrameTooLong`.
    ///         log_error();
    ///     }
    /// }
    /// ```
    pub async fn set_pixel(&mut self, rgb: RGB8) -> Result<(), Error> {
        if N < 25 {
            return Err(Error::BufferTooSmall);
        }
        Self::encode_color(rgb, &mut self.buffer[..24]);
        self.buffer[24] = PulseCode::end_marker();
        self.do_transmit_async(25).await
    }

    /// Sets multiple LEDs from a colour slice, yielding to the executor during transmission.
    ///
    /// Colours are transmitted in WS2812 GRB order.
    /// The buffer size `N` must be at least `rgbs.len() * 24 + 1`.
    ///
    /// Frames longer than one RMT channel's RAM (1 LED on ESP32-C3, ESP32-C6 and ESP32-S3, 2 LEDs on ESP32) do not yield.
    /// With the default [`LongFramePolicy::CriticalSection`] they busy-poll with interrupts masked, about 1.2–1.3 µs per pulse code, so a 12-LED frame holds the CPU for about 360 µs.
    /// Async firmware with a radio should build with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true` for this path, based on an earlier async bench; see [Radio firmware](crate#radio-firmware).
    /// With [`LongFramePolicy::Poll`] they busy-poll with interrupts live and still block the executor.
    /// A frame above the masked-time ceiling ([`DEFAULT_MAX_NOMINAL_MASKED_US`], see [`Ws2812Rmt::with_max_nominal_masked_us`]) returns [`Error::FrameTooLong`] instead.
    /// With [`LongFramePolicy::Reject`] they return [`Error::FrameTooLong`] instead.
    /// See [Long frames](crate#long-frames).
    /// A call that follows a blocking or long-frame transmit may first busy-wait the rest of the latch interval; see [Frame spacing (latch)](crate#frame-spacing-latch).
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] — returned immediately, **before** any `.await`, if
    ///   `N < rgbs.len() * 24 + 1`.
    ///   No data is written to the buffer and the channel remains fully operational.
    ///   Fix: use [`buffer_size`]`(num_leds)` to size `N` correctly at compile time,
    ///   or ensure the slice length does not exceed `(N - 1) / 24`.
    /// - [`Error::FrameTooLong`] — returned immediately, before any transmission, if the frame
    ///   exceeds one channel's RAM and the policy is [`LongFramePolicy::Reject`].
    /// - [`Error::Transmit`] — returned after `.await` completes if the RMT hardware signals
    ///   an error (very rare), or if a long frame times out.
    ///   An async driver never loses its channel, so it never returns [`Error::ChannelLost`].
    ///   Because the async channel is never consumed, the driver is reusable after this error.
    ///
    /// # Consecutive calls
    ///
    /// The returned future completes only after the RMT peripheral has sent all pulses,
    /// so calls never overlap and no explicit queuing is needed.
    /// Back-to-back calls are spaced by the latch interval; see [`Frame spacing (latch)`](crate#frame-spacing-latch).
    /// A cancelled call still schedules the latch wait for the next frame.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let colors = [RGB8::new(255, 0, 0); 12];
    /// match ws.set_pixels_slice(&colors).await {
    ///     Ok(()) => {}
    ///     Err(Error::BufferTooSmall) => {
    ///         // Slice is longer than N can hold — fix N or shorten the slice.
    ///         panic!("buffer too small: N={N}, needed={}", colors.len() * 24 + 1);
    ///     }
    ///     Err(Error::FrameTooLong) => {
    ///         // Only with `LongFramePolicy::Reject`: the frame needs a refill.
    ///         // Shorten the frame or choose another policy.
    ///         log_error();
    ///     }
    ///     Err(Error::Transmit) => {
    ///         // Hardware error — driver is still usable; retry or log.
    ///         log_error();
    ///     }
    ///     Err(_) => {
    ///         // `Error` is non-exhaustive.
    ///         log_error();
    ///     }
    /// }
    /// ```
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

    /// Sends `buffer[..len]` via the async RMT channel, awaiting completion.
    ///
    /// The async `Channel::transmit()` takes `&mut self` (does not consume the channel),
    /// so no `Option` dance is needed here.
    ///
    /// Frames that fit in one channel's RAM are truly async.
    /// For longer frames the [`LongFramePolicy`] decides, see [`long_frame_action`]:
    /// they are rejected, or driven to completion by [`Self::transmit_in_critical_section`].
    ///
    /// [`Error::FrameTooLong`] is returned before the latch wait.
    /// Otherwise [`Self::wait_until_next_frame`] runs first.
    ///
    /// The await path replaces the end marker in `buffer[len - 1]` with [`LATCH_END_CODE`],
    /// so the latch is on the wire when the frame completes.
    /// The critical-section path writes a plain end marker and relies on `next_frame_at`,
    /// because an encoded latch would add 280 µs of interrupts-off time.
    ///
    /// A [`NextFrameStamp`] is created before the transmit future and dropped after it.
    /// It records `next_frame_at` on every exit, including when this future is dropped mid-frame:
    /// esp-hal then stops the channel, the frame is cut short, and the next frame waits [`WS2812_RESET_US`].
    async fn do_transmit_async(&mut self, len: usize) -> Result<(), Error> {
        let action = long_frame_action(
            len,
            esp_hal::rmt::CHANNEL_RAM_SIZE,
            self.long_frame_policy,
            self.max_nominal_masked_us,
        );
        if matches!(action, Action::Reject) {
            return Err(Error::FrameTooLong);
        }
        if self.channel.is_none() {
            return Err(Error::ChannelLost);
        }
        self.wait_until_next_frame();
        let Self {
            channel,
            buffer,
            next_frame_at,
            ..
        } = self;
        let ch = channel.as_mut().ok_or(Error::ChannelLost)?;
        if let Some(last) = len.checked_sub(1).and_then(|i| buffer.get_mut(i)) {
            *last = if matches!(action, Action::Direct) {
                LATCH_END_CODE
            } else {
                PulseCode::end_marker()
            };
        }
        let mut stamp = NextFrameStamp {
            slot: next_frame_at,
            latch_on_wire: false,
        };
        let data = &buffer[..len];
        match action {
            Action::Direct => {
                // Named local declared after `stamp`: locals drop in reverse order, so on
                // cancellation the transmit (and the channel stop) ends before `stamp` records.
                let mut transmit = pin!(ch.transmit(data));
                transmit.as_mut().await.map_err(|_| Error::Transmit)?;
                stamp.latch_on_wire = true;
                Ok(())
            }
            Action::Poll => Self::transmit_polling(ch, data, false),
            Action::CriticalSection => Self::transmit_polling(ch, data, true),
            Action::Reject => Err(Error::FrameTooLong),
        }
    }

    /// Drives a transmit that needs RAM refills to completion, with interrupts masked or live.
    ///
    /// esp-hal refills the channel RAM from the future's `poll`, so the future is polled with a
    /// no-op waker, bounded by [`tx_timeout_us`].
    /// A stopped timer (see [`LATCH_WAIT_MAX_STALLED_READS`]) also ends the loop with [`Error::Transmit`],
    /// so a frozen clock cannot keep interrupts masked.
    /// Either way the executor runs no other task until the frame ends.
    ///
    /// With `masked` set, the loop runs inside `critical_section::with`.
    /// Otherwise it runs with the caller's interrupt state, which is the [`LongFramePolicy::Poll`] behaviour.
    ///
    /// The future is stored in `slot`, declared before the critical section.
    /// On timeout the closure returns an error and the unfinished future is dropped only after
    /// the critical section has ended, when `slot` goes out of scope.
    /// Dropping it stops the channel, and on the classic ESP32 (no immediate TX stop) esp-hal's
    /// `TxGuard::drop` spins until the hardware reaches the end marker.
    /// That spin runs after the critical section, so the timeout bounds the interrupts-off time
    /// on every chip.
    /// The critical section restores the caller's interrupt state:
    /// if the caller already runs with interrupts masked (inside a critical section or a high-priority interrupt executor),
    /// the cleanup spin runs masked too.
    fn transmit_polling(
        ch: &mut Channel<'d, Async, Tx>,
        data: &[PulseCode],
        masked: bool,
    ) -> Result<(), Error> {
        let mut slot = pin!(None);
        // A macro rather than a closure: a closure cannot store a borrow of `ch` in the outer `slot`.
        macro_rules! run {
            () => {{
                let budget = PollBudget::start(data.len());
                slot.set(Some(ch.transmit(data)));
                let Some(fut) = slot.as_mut().as_pin_mut() else {
                    return Err(Error::Transmit);
                };
                poll_to_completion(fut, budget)
            }};
        }
        if masked {
            critical_section::with(|_| run!())
        } else {
            run!()
        }
    }
}

/// Records `next_frame_at` when dropped, so every exit of an async transmit enforces the latch.
///
/// With `latch_on_wire` set, the frame carried its latch ([`LATCH_END_CODE`]) and completed,
/// so the next frame may start now.
/// Otherwise the next frame may start [`WS2812_RESET_US`] from now.
#[cfg(feature = "async")]
struct NextFrameStamp<'a> {
    slot: &'a mut Option<Instant>,
    latch_on_wire: bool,
}

#[cfg(feature = "async")]
impl Drop for NextFrameStamp<'_> {
    fn drop(&mut self) {
        *self.slot = Some(if self.latch_on_wire {
            Instant::now()
        } else {
            latch_deadline()
        });
    }
}

/// Polls between two clock reads in the long-frame loop.
///
/// A poll costs well under a microsecond, so reading the clock every poll would slow the refill loop.
/// Sixteen polls bound the timeout overshoot to a few microseconds.
const POLLS_PER_CLOCK_READ: u32 = 16;

/// Deadline and stopped-timer bookkeeping shared by the refill polling loops.
struct PollBudget {
    deadline_us: u64,
    polls: u32,
    previous_us: u64,
    stalled_reads: u32,
}

impl PollBudget {
    /// Starts a budget that expires [`tx_timeout_us`] for `len` pulse codes from now.
    #[inline(always)]
    fn start(len: usize) -> Self {
        Self {
            deadline_us: Instant::now().duration_since_epoch().as_micros() + tx_timeout_us(len),
            polls: 0,
            previous_us: 0,
            stalled_reads: 0,
        }
    }

    /// Counts one poll and returns `true` once the budget is spent.
    ///
    /// The clock is read every [`POLLS_PER_CLOCK_READ`] polls.
    /// The budget is spent at the deadline, or after [`LATCH_WAIT_MAX_STALLED_READS`] unchanged clock reads.
    /// Stalls are only counted at clock reads, so a stopped clock ends the loop after about 16000 polls, which takes milliseconds.
    #[inline(always)]
    fn spent(&mut self) -> bool {
        self.polls = self.polls.wrapping_add(1);
        if !self.polls.is_multiple_of(POLLS_PER_CLOCK_READ) {
            return false;
        }
        let now_us = Instant::now().duration_since_epoch().as_micros();
        if now_us >= self.deadline_us {
            return true;
        }
        if now_us == self.previous_us {
            self.stalled_reads += 1;
            self.stalled_reads >= LATCH_WAIT_MAX_STALLED_READS
        } else {
            self.stalled_reads = 0;
            self.previous_us = now_us;
            false
        }
    }
}

/// Polls `fut` to completion with a no-op waker, bounded by `budget`.
///
/// This is the loop that runs inside the critical section of a long async frame.
/// It is deliberately not generic: the future is type-erased, so the function is a single
/// instance that `#[esp_hal::ram]` places in RAM.
/// Executing it from flash would let a flash-cache miss stall the refill past its deadline.
/// The poll of the esp-hal future itself is placed by esp-hal's own `place-rmt-driver-in-ram` option.
///
/// A spent budget returns [`Error::Transmit`].
#[cfg(feature = "async")]
#[esp_hal::ram]
fn poll_to_completion(
    mut fut: Pin<&mut dyn Future<Output = Result<(), esp_hal::rmt::Error>>>,
    mut budget: PollBudget,
) -> Result<(), Error> {
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(result) = fut.as_mut().poll(&mut cx) {
            return result.map_err(|_| Error::Transmit);
        }
        if budget.spent() {
            return Err(Error::Transmit);
        }
    }
}

/// Starts a blocking transmit of `data` and polls its refill until it ends or its timeout expires.
///
/// Returns the transaction and whether it ended, or the channel back if the transmit could not start.
/// A transaction that did not end is still running; the caller must drain it, `wait()` for it or drop it.
///
/// This is the loop that runs inside the critical section of a masked blocking frame.
/// It is deliberately not generic, so `#[esp_hal::ram]` places a single instance in RAM;
/// see [`poll_to_completion`] for why, and for the part esp-hal places itself.
#[esp_hal::ram]
fn start_and_poll<'ch, 'data>(
    channel: Channel<'ch, Blocking, Tx>,
    data: &'data [PulseCode],
) -> Result<(TxTransaction<'ch, 'data>, bool), Channel<'ch, Blocking, Tx>> {
    let budget = PollBudget::start(data.len());
    let mut txn = channel.transmit(data).map_err(|(_, channel)| channel)?;
    let ended = poll_until_ended(&mut txn, budget);
    Ok((txn, ended))
}

/// Polls `txn` unmasked for a second budget of [`tx_timeout_us`] for `len` pulse codes, and returns whether it ended.
///
/// This is the recovery loop after [`start_and_poll`] ran out of budget.
/// Refills continue while it polls, so a slow but working frame still ends.
#[esp_hal::ram]
fn drain(txn: &mut TxTransaction<'_, '_>, len: usize) -> bool {
    poll_until_ended(txn, PollBudget::start(len))
}

/// Polls `txn` until it ends, returning `true`, or until `budget` is spent, returning `false`.
#[esp_hal::ram]
fn poll_until_ended(txn: &mut TxTransaction<'_, '_>, mut budget: PollBudget) -> bool {
    loop {
        if txn.poll() {
            return true;
        }
        if budget.spent() {
            return false;
        }
    }
}

/// What the driver does with a frame of a given length.
enum Action {
    /// The frame fits in channel RAM: no refill is needed, so the transmit is sent directly.
    Direct,
    /// Long frame: busy-poll the refill with interrupts live.
    Poll,
    /// Long frame: busy-poll the refill inside a critical section.
    CriticalSection,
    /// Long frame: refuse with [`Error::FrameTooLong`].
    Reject,
}

/// Decides how to send `len` pulse codes given `ram` codes of channel RAM, the policy and the masked-time ceiling.
///
/// A `CriticalSection` frame whose [`masked_us`] exceeds `max_nominal_masked_us` is rejected.
/// `Poll`, `Reject` and frames that fit in channel RAM ignore the ceiling.
const fn long_frame_action(
    len: usize,
    ram: usize,
    policy: LongFramePolicy,
    max_nominal_masked_us: Option<u32>,
) -> Action {
    if fits_channel_ram(len, ram) {
        return Action::Direct;
    }
    match policy {
        LongFramePolicy::Reject => Action::Reject,
        LongFramePolicy::Poll => Action::Poll,
        LongFramePolicy::CriticalSection => match max_nominal_masked_us {
            Some(ceiling) if masked_us(len) > ceiling as u64 => Action::Reject,
            _ => Action::CriticalSection,
        },
    }
}

/// RMT ticks per microsecond at the 10 MHz timing clock set by [`RMT_CLK_DIV`].
const TICKS_PER_US: u64 = 10;

/// Worst-case interrupts-off time in microseconds of a masked frame of `len` pulse codes, rounded up.
///
/// Takes the slowest code, `T1H + T1L` ticks, for every code.
const fn masked_us(len: usize) -> u64 {
    (len as u64 * (T1H + T1L) as u64).div_ceil(TICKS_PER_US)
}

/// The instant [`WS2812_RESET_US`] from now.
fn latch_deadline() -> Instant {
    Instant::now() + Duration::from_micros(WS2812_RESET_US as u64)
}

/// Microseconds left until `target_us`, given the current time `now_us`.
const fn remaining_us(now_us: u64, target_us: u64) -> u64 {
    target_us.saturating_sub(now_us)
}

const _: () = {
    assert!(remaining_us(1000, 1280) == 280);
    assert!(remaining_us(1279, 1280) == 1);
    assert!(remaining_us(1280, 1280) == 0);
    assert!(remaining_us(1500, 1280) == 0);
    assert!(WS2812_RESET_US * 10 <= PulseCode::MAX_LEN as u32);
};

#[cfg(feature = "async")]
const _: () = assert!(LATCH_END_CODE.is_end_marker());

const _: () = {
    const POLICIES: [LongFramePolicy; 3] = [
        LongFramePolicy::CriticalSection,
        LongFramePolicy::Poll,
        LongFramePolicy::Reject,
    ];
    let mut i = 0;
    while i < POLICIES.len() {
        let policy = POLICIES[i];
        // A single LED (25 codes) fits on every chip, whatever the policy.
        assert!(matches!(
            long_frame_action(25, 48, policy, None),
            Action::Direct
        ));
        assert!(matches!(
            long_frame_action(25, 64, policy, None),
            Action::Direct
        ));
        // Boundaries: 48 codes (C3, C6, S3) and 64 codes (ESP32).
        assert!(matches!(
            long_frame_action(48, 48, policy, None),
            Action::Direct
        ));
        assert!(matches!(
            long_frame_action(64, 64, policy, None),
            Action::Direct
        ));
        assert!(!matches!(
            long_frame_action(49, 48, policy, None),
            Action::Direct
        ));
        assert!(!matches!(
            long_frame_action(65, 64, policy, None),
            Action::Direct
        ));
        i += 1;
    }
    assert!(matches!(
        long_frame_action(49, 48, LongFramePolicy::CriticalSection, None),
        Action::CriticalSection
    ));
    assert!(matches!(
        long_frame_action(65, 64, LongFramePolicy::CriticalSection, None),
        Action::CriticalSection
    ));
    assert!(matches!(
        long_frame_action(49, 48, LongFramePolicy::Poll, None),
        Action::Poll
    ));
    assert!(matches!(
        long_frame_action(65, 64, LongFramePolicy::Poll, None),
        Action::Poll
    ));
    assert!(matches!(
        long_frame_action(289, 48, LongFramePolicy::Reject, None),
        Action::Reject
    ));
    assert!(matches!(
        long_frame_action(65, 64, LongFramePolicy::Reject, None),
        Action::Reject
    ));
    assert!(DEFAULT_MAX_NOMINAL_MASKED_US == 1000);
    assert!(masked_us(0) == 0);
    assert!(masked_us(1) == 2);
    assert!(masked_us(10) == 13);
    assert!(masked_us(289) == 376);
    assert!(masked_us(769) == 1000);
    assert!(masked_us(770) == 1001);
    // Ceiling routing: exactly at the ceiling passes, one code above is rejected.
    assert!(matches!(
        long_frame_action(769, 48, LongFramePolicy::CriticalSection, Some(1000)),
        Action::CriticalSection
    ));
    assert!(matches!(
        long_frame_action(770, 48, LongFramePolicy::CriticalSection, Some(1000)),
        Action::Reject
    ));
    assert!(matches!(
        long_frame_action(289, 48, LongFramePolicy::CriticalSection, Some(1000)),
        Action::CriticalSection
    ));
    // `None` lifts the ceiling.
    assert!(matches!(
        long_frame_action(1537, 48, LongFramePolicy::CriticalSection, None),
        Action::CriticalSection
    ));
    // `Poll`, `Reject` and `Direct` ignore the ceiling.
    assert!(matches!(
        long_frame_action(770, 48, LongFramePolicy::Poll, Some(1000)),
        Action::Poll
    ));
    assert!(matches!(
        long_frame_action(770, 48, LongFramePolicy::Reject, Some(1000)),
        Action::Reject
    ));
    assert!(matches!(
        long_frame_action(25, 48, LongFramePolicy::CriticalSection, Some(0)),
        Action::Direct
    ));
};

/// Returns `true` if `len` pulse codes fit in `ram` codes of one RMT channel's RAM,
/// so that no refill is needed during transmission.
const fn fits_channel_ram(len: usize, ram: usize) -> bool {
    len <= ram
}

/// Timeout budget per pulse code, in microseconds.
///
/// About 1.5 times the worst-case code time of 1.3 µs (`T1H + T1L` = 13 ticks at 100 ns).
/// Assumes the 10 MHz RMT tick set by [`RMT_CLK_DIV`]; a slower tick makes long frames time out
/// with [`Error::Transmit`].
const TIMEOUT_US_PER_CODE: u64 = 2;

/// Fixed timeout margin in microseconds, covering transmit start-up and polling overhead.
const TIMEOUT_MARGIN_US: u64 = 1000;

/// Upper bound in microseconds for a refill-driven transmit of `len` pulse codes.
const fn tx_timeout_us(len: usize) -> u64 {
    len as u64 * TIMEOUT_US_PER_CODE + TIMEOUT_MARGIN_US
}

const _: () = {
    assert!(fits_channel_ram(25, 48));
    assert!(!fits_channel_ram(49, 48));
    assert!(fits_channel_ram(49, 64));
    assert!(!fits_channel_ram(289, 48));
    assert!(tx_timeout_us(289) == 1578);
    // At 100 ns per tick, 10 ticks make 1 µs: the per-code budget must exceed the longest code.
    assert!(TIMEOUT_US_PER_CODE * 10 > (T0H + T0L) as u64);
    assert!(TIMEOUT_US_PER_CODE * 10 > (T1H + T1L) as u64);
};

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;

    // --- buffer_size tests ---------------------------------------------------

    #[test]
    fn buffer_size_formula() {
        // Verify the documented formula: num_leds * 24 + 1
        // Includes edge cases: 0 LEDs (end-marker only), common ring sizes.
        for n in [0usize, 1, 4, 8, 12, 16, 60] {
            assert_eq!(buffer_size(n), n * 24 + 1, "buffer_size({n}) mismatch");
        }
    }

    // --- max_leds capacity edge case -----------------------------------------

    #[test]
    fn max_leds_formula_n1_yields_zero() {
        // N=1: only the end-of-stream marker fits; no LED data can be stored.
        let max = (1usize.saturating_sub(1)) / 24;
        assert_eq!(max, 0);
    }

    // --- Error Display tests -------------------------------------------------

    #[test]
    fn error_display_rmt_config_message() {
        assert_eq!(
            Error::RmtConfig.to_string(),
            "RMT peripheral configuration failed"
        );
    }

    #[test]
    fn error_display_channel_lost_message() {
        assert_eq!(
            Error::ChannelLost.to_string(),
            "RMT channel lost; recreate the driver"
        );
    }

    #[test]
    fn error_display_transmit_message() {
        assert_eq!(Error::Transmit.to_string(), "RMT transmission failed");
    }

    #[test]
    fn error_display_buffer_too_small_message() {
        assert_eq!(
            Error::BufferTooSmall.to_string(),
            "pixel count exceeds buffer capacity"
        );
    }

    // --- Error derive trait tests --------------------------------------------
    //
    // These verify the PartialEq, Clone, and Debug derives, which matter for
    // callers that match on or log errors — including async callers where the
    // error is returned from an .await expression.

    #[test]
    fn error_partial_eq_same_variants() {
        assert_eq!(Error::BufferTooSmall, Error::BufferTooSmall);
        assert_eq!(Error::Transmit, Error::Transmit);
        assert_eq!(Error::RmtConfig, Error::RmtConfig);
    }

    #[test]
    fn error_partial_eq_different_variants() {
        assert_ne!(Error::BufferTooSmall, Error::Transmit);
        assert_ne!(Error::Transmit, Error::RmtConfig);
        assert_ne!(Error::BufferTooSmall, Error::RmtConfig);
    }

    #[test]
    fn error_clone_produces_equal_value() {
        assert_eq!(Error::BufferTooSmall.clone(), Error::BufferTooSmall);
        assert_eq!(Error::Transmit.clone(), Error::Transmit);
        assert_eq!(Error::RmtConfig.clone(), Error::RmtConfig);
    }

    #[test]
    fn error_debug_contains_variant_name() {
        // Debug output is used when logging errors from async tasks; verify
        // each variant formats recognisably.
        let s = std::format!("{:?}", Error::BufferTooSmall);
        assert!(s.contains("BufferTooSmall"), "got: {s}");

        let s = std::format!("{:?}", Error::Transmit);
        assert!(s.contains("Transmit"), "got: {s}");

        let s = std::format!("{:?}", Error::RmtConfig);
        assert!(s.contains("RmtConfig"), "got: {s}");
    }

    // --- Policy and error trait tests ----------------------------------------

    #[test]
    fn long_frame_policy_defaults_to_critical_section() {
        assert_eq!(LongFramePolicy::default(), LongFramePolicy::CriticalSection);
    }

    #[test]
    fn error_is_usable_as_core_error() {
        let err: &dyn core::error::Error = &Error::FrameTooLong;
        assert_eq!(
            err.to_string(),
            "frame exceeds RMT channel RAM and long frames are rejected by policy"
        );
    }

    // --- BufferTooSmall guard boundary tests ---------------------------------
    //
    // The guard `needed > N` is evaluated synchronously, before any hardware
    // interaction (and before any .await in async mode).
    // Verify the boundary arithmetic that drives it.

    #[test]
    fn buffer_too_small_boundary_single_led() {
        // buffer_size(1) = 25; N=24 is one slot short.
        let needed_for_one: usize = 1 * 24 + 1; // 25
        assert!(needed_for_one > 24, "N=24 must trigger BufferTooSmall");
        assert!(
            !(needed_for_one > 25),
            "N=25 must NOT trigger BufferTooSmall"
        );
    }

    #[test]
    fn buffer_too_small_boundary_twelve_leds() {
        // buffer_size(12) = 289; N=288 is one slot short.
        let needed: usize = 12 * 24 + 1; // 289
        assert!(needed > 288, "N=288 must trigger BufferTooSmall");
        assert!(!(needed > 289), "N=289 must NOT trigger BufferTooSmall");
    }

    #[test]
    fn buffer_too_small_empty_slice_never_triggers() {
        // Empty slice: needed = 0 * 24 + 1 = 1; fits in any buffer (min N=1 for end marker).
        let needed: usize = 0 * 24 + 1; // 1
        assert!(
            !(needed > 1),
            "empty slice must never trigger BufferTooSmall"
        );
    }
}

#[cfg(feature = "pennant")]
impl<'d, const N: usize> pennant::StatusLed for Ws2812Rmt<'d, Blocking, N> {
    type Error = Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

/// `SmartLedsWrite` implementation for [`Ws2812Rmt`].
///
/// Allows the driver to be used with any crate in the `smart-leds` ecosystem
/// (e.g. `smart-leds`, brightness adapters, gamma correction).
///
/// The frame is sent like [`Ws2812Rmt::set_pixels_slice`]: frames longer than one channel's RAM follow the [`LongFramePolicy`].
/// With the default [`LongFramePolicy::CriticalSection`] the transmit runs with interrupts masked, about 1.25 µs per pulse code:
/// roughly 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.
/// The call returns [`Error::FrameTooLong`] under [`LongFramePolicy::Reject`] or above the masked-time ceiling, and [`Error::Transmit`] if a long frame times out and then ends.
/// It returns [`Error::ChannelLost`] if the frame does not end during the drain and the channel is lost.
///
/// The iterator is drained directly into the pre-allocated pulse-code buffer —
/// no heap allocation occurs. If the iterator yields more colours than the buffer
/// can hold (`(N - 1) / 24` LEDs), transmission is aborted and
/// [`Error::BufferTooSmall`] is returned before any data is sent.
///
/// If the iterator is empty, `Ok(())` is returned immediately — no reset pulse
/// or blank frame is sent. Hardware that requires an explicit blank to turn off
/// LEDs should send a zeroed colour slice instead.
///
/// # Example
///
/// ```ignore
/// use smart_leds_trait::{SmartLedsWrite, RGB8};
///
/// let colors = [RGB8 { r: 255, g: 0, b: 0 }; 8];
/// led.write(colors.iter().cloned()).unwrap();
/// ```
impl<'d, const N: usize> SmartLedsWrite for Ws2812Rmt<'d, Blocking, N> {
    type Error = Error;
    type Color = smart_leds_trait::RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<smart_leds_trait::RGB8>,
    {
        let max_leds = (N.saturating_sub(1)) / 24;
        let mut num_leds = 0usize;

        for item in iterator {
            if num_leds >= max_leds {
                return Err(Error::BufferTooSmall);
            }
            let rgb: RGB8 = item.into();
            let start = num_leds * 24;
            Self::encode_color(rgb, &mut self.buffer[start..start + 24]);
            num_leds += 1;
        }

        if num_leds == 0 {
            return Ok(());
        }

        self.buffer[num_leds * 24] = PulseCode::end_marker();
        self.send(num_leds * 24 + 1)
    }
}

#[cfg(all(feature = "async", feature = "pennant"))]
#[allow(async_fn_in_trait)]
impl<'d, const N: usize> pennant::AsyncStatusLed for Ws2812Rmt<'d, Async, N> {
    type Error = Error;

    async fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color).await
    }
}

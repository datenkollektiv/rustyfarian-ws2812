#![no_std]
//! WS2812 (NeoPixel) LED driver using `esp-hal` RMT peripheral (bare-metal, `no_std`).
//!
//! This crate provides a bare-metal driver for WS2812/NeoPixel addressable LEDs
//! using the `esp-hal` RMT peripheral.
//! It is the `no_std` counterpart to `rustyfarian-esp-idf-ws2812`.
//!
//! Pure color utilities are available in the `bunting` crate for testing.
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
//! Configure the RMT channel with [`RMT_CLK_DIV`] to achieve the required 10 MHz clock.
//! Using a different divider will produce incorrect LED timing.
//!
//! # Blocking Example
//!
//! ```ignore
//! use esp_hal::{
//!     gpio::Level,
//!     rmt::{Rmt, TxChannelConfig, TxChannelCreator},
//!     time::Rate,
//! };
//! use rgb::RGB8;
//! use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, RMT_CLK_DIV};
//!
//! const N: usize = buffer_size(1);
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
//! let config = TxChannelConfig::default()
//!     .with_clk_divider(RMT_CLK_DIV)
//!     .with_idle_output_level(Level::Low)
//!     .with_idle_output(true)
//!     .with_carrier_modulation(false);
//! let channel = rmt.channel0.configure_tx(&config).unwrap().with_pin(peripherals.GPIO8);
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
//!     gpio::Level,
//!     rmt::{Rmt, TxChannelConfig, TxChannelCreator},
//!     time::Rate,
//! };
//! use rgb::RGB8;
//! use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, RMT_CLK_DIV};
//!
//! const NUM_LEDS: usize = 12;
//! const N: usize = buffer_size(NUM_LEDS);
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
//!     .unwrap()
//!     .into_async();
//! let config = TxChannelConfig::default()
//!     .with_clk_divider(RMT_CLK_DIV)
//!     .with_idle_output_level(Level::Low)
//!     .with_idle_output(true)
//!     .with_carrier_modulation(false);
//! let channel = rmt.channel0.configure_tx(&config).unwrap().with_pin(peripherals.GPIO18);
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
//! Frames longer than one RMT channel's RAM are the exception; see [Long frames in async mode](crate#long-frames-in-async-mode).
//!
//! The larger gain comes from inter-frame delays.
//! A typical animation loop waits 16–50 ms between frames.
//! With a blocking `delay_ms()`, the CPU is spinning the whole time.
//! With `Timer::after_millis(16).await`, the executor is free to handle Wi-Fi events,
//! button presses, sensor reads, or any other spawned task during that delay.
//!
//! # Long frames in async mode
//!
//! A frame of `num_leds * 24 + 1` pulse codes is truly async only while it fits in one RMT channel's RAM.
//! Longer frames follow the `LongFramePolicy` set with `with_long_frame_policy`.
//! The default, `LongFramePolicy::CriticalSection`, drives them to completion inside a critical section, so the executor cannot run other tasks while they transmit.
//! `LongFramePolicy::Reject` returns [`Error::FrameTooLong`] instead and sends nothing.
//! The blocking driver has no policy: it polls its refill in the caller's interrupt state, so long frames can miss the refill deadline there too.
//!
//! | Chip                         | Channel RAM | Largest truly async frame |
//! |:-----------------------------|:------------|:--------------------------|
//! | ESP32-C3, ESP32-C6, ESP32-S3 | 48 codes    | 1 LED                     |
//! | ESP32                        | 64 codes    | 2 LEDs                    |
//!
//! The rule uses one RAM block, so it holds even for a channel configured with a larger `memsize`.
//!
//! esp-hal 1.2.2 refills the channel RAM from the transmit future's `poll`, not from the interrupt, and the refill is due about 30 µs after the RAM half-empties.
//! The critical-section path removes executor latency from that refill path by polling the transmit with interrupts masked.
//! It does not guarantee the waveform: flash-cache misses and other stalls can still delay a refill, as listed under residual risks below.
//! A timeout of about 2 µs per pulse code plus 1 ms bounds the interrupts-off time on every chip.
//! When it expires, the frame is aborted and [`Error::Transmit`] is returned.
//!
//! Cost of the critical-section path:
//!
//! - Interrupts are masked for the whole frame, 1.2–1.3 µs per pulse code: roughly 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.
//! - Keep the interrupts-off time at or below about 1 ms on radio firmware, which is about 32 LEDs (24 codes at 1.3 µs per LED).
//!   Above that, use `LongFramePolicy::Reject`.
//! - The blocking driver is not a fallback for long frames.
//!   Without a critical section it has the same ~30 µs refill deadline, and radio or other interrupts can delay its polling refill, so corrupt output remains possible.
//!   Wrapped in `critical_section::with`, it has the same interrupts-off cost as `CriticalSection`.
//!
//! On esp-hal 1.2.2 no option guarantees correct long frames under radio load.
//! `Reject`, which keeps every frame within channel RAM, is the only option that never risks corrupt output or long interrupts-off time.
//! The real fix is an ISR-side refill upstream.
//! - Such interrupt-off windows can disturb Wi-Fi and ESP-NOW and overflow UART FIFOs, so test your radio and serial traffic with the real strip length.
//! - On dual-core chips (ESP32 and ESP32-S3), any `critical_section::with` on the other core waits for the whole frame; its interrupts stay live.
//! - The critical-section implementation comes from esp-hal's `rt` feature, so firmware without `rt` fails to link.
//! - The executor does not run other tasks during a long frame.
//!
//! Residual risks, not yet proven on hardware:
//!
//! - Flash-cache misses can still stall the refill path.
//!   Building with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true` is the mitigation to measure.
//! - On ESP32 and ESP32-S3, the other core can stall the refill path through its flash accesses.
//! - High-priority interrupt levels on Xtensa are not verified to be masked by `critical_section`.
//! - The waveform on the data pin has not been captured yet; the `hal_c3_frame_sweep_async` example is the planned capture target.
//!
//! This is a workaround.
//! It can be removed once esp-hal refills the channel RAM from the interrupt.
//! See ADR 009 (`docs/adr/009-long-frame-policy-and-latch.md`) for the policy decision.
//!
//! # Frame spacing (latch)
//!
//! WS2812 LEDs need a low period of at least 280 µs between frames to latch the previous one; the driver enforces this as [`WS2812_RESET_US`].
//! How it is enforced depends on the path:
//!
//! | Path | How the latch is enforced | Cost |
//! |:-----|:--------------------------|:-----|
//! | Async, frame fits in channel RAM | Encoded in the signal: the last slot is a 280 µs low code | The `await` takes about 280 µs longer; no CPU wait |
//! | Async, long frame (`CriticalSection`) | Busy-wait before the next frame | Up to 280 µs, only when frames come faster than that |
//! | Blocking | Busy-wait before the next frame | Up to 280 µs, only when frames come faster than that |
//!
//! Encoding the latch on the long and blocking paths would add 280 µs of interrupts-off or CPU time to every frame, so they keep the busy-wait.
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
//! The `async` feature gates the async `Ws2812Rmt<'d, Async, N>` API and pulls in no runtime: its only dependency is `critical-section`, which `esp-hal` already uses (see [Long frames in async mode](crate#long-frames-in-async-mode)).
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
    pin::pin,
    task::{Context, Poll, Waker},
};
use esp_hal::time::{Duration, Instant};
#[cfg(feature = "async")]
use esp_hal::Async;
use esp_hal::{
    gpio::Level,
    rmt::{Channel, PulseCode, Tx},
    Blocking,
};
use rgb::RGB8;
use smart_leds_trait::SmartLedsWrite;

/// Clock divider for the RMT peripheral to achieve the required 10 MHz timing clock.
///
/// At 80 MHz base clock, divider 8 yields 10 MHz (100 ns per tick).
/// Pass this constant to [`TxChannelConfig::with_clk_divider`] when constructing the channel.
pub const RMT_CLK_DIV: u8 = 8;

/// Minimum low time in microseconds between two frames, so the LEDs latch the previous one.
///
/// The WS2812B datasheet asks for at least 280 µs; older WS2812 parts need only 50 µs.
/// The driver waits for the remainder of this interval before it starts a frame
/// that follows the previous one too closely.
/// Async frames that fit in channel RAM carry this interval in the signal.
/// Other frames are followed by a busy-wait, which needs esp-hal's `rt` feature;
/// see [Frame spacing (latch)](crate#frame-spacing-latch).
pub const WS2812_RESET_US: u32 = 280;

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
/// Formula: `num_leds * 24 + 1` — 24 bits of color data per LED, plus one end-of-stream marker.
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

/// How the async driver sends a frame that does not fit in one RMT channel's RAM.
///
/// Frames that fit are always sent with a plain `await` and are not affected by the policy.
/// See [`Long frames in async mode`](crate#long-frames-in-async-mode).
#[cfg(feature = "async")]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LongFramePolicy {
    /// Busy-poll the transmit inside a critical section, bounded by a timeout.
    ///
    /// The executor and all interrupts are blocked for the whole frame.
    /// If the caller already runs with interrupts masked, they stay masked afterwards.
    #[default]
    CriticalSection,
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
/// **Blocking** ([`Ws2812Rmt<'d, Blocking, N>`](Ws2812Rmt)):
/// - If `Channel::transmit()` fails, the channel is consumed and unrecoverable; the driver must be dropped and re-created.
/// - If `SingleShotTxTransaction::wait()` fails, the channel is returned; the driver is immediately reusable.
///
/// **Async** ([`Ws2812Rmt<'d, Async, N>`](Ws2812Rmt)) — `Channel::transmit()` takes `&mut self`
/// (does not consume the channel), so the driver remains fully usable after [`Error::Transmit`].
///
/// [`Error::BufferTooSmall`] is always recoverable in both modes: the buffer is never written
/// and no transmission is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// RMT peripheral configuration failed.
    ///
    /// This variant is reserved for future constructors that configure the RMT peripheral
    /// internally.
    RmtConfig,
    /// RMT transmission failed or (blocking mode only) the channel was lost after a previous
    /// unrecoverable error.
    ///
    /// **Blocking mode**: if `transmit()` fails internally, the `Channel` is consumed by
    /// `esp-hal` and cannot be recovered.
    /// Every subsequent call on the same driver instance will also return `Transmit`.
    /// Recreate the driver from a new channel.
    ///
    /// **Async mode**: the channel is never consumed, so the driver is immediately reusable
    /// after this error.
    /// The failure typically indicates a hardware-level RMT error (very rare).
    Transmit,
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
    /// Returned before any transmission and before the latch wait.
    /// The channel remains fully operational.
    /// Only the async driver returns this error.
    FrameTooLong,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::RmtConfig => write!(f, "RMT peripheral configuration failed"),
            Error::Transmit => write!(f, "RMT transmission failed"),
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
/// (80 MHz base clock ÷ [`RMT_CLK_DIV`] = 8).
pub struct Ws2812Rmt<'d, Dm: esp_hal::DriverMode, const N: usize> {
    /// The RMT TX channel, wrapped in `Option` to support esp-hal's type-state transmit API
    /// (blocking transmit consumes the channel; wait returns it).
    channel: Option<Channel<'d, Dm, Tx>>,
    /// Pre-allocated pulse-code buffer to avoid runtime allocation.
    buffer: [PulseCode; N],
    /// Earliest instant the next frame may start, used to enforce [`WS2812_RESET_US`].
    next_frame_at: Option<Instant>,
    /// What to do with frames that do not fit in one channel's RAM.
    #[cfg(feature = "async")]
    long_frame_policy: LongFramePolicy,
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
    /// The channel **must** be configured with [`RMT_CLK_DIV`] (8) on an 80 MHz base clock.
    /// Using a different clock divider will produce incorrect WS2812 timing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use esp_hal::{
    ///     gpio::Level,
    ///     rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    ///     time::Rate,
    /// };
    /// use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, RMT_CLK_DIV};
    ///
    /// const N: usize = buffer_size(1);
    ///
    /// let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    /// let config = TxChannelConfig::default()
    ///     .with_clk_divider(RMT_CLK_DIV)
    ///     .with_idle_output_level(Level::Low)
    ///     .with_idle_output(true)
    ///     .with_carrier_modulation(false);
    /// let channel = rmt.channel0.configure_tx(&config).unwrap().with_pin(peripherals.GPIO8);
    ///
    /// let mut led = Ws2812Rmt::<_, N>::new(channel);
    /// ```
    pub fn new(channel: Channel<'d, Dm, Tx>) -> Self {
        Self {
            channel: Some(channel),
            buffer: [PulseCode::end_marker(); N],
            next_frame_at: None,
            #[cfg(feature = "async")]
            long_frame_policy: LongFramePolicy::default(),
        }
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
    /// Sets a single LED to the given color.
    ///
    /// The buffer size `N` must be at least 25 (`buffer_size(1)`).
    ///
    /// Color is transmitted in WS2812 GRB order.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] if `N < 25`.
    /// - [`Error::Transmit`] if the RMT transmission fails or the channel was previously lost.
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<(), Error> {
        if N < 25 {
            return Err(Error::BufferTooSmall);
        }
        Self::encode_color(rgb, &mut self.buffer[..24]);
        self.buffer[24] = PulseCode::end_marker();
        self.do_transmit(25)
    }

    /// Sets multiple LEDs from a color slice.
    ///
    /// Colors are transmitted in WS2812 GRB order.
    /// The buffer size `N` must be at least `rgbs.len() * 24 + 1`.
    ///
    /// The blocking refill is a polling loop, so interrupts can still delay it for long frames.
    /// Wrap the call in `critical_section::with` if your firmware has a heavy interrupt load.
    ///
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] if `N < rgbs.len() * 24 + 1`.
    /// - [`Error::Transmit`] if the RMT transmission fails or the channel was previously lost.
    pub fn set_pixels_slice(&mut self, rgbs: &[RGB8]) -> Result<(), Error> {
        let num_leds = rgbs.len();
        let needed = num_leds * 24 + 1;
        if needed > N {
            return Err(Error::BufferTooSmall);
        }
        for (i, &rgb) in rgbs.iter().enumerate() {
            Self::encode_color(rgb, &mut self.buffer[i * 24..(i + 1) * 24]);
        }
        self.buffer[num_leds * 24] = PulseCode::end_marker();
        self.do_transmit(needed)
    }

    /// Sends `buffer[..len]` via the RMT channel and waits for completion.
    ///
    /// The blocking refill is a polling loop, so interrupts can still delay it for long frames.
    ///
    /// Waits until the previous frame's latch is complete first, see [`WS2812_RESET_US`].
    /// The frame ends with a plain end marker, so the latch is enforced after it instead:
    /// `next_frame_at` is set to the end of this attempt plus [`WS2812_RESET_US`].
    /// Encoding the latch into the frame would add 280 µs of CPU time per frame, even for spaced frames.
    ///
    /// Uses `Option<Channel>` to handle esp-hal's ownership-based transmit API:
    /// `transmit()` consumes the channel and `wait()` returns it.
    fn do_transmit(&mut self, len: usize) -> Result<(), Error> {
        let ch = self.channel.take().ok_or(Error::Transmit)?;
        self.wait_until_next_frame();
        if let Some(last) = len.checked_sub(1).and_then(|i| self.buffer.get_mut(i)) {
            *last = PulseCode::end_marker();
        }
        // transmit() consumes `ch`; on Err the channel is unrecoverable
        let txn = match ch.transmit(&self.buffer[..len]) {
            Ok(txn) => txn,
            Err(_) => {
                self.next_frame_at = Some(latch_deadline());
                return Err(Error::Transmit);
            }
        };
        // wait() consumes `txn`, releasing the borrow on `self.buffer`
        let result = match txn.wait() {
            Ok(ch_back) => {
                self.channel = Some(ch_back);
                Ok(())
            }
            Err((_, ch_back)) => {
                self.channel = Some(ch_back);
                Err(Error::Transmit)
            }
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
    /// Sets how frames longer than one RMT channel's RAM are sent.
    ///
    /// The default is [`LongFramePolicy::CriticalSection`].
    #[must_use]
    pub fn with_long_frame_policy(mut self, policy: LongFramePolicy) -> Self {
        self.long_frame_policy = policy;
        self
    }

    /// Returns the current [`LongFramePolicy`].
    pub fn long_frame_policy(&self) -> LongFramePolicy {
        self.long_frame_policy
    }

    /// Sets a single LED to the given color, yielding to the executor during transmission.
    ///
    /// The buffer size `N` must be at least 25 (`buffer_size(1)`).
    ///
    /// A single LED is 25 pulse codes, which fits in one channel's RAM on every supported chip, so this method stays truly async.
    /// The latch after this LED is carried in the signal, so the call completes only once the LEDs have latched.
    /// A call that follows a blocking or long-frame transmit may first busy-wait the rest of the latch interval; see [Frame spacing (latch)](crate#frame-spacing-latch).
    ///
    /// Color is transmitted in WS2812 GRB order.
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

    /// Sets multiple LEDs from a color slice, yielding to the executor during transmission.
    ///
    /// Colors are transmitted in WS2812 GRB order.
    /// The buffer size `N` must be at least `rgbs.len() * 24 + 1`.
    ///
    /// Frames longer than one RMT channel's RAM (1 LED on ESP32-C3, ESP32-C6 and ESP32-S3, 2 LEDs on ESP32) do not yield.
    /// With the default [`LongFramePolicy::CriticalSection`] they busy-poll with interrupts masked, about 1.2–1.3 µs per pulse code, so a 12-LED frame holds the CPU for about 360 µs.
    /// With [`LongFramePolicy::Reject`] they return [`Error::FrameTooLong`] instead.
    /// See [`Long frames in async mode`](crate#long-frames-in-async-mode).
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
        let action = long_frame_action(len, esp_hal::rmt::CHANNEL_RAM_SIZE, self.long_frame_policy);
        if matches!(action, Action::Reject) {
            return Err(Error::FrameTooLong);
        }
        if self.channel.is_none() {
            return Err(Error::Transmit);
        }
        self.wait_until_next_frame();
        let Self {
            channel,
            buffer,
            next_frame_at,
            ..
        } = self;
        let ch = channel.as_mut().ok_or(Error::Transmit)?;
        if let Some(last) = len.checked_sub(1).and_then(|i| buffer.get_mut(i)) {
            *last = if matches!(action, Action::Await) {
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
            Action::Await => {
                // Named local declared after `stamp`: locals drop in reverse order, so on
                // cancellation the transmit (and the channel stop) ends before `stamp` records.
                let mut transmit = pin!(ch.transmit(data));
                transmit.as_mut().await.map_err(|_| Error::Transmit)?;
                stamp.latch_on_wire = true;
                Ok(())
            }
            Action::CriticalSection => Self::transmit_in_critical_section(ch, data),
            Action::Reject => Err(Error::FrameTooLong),
        }
    }

    /// Drives a transmit that needs RAM refills to completion inside a critical section.
    ///
    /// esp-hal refills the channel RAM from the future's `poll`, so the future is polled with a
    /// no-op waker, bounded by [`tx_timeout_us`].
    /// A stopped timer (see [`LATCH_WAIT_MAX_STALLED_READS`]) also ends the loop with [`Error::Transmit`],
    /// so a frozen clock cannot keep interrupts masked.
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
    fn transmit_in_critical_section(
        ch: &mut Channel<'d, Async, Tx>,
        data: &[PulseCode],
    ) -> Result<(), Error> {
        let mut slot = pin!(None);
        critical_section::with(|_| {
            let deadline_us =
                Instant::now().duration_since_epoch().as_micros() + tx_timeout_us(data.len());
            slot.set(Some(ch.transmit(data)));
            let mut cx = Context::from_waker(Waker::noop());
            let mut previous_us = 0;
            let mut stalled_reads = 0;
            loop {
                let Some(fut) = slot.as_mut().as_pin_mut() else {
                    return Err(Error::Transmit);
                };
                if let Poll::Ready(result) = fut.poll(&mut cx) {
                    return result.map_err(|_| Error::Transmit);
                }
                let now_us = Instant::now().duration_since_epoch().as_micros();
                if now_us >= deadline_us {
                    return Err(Error::Transmit);
                }
                if now_us == previous_us {
                    stalled_reads += 1;
                    if stalled_reads >= LATCH_WAIT_MAX_STALLED_READS {
                        return Err(Error::Transmit);
                    }
                } else {
                    stalled_reads = 0;
                    previous_us = now_us;
                }
            }
        })
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

/// What the async driver does with a frame of a given length.
#[cfg(feature = "async")]
enum Action {
    /// The frame fits in channel RAM: plain `await`.
    Await,
    /// Long frame: busy-poll inside a critical section.
    CriticalSection,
    /// Long frame: refuse with [`Error::FrameTooLong`].
    Reject,
}

/// Decides how to send `len` pulse codes given `ram` codes of channel RAM and the policy.
#[cfg(feature = "async")]
const fn long_frame_action(len: usize, ram: usize, policy: LongFramePolicy) -> Action {
    if fits_channel_ram(len, ram) {
        return Action::Await;
    }
    match policy {
        LongFramePolicy::Reject => Action::Reject,
        LongFramePolicy::CriticalSection => Action::CriticalSection,
    }
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

#[cfg(feature = "async")]
const _: () = {
    assert!(matches!(
        long_frame_action(25, 48, LongFramePolicy::CriticalSection),
        Action::Await
    ));
    assert!(matches!(
        long_frame_action(25, 48, LongFramePolicy::Reject),
        Action::Await
    ));
    assert!(matches!(
        long_frame_action(289, 48, LongFramePolicy::CriticalSection),
        Action::CriticalSection
    ));
    assert!(matches!(
        long_frame_action(289, 48, LongFramePolicy::Reject),
        Action::Reject
    ));
};

/// Returns `true` if `len` pulse codes fit in `ram` codes of one RMT channel's RAM,
/// so that no refill is needed during transmission.
#[cfg(feature = "async")]
const fn fits_channel_ram(len: usize, ram: usize) -> bool {
    len <= ram
}

/// Timeout budget per pulse code, in microseconds.
///
/// About 1.5 times the worst-case code time of 1.3 µs (`T1H + T1L` = 13 ticks at 100 ns).
/// Assumes the 10 MHz RMT tick set by [`RMT_CLK_DIV`]; a slower tick makes long frames time out
/// with [`Error::Transmit`].
#[cfg(feature = "async")]
const TIMEOUT_US_PER_CODE: u64 = 2;

/// Fixed timeout margin in microseconds, covering transmit start-up and polling overhead.
#[cfg(feature = "async")]
const TIMEOUT_MARGIN_US: u64 = 1000;

/// Upper bound in microseconds for a refill-driven transmit of `len` pulse codes.
#[cfg(feature = "async")]
const fn tx_timeout_us(len: usize) -> u64 {
    len as u64 * TIMEOUT_US_PER_CODE + TIMEOUT_MARGIN_US
}

#[cfg(feature = "async")]
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
/// The iterator is drained directly into the pre-allocated pulse-code buffer —
/// no heap allocation occurs. If the iterator yields more colors than the buffer
/// can hold (`(N - 1) / 24` LEDs), transmission is aborted and
/// [`Error::BufferTooSmall`] is returned before any data is sent.
///
/// If the iterator is empty, `Ok(())` is returned immediately — no reset pulse
/// or blank frame is sent. Hardware that requires an explicit blank to turn off
/// LEDs should send a zeroed color slice instead.
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
        self.do_transmit(num_leds * 24 + 1)
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

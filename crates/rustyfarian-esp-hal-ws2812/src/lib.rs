#![no_std]
#![allow(async_fn_in_trait)]
//! WS2812 (NeoPixel) LED driver using `esp-hal` RMT peripheral (bare-metal, `no_std`).
//!
//! This crate provides a bare-metal driver for WS2812/NeoPixel addressable LEDs
//! using the `esp-hal` RMT peripheral.
//! It is the `no_std` counterpart to `rustyfarian-esp-idf-ws2812`.
//!
//! Pure color utilities are available in the `ws2812-pure` crate for testing.
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
//! let channel = rmt.channel0.configure_tx(peripherals.GPIO8, config).unwrap();
//!
//! let mut led = Ws2812Rmt::<_, N>::new(channel);
//! led.set_pixel(RGB8::new(255, 0, 0)).unwrap();
//!
//! let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
//! led.set_pixels_slice(&colors).unwrap();
//! ```
//!
//! # Async Example (feature `async`)
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
//! let channel = rmt.channel0.configure_tx(peripherals.GPIO18, config).unwrap();
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
//!
//! The larger gain comes from inter-frame delays.
//! A typical animation loop waits 16–50 ms between frames.
//! With a blocking `delay_ms()`, the CPU is spinning the whole time.
//! With `Timer::after_millis(16).await`, the executor is free to handle Wi-Fi events,
//! button presses, sensor reads, or any other spawned task during that delay.
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
//! # Other async runtimes
//!
//! This crate's async support is built on `esp-hal`'s native async RMT channel and the
//! `esp-rtos` Embassy executor, which is the standard async runtime for `esp-hal 1.0+`.
//! Other Embassy-compatible executors (e.g., `embassy-executor` with a custom time driver)
//! are theoretically possible but untested; the `RmtTxFuture` in `esp-hal` is executor-agnostic
//! (it uses `core::task::Waker`), but the `embassy-time` timer support requires the
//! `esp-rtos` time driver to be initialised via `esp_rtos::start()`.
//!
//! `esp-idf-hal` (the std path) does not have async RMT support as of `esp-idf-hal 0.46`.
//! `rustyfarian-esp-idf-ws2812` therefore remains blocking-only.
//! If `esp-idf-hal` gains async RMT in a future release, async support can be added there
//! under a separate feature flag without affecting this crate.

#[cfg(feature = "async")]
use esp_hal::Async;
use esp_hal::{
    gpio::Level,
    rmt::{Channel, PulseCode, Tx},
    Blocking,
};
use rgb::RGB8;
use smart_leds_trait::SmartLedsWrite;
use ws2812_pure::rgb_to_grb;

/// Clock divider for the RMT peripheral to achieve the required 10 MHz timing clock.
///
/// At 80 MHz base clock, divider 8 yields 10 MHz (100 ns per tick).
/// Pass this constant to [`TxChannelConfig::with_clk_divider`] when constructing the channel.
pub const RMT_CLK_DIV: u8 = 8;

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

/// Errors that can occur during WS2812 RMT operations.
///
/// # Error recovery
///
/// Recovery depends on which driver mode is in use:
///
/// **Blocking** ([`Ws2812Rmt<'d, Blocking, N>`](Ws2812Rmt)) — when `transmit()` fails, the
/// underlying `Channel` is consumed by `esp-hal` and cannot be recovered.
/// All subsequent calls on the same driver instance will return [`Error::Transmit`].
/// The driver must be dropped and re-created from a fresh channel.
///
/// **Async** ([`Ws2812Rmt<'d, Async, N>`](Ws2812Rmt)) — the async `Channel::transmit()` takes
/// `&mut self` and never consumes the channel, so the driver remains fully usable after a
/// [`Error::Transmit`] — simply retry or handle the error as appropriate.
///
/// [`Error::BufferTooSmall`] is always recoverable in both modes: the buffer is never written
/// and no transmission is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::RmtConfig => write!(f, "RMT peripheral configuration failed"),
            Error::Transmit => write!(f, "RMT transmission failed"),
            Error::BufferTooSmall => write!(f, "pixel count exceeds buffer capacity"),
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
    /// let channel = rmt.channel0.configure_tx(peripherals.GPIO8, config).unwrap();
    ///
    /// let mut led = Ws2812Rmt::<_, N>::new(channel);
    /// ```
    pub fn new(channel: Channel<'d, Dm, Tx>) -> Self {
        Self {
            channel: Some(channel),
            buffer: [PulseCode::end_marker(); N],
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
    /// Uses `Option<Channel>` to handle esp-hal's ownership-based transmit API:
    /// `transmit()` consumes the channel and `wait()` returns it.
    fn do_transmit(&mut self, len: usize) -> Result<(), Error> {
        let ch = self.channel.take().ok_or(Error::Transmit)?;
        // transmit() consumes `ch`; on Err the channel is unrecoverable
        let txn = ch
            .transmit(&self.buffer[..len])
            .map_err(|_| Error::Transmit)?;
        // wait() consumes `txn`, releasing the borrow on `self.buffer`
        match txn.wait() {
            Ok(ch_back) => {
                self.channel = Some(ch_back);
                Ok(())
            }
            Err((_, ch_back)) => {
                self.channel = Some(ch_back);
                Err(Error::Transmit)
            }
        }
    }
}

/// Async methods (requires feature `async`).
///
/// Unlike the blocking variant, the async `Channel::transmit()` takes `&mut self` and returns
/// a `Future` directly — no channel ownership transfer occurs.
#[cfg(feature = "async")]
impl<'d, const N: usize> Ws2812Rmt<'d, Async, N> {
    /// Sets a single LED to the given color, yielding to the executor during transmission.
    ///
    /// The buffer size `N` must be at least 25 (`buffer_size(1)`).
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
    ///     Err(_) => unreachable!(),
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
    /// # Errors
    ///
    /// - [`Error::BufferTooSmall`] — returned immediately, **before** any `.await`, if
    ///   `N < rgbs.len() * 24 + 1`.
    ///   No data is written to the buffer and the channel remains fully operational.
    ///   Fix: use [`buffer_size`]`(num_leds)` to size `N` correctly at compile time,
    ///   or ensure the slice length does not exceed `(N - 1) / 24`.
    /// - [`Error::Transmit`] — returned after `.await` completes if the RMT hardware signals
    ///   an error (very rare).
    ///   Because the async channel is never consumed, the driver is reusable after this error.
    ///
    /// # Rapid consecutive calls
    ///
    /// The `transmit().await` future completes only after the RMT peripheral finishes sending
    /// all pulses.
    /// Awaiting completion before calling again is therefore the natural backpressure mechanism —
    /// no explicit queuing is needed.
    /// If you call `set_pixels_slice` in a tight loop without an inter-frame delay,
    /// the executor will context-switch to other tasks during each transmission
    /// (~30 µs for a 12-LED ring), then resume for the next frame.
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
    ///     Err(Error::Transmit) => {
    ///         // Hardware error — driver is still usable; retry or log.
    ///         log_error();
    ///     }
    ///     Err(_) => unreachable!(),
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
    async fn do_transmit_async(&mut self, len: usize) -> Result<(), Error> {
        let ch = self.channel.as_mut().ok_or(Error::Transmit)?;
        ch.transmit(&self.buffer[..len])
            .await
            .map_err(|_| Error::Transmit)
    }
}

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

#[cfg(feature = "led-effects")]
impl<'d, const N: usize> led_effects::StatusLed for Ws2812Rmt<'d, Blocking, N> {
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

#[cfg(all(feature = "async", feature = "led-effects"))]
impl<'d, const N: usize> led_effects::AsyncStatusLed for Ws2812Rmt<'d, Async, N> {
    type Error = Error;

    async fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color).await
    }
}

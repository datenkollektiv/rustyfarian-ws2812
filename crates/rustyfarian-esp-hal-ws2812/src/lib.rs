#![no_std]
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
//! # Example
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
//! let mut led = Ws2812Rmt::<N>::new(channel);
//! led.set_pixel(RGB8::new(255, 0, 0)).unwrap();
//!
//! let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
//! led.set_pixels_slice(&colors).unwrap();
//! ```

use esp_hal::{
    gpio::Level,
    rmt::{Channel, PulseCode, Tx},
    Blocking,
};
use rgb::RGB8;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// RMT peripheral configuration failed.
    ///
    /// This variant is reserved for future constructors that configure the RMT peripheral
    /// internally.
    RmtConfig,
    /// RMT transmission failed or the channel was lost after a previous unrecoverable error.
    ///
    /// If `transmit()` fails internally (very rare), the RMT channel is consumed and cannot
    /// be recovered.
    /// The driver must be re-created in that case.
    Transmit,
    /// The pixel count exceeds the buffer capacity `N`.
    ///
    /// Ensure `N >= num_leds * 24 + 1`.
    /// Use [`buffer_size`] to compute the correct `N`.
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
/// - `N` — pulse-code buffer size (`num_leds * 24 + 1`).
///
/// # Timing
///
/// The driver expects the RMT channel to be configured at 10 MHz
/// (80 MHz base clock ÷ [`RMT_CLK_DIV`] = 8).
pub struct Ws2812Rmt<'d, const N: usize> {
    /// The RMT TX channel, wrapped in `Option` to support esp-hal's type-state transmit API
    /// (transmit consumes the channel; wait returns it).
    channel: Option<Channel<'d, Blocking, Tx>>,
    /// Pre-allocated pulse-code buffer to avoid runtime allocation.
    buffer: [PulseCode; N],
}

impl<'d, const N: usize> Ws2812Rmt<'d, N> {
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
    ///     .with_idle_output(false)
    ///     .with_carrier_modulation(false);
    /// let channel = rmt.channel0.configure_tx(peripherals.GPIO8, config).unwrap();
    ///
    /// let mut led = Ws2812Rmt::<N>::new(channel);
    /// ```
    pub fn new(channel: Channel<'d, Blocking, Tx>) -> Self {
        Self {
            channel: Some(channel),
            buffer: [PulseCode::end_marker(); N],
        }
    }

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

#[cfg(feature = "led-effects")]
impl<'d, const N: usize> led_effects::StatusLed for Ws2812Rmt<'d, N> {
    type Error = Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

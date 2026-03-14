#![no_std]
//! WS2812 (NeoPixel) LED driver using SPI prerendered encoding (`no_std`, `embedded-hal` 1.0).
//!
//! This crate drives WS2812/NeoPixel LEDs over SPI using the prerendered encoding
//! from [`ws2812-pure`](https://crates.io/crates/ws2812-pure). Each WS2812 data bit is
//! encoded as 4 SPI bits, producing 12 SPI bytes per LED.
//! The encoding is byte-for-byte compatible with
//! [`ws2812-spi`](https://crates.io/crates/ws2812-spi) v0.5.1's prerendered module.
//!
//! # Buffer Sizing
//!
//! The driver uses a const-generic buffer `[u8; N]` where
//! `N = spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`.
//! Use [`spi_buffer_size`] to compute `N` at compile time:
//!
//! ```ignore
//! use rustyfarian_avr_ws2812::spi_buffer_size;
//! const N: usize = spi_buffer_size(8); // 8-LED ring
//! ```
//!
//! # SPI Clock Configuration
//!
//! The SPI peripheral **must** be configured at 2 MHz for correct WS2812 timing.
//! On an ATmega328P running at 16 MHz, use a clock prescaler of ÷8.
//! The caller is responsible for configuring the SPI clock before calling [`Ws2812Spi::write`].
//!
//! # Interrupt Safety
//!
//! The `write` call is not interrupt-safe by itself.
//! On AVR targets, wrap the call in a critical section:
//!
//! ```ignore
//! avr_device::interrupt::free(|_| {
//!     ws.write(&colors).unwrap();
//! });
//! ```
//!
//! The caller is responsible for this; the driver makes no assumptions about the
//! interrupt context.
//!
//! # Example
//!
//! ```ignore
//! use rustyfarian_avr_ws2812::{Ws2812Spi, spi_buffer_size};
//! use rgb::RGB8;
//!
//! const NUM_LEDS: usize = 8;
//! const N: usize = spi_buffer_size(NUM_LEDS);
//!
//! let mut ws: Ws2812Spi<_, N> = Ws2812Spi::new(spi_bus);
//! let colors = [RGB8::new(255, 0, 0); NUM_LEDS];
//! ws.write(&colors).unwrap();
//! ```

use core::fmt;
use embedded_hal::spi::SpiBus;
use rgb::RGB8;
use ws2812_pure::{prerender_spi, spi_data_len, SpiEncodeError, SPI_RESET_BYTES_2MHZ};

/// Errors that can occur during WS2812 SPI operations.
#[derive(Debug)]
pub enum SpiError<E> {
    /// The color slice was too large for the buffer.
    Encode(SpiEncodeError),
    /// The underlying SPI bus returned an error.
    Spi(E),
}

impl<E: fmt::Debug> fmt::Display for SpiError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(e) => write!(f, "SPI encode error: {e}"),
            Self::Spi(e) => write!(f, "SPI bus error: {e:?}"),
        }
    }
}

impl<E> From<SpiEncodeError> for SpiError<E> {
    fn from(e: SpiEncodeError) -> Self {
        Self::Encode(e)
    }
}

/// Returns the total SPI buffer size for `num_leds` LEDs (data + reset bytes).
///
/// Use this as the const generic `N` for [`Ws2812Spi`]:
///
/// ```
/// use rustyfarian_avr_ws2812::spi_buffer_size;
/// const N: usize = spi_buffer_size(8); // 8-LED ring → 176
/// assert_eq!(N, 176);
/// ```
pub const fn spi_buffer_size(num_leds: usize) -> usize {
    spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ
}

/// WS2812 LED driver using SPI prerendered encoding (`no_std`, `embedded-hal` 1.0).
///
/// `N` is the total SPI buffer size in bytes.
/// Compute it with [`spi_buffer_size`]: `N = spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`.
///
/// The struct is generic over any [`SpiBus`] implementation — no AVR-specific
/// dependencies are introduced here. The caller is responsible for:
/// - Configuring SPI at 2 MHz before calling [`write`](Ws2812Spi::write).
/// - Wrapping the `write` call in a critical section on interrupt-driven systems.
///
/// # Type Parameters
///
/// - `SPI` — any type implementing [`embedded_hal::spi::SpiBus`].
/// - `N` — total SPI buffer size (`spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`).
pub struct Ws2812Spi<SPI, const N: usize> {
    spi: SPI,
    buf: [u8; N],
}

impl<SPI: SpiBus, const N: usize> Ws2812Spi<SPI, N> {
    /// Creates a new WS2812 SPI driver wrapping the given SPI bus.
    ///
    /// The internal buffer is zero-initialised. The SPI bus **must** already be
    /// configured at 2 MHz before the first call to [`write`](Self::write).
    pub fn new(spi: SPI) -> Self {
        Self { spi, buf: [0u8; N] }
    }

    /// Encodes `colors` into the internal buffer and sends it over SPI.
    ///
    /// The first `spi_data_len(colors.len())` bytes of the buffer are filled with
    /// prerendered WS2812 SPI data.
    /// The remaining bytes are zeroed to provide the WS2812 reset pulse
    /// (`SPI_RESET_BYTES_2MHZ` = 80 bytes at 2 MHz).
    ///
    /// # Errors
    ///
    /// - [`SpiError::Encode`] if `colors.len() > (N - SPI_RESET_BYTES_2MHZ) / 12`
    ///   (the color slice is too large for the buffer).
    /// - [`SpiError::Spi`] if the underlying SPI bus returns an error.
    pub fn write(&mut self, colors: &[RGB8]) -> Result<(), SpiError<SPI::Error>> {
        let data_len = spi_data_len(colors.len());
        prerender_spi(colors, &mut self.buf[..data_len])?;
        // Zero the reset tail so the WS2812 latches the frame.
        self.buf[data_len..].fill(0);
        self.spi.write(&self.buf).map_err(SpiError::Spi)?;
        Ok(())
    }

    /// Releases the inner SPI bus, consuming the driver.
    pub fn release(self) -> SPI {
        self.spi
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::ToString;
    use ws2812_pure::SpiEncodeError;

    // --- spi_buffer_size tests -----------------------------------------------

    #[test]
    fn spi_buffer_size_zero_leds() {
        assert_eq!(spi_buffer_size(0), SPI_RESET_BYTES_2MHZ);
    }

    #[test]
    fn spi_buffer_size_one_led() {
        // 1 LED: 12 data bytes + 80 reset bytes = 92
        assert_eq!(spi_buffer_size(1), 92);
    }

    #[test]
    fn spi_buffer_size_eight_leds() {
        // 8 LEDs: 96 data bytes + 80 reset bytes = 176
        assert_eq!(spi_buffer_size(8), 176);
    }

    #[test]
    fn spi_buffer_size_twelve_leds() {
        // 12 LEDs: 144 data bytes + 80 reset bytes = 224
        assert_eq!(spi_buffer_size(12), 224);
    }

    // --- SpiError Display tests ----------------------------------------------

    #[test]
    fn spi_error_encode_display() {
        let e: SpiError<()> = SpiError::Encode(SpiEncodeError::BufferTooSmall);
        assert!(e.to_string().contains("encode"));
    }

    #[test]
    fn spi_error_spi_display() {
        let e: SpiError<&str> = SpiError::Spi("bus failure");
        assert!(e.to_string().contains("SPI bus error"));
    }

    // --- From<SpiEncodeError> ------------------------------------------------

    #[test]
    fn from_spi_encode_error() {
        let e: SpiError<()> = SpiEncodeError::BufferTooSmall.into();
        assert!(matches!(
            e,
            SpiError::Encode(SpiEncodeError::BufferTooSmall)
        ));
    }
}

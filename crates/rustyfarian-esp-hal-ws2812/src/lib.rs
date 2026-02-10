#![no_std]
//! WS2812 (NeoPixel) LED driver using `esp-hal` RMT peripheral (bare-metal, `no_std`).
//!
//! This crate provides a bare-metal driver for WS2812/NeoPixel addressable LEDs
//! using the `esp-hal` RMT peripheral.
//! It is the `no_std` counterpart to `rustyfarian-esp-idf-ws2812`.
//!
//! Pure color utilities are available in the `ws2812-pure` crate for testing.
//!
//! # Status
//!
//! **Skeleton only** — all methods currently call `todo!()`.
//! The real `esp-hal` dependency will be added when implementing (see ADR 005).
//!
//! # Example
//!
//! ```ignore
//! use rustyfarian_esp_hal_ws2812::Ws2812Rmt;
//! use rgb::RGB8;
//!
//! let mut led = Ws2812Rmt::new(rmt_channel, gpio_pin)?;
//!
//! led.set_pixel(RGB8::new(255, 0, 0))?;
//!
//! let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
//! led.set_pixels_slice(&colors)?;
//! ```

use rgb::RGB8;

/// Errors that can occur during WS2812 RMT operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// RMT peripheral configuration failed.
    RmtConfig,
    /// RMT transmission failed.
    Transmit,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::RmtConfig => write!(f, "RMT peripheral configuration failed"),
            Error::Transmit => write!(f, "RMT transmission failed"),
        }
    }
}

/// WS2812 LED driver using `esp-hal` RMT peripheral.
///
/// This is the bare-metal (`no_std`) counterpart to
/// [`rustyfarian-esp-idf-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)'s `WS2812RMT`.
///
/// The RMT peripheral provides precise timing control needed for the
/// WS2812 protocol without CPU intervention.
pub struct Ws2812Rmt {
    _private: (),
}

impl Ws2812Rmt {
    /// Creates a new WS2812 driver.
    ///
    /// The final signature will accept an RMT channel and GPIO pin once the
    /// `esp-hal` dependency is added (see ADR 005).
    ///
    /// # Errors
    ///
    /// Returns [`Error::RmtConfig`] if the RMT peripheral cannot be configured.
    pub fn new() -> Result<Self, Error> {
        todo!("esp-hal implementation — see ADR 005")
    }

    /// Sets a single pixel color.
    ///
    /// Use this for single-LED indicators or when updating one pixel at a time.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transmit`] if the RMT transmission fails.
    pub fn set_pixel(&mut self, _rgb: RGB8) -> Result<(), Error> {
        todo!("esp-hal implementation — see ADR 005")
    }

    /// Sets multiple pixels from a slice.
    ///
    /// Use this for LED strips or rings with multiple pixels.
    ///
    /// # Arguments
    ///
    /// * `rgbs` - Slice of colors, one per pixel in order
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transmit`] if the RMT transmission fails.
    pub fn set_pixels_slice(&mut self, _rgbs: &[RGB8]) -> Result<(), Error> {
        todo!("esp-hal implementation — see ADR 005")
    }
}

#[cfg(feature = "led-effects")]
impl led_effects::StatusLed for Ws2812Rmt {
    type Error = Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

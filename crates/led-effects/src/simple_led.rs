//! Simple GPIO LED adapter for StatusLed trait.
//!
//! Maps RGB colors to on/off based on a brightness threshold.
//! Use this for boards with a simple on/off LED instead of an RGB LED.
//!
//! The actual threshold logic is in the platform-independent [`exceeds_threshold`](crate::exceeds_threshold)
//! function, keeping this module as a thin hardware wrapper.
//!
//! Generic over [`embedded_hal::digital::OutputPin`], so it works with any HAL
//! (ESP-IDF, nrf-hal, stm32-hal, or test mocks).
//!
//! # ESP-IDF example
//!
//! `esp-idf-hal`'s `PinDriver` implements `OutputPin`, so you can use it directly:
//!
//! ```ignore
//! use esp_idf_hal::gpio::PinDriver;
//! use led_effects::{SimpleLed, StatusLed};
//!
//! let pin = PinDriver::output(peripherals.pins.gpio35)?;
//! let mut led = SimpleLed::new(pin);
//! led.set_color(rgb::RGB8::new(0, 0, 255))?;
//! ```

use crate::{exceeds_threshold, StatusLed, DEFAULT_BRIGHTNESS_THRESHOLD};
use embedded_hal::digital::OutputPin;
use rgb::RGB8;

/// Simple on/off LED that implements StatusLed.
///
/// Converts RGB colors to on/off by checking if any color channel
/// exceeds the brightness threshold (strict greater-than comparison).
/// Equality does not turn the LED on, avoiding false triggers from low-level noise.
pub struct SimpleLed<P: OutputPin> {
    pub(crate) pin: P,
    threshold: u8,
}

impl<P: OutputPin> SimpleLed<P> {
    /// Creates a new SimpleLed with the default brightness threshold (10).
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            threshold: DEFAULT_BRIGHTNESS_THRESHOLD,
        }
    }

    /// Creates a new SimpleLed with a custom brightness threshold.
    ///
    /// The LED turns on when any RGB channel exceeds this threshold.
    pub fn with_threshold(pin: P, threshold: u8) -> Self {
        Self { pin, threshold }
    }
}

impl<P: OutputPin> StatusLed for SimpleLed<P> {
    /// The error type is determined by the pin implementation.
    /// For HALs with infallible GPIO (e.g., mock pins), this is `core::convert::Infallible`.
    type Error = P::Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        if exceeds_threshold(color, self.threshold) {
            self.pin.set_high()
        } else {
            self.pin.set_low()
        }
    }
}

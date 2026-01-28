//! Simple GPIO LED adapter for StatusLed trait.
//!
//! Maps RGB colors to on/off based on a brightness threshold.
//! Use this for boards with a simple on/off LED instead of an RGB LED.
//!
//! The actual threshold logic is in the platform-independent [`exceeds_threshold`](crate::exceeds_threshold)
//! function, keeping this module as a thin hardware wrapper.
//!
//! # Example
//!
//! ```ignore
//! use esp_idf_hal::gpio::PinDriver;
//! use led_effects::{SimpleLed, StatusLed};
//!
//! let pin = PinDriver::output(peripherals.pins.gpio35)?;
//! let mut led = SimpleLed::new(pin);
//!
//! // LED turns on (any color with brightness > threshold)
//! led.set_color(rgb::RGB8::new(0, 0, 255))?;
//!
//! // LED turns off (all channels below threshold)
//! led.set_color(rgb::RGB8::new(0, 0, 0))?;
//! ```

use crate::{exceeds_threshold, StatusLed, DEFAULT_BRIGHTNESS_THRESHOLD};
use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::sys::EspError;
use rgb::RGB8;

/// Simple on/off LED that implements StatusLed.
///
/// Converts RGB colors to on/off by checking if any color channel
/// exceeds the brightness threshold.
pub struct SimpleLed<'d, P: OutputPin> {
    pin: PinDriver<'d, P, Output>,
    threshold: u8,
}

impl<'d, P: OutputPin> SimpleLed<'d, P> {
    /// Creates a new SimpleLed with the default brightness threshold (10).
    pub fn new(pin: PinDriver<'d, P, Output>) -> Self {
        Self {
            pin,
            threshold: DEFAULT_BRIGHTNESS_THRESHOLD,
        }
    }

    /// Creates a new SimpleLed with a custom brightness threshold.
    ///
    /// The LED turns on when any RGB channel exceeds this threshold.
    pub fn with_threshold(pin: PinDriver<'d, P, Output>, threshold: u8) -> Self {
        Self { pin, threshold }
    }
}

impl<P: OutputPin> StatusLed for SimpleLed<'_, P> {
    type Error = EspError;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        if exceeds_threshold(color, self.threshold) {
            self.pin.set_high()
        } else {
            self.pin.set_low()
        }
    }
}

//! Discrete RGB LED adapter for the [`StatusLed`](crate::StatusLed) trait.
//!
//! Drives a **plain (non-WS2812) RGB LED** — three separate GPIO channels for
//! red, green, and blue — from an [`RGB8`] colour. Each channel is switched on
//! or off independently based on a brightness threshold, giving eight possible
//! colours (on/off per channel). For smooth colour mixing you need a PWM-backed
//! adapter instead; this one is a thin on/off wrapper.
//!
//! Unlike the addressable WS2812 LEDs the rest of this workspace targets, a
//! discrete RGB LED is just three ordinary output pins, so no bit-encoding or
//! RMT peripheral is involved.
//!
//! The per-channel threshold decision lives in the platform-independent
//! [`channel_on`](crate::channel_on) function, keeping this module a thin
//! hardware wrapper — mirroring [`SimpleLed`](crate::SimpleLed).
//!
//! Generic over [`embedded_hal::digital::OutputPin`], so it works with any HAL
//! (ESP-IDF, esp-hal, nrf-hal, stm32-hal, or test mocks). The three pins may be
//! distinct types (HAL pin types usually differ per GPIO) but must share the
//! same error type.
//!
//! # Polarity
//!
//! Many boards wire a **common-anode** RGB LED, where a channel lights when its
//! pin is driven **low** ([`Polarity::ActiveLow`]). A common-cathode LED lights
//! when the pin is driven **high** ([`Polarity::ActiveHigh`], the default).
//!
//! # ESP-IDF example
//!
//! The Cheap Yellow Display (ESP32-2432S028R) carries a common-anode RGB LED on
//! GPIO 4 (red), 16 (green), and 17 (blue), wired **active-low**:
//!
//! ```ignore
//! use esp_idf_hal::gpio::PinDriver;
//! use pennant::{Polarity, RgbGpioLed, StatusLed};
//!
//! let r = PinDriver::output(peripherals.pins.gpio4)?;
//! let g = PinDriver::output(peripherals.pins.gpio16)?;
//! let b = PinDriver::output(peripherals.pins.gpio17)?;
//! let mut led = RgbGpioLed::new(r, g, b).with_polarity(Polarity::ActiveLow);
//! led.set_color(rgb::RGB8::new(0, 0, 255))?; // blue on
//! ```

use crate::{channel_on, StatusLed, DEFAULT_BRIGHTNESS_THRESHOLD};
use embedded_hal::digital::{ErrorType, OutputPin};
use rgb::RGB8;

/// Wiring polarity of a discrete LED channel.
///
/// Selects which pin level corresponds to the **lit / maximum-brightness** state.
/// Shared by [`RgbGpioLed`] (on/off) and [`RgbPwmLed`](crate::RgbPwmLed) (PWM); for
/// the PWM adapter, [`ActiveLow`](Polarity::ActiveLow) means the channel is
/// brightest at minimum duty (the pin spends the least time high), so the duty is
/// the inverse of brightness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Polarity {
    /// The channel is brightest when the pin is driven **high** (common-cathode).
    #[default]
    ActiveHigh,
    /// The channel is brightest when the pin is driven **low** (common-anode).
    ActiveLow,
}

/// A discrete (non-addressable) RGB LED driven over three GPIO pins.
///
/// Converts an [`RGB8`] colour to three independent on/off decisions — one per
/// channel — by comparing each component against a brightness threshold
/// (strict greater-than; equality is treated as off). The result is written to
/// the red, green, and blue pins honouring the configured [`Polarity`].
///
/// The three pin types may differ but must share the same
/// [`ErrorType::Error`], which becomes this adapter's [`StatusLed::Error`].
///
/// # Partial updates
///
/// [`set_color`](RgbGpioLed::set_color) writes the pins in the order red, then
/// green, then blue, returning immediately on the first pin error. A failure on
/// the green or blue write therefore leaves the LED partially updated (the red
/// channel already changed). The channels cannot be updated atomically over
/// three separate GPIOs, so callers needing a known state should retry or drive
/// the LED off. For the common HALs this is theoretical — GPIO writes are
/// [`Infallible`](core::convert::Infallible) on esp-hal, for instance.
pub struct RgbGpioLed<R, G, B>
where
    R: OutputPin,
    G: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
    B: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
{
    r: R,
    g: G,
    b: B,
    threshold: u8,
    polarity: Polarity,
}

impl<R, G, B> RgbGpioLed<R, G, B>
where
    R: OutputPin,
    G: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
    B: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
{
    /// Creates a new `RgbGpioLed` with the default brightness threshold
    /// ([`DEFAULT_BRIGHTNESS_THRESHOLD`]) and [`Polarity::ActiveHigh`].
    #[must_use]
    pub fn new(r: R, g: G, b: B) -> Self {
        Self {
            r,
            g,
            b,
            threshold: DEFAULT_BRIGHTNESS_THRESHOLD,
            polarity: Polarity::ActiveHigh,
        }
    }

    /// Sets a custom per-channel brightness threshold.
    ///
    /// A channel turns on when its colour component is strictly greater than
    /// this value.
    #[must_use]
    pub fn with_threshold(mut self, threshold: u8) -> Self {
        self.threshold = threshold;
        self
    }

    /// Sets the wiring polarity.
    ///
    /// Use [`Polarity::ActiveLow`] for common-anode LEDs (a channel lights when
    /// its pin is driven low).
    #[must_use]
    pub fn with_polarity(mut self, polarity: Polarity) -> Self {
        self.polarity = polarity;
        self
    }
}

/// Drives a single channel pin to reflect the desired on/off state, honouring
/// polarity.
#[inline]
fn set_channel<P: OutputPin>(pin: &mut P, on: bool, polarity: Polarity) -> Result<(), P::Error> {
    let drive_high = match polarity {
        Polarity::ActiveHigh => on,
        Polarity::ActiveLow => !on,
    };
    if drive_high {
        pin.set_high()
    } else {
        pin.set_low()
    }
}

impl<R, G, B> StatusLed for RgbGpioLed<R, G, B>
where
    R: OutputPin,
    G: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
    B: OutputPin + ErrorType<Error = <R as ErrorType>::Error>,
{
    /// The shared error type of the three pins.
    ///
    /// For HALs with infallible GPIO (e.g. esp-hal, mock pins) this is
    /// [`core::convert::Infallible`].
    type Error = <R as ErrorType>::Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        set_channel(
            &mut self.r,
            channel_on(color.r, self.threshold),
            self.polarity,
        )?;
        set_channel(
            &mut self.g,
            channel_on(color.g, self.threshold),
            self.polarity,
        )?;
        set_channel(
            &mut self.b,
            channel_on(color.b, self.threshold),
            self.polarity,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_BRIGHTNESS_THRESHOLD;
    use embedded_hal::digital::{Error, ErrorKind, ErrorType, OutputPin};

    /// Error type for [`MockPin`]; a single concrete type so all three channel
    /// pins share it (required by the equal-error-type bound).
    #[derive(Debug, PartialEq, Eq)]
    struct MockError;

    impl Error for MockError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    /// Mock output pin tracking its level, optionally failing every write.
    struct MockPin {
        is_high: bool,
        fail: bool,
    }

    impl MockPin {
        fn new() -> Self {
            Self {
                is_high: false,
                fail: false,
            }
        }

        /// A pin whose writes always return `Err`.
        fn failing() -> Self {
            Self {
                is_high: false,
                fail: true,
            }
        }
    }

    impl ErrorType for MockPin {
        type Error = MockError;
    }

    impl OutputPin for MockPin {
        fn set_low(&mut self) -> Result<(), Self::Error> {
            if self.fail {
                return Err(MockError);
            }
            self.is_high = false;
            Ok(())
        }
        fn set_high(&mut self) -> Result<(), Self::Error> {
            if self.fail {
                return Err(MockError);
            }
            self.is_high = true;
            Ok(())
        }
    }

    fn led() -> RgbGpioLed<MockPin, MockPin, MockPin> {
        RgbGpioLed::new(MockPin::new(), MockPin::new(), MockPin::new())
    }

    #[test]
    fn active_high_white_turns_all_channels_on() {
        let mut led = led();
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        assert!(led.r.is_high);
        assert!(led.g.is_high);
        assert!(led.b.is_high);
    }

    #[test]
    fn active_high_black_turns_all_channels_off() {
        let mut led = led();
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        led.set_color(RGB8::new(0, 0, 0)).unwrap();
        assert!(!led.r.is_high);
        assert!(!led.g.is_high);
        assert!(!led.b.is_high);
    }

    #[test]
    fn active_low_inverts_drive() {
        let mut led = led().with_polarity(Polarity::ActiveLow);
        // Bright colour: an "on" channel is driven LOW under active-low wiring.
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        assert!(!led.r.is_high);
        assert!(!led.g.is_high);
        assert!(!led.b.is_high);
        // Black: an "off" channel is driven HIGH under active-low wiring.
        led.set_color(RGB8::new(0, 0, 0)).unwrap();
        assert!(led.r.is_high);
        assert!(led.g.is_high);
        assert!(led.b.is_high);
    }

    #[test]
    fn mixed_colour_active_high() {
        let mut led = led();
        led.set_color(RGB8::new(200, 0, 200)).unwrap();
        assert!(led.r.is_high, "red on");
        assert!(!led.g.is_high, "green off");
        assert!(led.b.is_high, "blue on");
    }

    #[test]
    fn mixed_colour_active_low() {
        let mut led = led().with_polarity(Polarity::ActiveLow);
        led.set_color(RGB8::new(200, 0, 200)).unwrap();
        // On channels driven low, off channel driven high.
        assert!(!led.r.is_high, "red on -> low");
        assert!(led.g.is_high, "green off -> high");
        assert!(!led.b.is_high, "blue on -> low");
    }

    #[test]
    fn boundary_default_threshold() {
        let mut led = led();
        // Exactly at the threshold is off (strict greater-than).
        led.set_color(RGB8::new(DEFAULT_BRIGHTNESS_THRESHOLD, 0, 0))
            .unwrap();
        assert!(!led.r.is_high);
        // One above the threshold is on.
        led.set_color(RGB8::new(DEFAULT_BRIGHTNESS_THRESHOLD + 1, 0, 0))
            .unwrap();
        assert!(led.r.is_high);
    }

    #[test]
    fn boundary_custom_threshold() {
        let mut led = led().with_threshold(100);
        led.set_color(RGB8::new(100, 0, 0)).unwrap();
        assert!(!led.r.is_high, "at threshold -> off");
        led.set_color(RGB8::new(101, 0, 0)).unwrap();
        assert!(led.r.is_high, "above threshold -> on");
    }

    #[test]
    fn with_threshold_changes_behaviour() {
        // A colour bright enough at the default threshold (10)...
        let mut default_led = led();
        default_led.set_color(RGB8::new(50, 0, 0)).unwrap();
        assert!(default_led.r.is_high);
        // ...is below a higher custom threshold (100).
        let mut strict_led = led().with_threshold(100);
        strict_led.set_color(RGB8::new(50, 0, 0)).unwrap();
        assert!(!strict_led.r.is_high);
    }

    #[test]
    fn default_polarity_is_active_high() {
        assert_eq!(Polarity::default(), Polarity::ActiveHigh);
        let mut led = led();
        led.set_color(RGB8::new(255, 0, 0)).unwrap();
        assert!(
            led.r.is_high,
            "default polarity must drive an on channel high"
        );
    }

    #[test]
    fn builder_options_are_independent() {
        let mut led = led().with_polarity(Polarity::ActiveLow).with_threshold(100);
        // Threshold took effect: 50 <= 100 -> off. Polarity took effect: off -> high.
        led.set_color(RGB8::new(50, 200, 0)).unwrap();
        assert!(
            led.r.is_high,
            "red below custom threshold -> off -> active-low high"
        );
        assert!(
            !led.g.is_high,
            "green above custom threshold -> on -> active-low low"
        );
    }

    #[test]
    fn write_error_propagates_after_partial_update() {
        // Green pin fails; red succeeds first, blue is never reached.
        let mut led = RgbGpioLed::new(MockPin::new(), MockPin::failing(), MockPin::new());
        let result = led.set_color(RGB8::new(255, 255, 255));
        assert_eq!(result, Err(MockError));
        assert!(led.r.is_high, "red was written before the green failure");
    }

    #[test]
    fn write_error_on_blue_commits_red_and_green() {
        // Blue pin fails last; red and green are written first.
        let mut led = RgbGpioLed::new(MockPin::new(), MockPin::new(), MockPin::failing());
        let result = led.set_color(RGB8::new(255, 255, 255));
        assert_eq!(result, Err(MockError));
        assert!(led.r.is_high, "red committed before the blue failure");
        assert!(led.g.is_high, "green committed before the blue failure");
    }
}

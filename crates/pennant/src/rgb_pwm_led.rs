//! PWM-backed discrete RGB LED adapter for the [`StatusLed`](crate::StatusLed) trait.
//!
//! Drives a **plain (non-WS2812) RGB LED** — three separate PWM channels for
//! red, green, and blue — from an [`RGB8`] colour. Unlike the on/off
//! [`RgbGpioLed`](crate::RgbGpioLed), each channel's brightness is set from a
//! duty cycle, so this adapter renders **smooth colour mixing** and the full
//! range of a brightness effect such as [`PulseEffect`](crate::PulseEffect).
//!
//! Each 8-bit colour component is mapped onto the channel's duty cycle via
//! [`SetDutyCycle::set_duty_cycle_fraction`], so it works at any PWM resolution
//! (the fraction is relative to [`max_duty_cycle`](SetDutyCycle::max_duty_cycle)).
//! The mapping is pure and host-testable; the peripheral wiring (LEDC, MCPWM, …)
//! lives in the consumer, keeping this a thin, HAL-agnostic wrapper — mirroring
//! [`RgbGpioLed`](crate::RgbGpioLed).
//!
//! Generic over [`embedded_hal::pwm::SetDutyCycle`], so it works with any HAL
//! (ESP-IDF LEDC, esp-hal, nrf-hal, stm32-hal, or test mocks). The three
//! channels may be distinct types but must share the same error type.
//!
//! # Polarity
//!
//! A **common-anode** RGB LED (the Cheap Yellow Display's onboard LED) lights a
//! channel brightest when its pin sits **low**, so brightness is the *inverse*
//! of the PWM duty (fraction of time high): use [`Polarity::ActiveLow`]. A
//! common-cathode LED lights with a higher duty: [`Polarity::ActiveHigh`], the
//! default.
//!
//! # ESP-IDF example
//!
//! ```ignore
//! use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver};
//! use esp_idf_hal::units::*; // FromValueType, for `5.kHz()`
//! use pennant::{Polarity, RgbPwmLed, StatusLed};
//!
//! let timer = LedcTimerDriver::new(peripherals.ledc.timer0, &TimerConfig::new().frequency(5.kHz().into()))?;
//! let r = LedcDriver::new(peripherals.ledc.channel0, &timer, peripherals.pins.gpio4)?;
//! let g = LedcDriver::new(peripherals.ledc.channel1, &timer, peripherals.pins.gpio16)?;
//! let b = LedcDriver::new(peripherals.ledc.channel2, &timer, peripherals.pins.gpio17)?;
//! let mut led = RgbPwmLed::new(r, g, b).with_polarity(Polarity::ActiveLow);
//! led.set_color(rgb::RGB8::new(0, 0, 128))?; // half-brightness blue
//! ```

use crate::{Polarity, StatusLed};
use embedded_hal::pwm::{ErrorType, SetDutyCycle};
use rgb::RGB8;

/// Full-scale value of an 8-bit colour component, used as the fraction denominator.
const CHANNEL_FULL_SCALE: u16 = 255;

/// A discrete (non-addressable) RGB LED driven over three PWM channels.
///
/// Converts an [`RGB8`] colour to three duty cycles — one per channel — scaling
/// each 8-bit component to the channel's full range and honouring the configured
/// [`Polarity`]. This gives true brightness control, unlike the on/off
/// [`RgbGpioLed`](crate::RgbGpioLed).
///
/// Unlike [`RgbGpioLed`](crate::RgbGpioLed), there is deliberately no brightness
/// threshold (`with_threshold`): full analogue scaling — every component maps
/// straight to a duty cycle — is the whole point of a PWM adapter, so the only
/// builder knob is [`with_polarity`](RgbPwmLed::with_polarity).
///
/// The three channel types may differ but must share the same
/// [`ErrorType::Error`], which becomes this adapter's [`StatusLed::Error`].
///
/// # Partial updates
///
/// [`set_color`](RgbPwmLed::set_color) writes the channels in the order red,
/// green, then blue, returning immediately on the first error. A failure on the
/// green or blue write therefore leaves the LED partially updated. The channels
/// cannot be updated atomically over three separate peripherals, so callers
/// needing a known state should retry or drive the LED off.
pub struct RgbPwmLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
    B: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
{
    r: R,
    g: G,
    b: B,
    polarity: Polarity,
}

impl<R, G, B> RgbPwmLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
    B: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
{
    /// Creates a new `RgbPwmLed` with [`Polarity::ActiveHigh`].
    #[must_use]
    pub fn new(r: R, g: G, b: B) -> Self {
        Self {
            r,
            g,
            b,
            polarity: Polarity::ActiveHigh,
        }
    }

    /// Sets the wiring polarity.
    ///
    /// Use [`Polarity::ActiveLow`] for common-anode LEDs (a channel is brightest
    /// when its pin is driven low), such as the Cheap Yellow Display's onboard LED.
    #[must_use]
    pub fn with_polarity(mut self, polarity: Polarity) -> Self {
        self.polarity = polarity;
        self
    }
}

/// Drives a single channel to the brightness implied by `component`, honouring
/// polarity.
///
/// For [`Polarity::ActiveHigh`] the duty fraction equals the component; for
/// [`Polarity::ActiveLow`] (common-anode) it is inverted, since such a channel
/// is brightest when the pin spends the least time high.
#[inline]
fn set_channel_duty<P: SetDutyCycle>(
    channel: &mut P,
    component: u8,
    polarity: Polarity,
) -> Result<(), P::Error> {
    let num = match polarity {
        Polarity::ActiveHigh => u16::from(component),
        Polarity::ActiveLow => CHANNEL_FULL_SCALE - u16::from(component),
    };
    channel.set_duty_cycle_fraction(num, CHANNEL_FULL_SCALE)
}

impl<R, G, B> StatusLed for RgbPwmLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
    B: SetDutyCycle + ErrorType<Error = <R as ErrorType>::Error>,
{
    /// The shared error type of the three PWM channels.
    type Error = <R as ErrorType>::Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        set_channel_duty(&mut self.r, color.r, self.polarity)?;
        set_channel_duty(&mut self.g, color.g, self.polarity)?;
        set_channel_duty(&mut self.b, color.b, self.polarity)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal::pwm::{Error, ErrorKind, ErrorType, SetDutyCycle};

    /// Error type for [`MockPwm`]; a single concrete type so all three channels
    /// share it (required by the equal-error-type bound).
    #[derive(Debug, PartialEq, Eq)]
    struct MockError;

    impl Error for MockError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    /// Mock PWM channel recording the last duty written, with a configurable
    /// full scale and an optional always-fail mode.
    struct MockPwm {
        duty: u16,
        max: u16,
        fail: bool,
    }

    impl MockPwm {
        /// A channel with an 8-bit full scale (255), so a written duty equals the
        /// requested `num` for an `n/255` fraction — easy to assert against.
        fn new() -> Self {
            Self {
                duty: 0,
                max: 255,
                fail: false,
            }
        }

        /// A channel with a custom full scale, to exercise fraction scaling.
        fn with_max(max: u16) -> Self {
            Self {
                duty: 0,
                max,
                fail: false,
            }
        }

        /// A channel whose writes always return `Err`.
        fn failing() -> Self {
            Self {
                duty: 0,
                max: 255,
                fail: true,
            }
        }
    }

    impl ErrorType for MockPwm {
        type Error = MockError;
    }

    impl SetDutyCycle for MockPwm {
        fn max_duty_cycle(&self) -> u16 {
            self.max
        }
        fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
            if self.fail {
                return Err(MockError);
            }
            self.duty = duty;
            Ok(())
        }
    }

    fn led() -> RgbPwmLed<MockPwm, MockPwm, MockPwm> {
        RgbPwmLed::new(MockPwm::new(), MockPwm::new(), MockPwm::new())
    }

    #[test]
    fn active_high_white_is_full_duty() {
        let mut led = led();
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        assert_eq!(led.r.duty, 255);
        assert_eq!(led.g.duty, 255);
        assert_eq!(led.b.duty, 255);
    }

    #[test]
    fn active_high_black_is_zero_duty() {
        let mut led = led();
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        led.set_color(RGB8::new(0, 0, 0)).unwrap();
        assert_eq!(led.r.duty, 0);
        assert_eq!(led.g.duty, 0);
        assert_eq!(led.b.duty, 0);
    }

    #[test]
    fn active_high_scales_midpoint() {
        let mut led = led();
        led.set_color(RGB8::new(128, 0, 64)).unwrap();
        assert_eq!(led.r.duty, 128);
        assert_eq!(led.g.duty, 0);
        assert_eq!(led.b.duty, 64);
    }

    #[test]
    fn active_low_inverts_duty() {
        let mut led = led().with_polarity(Polarity::ActiveLow);
        // Full brightness on a common-anode channel means the pin sits low: duty 0.
        led.set_color(RGB8::new(255, 255, 255)).unwrap();
        assert_eq!(led.r.duty, 0);
        assert_eq!(led.g.duty, 0);
        assert_eq!(led.b.duty, 0);
        // Off means the pin sits high: full duty.
        led.set_color(RGB8::new(0, 0, 0)).unwrap();
        assert_eq!(led.r.duty, 255);
        assert_eq!(led.g.duty, 255);
        assert_eq!(led.b.duty, 255);
    }

    #[test]
    fn active_low_inverts_midpoint() {
        let mut led = led().with_polarity(Polarity::ActiveLow);
        led.set_color(RGB8::new(200, 0, 0)).unwrap();
        assert_eq!(led.r.duty, 55, "255 - 200");
        assert_eq!(led.g.duty, 255, "255 - 0");
        assert_eq!(led.b.duty, 255);
    }

    #[test]
    fn fraction_scales_to_channel_resolution() {
        // A 10-bit-ish channel (max 1000): duty = component * 1000 / 255.
        let mut led = RgbPwmLed::new(
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
        );
        led.set_color(RGB8::new(255, 128, 0)).unwrap();
        assert_eq!(led.r.duty, 1000, "full component -> full scale");
        // Matches SetDutyCycle::set_duty_cycle_fraction's u32 math: num*max/denom.
        assert_eq!(led.g.duty, (128_u32 * 1000 / 255) as u16);
        assert_eq!(led.b.duty, 0);
    }

    #[test]
    fn default_polarity_is_active_high() {
        assert_eq!(Polarity::default(), Polarity::ActiveHigh);
        let mut led = led();
        led.set_color(RGB8::new(255, 0, 0)).unwrap();
        assert_eq!(led.r.duty, 255, "active-high full red -> full duty");
    }

    #[test]
    fn write_error_propagates_after_partial_update() {
        // Green channel fails; red is written first, blue is never reached.
        let mut led = RgbPwmLed::new(MockPwm::new(), MockPwm::failing(), MockPwm::new());
        let result = led.set_color(RGB8::new(255, 255, 255));
        assert_eq!(result, Err(MockError));
        assert_eq!(led.r.duty, 255, "red written before the green failure");
    }

    #[test]
    fn write_error_on_blue_commits_red_and_green() {
        // Blue channel fails last; red and green are written first.
        let mut led = RgbPwmLed::new(MockPwm::new(), MockPwm::new(), MockPwm::failing());
        let result = led.set_color(RGB8::new(255, 255, 255));
        assert_eq!(result, Err(MockError));
        assert_eq!(led.r.duty, 255, "red committed before the blue failure");
        assert_eq!(led.g.duty, 255, "green committed before the blue failure");
    }

    #[test]
    fn active_low_non_symmetric_scaling_at_custom_resolution() {
        // Distinct per-channel components at a non-8-bit max: each channel must be
        // inverted (255 - component) *then* scaled by its own num*max/255 fraction —
        // locks in the per-channel math beyond the symmetric full-on/off cases.
        let mut led = RgbPwmLed::new(
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
        )
        .with_polarity(Polarity::ActiveLow);
        led.set_color(RGB8::new(64, 128, 192)).unwrap();
        assert_eq!(
            led.r.duty,
            ((255 - 64_u32) * 1000 / 255) as u16,
            "r: (255-64)/255 of 1000"
        );
        assert_eq!(
            led.g.duty,
            ((255 - 128_u32) * 1000 / 255) as u16,
            "g: (255-128)/255 of 1000"
        );
        assert_eq!(
            led.b.duty,
            ((255 - 192_u32) * 1000 / 255) as u16,
            "b: (255-192)/255 of 1000"
        );
    }

    #[test]
    fn active_low_inverts_before_scaling_to_channel_resolution() {
        // The 255-component inversion must happen before the fraction scales to a
        // non-8-bit resolution — guards against an order-of-operations regression.
        let mut led = RgbPwmLed::new(
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
            MockPwm::with_max(1000),
        )
        .with_polarity(Polarity::ActiveLow);
        led.set_color(RGB8::new(255, 0, 0)).unwrap();
        assert_eq!(
            led.r.duty, 0,
            "full brightness, active-low -> duty 0 at any resolution"
        );
        assert_eq!(
            led.g.duty, 1000,
            "off, active-low -> full duty at custom resolution"
        );
    }
}

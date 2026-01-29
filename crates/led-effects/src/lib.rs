//! LED animation effects for embedded projects.
//!
//! This crate provides reusable animation effects that work with RGB LEDs.
//! It is `no_std` compatible for embedded use.
//!
//! # StatusLed Trait
//!
//! The `StatusLed` trait provides a common interface for LED drivers that can
//! display status colors. This enables crates like `esp32-wifi-manager` to
//! show connection status without depending on a specific LED implementation.
//!
//! # SimpleLed (requires `hal` feature, enabled by default)
//!
//! For simple on/off GPIO LEDs (not RGB), use the [`SimpleLed`] adapter which
//! implements `StatusLed` by mapping RGB colors to on/off based on brightness.
//! It is generic over [`embedded_hal::digital::OutputPin`], so it works with
//! any HAL or test mock.

use rgb::RGB8;

#[cfg(feature = "hal")]
mod simple_led;

#[cfg(feature = "hal")]
pub use simple_led::SimpleLed;

/// Error type for PulseEffect configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PulseEffectError {
    /// min must be less than max
    InvalidRange { min: u8, max: u8 },
    /// step must be greater than 0
    ZeroStep,
}

impl core::fmt::Display for PulseEffectError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PulseEffectError::InvalidRange { min, max } => {
                write!(
                    f,
                    "invalid range: min ({}) must be less than max ({})",
                    min, max
                )
            }
            PulseEffectError::ZeroStep => {
                write!(f, "step must be greater than 0")
            }
        }
    }
}

/// Trait for LED status indicators.
///
/// Implement this trait for your LED driver to enable status feedback
/// in other crates like `esp32-wifi-manager`.
///
/// # Example
///
/// ```ignore
/// use led_effects::StatusLed;
/// use rgb::RGB8;
///
/// struct MyLed { /* ... */ }
///
/// impl StatusLed for MyLed {
///     type Error = MyError;
///
///     fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
///         // Set the LED color
///         Ok(())
///     }
/// }
/// ```
pub trait StatusLed {
    /// The error type returned by LED operations.
    type Error;

    /// Sets the LED to the specified color.
    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error>;
}

/// Default brightness threshold for simple on/off LED decisions.
pub const DEFAULT_BRIGHTNESS_THRESHOLD: u8 = 10;

/// Calculates the maximum channel value from an RGB color.
///
/// Returns the highest value among red, green, and blue channels.
/// Useful for simple brightness-based on/off decisions.
///
/// # Example
///
/// ```
/// use led_effects::max_channel_brightness;
/// use rgb::RGB8;
///
/// let color = RGB8::new(10, 50, 30);
/// assert_eq!(max_channel_brightness(color), 50);
/// ```
#[inline]
pub fn max_channel_brightness(color: RGB8) -> u8 {
    color.r.max(color.g).max(color.b)
}

/// Determines if an RGB color exceeds a brightness threshold.
///
/// Returns `true` if any channel is strictly greater than the threshold.
/// A color exactly equal to the threshold is considered "off".
/// Useful for converting RGB colors to simple on/off states.
///
/// # Example
///
/// ```
/// use led_effects::exceeds_threshold;
/// use rgb::RGB8;
///
/// let color = RGB8::new(0, 0, 15);
/// assert!(exceeds_threshold(color, 10));  // 15 > 10
/// assert!(!exceeds_threshold(color, 20)); // 15 <= 20
/// ```
#[inline]
pub fn exceeds_threshold(color: RGB8, threshold: u8) -> bool {
    max_channel_brightness(color) > threshold
}

/// A pulsing brightness effect that smoothly oscillates between dim and bright.
///
/// # Example
///
/// ```
/// use led_effects::PulseEffect;
///
/// let mut pulse = PulseEffect::new();
/// let base_color = (255, 0, 0); // Red
///
/// // Call update() in your main loop to get the next animation frame
/// let current_color = pulse.update(base_color);
/// ```
#[derive(Debug)]
pub struct PulseEffect {
    brightness: u8,
    increasing: bool,
    min_brightness: u8,
    max_brightness: u8,
    step: u8,
}

impl Default for PulseEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl PulseEffect {
    /// Creates a new pulse effect with default parameters.
    ///
    /// Default range: 0-30 brightness, step size: 2
    pub fn new() -> Self {
        Self {
            brightness: 0,
            increasing: true,
            min_brightness: 2,
            max_brightness: 30,
            step: 2,
        }
    }

    /// Creates a pulse effect with custom brightness range and step size.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum brightness (0-255)
    /// * `max` - Maximum brightness (0-255), must be > min
    /// * `step` - Brightness change per update call, must be > 0
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `min >= max`
    /// - `step == 0`
    pub fn with_range(min: u8, max: u8, step: u8) -> Result<Self, PulseEffectError> {
        if min >= max {
            return Err(PulseEffectError::InvalidRange { min, max });
        }
        if step == 0 {
            return Err(PulseEffectError::ZeroStep);
        }

        Ok(Self {
            brightness: min,
            increasing: true,
            min_brightness: min,
            max_brightness: max,
            step,
        })
    }

    /// Updates the effect state and returns the next color frame.
    ///
    /// Call this method repeatedly in your animation loop.
    ///
    /// # Arguments
    ///
    /// * `rgb` - Base color as (red, green, blue) tuple
    ///
    /// # Returns
    ///
    /// The color is scaled by the current brightness level
    pub fn update(&mut self, rgb: (u8, u8, u8)) -> RGB8 {
        let color = RGB8::new(
            ((rgb.0 as u16 * self.brightness as u16) / 255) as u8,
            ((rgb.1 as u16 * self.brightness as u16) / 255) as u8,
            ((rgb.2 as u16 * self.brightness as u16) / 255) as u8,
        );

        if self.increasing {
            if self.brightness >= self.max_brightness {
                self.increasing = false;
            } else {
                self.brightness = self.brightness.saturating_add(self.step);
            }
        } else if self.brightness <= self.min_brightness {
            self.increasing = true;
        } else {
            self.brightness = self.brightness.saturating_sub(self.step);
        }

        color
    }

    /// Returns the current brightness level (0-255).
    ///
    /// This value oscillates between `min_brightness` and `max_brightness`
    /// as `update()` is called repeatedly.
    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Resets the effect to its initial state.
    ///
    /// After calling this method:
    /// - Brightness is set to `min_brightness`
    /// - Direction is set to increasing
    ///
    /// Use this to restart the pulse animation from the beginning.
    pub fn reset(&mut self) {
        self.brightness = self.min_brightness;
        self.increasing = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "hal")]
    mod simple_led_tests {
        use super::*;
        use core::convert::Infallible;
        use embedded_hal::digital::{ErrorType, OutputPin};

        /// Minimal mock pin that tracks its on/off state.
        struct MockPin {
            is_high: bool,
        }

        impl MockPin {
            fn new() -> Self {
                Self { is_high: false }
            }
        }

        impl ErrorType for MockPin {
            type Error = Infallible;
        }

        impl OutputPin for MockPin {
            fn set_low(&mut self) -> Result<(), Self::Error> {
                self.is_high = false;
                Ok(())
            }
            fn set_high(&mut self) -> Result<(), Self::Error> {
                self.is_high = true;
                Ok(())
            }
        }

        #[test]
        fn test_simple_led_turns_on_for_bright_color() {
            let mut led = SimpleLed::new(MockPin::new());
            led.set_color(RGB8::new(0, 0, 255)).unwrap();
            assert!(led.pin.is_high);
        }

        #[test]
        fn test_simple_led_turns_off_for_black() {
            let mut led = SimpleLed::new(MockPin::new());
            led.set_color(RGB8::new(255, 0, 0)).unwrap();
            assert!(led.pin.is_high);
            led.set_color(RGB8::new(0, 0, 0)).unwrap();
            assert!(!led.pin.is_high);
        }

        #[test]
        fn test_simple_led_respects_custom_threshold() {
            let mut led = SimpleLed::with_threshold(MockPin::new(), 100);
            led.set_color(RGB8::new(50, 50, 50)).unwrap();
            assert!(!led.pin.is_high);
            led.set_color(RGB8::new(101, 0, 0)).unwrap();
            assert!(led.pin.is_high);
        }

        #[test]
        fn test_simple_led_at_threshold_stays_off() {
            let mut led = SimpleLed::with_threshold(MockPin::new(), 100);
            led.set_color(RGB8::new(100, 100, 100)).unwrap();
            assert!(!led.pin.is_high);
        }
    }

    #[test]
    fn test_max_channel_brightness_returns_highest() {
        assert_eq!(max_channel_brightness(RGB8::new(100, 50, 25)), 100);
        assert_eq!(max_channel_brightness(RGB8::new(25, 100, 50)), 100);
        assert_eq!(max_channel_brightness(RGB8::new(50, 25, 100)), 100);
    }

    #[test]
    fn test_max_channel_brightness_all_same() {
        assert_eq!(max_channel_brightness(RGB8::new(42, 42, 42)), 42);
    }

    #[test]
    fn test_max_channel_brightness_black() {
        assert_eq!(max_channel_brightness(RGB8::new(0, 0, 0)), 0);
    }

    #[test]
    fn test_max_channel_brightness_white() {
        assert_eq!(max_channel_brightness(RGB8::new(255, 255, 255)), 255);
    }

    #[test]
    fn test_exceeds_threshold_above() {
        assert!(exceeds_threshold(RGB8::new(0, 0, 15), 10));
        assert!(exceeds_threshold(RGB8::new(11, 0, 0), 10));
        assert!(exceeds_threshold(RGB8::new(0, 255, 0), 10));
    }

    #[test]
    fn test_exceeds_threshold_at_boundary() {
        // Equal to a threshold should return false (must be greater than)
        assert!(!exceeds_threshold(RGB8::new(10, 10, 10), 10));
    }

    #[test]
    fn test_exceeds_threshold_below() {
        assert!(!exceeds_threshold(RGB8::new(5, 5, 5), 10));
        assert!(!exceeds_threshold(RGB8::new(0, 0, 0), 10));
    }

    #[test]
    fn test_pulse_increases_then_decreases() {
        let mut pulse = PulseEffect::new();
        let mut prev_brightness = pulse.brightness();

        // Should increase initially
        for _ in 0..5 {
            pulse.update((255, 255, 255));
            assert!(pulse.brightness() >= prev_brightness);
            prev_brightness = pulse.brightness();
        }
    }

    #[test]
    fn test_custom_range() {
        let pulse = PulseEffect::with_range(10, 100, 5).unwrap();
        assert_eq!(pulse.brightness(), 10);
    }

    #[test]
    fn test_with_range_rejects_min_greater_than_max() {
        let err = PulseEffect::with_range(100, 10, 5).unwrap_err();
        assert_eq!(err, PulseEffectError::InvalidRange { min: 100, max: 10 });
    }

    #[test]
    fn test_with_range_rejects_equal_min_max() {
        let err = PulseEffect::with_range(50, 50, 5).unwrap_err();
        assert_eq!(err, PulseEffectError::InvalidRange { min: 50, max: 50 });
    }

    #[test]
    fn test_with_range_rejects_zero_step() {
        let err = PulseEffect::with_range(10, 100, 0).unwrap_err();
        assert_eq!(err, PulseEffectError::ZeroStep);
    }

    #[test]
    fn test_error_display() {
        let err = PulseEffectError::InvalidRange { min: 100, max: 10 };
        assert_eq!(
            format!("{}", err),
            "invalid range: min (100) must be less than max (10)"
        );

        let err = PulseEffectError::ZeroStep;
        assert_eq!(format!("{}", err), "step must be greater than 0");
    }
}

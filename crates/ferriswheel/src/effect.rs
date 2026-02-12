//! Common trait and types for LED ring effects.
//!
//! All effects share the [`Effect`] trait, which provides a uniform interface
//! for rendering animations into an `RGB8` buffer.

use rgb::RGB8;

/// Maximum supported number of LEDs in a ring.
///
/// This limit ensures correct hue distribution across LEDs using simple integer math.
/// LED rings larger than this are rare in practice.
/// See ADR-002 for the rationale.
pub const MAX_LEDS: usize = 256;

/// Error type for effect configuration and operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    /// The number of LEDs must be greater than 0.
    ZeroLeds,
    /// The number of LEDs exceeds the maximum supported (256).
    TooManyLeds {
        /// Number of LEDs requested.
        requested: usize,
        /// Maximum supported.
        max: usize,
    },
    /// Speed/step must be greater than 0.
    ZeroStep,
    /// On and off durations must both be greater than 0.
    ZeroDuty,
    /// Buffer is too small for the configured number of LEDs.
    BufferTooSmall {
        /// Number of LEDs configured.
        required: usize,
        /// Actual buffer size provided.
        actual: usize,
    },
}

impl core::fmt::Display for EffectError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EffectError::ZeroLeds => write!(f, "number of LEDs must be greater than 0"),
            EffectError::TooManyLeds { requested, max } => {
                write!(
                    f,
                    "too many LEDs: requested {}, maximum supported is {}",
                    requested, max
                )
            }
            EffectError::ZeroStep => write!(f, "speed must be greater than 0"),
            EffectError::ZeroDuty => {
                write!(f, "on and off durations must both be greater than 0")
            }
            EffectError::BufferTooSmall { required, actual } => {
                write!(
                    f,
                    "buffer too small: need {} LEDs, got {}",
                    required, actual
                )
            }
        }
    }
}

/// Direction of animation rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Animation rotates clockwise.
    #[default]
    Clockwise,
    /// Animation rotates counter-clockwise.
    CounterClockwise,
}

/// A reusable LED ring effect.
///
/// All effects implement this trait, allowing polymorphic usage via `&dyn Effect`.
///
/// # Example
///
/// ```
/// use ferriswheel::{Effect, RainbowEffect};
/// use rgb::RGB8;
///
/// let mut effect = RainbowEffect::new(12).unwrap();
/// let effect_ref: &dyn Effect = &effect;
///
/// let mut buffer = [RGB8::default(); 12];
/// effect_ref.current(&mut buffer).unwrap();
/// ```
pub trait Effect {
    /// Fills the buffer with current colors and advances the animation.
    ///
    /// Call this in your animation loop.
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError>;

    /// Fills the buffer with current colors without advancing the animation.
    ///
    /// Use this to read the current state without changing it.
    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError>;

    /// Resets the animation to its initial state.
    fn reset(&mut self);
}

/// Validates that the speed is greater than 0.
pub(crate) fn validate_speed(speed: u8) -> Result<(), EffectError> {
    if speed == 0 {
        return Err(EffectError::ZeroStep);
    }
    Ok(())
}

/// Validates the number of LEDs for an effect.
///
/// Returns `Ok(())` if `num_leds` is within the valid range (1..=MAX_LEDS).
pub(crate) fn validate_num_leds(num_leds: usize) -> Result<(), EffectError> {
    if num_leds == 0 {
        return Err(EffectError::ZeroLeds);
    }
    if num_leds > MAX_LEDS {
        return Err(EffectError::TooManyLeds {
            requested: num_leds,
            max: MAX_LEDS,
        });
    }
    Ok(())
}

/// Validates that the buffer is large enough for the configured number of LEDs.
pub(crate) fn validate_buffer(buffer: &[RGB8], num_leds: usize) -> Result<(), EffectError> {
    if buffer.len() < num_leds {
        return Err(EffectError::BufferTooSmall {
            required: num_leds,
            actual: buffer.len(),
        });
    }
    Ok(())
}

/// Advances a position around a ring of `num_leds` LEDs.
///
/// Returns the new position after moving `speed` steps in the given `direction`,
/// wrapping around the ring using modular arithmetic.
pub(crate) fn advance_position(
    position: u8,
    speed: u8,
    num_leds: usize,
    direction: Direction,
) -> u8 {
    match direction {
        Direction::Clockwise => (position as usize + speed as usize).rem_euclid(num_leds) as u8,
        Direction::CounterClockwise => {
            (position as isize - speed as isize).rem_euclid(num_leds as isize) as u8
        }
    }
}

/// Validates that both on and off tick durations are greater than 0.
pub(crate) fn validate_duty(on_ticks: u8, off_ticks: u8) -> Result<(), EffectError> {
    if on_ticks == 0 || off_ticks == 0 {
        return Err(EffectError::ZeroDuty);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_speed_zero() {
        assert_eq!(validate_speed(0).unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_validate_speed_valid() {
        assert!(validate_speed(1).is_ok());
        assert!(validate_speed(255).is_ok());
    }

    #[test]
    fn test_validate_num_leds_zero() {
        assert_eq!(validate_num_leds(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_validate_num_leds_valid() {
        assert!(validate_num_leds(1).is_ok());
        assert!(validate_num_leds(12).is_ok());
        assert!(validate_num_leds(MAX_LEDS).is_ok());
    }

    #[test]
    fn test_validate_num_leds_too_many() {
        assert_eq!(
            validate_num_leds(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds {
                requested: MAX_LEDS + 1,
                max: MAX_LEDS
            }
        );
    }

    #[test]
    fn test_validate_buffer_ok() {
        let buffer = [RGB8::default(); 12];
        assert!(validate_buffer(&buffer, 12).is_ok());
        assert!(validate_buffer(&buffer, 8).is_ok());
    }

    #[test]
    fn test_validate_buffer_too_small() {
        let buffer = [RGB8::default(); 8];
        assert_eq!(
            validate_buffer(&buffer, 12).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8
            }
        );
    }

    #[test]
    fn test_validate_duty_zero_on() {
        assert_eq!(validate_duty(0, 4).unwrap_err(), EffectError::ZeroDuty);
    }

    #[test]
    fn test_validate_duty_zero_off() {
        assert_eq!(validate_duty(4, 0).unwrap_err(), EffectError::ZeroDuty);
    }

    #[test]
    fn test_validate_duty_both_zero() {
        assert_eq!(validate_duty(0, 0).unwrap_err(), EffectError::ZeroDuty);
    }

    #[test]
    fn test_validate_duty_valid() {
        assert!(validate_duty(1, 1).is_ok());
        assert!(validate_duty(4, 4).is_ok());
        assert!(validate_duty(255, 1).is_ok());
    }

    #[test]
    fn test_direction_default_is_clockwise() {
        assert_eq!(Direction::default(), Direction::Clockwise);
    }

    #[test]
    fn test_advance_position_clockwise() {
        assert_eq!(advance_position(0, 1, 8, Direction::Clockwise), 1);
        assert_eq!(advance_position(5, 3, 8, Direction::Clockwise), 0);
    }

    #[test]
    fn test_advance_position_counter_clockwise() {
        assert_eq!(advance_position(3, 1, 8, Direction::CounterClockwise), 2);
        assert_eq!(advance_position(0, 1, 8, Direction::CounterClockwise), 7);
    }

    #[test]
    fn test_advance_position_wraps_with_large_speed() {
        assert_eq!(advance_position(0, 10, 8, Direction::Clockwise), 2);
        assert_eq!(advance_position(0, 10, 8, Direction::CounterClockwise), 6);
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", EffectError::ZeroLeds),
            "number of LEDs must be greater than 0"
        );
        assert_eq!(
            format!(
                "{}",
                EffectError::TooManyLeds {
                    requested: 300,
                    max: 256
                }
            ),
            "too many LEDs: requested 300, maximum supported is 256"
        );
        assert_eq!(
            format!("{}", EffectError::ZeroStep),
            "speed must be greater than 0"
        );
        assert_eq!(
            format!("{}", EffectError::ZeroDuty),
            "on and off durations must both be greater than 0"
        );
        assert_eq!(
            format!(
                "{}",
                EffectError::BufferTooSmall {
                    required: 12,
                    actual: 8
                }
            ),
            "buffer too small: need 12 LEDs, got 8"
        );
    }
}

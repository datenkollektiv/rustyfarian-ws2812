//! Rainbow animation effect for LED rings.
//!
//! Creates smooth rainbow animations that cycle through the full color spectrum.
//! Works with any LED ring size.

use crate::hsv::hsv_to_rgb;
use rgb::RGB8;

/// Maximum supported number of LEDs in a ring.
///
/// This limit ensures correct hue distribution across LEDs using simple integer math.
/// LED rings larger than this are rare in practice.
/// See ADR-002 for the rationale.
pub const MAX_LEDS: usize = 256;

/// Error type for RainbowEffect configuration and operations.
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

/// Direction of the rainbow animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Rainbow rotates clockwise (hue increases).
    #[default]
    Clockwise,
    /// Rainbow rotates counter-clockwise (hue decreases).
    CounterClockwise,
}

/// A rainbow animation effect for LED rings.
///
/// Creates a smooth rainbow gradient across all LEDs that animates
/// by rotating the colors around the ring.
///
/// # Example
///
/// ```
/// use ferriswheel::{RainbowEffect, Direction};
/// use rgb::RGB8;
///
/// let mut rainbow = RainbowEffect::new(12).unwrap();
/// let mut buffer = [RGB8::default(); 12];
///
/// // Fill the buffer with rainbow colors and advance animation
/// rainbow.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct RainbowEffect {
    num_leds: usize,
    hue_offset: u8,
    speed: u8,
    brightness: u8,
    saturation: u8,
    direction: Direction,
}

impl RainbowEffect {
    /// Creates a new rainbow effect for the specified number of LEDs.
    ///
    /// # Arguments
    ///
    /// * `num_leds` - Number of LEDs in the ring (must be > 0)
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    ///
    /// # Default Configuration
    ///
    /// - Speed: 1 (slow rotation)
    /// - Brightness: 255 (full)
    /// - Saturation: 255 (full)
    /// - Direction: Clockwise
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        if num_leds == 0 {
            return Err(EffectError::ZeroLeds);
        }
        if num_leds > MAX_LEDS {
            return Err(EffectError::TooManyLeds {
                requested: num_leds,
                max: MAX_LEDS,
            });
        }

        Ok(Self {
            num_leds,
            hue_offset: 0,
            speed: 1,
            brightness: 255,
            saturation: 255,
            direction: Direction::Clockwise,
        })
    }

    /// Sets the animation speed (hue increment per update).
    ///
    /// Higher values make the rainbow rotate faster.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroStep` if `speed` is 0.
    pub fn with_speed(mut self, speed: u8) -> Result<Self, EffectError> {
        if speed == 0 {
            return Err(EffectError::ZeroStep);
        }
        self.speed = speed;
        Ok(self)
    }

    /// Sets the brightness level (0-255).
    ///
    /// Controls the overall brightness of the rainbow colors.
    pub fn with_brightness(mut self, brightness: u8) -> Self {
        self.brightness = brightness;
        self
    }

    /// Sets the saturation level (0-255).
    ///
    /// Lower values produce more pastel colors.
    pub fn with_saturation(mut self, saturation: u8) -> Self {
        self.saturation = saturation;
        self
    }

    /// Sets the animation direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Resets the animation to its initial state.
    ///
    /// Sets the hue offset back to 0, restarting the animation.
    pub fn reset(&mut self) {
        self.hue_offset = 0;
    }

    /// Fills the buffer with current rainbow colors without advancing the animation.
    ///
    /// Use this when you need to read the current state multiple times
    /// without changing the animation position.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::BufferTooSmall` if the buffer has fewer
    /// elements than `num_leds`.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        if buffer.len() < self.num_leds {
            return Err(EffectError::BufferTooSmall {
                required: self.num_leds,
                actual: buffer.len(),
            });
        }

        for (i, pixel) in buffer.iter_mut().take(self.num_leds).enumerate() {
            // Spread the full hue range (0-255) evenly across all LEDs.
            // Multiply first to avoid integer division truncation issues.
            let led_hue = ((i as u32 * 256) / self.num_leds as u32) as u8;
            let hue = led_hue.wrapping_add(self.hue_offset);

            *pixel = hsv_to_rgb(hue, self.saturation, self.brightness);
        }

        Ok(())
    }

    /// Fills the buffer with rainbow colors and advances the animation.
    ///
    /// Call this method in your animation loop to update the LED colors.
    /// Each call advances the rainbow rotation by the configured speed.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::BufferTooSmall` if the buffer has fewer
    /// elements than `num_leds`.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;

        // Advance the animation
        match self.direction {
            Direction::Clockwise => {
                self.hue_offset = self.hue_offset.wrapping_add(self.speed);
            }
            Direction::CounterClockwise => {
                self.hue_offset = self.hue_offset.wrapping_sub(self.speed);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        let result = RainbowEffect::new(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = RainbowEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_max_leds_succeeds() {
        let effect = RainbowEffect::new(MAX_LEDS).unwrap();
        assert_eq!(effect.num_leds(), MAX_LEDS);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        let result = RainbowEffect::new(MAX_LEDS + 1);
        assert_eq!(
            result.unwrap_err(),
            EffectError::TooManyLeds {
                requested: MAX_LEDS + 1,
                max: MAX_LEDS
            }
        );
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        let result = RainbowEffect::new(12).unwrap().with_speed(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_with_speed_valid_succeeds() {
        let effect = RainbowEffect::new(12).unwrap().with_speed(5).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = RainbowEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        let result = effect.current(&mut buffer);
        assert_eq!(
            result.unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8
            }
        );
    }

    #[test]
    fn test_update_fills_buffer_with_different_colors() {
        let mut effect = RainbowEffect::new(6).unwrap();
        let mut buffer = [RGB8::default(); 6];

        effect.update(&mut buffer).unwrap();

        // Each LED should have a different color (different hue)
        // At least some colors should be distinct
        let first = buffer[0];
        let middle = buffer[3];
        assert_ne!(first, middle, "LEDs should have different colors");
    }

    #[test]
    fn test_update_advances_hue_offset() {
        let mut effect = RainbowEffect::new(12).unwrap().with_speed(10).unwrap();
        let mut buffer1 = [RGB8::default(); 12];
        let mut buffer2 = [RGB8::default(); 12];

        effect.update(&mut buffer1).unwrap();
        effect.update(&mut buffer2).unwrap();

        // After two updates, the colors should have shifted
        assert_ne!(
            buffer1[0], buffer2[0],
            "Colors should change between updates"
        );
    }

    #[test]
    fn test_counter_clockwise_decrements_hue() {
        let mut effect = RainbowEffect::new(12)
            .unwrap()
            .with_speed(30)
            .unwrap()
            .with_direction(Direction::CounterClockwise);

        let mut buffer1 = [RGB8::default(); 12];
        let mut buffer2 = [RGB8::default(); 12];

        effect.update(&mut buffer1).unwrap();
        effect.update(&mut buffer2).unwrap();

        // Colors should have shifted between updates
        assert_ne!(
            buffer1[0], buffer2[0],
            "Counter-clockwise should shift colors"
        );
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = RainbowEffect::new(12).unwrap().with_speed(50).unwrap();
        let mut buffer_initial = [RGB8::default(); 12];
        let mut buffer_after_reset = [RGB8::default(); 12];

        effect.current(&mut buffer_initial).unwrap();

        // Advance the animation several times
        let mut temp_buffer = [RGB8::default(); 12];
        for _ in 0..10 {
            effect.update(&mut temp_buffer).unwrap();
        }

        // Reset and check
        effect.reset();
        effect.current(&mut buffer_after_reset).unwrap();

        assert_eq!(buffer_initial, buffer_after_reset);
    }

    #[test]
    fn test_current_does_not_advance_animation() {
        let effect = RainbowEffect::new(12).unwrap();
        let mut buffer1 = [RGB8::default(); 12];
        let mut buffer2 = [RGB8::default(); 12];

        effect.current(&mut buffer1).unwrap();
        effect.current(&mut buffer2).unwrap();

        assert_eq!(buffer1, buffer2, "current() should not change state");
    }

    #[test]
    fn test_with_brightness_affects_output() {
        let bright_effect = RainbowEffect::new(1).unwrap().with_brightness(255);
        let dim_effect = RainbowEffect::new(1).unwrap().with_brightness(50);

        let mut bright_buffer = [RGB8::default(); 1];
        let mut dim_buffer = [RGB8::default(); 1];

        bright_effect.current(&mut bright_buffer).unwrap();
        dim_effect.current(&mut dim_buffer).unwrap();

        // Bright LED should have higher values
        let bright_max = bright_buffer[0]
            .r
            .max(bright_buffer[0].g)
            .max(bright_buffer[0].b);
        let dim_max = dim_buffer[0].r.max(dim_buffer[0].g).max(dim_buffer[0].b);
        assert!(bright_max > dim_max);
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

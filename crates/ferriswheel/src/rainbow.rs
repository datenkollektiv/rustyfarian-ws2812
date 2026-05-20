//! Rainbow animation effect for LED rings.
//!
//! Creates smooth rainbow animations that cycle through the full color spectrum.
//! Supports ring sizes in the range `1..=MAX_LEDS`.

use crate::effect::{
    validate_buffer, validate_num_leds, validate_speed, Direction, Effect, EffectError,
};
use crate::hsv::hsv_to_rgb;
use rgb::RGB8;

/// A rainbow animation effect for LED rings.
///
/// Creates a smooth rainbow gradient across all LEDs that animates
/// by rotating the colors around the ring.
///
/// # Example
///
/// ```
/// use ferriswheel::{RainbowEffect, Direction};
/// use ferriswheel::RGB8;
///
/// let mut rainbow = RainbowEffect::new(12).unwrap();
/// let mut buffer = [RGB8::default(); 12];
///
/// // Fill the buffer with rainbow colors and advance animation
/// rainbow.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
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
    /// * `num_leds` - Number of LEDs in the ring (`1..=MAX_LEDS`)
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    ///
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Speed: 1 (slow rotation)
    /// - Brightness: 255 (full)
    /// - Saturation: 255 (full)
    /// - Direction: Clockwise
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

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
        validate_speed(speed)?;
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

    /// Sets the initial hue offset (builder).
    pub fn with_hue_offset(mut self, offset: u8) -> Self {
        self.hue_offset = offset;
        self
    }

    /// Sets the hue offset without resetting the animation.
    ///
    /// Use this to shift the rainbow color live (e.g., from a rotary encoder)
    /// without restarting the rotation cycle.
    pub fn set_hue_offset(&mut self, offset: u8) {
        self.hue_offset = offset;
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
        validate_buffer(buffer, self.num_leds)?;

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

impl Effect for RainbowEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.update(buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::MAX_LEDS;

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
    fn test_hue_distribution_matches_adr_002() {
        // Regression guard for ADR 002 (docs/adr/002-rainbow-effect-led-limit.md):
        // hues are computed multiplication-first as `floor(i * 256 / num_leds)`
        // — not `i * floor(256 / num_leds)` — so the gradient stays smooth on
        // ring sizes that don't divide 256 evenly. The 256 here is the size of
        // the u8 hue space; coincidentally also `MAX_LEDS`, but unrelated.
        let effect = RainbowEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        let expected_hues = [0u8, 21, 42, 64, 85, 106, 128, 149, 170, 192, 213, 234];

        for (i, expected_hue) in expected_hues.into_iter().enumerate() {
            assert_eq!(
                buffer[i],
                hsv_to_rgb(expected_hue, 255, 255),
                "unexpected hue mapping at LED {}",
                i
            );
        }
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
        // Both effects start at hue_offset 0 with speed=1.
        // After one update, CW increments hue_offset by 1 (wrapping_add),
        // CCW decrements hue_offset by 1 (wrapping_sub → 255).
        // The second call to current() reflects the post-update state.
        let mut cw_effect = RainbowEffect::new(12).unwrap().with_speed(1).unwrap();
        let mut ccw_effect = RainbowEffect::new(12)
            .unwrap()
            .with_speed(1)
            .unwrap()
            .with_direction(Direction::CounterClockwise);

        let mut cw_buf = [RGB8::default(); 12];
        let mut ccw_buf = [RGB8::default(); 12];

        // update() renders current state, then advances hue_offset
        cw_effect.update(&mut cw_buf).unwrap();
        ccw_effect.update(&mut ccw_buf).unwrap();

        // Now read the post-update state for both
        cw_effect.current(&mut cw_buf).unwrap();
        ccw_effect.current(&mut ccw_buf).unwrap();

        // CW hue_offset is now 1; CCW hue_offset is now 255 (wrapped downward).
        // LED 0 color: CW = hsv(1,255,255), CCW = hsv(255,255,255).
        // These must differ, confirming CW and CCW advance in opposite directions.
        assert_ne!(
            cw_buf[0], ccw_buf[0],
            "CW and CCW effects starting at hue_offset=0 must produce different colors after one step"
        );

        // Additionally verify CCW shifts colors between consecutive updates
        let mut buffer1 = [RGB8::default(); 12];
        let mut buffer2 = [RGB8::default(); 12];
        let mut ccw2 = RainbowEffect::new(12)
            .unwrap()
            .with_speed(30)
            .unwrap()
            .with_direction(Direction::CounterClockwise);
        ccw2.update(&mut buffer1).unwrap();
        ccw2.update(&mut buffer2).unwrap();
        assert_ne!(
            buffer1[0], buffer2[0],
            "Counter-clockwise should shift colors between updates"
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
    fn test_trait_object_current() {
        let effect = RainbowEffect::new(12).unwrap();
        let effect_ref: &dyn Effect = &effect;

        let mut buffer = [RGB8::default(); 12];
        effect_ref.current(&mut buffer).unwrap();

        // Should produce non-black colors (hue 0 = red at full brightness)
        assert_ne!(buffer[0], RGB8::default());
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = RainbowEffect::new(12).unwrap().with_speed(10).unwrap();
        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buffer1 = [RGB8::default(); 12];
        let mut buffer2 = [RGB8::default(); 12];

        effect_ref.update(&mut buffer1).unwrap();
        effect_ref.update(&mut buffer2).unwrap();

        assert_ne!(buffer1[0], buffer2[0], "trait update should advance state");
    }

    #[test]
    fn test_with_hue_offset_changes_initial_colors() {
        let effect_zero = RainbowEffect::new(6).unwrap();
        let effect_offset = RainbowEffect::new(6).unwrap().with_hue_offset(128);

        let mut buf_zero = [RGB8::default(); 6];
        let mut buf_offset = [RGB8::default(); 6];

        effect_zero.current(&mut buf_zero).unwrap();
        effect_offset.current(&mut buf_offset).unwrap();

        assert_ne!(
            buf_zero[0], buf_offset[0],
            "different hue offsets should produce different colors"
        );
    }

    #[test]
    fn test_set_hue_offset_changes_output_without_reset() {
        let mut effect = RainbowEffect::new(6).unwrap().with_speed(10).unwrap();
        let mut buffer = [RGB8::default(); 6];

        // Advance a few ticks so animation is in progress
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }

        // Capture colors before and after set_hue_offset
        let mut before = [RGB8::default(); 6];
        effect.current(&mut before).unwrap();

        effect.set_hue_offset(200);

        let mut after = [RGB8::default(); 6];
        effect.current(&mut after).unwrap();

        assert_ne!(
            before[0], after[0],
            "set_hue_offset should change rendered colors"
        );
    }

    #[test]
    fn test_trait_object_reset() {
        let mut effect = RainbowEffect::new(12).unwrap().with_speed(50).unwrap();

        let mut buffer_initial = [RGB8::default(); 12];
        effect.current(&mut buffer_initial).unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;
        let mut temp = [RGB8::default(); 12];
        for _ in 0..5 {
            effect_ref.update(&mut temp).unwrap();
        }
        effect_ref.reset();

        let mut buffer_after = [RGB8::default(); 12];
        effect_ref.current(&mut buffer_after).unwrap();

        assert_eq!(buffer_initial, buffer_after);
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = RainbowEffect::new(4).unwrap();
        let mut buffer = [sentinel; 8];
        effect.current(&mut buffer).unwrap();
        for i in 4..8 {
            assert_eq!(
                buffer[i], sentinel,
                "LED {} beyond num_leds must not be modified",
                i
            );
        }
    }
}

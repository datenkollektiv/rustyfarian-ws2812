//! Rotating dot with a fading tail effect for LED rings.
//!
//! A single bright LED rotates around the ring with a fading tail behind it.

use crate::effect::{
    advance_position, validate_buffer, validate_num_leds, validate_speed, Direction, Effect,
    EffectError,
};
use crate::util::scale_brightness;
use rgb::RGB8;

/// A rotating spinner effect with a fading tail.
///
/// A bright head LED rotates around the ring, followed by a tail of LEDs
/// with linearly decreasing brightness.
///
/// # Example
///
/// ```
/// use ferriswheel::{SpinnerEffect, Effect, Direction};
/// use ferriswheel::RGB8;
///
/// let mut spinner = SpinnerEffect::new(12).unwrap()
///     .with_color(RGB8::new(0, 255, 0))
///     .with_tail_length(4);
/// let mut buffer = [RGB8::default(); 12];
///
/// spinner.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerEffect {
    num_leds: usize,
    color: RGB8,
    position: u8,
    speed: u8,
    tail_length: u8,
    direction: Direction,
}

impl SpinnerEffect {
    /// Creates a new spinner effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Color: white (255, 255, 255)
    /// - Speed: 1
    /// - Tail length: 2
    /// - Direction: Clockwise
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            position: 0,
            speed: 1,
            tail_length: 2,
            direction: Direction::Clockwise,
        })
    }

    /// Sets the spinner color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
        self
    }

    /// Sets the animation speed (position increment per update).
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroStep` if `speed` is 0.
    pub fn with_speed(mut self, speed: u8) -> Result<Self, EffectError> {
        validate_speed(speed)?;
        self.speed = speed;
        Ok(self)
    }

    /// Sets the number of LEDs in the fading tail behind the head.
    ///
    /// Clamped to `num_leds` to prevent the tail from wrapping around
    /// and overwriting the head.
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        let max_tail = self.num_leds.min(u8::MAX as usize) as u8;
        self.tail_length = tail_length.min(max_tail);
        self
    }

    /// Sets the rotation direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the spinner color without resetting the animation position.
    ///
    /// Use this to change the color live (e.g., from a rotary encoder)
    /// without restarting the spin cycle.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current spinner state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let n = self.num_leds;
        let head = self.position as usize % n;

        // Clear all LEDs
        for led in buffer.iter_mut().take(n) {
            *led = RGB8::new(0, 0, 0);
        }

        // Head at full brightness
        buffer[head] = self.color;

        // Tail with linearly decreasing brightness.
        // Cap to n-1: with tail_length = n we'd have n + 1 LEDs total (head + n tail),
        // so the last tail LED would land back on the head position.
        let effective_tail = (self.tail_length as usize).min(n.saturating_sub(1));
        let total = effective_tail + 1; // head + effective tail LEDs
        for i in 1..=effective_tail {
            let tail_idx = match self.direction {
                Direction::Clockwise => (head + n - i) % n,
                Direction::CounterClockwise => (head + i) % n,
            };
            // Linear fade: tail LED 1 is brightest, last is dimmest.
            // Use u16 arithmetic to avoid truncation, then clamp to at least 1 so every
            // tail LED stays visibly lit regardless of tail length.
            let brightness = ((255u16 * (total - i) as u16) / total as u16).max(1) as u8;
            buffer[tail_idx] = scale_brightness(self.color, brightness);
        }

        Ok(())
    }

    /// Fills the buffer with spinner state and advances the animation.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;
        self.position = advance_position(self.position, self.speed, self.num_leds, self.direction);
        Ok(())
    }

    /// Resets the animation to its initial state.
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

impl Effect for SpinnerEffect {
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

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(SpinnerEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = SpinnerEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        let result = SpinnerEffect::new(12).unwrap().with_speed(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = SpinnerEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        assert_eq!(
            effect.current(&mut buffer).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8
            }
        );
    }

    #[test]
    fn test_head_at_full_brightness() {
        let effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(2);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Head is at position 0
        assert_eq!(buffer[0], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_tail_fade_ordering() {
        let effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_tail_length(3);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Head at 0, tail at 7, 6, 5 (clockwise, behind head)
        let head_brightness = buffer[0].r;
        let tail1_brightness = buffer[7].r;
        let tail2_brightness = buffer[6].r;
        let tail3_brightness = buffer[5].r;

        assert_eq!(head_brightness, 255);
        assert!(
            tail1_brightness > tail2_brightness,
            "closer tail should be brighter"
        );
        assert!(
            tail2_brightness > tail3_brightness,
            "closer tail should be brighter"
        );
        assert!(
            tail3_brightness > 0,
            "last tail LED should still have some brightness"
        );
    }

    #[test]
    fn test_non_tail_leds_are_off() {
        let effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(2);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Head at 0, tail at 7, 6. LEDs 1-5 should be off
        for i in 1..=5 {
            assert_eq!(buffer[i], RGB8::new(0, 0, 0), "LED {} should be off", i);
        }
    }

    #[test]
    fn test_clockwise_advances_position() {
        let mut effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        effect.update(&mut buffer).unwrap();
        // After first update, head was at 0, now at 1
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[1], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_counter_clockwise_direction() {
        let mut effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_direction(Direction::CounterClockwise)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        effect.update(&mut buffer).unwrap();
        // After the first update, head was at 0, now at 7 (wrapped backward)
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[7], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_wrapping_around_ring() {
        let mut effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0))
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        // Advance 8 times to wrap around
        for _ in 0..8 {
            effect.update(&mut buffer).unwrap();
        }
        // Should be back at position 0
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RGB8::new(0, 255, 0));
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = SpinnerEffect::new(8).unwrap().with_speed(3).unwrap();

        let mut initial = [RGB8::default(); 8];
        effect.current(&mut initial).unwrap();

        let mut temp = [RGB8::default(); 8];
        for _ in 0..10 {
            effect.update(&mut temp).unwrap();
        }

        effect.reset();
        let mut after_reset = [RGB8::default(); 8];
        effect.current(&mut after_reset).unwrap();

        assert_eq!(initial, after_reset);
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 0, 255))
            .with_speed(2)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "spinner should advance between updates");
    }

    #[test]
    fn test_set_color_does_not_reset_position() {
        let mut effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_speed(3)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        // Advance to a non-zero position
        for _ in 0..3 {
            effect.update(&mut buffer).unwrap();
        }

        // Capture position by reading which LED is the head
        let mut before = [RGB8::default(); 8];
        effect.current(&mut before).unwrap();
        let head_pos_before = before
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        // Change color — should not reset position
        effect.set_color(RGB8::new(0, 0, 255));

        let mut after = [RGB8::default(); 8];
        effect.current(&mut after).unwrap();
        let head_pos_after = after
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        assert_eq!(
            head_pos_before, head_pos_after,
            "position should be unchanged after set_color"
        );
    }

    // --- Tests for with_tail_length clamping ---

    /// with_tail_length(255) on a 4-LED ring must clamp to 4 (max_tail = min(4,255) = 4).
    /// effective_tail = min(4, 4-1) = 3, so head + 3 tail LEDs fill all 4 slots.
    /// The head LED must remain at full brightness and not be overwritten by a tail LED.
    #[test]
    fn test_tail_length_clamps_to_num_leds() {
        let color = RGB8::new(255, 255, 255);
        let effect = SpinnerEffect::new(4)
            .unwrap()
            .with_color(color)
            .with_tail_length(255); // would exceed ring size; must clamp

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        // Head at position 0 must be at full brightness
        assert_eq!(
            buffer[0], color,
            "head LED must be full brightness even when tail_length was clamped from 255 to 4"
        );
        // effective_tail = min(4, 3) = 3: LEDs 3, 2, 1 are tail (clockwise behind head)
        // All four LEDs must be non-black (head + 3 tail)
        for (i, led) in buffer.iter().enumerate() {
            assert!(
                led.r > 0 || led.g > 0 || led.b > 0,
                "LED {} should be lit (head or tail) after clamping tail_length",
                i
            );
        }
    }

    /// with_tail_length(0) must stay 0: only the head LED is lit, no tail.
    #[test]
    fn test_tail_length_zero_only_head_lit() {
        let color = RGB8::new(255, 0, 0);
        let effect = SpinnerEffect::new(4)
            .unwrap()
            .with_color(color)
            .with_tail_length(0);

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[0], color, "head LED should be lit");
        for i in 1..4 {
            assert_eq!(
                buffer[i],
                RGB8::new(0, 0, 0),
                "LED {} should be off with tail_length=0",
                i
            );
        }
    }

    /// with_tail_length within num_leds must not be clamped.
    /// tail_length=4 on an 8-LED ring: max_tail = min(8, 255) = 8, so 4 < 8, no clamp.
    #[test]
    fn test_tail_length_within_ring_not_clamped() {
        let color = RGB8::new(0, 255, 0);
        let effect = SpinnerEffect::new(8)
            .unwrap()
            .with_color(color)
            .with_tail_length(4);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Head at 0; tail at 7, 6, 5, 4 (clockwise, 4 LEDs)
        assert_eq!(buffer[0], color, "head should be full brightness");
        // LEDs 1, 2, 3 are beyond the tail and must be off
        for i in 1..=3 {
            assert_eq!(buffer[i], RGB8::new(0, 0, 0), "LED {} should be off", i);
        }
        // All four tail LEDs must be lit (non-zero)
        for i in [4usize, 5, 6, 7] {
            assert!(
                buffer[i].g > 0,
                "tail LED {} should be lit (not clamped away)",
                i
            );
        }
    }

    // --- Test for brightness floor at 1 ---

    /// On a 256-LED ring with tail_length=255, effective_tail=255 and total=256.
    /// The last tail LED (i=255) computes brightness = 255*1/256 = 0 via integer division.
    /// The floor clamps this to 1, so the LED is never fully off.
    /// Existing tests only cover tail_length=3 where this floor is never reached.
    #[test]
    fn test_brightness_floor_prevents_fully_dark_tail_led() {
        use crate::effect::MAX_LEDS;
        // MAX_LEDS must be >= 256 for this test to exercise the floor.
        // If the ring is smaller, skip gracefully by checking at runtime.
        if MAX_LEDS < 256 {
            // Cannot exercise the floor with this MAX_LEDS; nothing to assert.
            return;
        }

        let color = RGB8::new(255, 255, 255);
        let effect = SpinnerEffect::new(256)
            .unwrap()
            .with_color(color)
            .with_tail_length(255); // max_tail = min(256, 255) = 255; no further clamp

        let mut buffer = [RGB8::default(); 256];
        effect.current(&mut buffer).unwrap();

        // Head at position 0. Tail goes clockwise behind head: positions 255, 254, …, 1.
        // effective_tail = min(255, 255) = 255. total = 256.
        // Last tail LED is position 1 (i=255 in the loop, tail_idx = (0+256-255)%256 = 1).
        // brightness = 255*(256-255)/256 = 255*1/256 = 0 → floored to 1.
        let last_tail = buffer[1];
        assert!(
            last_tail.r > 0 || last_tail.g > 0 || last_tail.b > 0,
            "last tail LED must not be fully dark; brightness floor should have applied (got {:?})",
            last_tail
        );
        // scale_brightness(white, 1): r = (255 * 1) / 255 = 1.
        assert_eq!(
            last_tail.r, 1,
            "last tail LED should have r=1 after brightness floor (got {})",
            last_tail.r
        );
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = SpinnerEffect::new(4).unwrap();
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

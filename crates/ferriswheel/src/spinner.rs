//! Rotating dot with a fading tail effect for LED rings.
//!
//! A single bright LED rotates around the ring with a fading tail behind it.

use crate::effect::{
    validate_buffer, validate_num_leds, validate_speed, Direction, Effect, EffectError,
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
/// use rgb::RGB8;
///
/// let mut spinner = SpinnerEffect::new(12).unwrap()
///     .with_color(RGB8::new(0, 255, 0))
///     .with_tail_length(4);
/// let mut buffer = [RGB8::default(); 12];
///
/// spinner.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
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
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        self.tail_length = tail_length;
        self
    }

    /// Sets the rotation direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
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

        // Tail with linearly decreasing brightness
        let total = self.tail_length as usize + 1; // head + tail
        for i in 1..=self.tail_length as usize {
            let tail_idx = match self.direction {
                Direction::Clockwise => (head + n - i) % n,
                Direction::CounterClockwise => (head + i) % n,
            };
            // Linear fade: tail LED 1 is brightest, last is dimmest
            let brightness = (255 * (total - i) / total) as u8;
            buffer[tail_idx] = scale_brightness(self.color, brightness);
        }

        Ok(())
    }

    /// Fills the buffer with spinner state and advances the animation.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;

        match self.direction {
            Direction::Clockwise => {
                self.position =
                    (self.position as usize + self.speed as usize).rem_euclid(self.num_leds) as u8;
            }
            Direction::CounterClockwise => {
                self.position = (self.position as isize - self.speed as isize)
                    .rem_euclid(self.num_leds as isize) as u8;
            }
        }

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
}

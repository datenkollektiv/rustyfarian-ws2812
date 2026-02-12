//! Moving segment chase effect for LED rings.
//!
//! A solid block of lit LEDs moves around the ring, distinct from
//! [`SpinnerEffect`](crate::SpinnerEffect) which uses a single dot with a fading tail.

use crate::effect::{
    advance_position, validate_buffer, validate_num_leds, validate_speed, Direction, Effect,
    EffectError,
};
use rgb::RGB8;

/// A chase effect where a solid segment moves around the ring.
///
/// A contiguous block of LEDs at full brightness travels around the ring,
/// with all other LEDs turned off.
///
/// # Example
///
/// ```
/// use ferriswheel::{ChaseEffect, Effect, Direction};
/// use rgb::RGB8;
///
/// let mut chase = ChaseEffect::new(12).unwrap()
///     .with_color(RGB8::new(255, 0, 0))
///     .with_segment_length(4);
/// let mut buffer = [RGB8::default(); 12];
///
/// chase.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ChaseEffect {
    num_leds: usize,
    color: RGB8,
    position: u8,
    speed: u8,
    segment_length: u8,
    direction: Direction,
}

impl ChaseEffect {
    /// Creates a new chase effect for the specified number of LEDs.
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
    /// - Segment length: 3
    /// - Direction: Clockwise
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            position: 0,
            speed: 1,
            segment_length: 3,
            direction: Direction::Clockwise,
        })
    }

    /// Sets the segment color.
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

    /// Sets the number of LEDs in the moving segment.
    pub fn with_segment_length(mut self, segment_length: u8) -> Self {
        self.segment_length = segment_length;
        self
    }

    /// Sets the movement direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current chase state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let n = self.num_leds;

        // Clear all LEDs
        for led in buffer.iter_mut().take(n) {
            *led = RGB8::new(0, 0, 0);
        }

        // Fill the segment at the current position (wrapping around)
        for i in 0..self.segment_length as usize {
            let idx = (self.position as usize + i) % n;
            buffer[idx] = self.color;
        }

        Ok(())
    }

    /// Fills the buffer with chase state and advances the animation.
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

impl Effect for ChaseEffect {
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
        assert_eq!(ChaseEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = ChaseEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        let result = ChaseEffect::new(12).unwrap().with_speed(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = ChaseEffect::new(12).unwrap();
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
    fn test_segment_pixels_are_colored() {
        let effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_segment_length(3);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Segment starts at position 0, covers 0, 1, 2
        assert_eq!(buffer[0], RGB8::new(255, 0, 0));
        assert_eq!(buffer[1], RGB8::new(255, 0, 0));
        assert_eq!(buffer[2], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_non_segment_pixels_are_off() {
        let effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_segment_length(3);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // LEDs 3-7 should be off
        for i in 3..8 {
            assert_eq!(buffer[i], RGB8::new(0, 0, 0), "LED {} should be off", i);
        }
    }

    #[test]
    fn test_clockwise_advances_position() {
        let mut effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_segment_length(1)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        effect.update(&mut buffer).unwrap();
        // After first update, segment was at 0, now at 1
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[1], RGB8::new(255, 0, 0));
        assert_eq!(buffer[0], RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_counter_clockwise_direction() {
        let mut effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_segment_length(1)
            .with_direction(Direction::CounterClockwise)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        effect.update(&mut buffer).unwrap();
        // After the first update, segment was at 0, now at 7 (wrapped backward)
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[7], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_wrapping_around_ring() {
        let mut effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0))
            .with_segment_length(3)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];

        // Advance to position 6, segment covers 6, 7, 0 (wraps)
        for _ in 0..6 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[6], RGB8::new(0, 255, 0));
        assert_eq!(buffer[7], RGB8::new(0, 255, 0));
        assert_eq!(buffer[0], RGB8::new(0, 255, 0));
        assert_eq!(buffer[1], RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = ChaseEffect::new(8).unwrap().with_speed(3).unwrap();

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
        let mut effect = ChaseEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 0, 255))
            .with_speed(2)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "chase should advance between updates");
    }
}

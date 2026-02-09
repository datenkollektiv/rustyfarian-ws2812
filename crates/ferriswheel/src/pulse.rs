//! Breathing/pulsing animation effect for LED rings.
//!
//! All LEDs display the same color with brightness oscillating via a sine wave.

use crate::effect::{validate_buffer, validate_num_leds, validate_speed, Effect, EffectError};
use crate::util::{scale_brightness, sine_wave};
use rgb::RGB8;

/// A breathing/pulsing animation effect.
///
/// All LEDs share the same color, with brightness oscillating smoothly
/// between configurable minimum and maximum values using a sine wave.
///
/// # Example
///
/// ```
/// use ferriswheel::{PulseEffect, Effect};
/// use rgb::RGB8;
///
/// let mut pulse = PulseEffect::new(12).unwrap()
///     .with_color(RGB8::new(0, 0, 255));
/// let mut buffer = [RGB8::default(); 12];
///
/// pulse.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct PulseEffect {
    num_leds: usize,
    color: RGB8,
    phase: u8,
    speed: u8,
    min_brightness: u8,
    max_brightness: u8,
}

impl PulseEffect {
    /// Creates a new pulse effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Color: white (255, 255, 255)
    /// - Speed: 2
    /// - Min brightness: 0
    /// - Max brightness: 255
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            phase: 0,
            speed: 2,
            min_brightness: 0,
            max_brightness: 255,
        })
    }

    /// Sets the pulse color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
        self
    }

    /// Sets the animation speed (phase increment per update).
    ///
    /// Higher values make the pulse faster.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroStep` if `speed` is 0.
    pub fn with_speed(mut self, speed: u8) -> Result<Self, EffectError> {
        validate_speed(speed)?;
        self.speed = speed;
        Ok(self)
    }

    /// Sets the minimum brightness (0-255).
    pub fn with_min_brightness(mut self, min: u8) -> Self {
        self.min_brightness = min;
        self
    }

    /// Sets the maximum brightness (0-255).
    pub fn with_max_brightness(mut self, max: u8) -> Self {
        self.max_brightness = max;
        self
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Computes the current brightness from the sine wave phase.
    fn current_brightness(&self) -> u8 {
        let sine_val = sine_wave(self.phase) as u16;
        let range = self.max_brightness as u16 - self.min_brightness as u16;
        (self.min_brightness as u16 + (sine_val * range) / 255) as u8
    }

    /// Fills the buffer with the current pulse colors without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let brightness = self.current_brightness();
        let pixel = scale_brightness(self.color, brightness);

        for led in buffer.iter_mut().take(self.num_leds) {
            *led = pixel;
        }

        Ok(())
    }

    /// Fills the buffer with pulse colors and advances the animation.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;
        self.phase = self.phase.wrapping_add(self.speed);
        Ok(())
    }

    /// Resets the animation to its initial state.
    pub fn reset(&mut self) {
        self.phase = 0;
    }
}

impl Effect for PulseEffect {
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
        assert_eq!(PulseEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = PulseEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        let result = PulseEffect::new(12).unwrap().with_speed(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = PulseEffect::new(12).unwrap();
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
    fn test_all_leds_same_color() {
        let mut effect = PulseEffect::new(6)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0));
        let mut buffer = [RGB8::default(); 6];

        // Advance past phase 0 to get non-zero brightness
        for _ in 0..10 {
            effect.update(&mut buffer).unwrap();
        }

        // All LEDs should have the same color
        for i in 1..6 {
            assert_eq!(buffer[0], buffer[i], "LED {} should match LED 0", i);
        }
    }

    #[test]
    fn test_breathing_changes_brightness() {
        let mut effect = PulseEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_speed(20)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];

        // Collect brightness over several updates
        let mut values = Vec::new();
        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
            values.push(buffer[0].r);
        }

        // Should have varying brightness values
        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();
        assert!(
            max > min,
            "brightness should vary: min={}, max={}",
            min,
            max
        );
    }

    #[test]
    fn test_min_brightness_floor() {
        let mut effect = PulseEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_min_brightness(100)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];

        // Sample many phases
        for _ in 0..256 {
            effect.update(&mut buffer).unwrap();
            let brightness = buffer[0].r.max(buffer[0].g).max(buffer[0].b);
            assert!(
                brightness >= 99,
                "brightness {} should be >= min 100 (allowing rounding)",
                brightness
            );
        }
    }

    #[test]
    fn test_max_brightness_ceiling() {
        let mut effect = PulseEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_max_brightness(100)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];

        for _ in 0..256 {
            effect.update(&mut buffer).unwrap();
            let brightness = buffer[0].r.max(buffer[0].g).max(buffer[0].b);
            assert!(
                brightness <= 101,
                "brightness {} should be <= max 100 (allowing rounding)",
                brightness
            );
        }
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = PulseEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0))
            .with_speed(10)
            .unwrap();

        let mut initial = [RGB8::default(); 4];
        effect.current(&mut initial).unwrap();

        let mut temp = [RGB8::default(); 4];
        for _ in 0..20 {
            effect.update(&mut temp).unwrap();
        }

        effect.reset();
        let mut after_reset = [RGB8::default(); 4];
        effect.current(&mut after_reset).unwrap();

        assert_eq!(initial, after_reset);
    }

    #[test]
    fn test_current_does_not_advance() {
        let effect = PulseEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0));

        let mut buf1 = [RGB8::default(); 4];
        let mut buf2 = [RGB8::default(); 4];

        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();

        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = PulseEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_speed(50)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 4];
        let mut buf2 = [RGB8::default(); 4];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        // After advancing, colors may differ (unless the phase happens to land on the same sine value)
        // At least the trait call should not panic
    }
}

//! Breathing animation effect for LED rings.
//!
//! All LEDs display the same color with brightness oscillating smoothly
//! over a full symmetric sine cycle — no flat-bottom plateau.

use crate::effect::{validate_buffer, validate_num_leds, validate_speed, Effect, EffectError};
use crate::util::{scale_brightness, sine_full};
use rgb::RGB8;

/// A smooth breathing animation effect.
///
/// All LEDs share the same color, with brightness oscillating smoothly
/// between configurable minimum and maximum values using a full symmetric
/// sine wave. The brightness rises continuously from minimum to maximum
/// and back again, with no pause at the floor.
///
/// # Comparison with [`PulseEffect`]
///
/// [`PulseEffect`] uses a half-wave rectified sine: brightness rises to the
/// peak, falls to zero, then *pauses at zero* for roughly a quarter of the
/// cycle before rising again — producing a heartbeat or pulse character.
///
/// `BreatheEffect` uses a full symmetric sine wave: brightness rises to the
/// peak and descends symmetrically back to the minimum without any pause,
/// giving a continuous in-out breathing rhythm.
///
/// # Example
///
/// ```
/// use ferriswheel::{BreatheEffect, Effect};
/// use rgb::RGB8;
///
/// let mut breathe = BreatheEffect::new(12).unwrap()
///     .with_color(RGB8::new(0, 128, 255));
/// let mut buffer = [RGB8::default(); 12];
///
/// breathe.update(&mut buffer).unwrap();
/// ```
///
/// [`PulseEffect`]: crate::PulseEffect
#[derive(Debug, Clone)]
pub struct BreatheEffect {
    num_leds: usize,
    color: RGB8,
    phase: u8,
    speed: u8,
    min_brightness: u8,
    max_brightness: u8,
}

impl BreatheEffect {
    /// Creates a new breathe effect for the specified number of LEDs.
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

    /// Sets the breathe color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
        self
    }

    /// Sets the animation speed (phase increment per update).
    ///
    /// Higher values make the breathing cycle faster.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroStep` if `speed` is 0.
    pub fn with_speed(mut self, speed: u8) -> Result<Self, EffectError> {
        validate_speed(speed)?;
        self.speed = speed;
        Ok(self)
    }

    /// Sets the minimum brightness (0–255).
    pub fn with_min_brightness(mut self, min: u8) -> Self {
        self.min_brightness = min;
        self
    }

    /// Sets the maximum brightness (0–255).
    pub fn with_max_brightness(mut self, max: u8) -> Self {
        self.max_brightness = max;
        self
    }

    /// Sets the breathe color without resetting the animation phase.
    ///
    /// Use this to change the color live (e.g., from a rotary encoder)
    /// without restarting the breathing cycle.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    fn current_brightness(&self) -> u8 {
        let lo = self.min_brightness.min(self.max_brightness) as u16;
        let hi = self.min_brightness.max(self.max_brightness) as u16;
        let sine_val = sine_full(self.phase) as u16;
        (lo + (sine_val * (hi - lo)) / 255) as u8
    }

    /// Fills the buffer with the current breathe colors without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        let brightness = self.current_brightness();
        let pixel = scale_brightness(self.color, brightness);
        for led in buffer.iter_mut().take(self.num_leds) {
            *led = pixel;
        }
        Ok(())
    }

    /// Fills the buffer with breathe colors and advances the animation.
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

impl Effect for BreatheEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        BreatheEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        BreatheEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        BreatheEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::MAX_LEDS;

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(BreatheEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert_eq!(
            BreatheEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds {
                requested: MAX_LEDS + 1,
                max: MAX_LEDS
            }
        );
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = BreatheEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        let result = BreatheEffect::new(12).unwrap().with_speed(0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroStep);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = BreatheEffect::new(12).unwrap();
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
        let mut effect = BreatheEffect::new(6)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0));
        let mut buffer = [RGB8::default(); 6];

        for _ in 0..10 {
            effect.update(&mut buffer).unwrap();
        }

        for i in 1..6 {
            assert_eq!(buffer[0], buffer[i], "LED {} should match LED 0", i);
        }
    }

    #[test]
    fn test_breathing_changes_brightness() {
        let mut effect = BreatheEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_speed(20)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];
        let mut values = Vec::new();
        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
            values.push(buffer[0].r);
        }

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
    fn test_no_extended_brightness_plateau() {
        // With min_brightness=0 (default), count zero-brightness phases across a full
        // 256-step cycle. BreatheEffect uses a symmetric full sine: only phases at
        // exactly 0 and 255 map to zero, so at most 3 zero phases are expected.
        // PulseEffect has ~19 zero-brightness phases due to its half-wave plateau.
        let mut effect = BreatheEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];
        let mut zero_count = 0usize;
        for _ in 0..256 {
            effect.update(&mut buffer).unwrap();
            let brightness = buffer[0].r.max(buffer[0].g).max(buffer[0].b);
            if brightness == 0 {
                zero_count += 1;
            }
        }
        assert!(
            zero_count <= 3,
            "BreatheEffect should have at most 3 zero-brightness phases, got {}",
            zero_count
        );
    }

    #[test]
    fn test_inverted_brightness_range_does_not_underflow() {
        // min > max would underflow with naive u16 subtraction; the clamp must
        // treat this as the same range as (max, min) and never produce nonsense.
        let mut effect = BreatheEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_min_brightness(200)
            .with_max_brightness(100)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];
        for _ in 0..256 {
            effect.update(&mut buffer).unwrap();
            let brightness = buffer[0].r.max(buffer[0].g).max(buffer[0].b);
            assert!(
                brightness >= 99 && brightness <= 201,
                "brightness {} should be within [100, 200] (allowing rounding)",
                brightness
            );
        }
    }

    #[test]
    fn test_min_brightness_floor() {
        let mut effect = BreatheEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_min_brightness(100)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 1];
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
        let mut effect = BreatheEffect::new(1)
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
        let mut effect = BreatheEffect::new(4)
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
        let effect = BreatheEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0));

        let mut buf1 = [RGB8::default(); 4];
        let mut buf2 = [RGB8::default(); 4];

        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();

        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_set_color_does_not_reset_phase() {
        let mut effect = BreatheEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_speed(20)
            .unwrap();

        let mut buffer = [RGB8::default(); 4];
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }

        let mut before = [RGB8::default(); 4];
        effect.current(&mut before).unwrap();

        effect.set_color(RGB8::new(0, 255, 0));

        let mut after = [RGB8::default(); 4];
        effect.current(&mut after).unwrap();

        // Brightness should be unchanged (same phase), only hue differs.
        let brightness_before = before[0].r.max(before[0].g).max(before[0].b);
        let brightness_after = after[0].r.max(after[0].g).max(after[0].b);
        assert_eq!(
            brightness_before, brightness_after,
            "phase should be unchanged after set_color"
        );
        assert_ne!(before[0], after[0], "color should have changed");
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = BreatheEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_speed(50)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 4];
        let mut buf2 = [RGB8::default(); 4];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();
        // At speed=50, phase 0 (buf1) is black; phase 50 (buf2) maps to a
        // non-zero sine value — successive updates must produce different output.
        assert_ne!(
            buf1, buf2,
            "successive updates should produce different output"
        );
    }

    #[test]
    fn test_new_with_max_leds_succeeds() {
        assert!(BreatheEffect::new(MAX_LEDS).is_ok());
    }

    #[test]
    fn test_update_advances_state() {
        // At phase=0, sine_full=0 so output is black.
        // After one update at speed=20, phase advances to 20, which maps to a
        // non-zero sine value and therefore a brighter output.
        let mut effect = BreatheEffect::new(1)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_speed(20)
            .unwrap();

        let mut buf = [RGB8::default(); 1];
        effect.current(&mut buf).unwrap();
        let at_phase_0 = buf[0];

        effect.update(&mut buf).unwrap(); // advances phase 0 → 20
        effect.current(&mut buf).unwrap();
        let at_phase_20 = buf[0];

        assert_eq!(at_phase_0, RGB8::new(0, 0, 0), "phase=0 should be black");
        assert_ne!(
            at_phase_0, at_phase_20,
            "phase=20 should differ from phase=0"
        );
    }

    #[test]
    fn test_with_color_affects_output() {
        // Advance past phase=0 (where output is black regardless of color).
        // At phase=64 the sine value is non-zero, so the chosen color is visible.
        let make_output = |color: RGB8| {
            let mut e = BreatheEffect::new(1)
                .unwrap()
                .with_color(color)
                .with_speed(64)
                .unwrap();
            let mut buf = [RGB8::default(); 1];
            e.update(&mut buf).unwrap(); // advance phase 0 → 64
            e.current(&mut buf).unwrap(); // read at phase=64
            buf[0]
        };

        let red = make_output(RGB8::new(255, 0, 0));
        let blue = make_output(RGB8::new(0, 0, 255));

        assert_ne!(
            red, blue,
            "different colors should produce different output"
        );
        assert_eq!(red.g, 0, "red effect should have no green channel");
        assert_eq!(red.b, 0, "red effect should have no blue channel");
        assert_eq!(blue.r, 0, "blue effect should have no red channel");
        assert_eq!(blue.g, 0, "blue effect should have no green channel");
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = BreatheEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0))
            .with_speed(10)
            .unwrap();

        let mut initial = [RGB8::default(); 4];
        effect.current(&mut initial).unwrap();

        let effect_dyn: &mut dyn Effect = &mut effect;
        let mut buf = [RGB8::default(); 4];
        for _ in 0..10 {
            effect_dyn.update(&mut buf).unwrap();
        }

        effect_dyn.reset();

        let mut after_reset = [RGB8::default(); 4];
        effect_dyn.current(&mut after_reset).unwrap();

        assert_eq!(
            initial, after_reset,
            "reset via trait object should restore initial state"
        );
    }
}

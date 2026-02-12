//! Rapid on/off flash effect for LED rings.
//!
//! All LEDs toggle between a color and an off color on a configurable duty cycle.
//! Each [`update`](FlashEffect::update) call advances a tick counter; the phase
//! (on/off) is determined by where the counter sits in the cycle.

use crate::effect::{validate_buffer, validate_duty, validate_num_leds, Effect, EffectError};
use crate::util::fill_solid;
use rgb::RGB8;

/// A flash effect that toggles all LEDs between two colors.
///
/// The effect alternates between an on-color and an off-color based on
/// configurable tick durations for each phase.
///
/// # Example
///
/// ```
/// use ferriswheel::{FlashEffect, Effect};
/// use rgb::RGB8;
///
/// let mut flash = FlashEffect::new(8).unwrap()
///     .with_color(RGB8::new(255, 0, 0))
///     .with_duty(2, 3).unwrap();
/// let mut buffer = [RGB8::default(); 8];
///
/// flash.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct FlashEffect {
    num_leds: usize,
    color: RGB8,
    off_color: RGB8,
    on_ticks: u8,
    off_ticks: u8,
    counter: u8,
}

impl FlashEffect {
    /// Creates a new flash effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Color: white (255, 255, 255)
    /// - Off color: black (0, 0, 0)
    /// - On ticks: 4
    /// - Off ticks: 4
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            off_color: RGB8::new(0, 0, 0),
            on_ticks: 4,
            off_ticks: 4,
            counter: 0,
        })
    }

    /// Sets the on-phase color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
        self
    }

    /// Sets the off-phase color.
    pub fn with_off_color(mut self, off_color: RGB8) -> Self {
        self.off_color = off_color;
        self
    }

    /// Sets the duty cycle as on/off tick counts.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroDuty` if either `on_ticks` or `off_ticks` is 0.
    pub fn with_duty(mut self, on_ticks: u8, off_ticks: u8) -> Result<Self, EffectError> {
        validate_duty(on_ticks, off_ticks)?;
        self.on_ticks = on_ticks;
        self.off_ticks = off_ticks;
        Ok(self)
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Returns `true` if the effect is currently in the on phase.
    pub fn is_on(&self) -> bool {
        self.counter < self.on_ticks
    }

    /// Fills the buffer with the current flash state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let color = if self.is_on() {
            self.color
        } else {
            self.off_color
        };
        fill_solid(&mut buffer[..self.num_leds], color);

        Ok(())
    }

    /// Fills the buffer with flash state and advances the tick counter.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;

        let cycle = self.on_ticks as u16 + self.off_ticks as u16;
        self.counter = ((self.counter as u16 + 1) % cycle) as u8;

        Ok(())
    }

    /// Resets the animation to its initial state.
    pub fn reset(&mut self) {
        self.counter = 0;
    }
}

impl Effect for FlashEffect {
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
        assert_eq!(FlashEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = FlashEffect::new(8).unwrap();
        assert_eq!(effect.num_leds(), 8);
    }

    #[test]
    fn test_zero_duty_returns_error() {
        let result = FlashEffect::new(8).unwrap().with_duty(0, 4);
        assert_eq!(result.unwrap_err(), EffectError::ZeroDuty);

        let result = FlashEffect::new(8).unwrap().with_duty(4, 0);
        assert_eq!(result.unwrap_err(), EffectError::ZeroDuty);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = FlashEffect::new(12).unwrap();
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
    fn test_on_phase_fills_with_color() {
        let effect = FlashEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0));

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        for pixel in &buffer {
            assert_eq!(*pixel, RGB8::new(255, 0, 0));
        }
    }

    #[test]
    fn test_off_phase_fills_with_off_color() {
        let mut effect = FlashEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_duty(1, 1)
            .unwrap();

        let mut buffer = [RGB8::default(); 4];

        // First update: on phase (counter 0 < on_ticks 1), then advances to counter 1
        effect.update(&mut buffer).unwrap();

        // Now counter=1, which is >= on_ticks=1, so off phase
        effect.current(&mut buffer).unwrap();
        for pixel in &buffer {
            assert_eq!(*pixel, RGB8::new(0, 0, 0));
        }
    }

    #[test]
    fn test_counter_cycles_through_full_period() {
        let mut effect = FlashEffect::new(4).unwrap().with_duty(2, 3).unwrap();

        let mut buffer = [RGB8::default(); 4];

        // Cycle length is 2 + 3 = 5
        // counter: 0(on), 1(on), 2(off), 3(off), 4(off), 0(on) ...
        assert!(effect.is_on()); // counter=0
        effect.update(&mut buffer).unwrap();
        assert!(effect.is_on()); // counter=1
        effect.update(&mut buffer).unwrap();
        assert!(!effect.is_on()); // counter=2
        effect.update(&mut buffer).unwrap();
        assert!(!effect.is_on()); // counter=3
        effect.update(&mut buffer).unwrap();
        assert!(!effect.is_on()); // counter=4
        effect.update(&mut buffer).unwrap();
        assert!(effect.is_on()); // counter=0 (wrapped)
    }

    #[test]
    fn test_custom_off_color() {
        let mut effect = FlashEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_off_color(RGB8::new(0, 0, 255))
            .with_duty(1, 1)
            .unwrap();

        let mut buffer = [RGB8::default(); 4];

        // Advance past on phase
        effect.update(&mut buffer).unwrap();

        // Now in off phase, should use custom off_color
        effect.current(&mut buffer).unwrap();
        for pixel in &buffer {
            assert_eq!(*pixel, RGB8::new(0, 0, 255));
        }
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = FlashEffect::new(4).unwrap().with_duty(2, 2).unwrap();

        let mut buffer = [RGB8::default(); 4];

        // Advance several times
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }

        effect.reset();
        assert!(effect.is_on(), "reset should return to on phase");
    }

    #[test]
    fn test_is_on_tracks_phase() {
        let effect = FlashEffect::new(4).unwrap().with_duty(3, 2).unwrap();

        // Initial state: counter=0, on_ticks=3, so is_on
        assert!(effect.is_on());
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = FlashEffect::new(4)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_duty(1, 1)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 4];
        let mut buf2 = [RGB8::default(); 4];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "flash should toggle between updates");
    }
}

//! Meteor / comet effect for LED rings.
//!
//! A bright head LED travels around the ring with an exponentially-decaying
//! tail behind it, creating a comet-like streak that fades naturally to black.

use crate::effect::{
    advance_position, validate_buffer, validate_num_leds, validate_speed, Direction, Effect,
    EffectError,
};
use crate::util::scale_brightness;
use rgb::RGB8;

/// A meteor / comet effect with an exponentially-decaying tail.
///
/// A bright head LED travels around the ring, leaving a trail of LEDs with
/// multiplicatively-decreasing brightness — each successive tail LED is a
/// fixed fraction of the previous one's brightness.
/// This produces an exponential falloff distinct from [`SpinnerEffect`]'s
/// linear fade: the tail is bright near the head and genuinely fades to black
/// at its tip.
///
/// # Comparison with [`SpinnerEffect`](crate::SpinnerEffect)
///
/// [`SpinnerEffect`](crate::SpinnerEffect) uses a linear fade where every tail
/// LED stays at least dimly lit (brightness floored to 1).
/// `MeteorEffect` uses a multiplicative fade controlled by [`with_decay`](MeteorEffect::with_decay) —
/// tail LEDs can reach true black, producing the characteristic comet trail.
///
/// # Example
///
/// ```
/// use ferriswheel::{MeteorEffect, Effect, Direction};
/// use ferriswheel::RGB8;
///
/// let mut meteor = MeteorEffect::new(12).unwrap()
///     .with_color(RGB8::new(255, 200, 80))
///     .with_tail_length(6)
///     .with_decay(192);
/// let mut buffer = [RGB8::default(); 12];
///
/// meteor.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct MeteorEffect {
    num_leds: usize,
    color: RGB8,
    position: u8,
    speed: u8,
    tail_length: u8,
    /// Per-step multiplicative brightness factor (0–255).
    ///
    /// Each tail LED is rendered at `decay / 255` of the previous step's
    /// brightness. `255` = no decay (uniform tail). `0` = instant black
    /// (only the head is lit). Default `192` ≈ 75% per step.
    decay: u8,
    direction: Direction,
}

impl MeteorEffect {
    /// Creates a new meteor effect for the specified number of LEDs.
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
    /// - Tail length: 6
    /// - Decay: 192 (~75% per step)
    /// - Direction: Clockwise
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;
        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            position: 0,
            speed: 1,
            tail_length: 6,
            decay: 192,
            direction: Direction::Clockwise,
        })
    }

    /// Sets the meteor color.
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

    /// Sets the number of LEDs in the decaying tail behind the head.
    ///
    /// Clamped to `num_leds - 1` so the tail can never wrap around and
    /// overwrite the head LED.
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        let max_tail = self.num_leds.saturating_sub(1).min(u8::MAX as usize) as u8;
        self.tail_length = tail_length.min(max_tail);
        self
    }

    /// Sets the per-step multiplicative brightness decay factor (0–255).
    ///
    /// Each tail LED is rendered at `decay / 255` of the previous step's
    /// brightness. `255` = no decay (uniform tail at full brightness).
    /// `0` = instant black (only the head is lit). Default is `192` (~75%).
    pub fn with_decay(mut self, decay: u8) -> Self {
        self.decay = decay;
        self
    }

    /// Sets the rotation direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the meteor color without resetting the animation position.
    ///
    /// Use this to change the color live (e.g., driven by a sensor or event)
    /// without restarting the travel cycle.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current meteor state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let n = self.num_leds;
        let head = self.position as usize % n;

        buffer[..n].fill(RGB8::default());

        // Head at full brightness.
        buffer[head] = self.color;

        // Tail with multiplicative (exponential) brightness decay.
        // Each step: brightness = prev_brightness * decay / 255.
        // When brightness reaches 0 the remaining LEDs stay black (buffer already cleared).
        // tail_length is already clamped to n-1 by with_tail_length(), so no further clamp needed.
        let effective_tail = self.tail_length as usize;
        let mut brightness: u16 = 255;
        for i in 1..=effective_tail {
            brightness = brightness * self.decay as u16 / 255;
            if brightness == 0 {
                break;
            }
            let tail_idx = match self.direction {
                Direction::Clockwise => (head + n - i) % n,
                Direction::CounterClockwise => (head + i) % n,
            };
            buffer[tail_idx] = scale_brightness(self.color, brightness as u8);
        }

        Ok(())
    }

    /// Fills the buffer with the meteor state and advances the animation.
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

impl Effect for MeteorEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        MeteorEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        MeteorEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        MeteorEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(MeteorEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = MeteorEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        use crate::effect::MAX_LEDS;
        assert!(matches!(
            MeteorEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        assert_eq!(
            MeteorEffect::new(12).unwrap().with_speed(0).unwrap_err(),
            EffectError::ZeroStep
        );
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = MeteorEffect::new(12).unwrap();
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
        let color = RGB8::new(255, 0, 0);
        let effect = MeteorEffect::new(12).unwrap().with_color(color);
        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[0], color, "head LED should be at full brightness");
    }

    #[test]
    fn test_tail_decay_ordering() {
        // With decay=192: tail[1]=192, tail[2]=144, tail[3]=108, tail[4]=81 — strictly decreasing.
        let effect = MeteorEffect::new(12)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_tail_length(4)
            .with_decay(192);

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        // Head at 0; CW tail at 11, 10, 9, 8.
        // Color is white (255,255,255), so comparing .r is equivalent to comparing brightness.
        let head_b = buffer[0].r;
        let tail1_b = buffer[11].r;
        let tail2_b = buffer[10].r;
        let tail3_b = buffer[9].r;
        let tail4_b = buffer[8].r;

        assert_eq!(head_b, 255, "head must be full brightness");
        assert!(tail1_b > tail2_b, "tail1 should be brighter than tail2");
        assert!(tail2_b > tail3_b, "tail2 should be brighter than tail3");
        assert!(tail3_b > tail4_b, "tail3 should be brighter than tail4");
        assert!(tail4_b > 0, "tail4 should be non-zero with decay=192");
    }

    #[test]
    fn test_non_tail_leds_are_off() {
        // tail_length=3, head at 0, tail at 11/10/9; LEDs 1–8 must be black.
        let effect = MeteorEffect::new(12)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(3);

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        for i in 1..=8 {
            assert_eq!(buffer[i], RGB8::new(0, 0, 0), "LED {} should be off", i);
        }
    }

    #[test]
    fn test_tail_fades_to_black_with_zero_decay() {
        // decay=0: brightness = 255 * 0 / 255 = 0 on the first tail step → break.
        // Only the head should be lit.
        let effect = MeteorEffect::new(12)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_tail_length(6)
            .with_decay(0);

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[0].r, 255, "head should be at full brightness");
        for i in 1..12 {
            assert_eq!(
                buffer[i],
                RGB8::new(0, 0, 0),
                "LED {} should be black with decay=0",
                i
            );
        }
    }

    #[test]
    fn test_clockwise_advances_position() {
        let mut effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        // After first update, head was at 0, now at 1.
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[1], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_counter_clockwise_direction() {
        let mut effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_direction(Direction::CounterClockwise)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        // CCW from position 0 wraps to 7.
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[7], RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_counter_clockwise_tail_ahead_of_head() {
        // CCW: head travels backward in index space, so the tail appears at higher indices.
        let effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_tail_length(2)
            .with_decay(192)
            .with_direction(Direction::CounterClockwise);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // Head at index 0; CCW tail at indices 1, 2.
        assert_eq!(buffer[0].r, 255, "head at index 0 must be full brightness");
        assert!(buffer[1].r > 0, "first tail LED at index 1 must be lit");
        assert!(buffer[2].r > 0, "second tail LED at index 2 must be lit");
        assert!(
            buffer[1].r > buffer[2].r,
            "tail must fade away from the head"
        );
    }

    #[test]
    fn test_wrapping_around_ring() {
        let mut effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 255, 0))
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..8 {
            effect.update(&mut buffer).unwrap();
        }
        // After 8 steps, back at position 0.
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RGB8::new(0, 255, 0));
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = MeteorEffect::new(8).unwrap().with_speed(3).unwrap();

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
    fn test_set_color_does_not_reset_position() {
        let mut effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0))
            .with_tail_length(0)
            .with_speed(3)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..3 {
            effect.update(&mut buffer).unwrap();
        }

        let mut before = [RGB8::default(); 8];
        effect.current(&mut before).unwrap();
        let pos_before = before
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        effect.set_color(RGB8::new(0, 0, 255));

        let mut after = [RGB8::default(); 8];
        effect.current(&mut after).unwrap();
        let pos_after = after
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        assert_eq!(
            pos_before, pos_after,
            "position should not change after set_color"
        );
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(0, 0, 255))
            .with_speed(2)
            .unwrap();

        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "meteor should advance between updates");
    }

    #[test]
    fn test_tail_length_zero_only_head_lit() {
        let color = RGB8::new(255, 0, 0);
        let effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(color)
            .with_tail_length(0);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[0], color, "head LED should be lit");
        for i in 1..8 {
            assert_eq!(
                buffer[i],
                RGB8::new(0, 0, 0),
                "LED {} should be off with tail_length=0",
                i
            );
        }
    }

    #[test]
    fn test_current_does_not_advance() {
        let effect = MeteorEffect::new(8)
            .unwrap()
            .with_color(RGB8::new(255, 0, 0));

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();

        assert_eq!(buf1, buf2, "current() must not advance position");
    }
}

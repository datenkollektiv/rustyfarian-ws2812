//! Rainbow comet effect for LED rings.
//!
//! A bright head LED travels around the ring with a color-cycling fading tail.
//! Each successive tail LED steps further along the HSV hue wheel and decays
//! multiplicatively in brightness, producing a vivid comet-like streak where
//! the colors shift as the tail fades to black.

use crate::effect::{
    advance_position, validate_buffer, validate_num_leds, validate_speed, Direction, Effect,
    EffectError,
};
use crate::hsv::hsv_to_rgb;
use crate::util::clamp_tail_length;
use rgb::RGB8;

/// A rainbow comet effect with a hue-cycling fading tail.
///
/// A bright head LED travels around the ring at a configurable hue.
/// Each tail LED steps further along the HSV hue wheel by `hue_step` and
/// decays multiplicatively in brightness — the nearest tail LED is bright and
/// close in hue to the head; the farthest is dim and furthest along the wheel.
///
/// This is a chromatic variant of [`MeteorEffect`](crate::MeteorEffect):
/// where `MeteorEffect` keeps a single color throughout the tail, `RainbowCometEffect`
/// rotates the hue on every tail step, producing a rainbow streak behind the head.
///
/// # Example
///
/// ```
/// use ferriswheel::{RainbowCometEffect, Effect, Direction};
/// use ferriswheel::RGB8;
///
/// let mut comet = RainbowCometEffect::new(12).unwrap()
///     .with_hue(0)
///     .with_tail_length(6)
///     .with_decay(192);
/// let mut buffer = [RGB8::default(); 12];
///
/// comet.update(&mut buffer).unwrap();
/// ```
///
/// The maximum number of LEDs is [`MAX_LEDS`](crate::effect::MAX_LEDS).
#[derive(Debug, Clone, PartialEq)]
pub struct RainbowCometEffect {
    num_leds: usize,
    /// Head hue (0–255, wraps the HSV color wheel).
    hue: u8,
    /// HSV saturation applied to all LEDs (0–255).
    saturation: u8,
    /// Peak brightness for the head LED (0–255).
    brightness: u8,
    /// Hue offset added per tail LED step.
    hue_step: u8,
    /// Number of tail LEDs (clamped to `num_leds - 1`).
    tail_length: u8,
    /// Position increment per `update()` call.
    speed: u8,
    direction: Direction,
    /// Per-step multiplicative brightness factor (0–255).
    ///
    /// Each tail LED is rendered at `decay / 255` of the previous step's
    /// brightness. `255` = no decay (uniform tail at full brightness).
    /// `0` = instant black (only the head is lit). Default is `192` (~75%).
    decay: u8,
    /// Current head position (0..`num_leds - 1`).
    position: u8,
}

impl RainbowCometEffect {
    /// Creates a new rainbow comet effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::ZeroLeds`] if `num_leds` is 0.
    /// Returns [`EffectError::TooManyLeds`] if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Hue: 0 (red)
    /// - Saturation: 255
    /// - Brightness: 255
    /// - Hue step: 16
    /// - Tail length: 6
    /// - Speed: 1
    /// - Direction: Clockwise
    /// - Decay: 192 (~75% per step)
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;
        let max_tail = num_leds.saturating_sub(1).min(u8::MAX as usize) as u8;
        Ok(Self {
            num_leds,
            hue: 0,
            saturation: 255,
            brightness: 255,
            hue_step: 16,
            tail_length: 6_u8.min(max_tail),
            speed: 1,
            direction: Direction::Clockwise,
            decay: 192,
            position: 0,
        })
    }

    /// Sets the head hue (0–255, wraps the HSV color wheel).
    pub fn with_hue(mut self, hue: u8) -> Self {
        self.hue = hue;
        self
    }

    /// Sets the HSV saturation applied to all LEDs (0–255).
    pub fn with_saturation(mut self, saturation: u8) -> Self {
        self.saturation = saturation;
        self
    }

    /// Sets the peak brightness for the head LED (0–255).
    pub fn with_brightness(mut self, brightness: u8) -> Self {
        self.brightness = brightness;
        self
    }

    /// Sets the hue offset added per tail LED step.
    pub fn with_hue_step(mut self, hue_step: u8) -> Self {
        self.hue_step = hue_step;
        self
    }

    /// Sets the number of tail LEDs.
    ///
    /// Clamped to `num_leds - 1` so the tail can never wrap around and
    /// overwrite the head LED.
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        self.tail_length = clamp_tail_length(tail_length, self.num_leds);
        self
    }

    /// Sets the animation speed (position increment per update).
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::ZeroStep`] if `speed` is 0.
    pub fn with_speed(mut self, speed: u8) -> Result<Self, EffectError> {
        validate_speed(speed)?;
        self.speed = speed;
        Ok(self)
    }

    /// Sets the rotation direction.
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
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

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Sets the head hue without resetting the animation position.
    ///
    /// Use this to change the hue live (e.g., driven by a sensor or event)
    /// without restarting the travel cycle.
    pub fn set_hue(&mut self, hue: u8) {
        self.hue = hue;
    }

    /// Fills the buffer with the current comet state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let n = self.num_leds;
        let head = self.position as usize % n;

        buffer[..n].fill(RGB8::default());

        // Head at full brightness using the configured hue.
        buffer[head] = hsv_to_rgb(self.hue, self.saturation, self.brightness);

        // Tail with multiplicative (exponential) brightness decay and hue rotation.
        // Each step: brightness_val = prev_brightness_val * decay / 255.
        // When brightness_val reaches 0 the remaining LEDs stay black.
        // tail_length is already clamped to n-1 by with_tail_length(), so no further clamp needed.
        let effective_tail = self.tail_length as usize;
        let mut brightness_val: u16 = 255;
        for i in 1..=effective_tail {
            brightness_val = brightness_val * self.decay as u16 / 255;
            if brightness_val == 0 {
                break;
            }
            let tail_idx = match self.direction {
                Direction::Clockwise => (head + n - i) % n,
                Direction::CounterClockwise => (head + i) % n,
            };
            let tail_hue = self.hue.wrapping_add((i as u8).wrapping_mul(self.hue_step));
            let tail_brightness = (brightness_val * self.brightness as u16 / 255) as u8;
            buffer[tail_idx] = hsv_to_rgb(tail_hue, self.saturation, tail_brightness);
        }

        Ok(())
    }

    /// Fills the buffer with the comet state and advances the animation.
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

impl Effect for RainbowCometEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        RainbowCometEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        RainbowCometEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        RainbowCometEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::MAX_LEDS;

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(
            RainbowCometEffect::new(0).unwrap_err(),
            EffectError::ZeroLeds
        );
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = RainbowCometEffect::new(12).unwrap();
        assert_eq!(effect.num_leds, 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert!(matches!(
            RainbowCometEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        assert_eq!(
            RainbowCometEffect::new(12)
                .unwrap()
                .with_speed(0)
                .unwrap_err(),
            EffectError::ZeroStep
        );
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = RainbowCometEffect::new(12).unwrap();
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
    fn test_head_at_full_brightness_initial() {
        // Default: hue=0, saturation=255, brightness=255. Position=0 → head at index 0.
        let effect = RainbowCometEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        let expected = hsv_to_rgb(0, 255, 255);
        assert_eq!(
            buffer[0], expected,
            "head LED must equal hsv_to_rgb(0,255,255)"
        );
    }

    #[test]
    fn test_tail_hue_increases_with_distance() {
        // Advance to position 6 on a 12-LED ring so the tail doesn't wrap awkwardly.
        // With hue_step=16, each successive tail LED shifts the hue by 16.
        // We can verify by checking that tail LEDs differ in a channel that changes with hue.
        let mut effect = RainbowCometEffect::new(12)
            .unwrap()
            .with_hue(0)
            .with_saturation(255)
            .with_brightness(255)
            .with_hue_step(16)
            .with_tail_length(4)
            .with_decay(255); // no brightness decay so only hue differs

        let mut buf = [RGB8::default(); 12];
        // Advance to position 6.
        for _ in 0..6 {
            effect.update(&mut buf).unwrap();
        }
        effect.current(&mut buf).unwrap();

        // Head at index 6; CW tail at 5, 4, 3, 2.
        let head = buf[6];
        let tail1 = buf[5];
        let tail2 = buf[4];
        let tail3 = buf[3];
        let tail4 = buf[2];

        // With hue_step=16 and full saturation, the green channel should change between steps.
        // Not all will differ on the same channel, but the raw u32 representation must differ.
        assert_ne!(head, tail1, "head and tail[1] must differ in hue");
        assert_ne!(tail1, tail2, "tail[1] and tail[2] must differ in hue");
        assert_ne!(tail2, tail3, "tail[2] and tail[3] must differ in hue");
        assert_ne!(tail3, tail4, "tail[3] and tail[4] must differ in hue");
    }

    #[test]
    fn test_tail_brightness_decreases_with_distance() {
        // saturation=0 → grayscale: r=g=b=brightness. Use this to check brightness ordering.
        let effect = RainbowCometEffect::new(12)
            .unwrap()
            .with_hue(0)
            .with_saturation(0)
            .with_brightness(255)
            .with_tail_length(4)
            .with_decay(192);

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        // Head at 0; CW tail at 11, 10, 9, 8.
        let head_v = buffer[0].r;
        let tail1_v = buffer[11].r;
        let tail2_v = buffer[10].r;
        let tail3_v = buffer[9].r;
        let tail4_v = buffer[8].r;

        assert_eq!(head_v, 255, "head must be at peak brightness");
        assert!(tail1_v > tail2_v, "tail[1] must be brighter than tail[2]");
        assert!(tail2_v > tail3_v, "tail[2] must be brighter than tail[3]");
        assert!(tail3_v > tail4_v, "tail[3] must be brighter than tail[4]");
        assert!(tail4_v > 0, "tail[4] must be non-zero with decay=192");
    }

    #[test]
    fn test_clockwise_advances_position() {
        let mut effect = RainbowCometEffect::new(8)
            .unwrap()
            .with_hue(0)
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        // After first update, head was at 0 (rendered), then advanced to 1.
        effect.current(&mut buffer).unwrap();
        // Head should now be at index 1.
        let expected = hsv_to_rgb(0, 255, 255);
        assert_eq!(
            buffer[1], expected,
            "head should be at index 1 after one update"
        );
        assert_eq!(buffer[0], RGB8::default(), "index 0 should be off");
    }

    #[test]
    fn test_counter_clockwise_direction() {
        let mut effect = RainbowCometEffect::new(8)
            .unwrap()
            .with_hue(0)
            .with_tail_length(0)
            .with_direction(Direction::CounterClockwise)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        // CCW from position 0 wraps to 7.
        effect.current(&mut buffer).unwrap();
        let expected = hsv_to_rgb(0, 255, 255);
        assert_eq!(
            buffer[7], expected,
            "head should be at index 7 after CCW wrap"
        );
        assert_eq!(buffer[0], RGB8::default(), "index 0 should be off");
    }

    #[test]
    fn test_wrapping_around_ring() {
        let mut effect = RainbowCometEffect::new(8)
            .unwrap()
            .with_hue(0)
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..8 {
            effect.update(&mut buffer).unwrap();
        }
        // After 8 steps, back at position 0.
        effect.current(&mut buffer).unwrap();
        let expected = hsv_to_rgb(0, 255, 255);
        assert_eq!(buffer[0], expected, "head should wrap back to index 0");
    }

    #[test]
    fn test_large_speed_no_panic() {
        let mut effect = RainbowCometEffect::new(4).unwrap().with_speed(200).unwrap();

        let mut buffer = [RGB8::default(); 4];
        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
        }
    }

    #[test]
    fn test_current_does_not_advance() {
        let effect = RainbowCometEffect::new(8).unwrap();

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();

        assert_eq!(buf1, buf2, "current() must not advance position");
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = RainbowCometEffect::new(8).unwrap().with_speed(3).unwrap();

        let mut initial = [RGB8::default(); 8];
        effect.current(&mut initial).unwrap();

        let mut temp = [RGB8::default(); 8];
        for _ in 0..20 {
            effect.update(&mut temp).unwrap();
        }

        effect.reset();
        let mut after_reset = [RGB8::default(); 8];
        effect.current(&mut after_reset).unwrap();

        assert_eq!(
            initial, after_reset,
            "state after reset must match initial state"
        );
    }

    #[test]
    fn test_set_hue_does_not_reset_position() {
        let mut effect = RainbowCometEffect::new(8)
            .unwrap()
            .with_hue(0)
            .with_tail_length(0)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }

        // Record which index is lit before changing hue.
        effect.current(&mut buffer).unwrap();
        let pos_before = buffer
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        effect.set_hue(128);

        effect.current(&mut buffer).unwrap();
        let pos_after = buffer
            .iter()
            .position(|led| *led != RGB8::default())
            .unwrap();

        assert_eq!(
            pos_before, pos_after,
            "position must not change after set_hue"
        );
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        // Buffer larger than num_leds; only first num_leds entries are touched.
        let effect = RainbowCometEffect::new(4).unwrap();
        let mut buffer = [RGB8::new(100, 100, 100); 12];
        effect.current(&mut buffer).unwrap();

        // The first 4 entries are cleared and then painted by the effect.
        // Entries 4–11 are untouched (still the sentinel value).
        for i in 4..12 {
            assert_eq!(
                buffer[i],
                RGB8::new(100, 100, 100),
                "LED {} beyond num_leds must not be modified",
                i
            );
        }
    }

    #[test]
    fn test_tail_zero_decay_only_head_lit() {
        // decay=0: brightness_val = 255 * 0 / 255 = 0 on first tail step → break immediately.
        let effect = RainbowCometEffect::new(12)
            .unwrap()
            .with_tail_length(6)
            .with_decay(0);

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        let expected_head = hsv_to_rgb(0, 255, 255);
        assert_eq!(buffer[0], expected_head, "head must be lit");
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
    fn test_trait_object_update() {
        let mut effect = RainbowCometEffect::new(8).unwrap().with_speed(2).unwrap();
        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];

        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        assert_ne!(buf1, buf2, "comet should advance between updates");
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = RainbowCometEffect::new(8).unwrap();

        let mut initial = [RGB8::default(); 8];
        effect.current(&mut initial).unwrap();

        let mut temp = [RGB8::default(); 8];
        for _ in 0..5 {
            effect.update(&mut temp).unwrap();
        }

        // Reset via trait object.
        let effect_ref: &mut dyn Effect = &mut effect;
        effect_ref.reset();

        let mut after_reset = [RGB8::default(); 8];
        effect_ref.update(&mut after_reset).unwrap();

        assert_eq!(
            initial, after_reset,
            "first update after reset must replay the initial frame"
        );
    }

    #[test]
    fn test_new_with_max_leds_succeeds() {
        assert!(RainbowCometEffect::new(MAX_LEDS).is_ok());
    }

    #[test]
    fn test_counter_clockwise_tail_trails_at_higher_indices() {
        // In CCW mode the head moves toward lower indices; the tail trails at higher indices.
        // After 4 CCW updates from 0 on n=8: head at 4 (0→7→6→5→4).
        // Tail indices: (4+1)%8=5, (4+2)%8=6, (4+3)%8=7.
        // Index 3 is ahead of the head (next in CCW direction) and must be off.
        let mut effect = RainbowCometEffect::new(8)
            .unwrap()
            .with_saturation(0) // grayscale for simple brightness comparison
            .with_tail_length(3)
            .with_decay(255)
            .with_direction(Direction::CounterClockwise)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();

        assert!(buffer[4].r > 0, "head at 4 should be lit");
        assert!(buffer[5].r > 0, "tail at 5 should be lit (behind CCW head)");
        assert!(buffer[6].r > 0, "tail at 6 should be lit");
        assert!(buffer[7].r > 0, "tail at 7 should be lit");
        assert_eq!(
            buffer[3],
            RGB8::default(),
            "index 3 should be off (ahead of head in CCW)"
        );
    }

    #[test]
    fn test_with_brightness_dims_head() {
        // with_brightness(128) should produce a dimmer head than the default brightness=255.
        // saturation=0 gives grayscale so r=g=b=brightness for easy comparison.
        let full_brightness = {
            let e = RainbowCometEffect::new(8).unwrap().with_saturation(0);
            let mut buf = [RGB8::default(); 8];
            e.current(&mut buf).unwrap();
            buf[0].r
        };
        let half_brightness = {
            let e = RainbowCometEffect::new(8)
                .unwrap()
                .with_saturation(0)
                .with_brightness(128);
            let mut buf = [RGB8::default(); 8];
            e.current(&mut buf).unwrap();
            buf[0].r
        };
        assert_eq!(full_brightness, 255, "default brightness must be full");
        assert!(
            half_brightness < full_brightness,
            "with_brightness(128) must be dimmer than default"
        );
        assert!(half_brightness > 0, "with_brightness(128) must not be zero");
    }

    #[test]
    fn test_tail_length_clamped_by_builder() {
        // Requesting tail_length=255 on a 4-LED ring must clamp to 3 (num_leds − 1).
        // Advance to position 3 so the full clamped tail extends without wrap confusion.
        let mut effect = RainbowCometEffect::new(4)
            .unwrap()
            .with_saturation(0)
            .with_tail_length(255)
            .with_decay(255)
            .with_speed(1)
            .unwrap();

        let mut buffer = [RGB8::default(); 4];
        for _ in 0..3 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        // Head at 3; CW tail at 2, 1, 0 (length clamped to 3, not 255).
        assert!(buffer[3].r > 0, "head at 3");
        assert!(buffer[2].r > 0, "tail at 2");
        assert!(buffer[1].r > 0, "tail at 1");
        assert!(buffer[0].r > 0, "tail at 0");
    }
}

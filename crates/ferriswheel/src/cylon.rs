//! Cylon / bouncing scanner effect for LED strips and rings.
//!
//! A bright head LED sweeps back and forth between the two ends, automatically
//! reversing direction, with a fading tail that always trails behind the direction
//! of travel — distinct from [`ChaseEffect`](crate::ChaseEffect), which wraps
//! around the ring without bouncing.

use crate::effect::{validate_buffer, validate_num_leds, validate_speed, Effect, EffectError};
use crate::util::{clamp_tail_length, draw_scanner_head, scanner_bounce};
use rgb::RGB8;

/// A Cylon / bouncing scanner effect.
///
/// A bright head LED sweeps back and forth across the strip or ring,
/// automatically reversing direction when it reaches either end.
/// A configurable fading tail trails behind the direction of travel.
///
/// This is distinct from [`ChaseEffect`](crate::ChaseEffect), which wraps
/// around the ring unidirectionally. `CylonEffect` bounces between the two
/// ends, producing the characteristic KITT scanner look.
///
/// # Example
///
/// ```
/// use ferriswheel::{CylonEffect, Effect};
/// use ferriswheel::RGB8;
///
/// let mut cylon = CylonEffect::new(12).unwrap()
///     .with_color(RGB8::new(255, 0, 0))
///     .with_tail_length(4)
///     .with_decay(192);
/// let mut buffer = [RGB8::default(); 12];
///
/// cylon.update(&mut buffer).unwrap();
/// ```
///
/// The maximum number of LEDs is [`MAX_LEDS`](crate::effect::MAX_LEDS).
#[derive(Debug, Clone, PartialEq)]
pub struct CylonEffect {
    num_leds: usize,
    color: RGB8,
    /// Current head position (index into the LED strip).
    ///
    /// Stored as `u8` because `MAX_LEDS` is 256, so valid indices (0–255)
    /// always fit without overflow. `validate_num_leds` enforces this cap.
    position: u8,
    /// `true` = moving toward higher indices; `false` = moving toward lower indices.
    forward: bool,
    speed: u8,
    tail_length: u8,
    /// Per-step multiplicative brightness factor (0–255).
    ///
    /// Each tail LED is rendered at `decay / 255` of the previous step's
    /// brightness. `255` = no decay (uniform tail). `0` = only the head is lit.
    /// Default `192` ≈ 75% per step.
    decay: u8,
}

impl CylonEffect {
    /// Creates a new Cylon effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::ZeroLeds`] if `num_leds` is 0.
    /// Returns [`EffectError::TooManyLeds`] if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Color: red (255, 0, 0)
    /// - Speed: 1
    /// - Tail length: 4
    /// - Decay: 192 (~75% per step)
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;
        Ok(Self {
            num_leds,
            color: RGB8::new(255, 0, 0),
            position: 0,
            forward: true,
            speed: 1,
            tail_length: 4,
            decay: 192,
        })
    }

    /// Sets the scanner color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
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

    /// Sets the number of LEDs in the fading tail behind the head.
    ///
    /// Clamped to `num_leds − 1` so the tail can never overlap the head.
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        self.tail_length = clamp_tail_length(tail_length, self.num_leds);
        self
    }

    /// Sets the per-step multiplicative brightness decay factor (0–255).
    ///
    /// Each successive tail LED is rendered at `decay / 255` of the previous
    /// step's brightness. `255` = no decay (uniform tail at full brightness).
    /// `0` = only the head is lit. Default is `192` (~75%).
    pub fn with_decay(mut self, decay: u8) -> Self {
        self.decay = decay;
        self
    }

    /// Sets the scanner color without resetting the animation position.
    ///
    /// Use this to change the color live without restarting the bounce cycle.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current Cylon state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        let n = self.num_leds;

        buffer[..n].fill(RGB8::default());
        draw_scanner_head(
            buffer,
            n,
            self.position as usize,
            self.forward,
            self.color,
            self.tail_length,
            self.decay,
        );

        Ok(())
    }

    /// Advances the scanner one step and fills the buffer.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;
        self.advance();
        Ok(())
    }

    /// Resets the scanner to its initial state (position 0, moving forward).
    pub fn reset(&mut self) {
        self.position = 0;
        self.forward = true;
    }

    /// Advances position by `speed` steps, bouncing at both ends.
    ///
    /// Uses reflection arithmetic so that large `speed` values produce
    /// a sensible bounce rather than wrapping or panicking.
    fn advance(&mut self) {
        let (position, forward) =
            scanner_bounce(self.position, self.forward, self.speed, self.num_leds);
        self.position = position;
        self.forward = forward;
    }
}

impl Effect for CylonEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        CylonEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        CylonEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        CylonEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::MAX_LEDS;

    const RED: RGB8 = RGB8::new(255, 0, 0);
    const BLACK: RGB8 = RGB8::new(0, 0, 0);

    // ── constructor ──────────────────────────────────────────────────────────

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(CylonEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = CylonEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert!(matches!(
            CylonEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        assert_eq!(
            CylonEffect::new(12).unwrap().with_speed(0).unwrap_err(),
            EffectError::ZeroStep
        );
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = CylonEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        assert_eq!(
            effect.current(&mut buffer).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8
            }
        );
    }

    // ── initial state ────────────────────────────────────────────────────────

    #[test]
    fn test_head_at_full_brightness_initial() {
        let effect = CylonEffect::new(8).unwrap().with_color(RED);
        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();
        assert_eq!(
            buffer[0], RED,
            "head should be at full brightness at position 0"
        );
    }

    // ── forward movement ─────────────────────────────────────────────────────

    #[test]
    fn test_advance_forward_moves_head() {
        // After 1 update, head should be at index 1.
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap(); // renders 0, advances to 1
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[1], RED, "head should be at index 1 after one step");
        assert_eq!(buffer[0], BLACK, "index 0 should be off");
    }

    #[test]
    fn test_speed_two_advances_by_two() {
        let mut effect = CylonEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(2)
            .unwrap();
        let mut buffer = [RGB8::default(); 12];
        effect.update(&mut buffer).unwrap(); // renders 0, advances to 2
        effect.current(&mut buffer).unwrap();
        assert_eq!(
            buffer[2], RED,
            "head should be at index 2 after one step with speed=2"
        );
    }

    #[test]
    fn test_large_speed_no_panic() {
        // speed >> n should never panic or overflow; reflection clamps the result.
        let mut effect = CylonEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(200)
            .unwrap();
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
        }
        // Just verifying no panic; position must remain a valid index.
        effect.current(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != BLACK),
            "at least one LED should be lit after large-speed updates"
        );
    }

    #[test]
    fn test_tail_length_clamped_by_builder() {
        // with_tail_length clamps to num_leds-1; requesting 255 on n=4 gives 3.
        let effect = CylonEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(255)
            .with_decay(255);
        // Advance to pos=3 (after 3 updates) so the tail can fully extend backward.
        let mut e = effect;
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..3 {
            e.update(&mut buffer).unwrap();
        }
        e.current(&mut buffer).unwrap();
        // Head at 3; tail at 2, 1, 0 (length=3, not 255). No LED should be double-written.
        assert_eq!(buffer[3], RED, "head at 3");
        assert!(buffer[2].r > 0, "tail at 2");
        assert!(buffer[1].r > 0, "tail at 1");
        assert!(buffer[0].r > 0, "tail at 0");
    }

    // ── reversal at boundaries ───────────────────────────────────────────────

    #[test]
    fn test_reverses_at_top_boundary() {
        // n=4: positions rendered are 0,1,2,3,2,1,...
        // After 4 updates: rendered 3 and bounced; state = (2, backward).
        let mut effect = CylonEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[2], RED, "head should be at 2 after bouncing off top");
        assert_eq!(buffer[3], BLACK, "index 3 should be off after bounce");
    }

    #[test]
    fn test_reverses_at_bottom_boundary() {
        // n=4: after 7 updates (rendered 0 going backward), bounces → state = (1, forward).
        let mut effect = CylonEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..7 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(
            buffer[1], RED,
            "head should be at 1 after bouncing off bottom"
        );
        assert_eq!(buffer[0], BLACK, "index 0 should be off after bounce");
    }

    #[test]
    fn test_full_sweep_returns_to_start_position() {
        // With n=4, a full scan visits 0→1→2→3→2→1→0 — period 2*(n-1)=6 updates.
        // After 6 updates the head is back at 0 (direction backward; buffer shows same head pos).
        let mut effect = CylonEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buf_initial = [RGB8::default(); 4];
        effect.current(&mut buf_initial).unwrap(); // state (0, forward)

        let mut buf_after = [RGB8::default(); 4];
        for _ in 0..6 {
            effect.update(&mut buf_after).unwrap();
        }
        effect.current(&mut buf_after).unwrap(); // state (0, backward)

        assert_eq!(
            buf_after[0], RED,
            "head should be back at index 0 after a full sweep"
        );
    }

    // ── tail direction ───────────────────────────────────────────────────────

    #[test]
    fn test_tail_follows_forward_direction() {
        // After 4 updates with n=8: state = (4, forward).
        // Tail should trail toward lower indices (3, 2, 1).
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(3)
            .with_decay(255); // no decay — all tail LEDs at full brightness for easy comparison
        let mut buffer = [RGB8::default(); 8];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[4], RED, "head at 4");
        assert!(buffer[3].r > 0, "tail LED at 3 should be lit");
        assert!(buffer[2].r > 0, "tail LED at 2 should be lit");
        assert!(buffer[1].r > 0, "tail LED at 1 should be lit");
        assert_eq!(buffer[5], BLACK, "index 5 should be off (ahead of head)");
    }

    #[test]
    fn test_tail_follows_backward_direction() {
        // After 9 updates with n=8: state = (5, backward).
        // Tail should trail toward higher indices (6, 7).
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(3)
            .with_decay(255);
        let mut buffer = [RGB8::default(); 8];
        for _ in 0..9 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();

        assert_eq!(buffer[5], RED, "head at 5");
        assert!(buffer[6].r > 0, "tail LED at 6 should be lit");
        assert!(buffer[7].r > 0, "tail LED at 7 should be lit");
        assert_eq!(buffer[4], BLACK, "index 4 should be off (ahead of head)");
    }

    // ── tail decay ───────────────────────────────────────────────────────────

    #[test]
    fn test_tail_decay_ordering() {
        // With decay=192: each tail LED is dimmer than the one before it.
        // After 4 updates: head at 4 (forward), tail at 3, 2, 1.
        let effect_after_4 = {
            let mut e = CylonEffect::new(8)
                .unwrap()
                .with_color(RGB8::new(255, 255, 255))
                .with_tail_length(3)
                .with_decay(192);
            let mut buf = [RGB8::default(); 8];
            for _ in 0..4 {
                e.update(&mut buf).unwrap();
            }
            e
        };
        let mut buffer = [RGB8::default(); 8];
        effect_after_4.current(&mut buffer).unwrap();

        let head_b = buffer[4].r;
        let tail1_b = buffer[3].r;
        let tail2_b = buffer[2].r;
        let tail3_b = buffer[1].r;

        assert_eq!(head_b, 255, "head must be at full brightness");
        assert!(tail1_b > tail2_b, "tail[1] should be brighter than tail[2]");
        assert!(tail2_b > tail3_b, "tail[2] should be brighter than tail[3]");
        assert!(tail3_b > 0, "tail[3] should be non-zero with decay=192");
    }

    #[test]
    fn test_tail_zero_decay_only_head_lit() {
        let effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(4)
            .with_decay(0);
        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RED, "head should be lit");
        for i in 1..8 {
            assert_eq!(buffer[i], BLACK, "LED {i} should be off with decay=0");
        }
    }

    // ── tail clamping at strip boundaries ────────────────────────────────────

    #[test]
    fn test_tail_clamped_at_strip_start() {
        // At position 0 (forward), tail would go to -1, -2, ... — all clipped.
        // Only the head should be lit.
        let effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(6)
            .with_decay(255);
        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RED, "head at 0 should be lit");
        for i in 1..8 {
            assert_eq!(
                buffer[i], BLACK,
                "LED {i} should be off — no tail behind index 0"
            );
        }
    }

    #[test]
    fn test_tail_clamped_at_strip_end() {
        // At position 7 (n=8, forward), tail fits at 6, 5, 4, 3, 2, 1 — all within bounds.
        // After 7 updates: state = (7, forward); 7th render showed pos=6, then advanced to 7.
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(6)
            .with_decay(255);
        let mut buffer = [RGB8::default(); 8];
        for _ in 0..7 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        // Head at 7; tail at 6,5,4,3,2,1 (all fit); index 0 should be off.
        assert_eq!(buffer[7], RED, "head at 7");
        for i in 1..=6 {
            assert!(buffer[i].r > 0, "tail LED at {i} should be lit");
        }
        assert_eq!(
            buffer[0], BLACK,
            "index 0 should be off — tail length is 6, not 7"
        );
    }

    // ── current / update contract ────────────────────────────────────────────

    #[test]
    fn test_current_does_not_advance() {
        let effect = CylonEffect::new(8).unwrap();
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();
        assert_eq!(buf1, buf2, "current() must not change state");
    }

    // ── reset ────────────────────────────────────────────────────────────────

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = CylonEffect::new(8).unwrap().with_tail_length(0);

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

    // ── live setters ─────────────────────────────────────────────────────────

    #[test]
    fn test_set_color_does_not_reset_position() {
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(3)
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        for _ in 0..3 {
            effect.update(&mut buffer).unwrap();
        }

        let mut before = [RGB8::default(); 8];
        effect.current(&mut before).unwrap();
        let pos_before = before.iter().position(|led| *led != BLACK).unwrap();

        effect.set_color(RGB8::new(0, 0, 255));

        let mut after = [RGB8::default(); 8];
        effect.current(&mut after).unwrap();
        let pos_after = after.iter().position(|led| *led != BLACK).unwrap();

        assert_eq!(
            pos_before, pos_after,
            "position should be unchanged after set_color"
        );
    }

    // ── trait object dispatch ─────────────────────────────────────────────────

    #[test]
    fn test_trait_object_update() {
        let mut effect = CylonEffect::new(8)
            .unwrap()
            .with_color(RED)
            .with_speed(2)
            .unwrap();
        let effect_ref: &mut dyn Effect = &mut effect;

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();
        assert_ne!(
            buf1, buf2,
            "Cylon should advance between trait-object updates"
        );
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = CylonEffect::new(8).unwrap().with_tail_length(0);
        let mut buf_before = [RGB8::default(); 8];
        let mut buf_after = [RGB8::default(); 8];
        let effect_ref: &mut dyn Effect = &mut effect;
        effect_ref.update(&mut buf_before).unwrap();
        effect_ref.reset();
        effect_ref.update(&mut buf_after).unwrap();
        assert_eq!(
            buf_before, buf_after,
            "trait reset must replay the same first step"
        );
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = CylonEffect::new(4).unwrap();
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

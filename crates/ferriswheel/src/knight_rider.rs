//! Knight Rider / dual-headed scanner effect for LED strips and rings.
//!
//! Two bright head LEDs sweep in opposite directions, crossing in the middle
//! and reversing independently at each end — a natural extension of the
//! [`CylonEffect`](crate::CylonEffect).

use crate::effect::{validate_buffer, validate_num_leds, validate_speed, Effect, EffectError};
use crate::util::{draw_scanner_head, scanner_bounce};
use rgb::RGB8;

/// A Knight Rider / dual-headed scanner effect.
///
/// Two bright head LEDs sweep across the strip or ring in opposite directions,
/// crossing in the middle and reversing independently when they reach either end.
/// Each head trails a fading tail behind it.
///
/// This extends [`CylonEffect`](crate::CylonEffect), which has a single bouncing head.
///
/// # Buffer write order
///
/// Head A (starting at index 0) is drawn first; head B (starting at the far end)
/// is drawn second. If both heads occupy the same index, head B's color takes
/// precedence.
///
/// # Example
///
/// ```
/// use ferriswheel::{KnightRiderEffect, Effect};
/// use ferriswheel::RGB8;
///
/// let mut effect = KnightRiderEffect::new(12).unwrap()
///     .with_color(RGB8::new(255, 0, 0))
///     .with_tail_length(4)
///     .with_decay(192);
/// let mut buffer = [RGB8::default(); 12];
///
/// effect.update(&mut buffer).unwrap();
/// ```
///
/// The maximum number of LEDs is [`MAX_LEDS`](crate::effect::MAX_LEDS).
#[derive(Debug, Clone)]
pub struct KnightRiderEffect {
    num_leds: usize,
    color: RGB8,
    /// Head A: starts at index 0, moving toward higher indices.
    ///
    /// Stored as `u8` because `MAX_LEDS` is 256, so valid indices (0–255)
    /// always fit without overflow. `validate_num_leds` enforces this cap.
    pos_a: u8,
    /// `true` = head A is moving toward higher indices.
    forward_a: bool,
    /// Head B: starts at index `num_leds − 1`, moving toward lower indices.
    pos_b: u8,
    /// `true` = head B is moving toward higher indices.
    forward_b: bool,
    speed: u8,
    tail_length: u8,
    /// Per-step multiplicative brightness factor (0–255).
    ///
    /// Each tail LED is rendered at `decay / 255` of the previous step's
    /// brightness. `255` = no decay (uniform tail). `0` = only the heads are lit.
    /// Default `192` ≈ 75% per step.
    decay: u8,
}

impl KnightRiderEffect {
    /// Creates a new Knight Rider effect for the specified number of LEDs.
    ///
    /// Head A starts at index 0 (moving forward); head B starts at index
    /// `num_leds − 1` (moving backward). Both reverse independently when
    /// they reach either end.
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
        let pos_b = num_leds.saturating_sub(1).min(u8::MAX as usize) as u8;
        Ok(Self {
            num_leds,
            color: RGB8::new(255, 0, 0),
            pos_a: 0,
            forward_a: true,
            pos_b,
            forward_b: false,
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

    /// Sets the number of LEDs in the fading tail behind each head.
    ///
    /// Clamped to `num_leds − 1` so a tail can never overlap its own head.
    pub fn with_tail_length(mut self, tail_length: u8) -> Self {
        let max_tail = self.num_leds.saturating_sub(1).min(u8::MAX as usize) as u8;
        self.tail_length = tail_length.min(max_tail);
        self
    }

    /// Sets the per-step multiplicative brightness decay factor (0–255).
    ///
    /// Each successive tail LED is rendered at `decay / 255` of the previous
    /// step's brightness. `255` = no decay (uniform tail at full brightness).
    /// `0` = only the heads are lit. Default is `192` (~75%).
    pub fn with_decay(mut self, decay: u8) -> Self {
        self.decay = decay;
        self
    }

    /// Sets the scanner color without resetting the animation position.
    ///
    /// Use this to change the color live without restarting the scan cycle.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current Knight Rider state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        let n = self.num_leds;
        buffer[..n].fill(RGB8::default());
        self.draw_head(buffer, self.pos_a as usize, self.forward_a);
        self.draw_head(buffer, self.pos_b as usize, self.forward_b);
        Ok(())
    }

    /// Advances both heads one step and fills the buffer.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)?;
        self.advance();
        Ok(())
    }

    /// Resets both heads to their initial positions.
    pub fn reset(&mut self) {
        self.pos_a = 0;
        self.forward_a = true;
        self.pos_b = self.num_leds.saturating_sub(1).min(u8::MAX as usize) as u8;
        self.forward_b = false;
    }

    /// Draws one head and its fading tail into `buffer`.
    ///
    /// The tail trails behind the direction of travel — no wrap-around at the ends.
    /// `forward` = `true` means the head is moving toward higher indices, so the
    /// tail lies at lower indices.
    fn draw_head(&self, buffer: &mut [RGB8], head: usize, forward: bool) {
        draw_scanner_head(
            buffer,
            self.num_leds,
            head,
            forward,
            self.color,
            self.tail_length,
            self.decay,
        );
    }

    /// Advances both heads by `speed` steps, bouncing each independently.
    fn advance(&mut self) {
        let (new_pos, new_fwd) =
            Self::advance_head(self.pos_a, self.forward_a, self.speed, self.num_leds);
        self.pos_a = new_pos;
        self.forward_a = new_fwd;

        let (new_pos, new_fwd) =
            Self::advance_head(self.pos_b, self.forward_b, self.speed, self.num_leds);
        self.pos_b = new_pos;
        self.forward_b = new_fwd;
    }

    /// Reflects a single head position off both ends using the same reflection
    /// arithmetic as [`CylonEffect`](crate::CylonEffect).
    fn advance_head(position: u8, forward: bool, speed: u8, num_leds: usize) -> (u8, bool) {
        scanner_bounce(position, forward, speed, num_leds)
    }
}

impl Effect for KnightRiderEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        KnightRiderEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        KnightRiderEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        KnightRiderEffect::reset(self);
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
        assert_eq!(
            KnightRiderEffect::new(0).unwrap_err(),
            EffectError::ZeroLeds
        );
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = KnightRiderEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert!(matches!(
            KnightRiderEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_with_speed_zero_returns_error() {
        assert_eq!(
            KnightRiderEffect::new(12)
                .unwrap()
                .with_speed(0)
                .unwrap_err(),
            EffectError::ZeroStep
        );
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = KnightRiderEffect::new(12).unwrap();
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
    fn test_both_heads_lit_initially() {
        let effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RED, "head A should be lit at index 0");
        assert_eq!(buffer[11], RED, "head B should be lit at index 11");
    }

    #[test]
    fn test_single_led_both_heads_at_same_position() {
        // n=1: both heads resolve to index 0; head B (drawn last) takes precedence.
        let effect = KnightRiderEffect::new(1).unwrap().with_color(RED);
        let mut buffer = [RGB8::default(); 1];
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], RED);
    }

    // ── movement ─────────────────────────────────────────────────────────────

    #[test]
    fn test_heads_move_toward_each_other_after_one_update() {
        // n=12, tail_length=0: after 1 update, A should be at 1, B at 10.
        let mut effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 12];
        effect.update(&mut buffer).unwrap(); // renders 0/11, advances
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[1], RED, "head A should be at index 1");
        assert_eq!(buffer[10], RED, "head B should be at index 10");
    }

    #[test]
    fn test_speed_two_advances_by_two() {
        let mut effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(2)
            .unwrap();
        let mut buffer = [RGB8::default(); 12];
        effect.update(&mut buffer).unwrap(); // renders 0/11, advances
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[2], RED, "head A should be at index 2");
        assert_eq!(buffer[9], RED, "head B should be at index 9");
    }

    #[test]
    fn test_large_speed_no_panic() {
        // speed >> n should never panic or overflow; reflection clamps the result.
        let mut effect = KnightRiderEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(200)
            .unwrap();
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != BLACK),
            "at least one LED should be lit after large-speed updates"
        );
    }

    // ── independent reversal ─────────────────────────────────────────────────

    #[test]
    fn test_head_a_reverses_at_top_boundary() {
        // n=4, tail_length=0. Head A visits 0,1,2,3 then bounces.
        // After 4 updates: A rendered 3, advanced to 2 (backward).
        // Head B: 3→2→1→0→1 after 4 updates.
        // index 3 should be off (both heads have moved away from it).
        let mut effect = KnightRiderEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[3], BLACK, "index 3 should be off: head A bounced");
    }

    #[test]
    fn test_head_b_reverses_at_bottom_boundary() {
        // n=4, tail_length=0. Head B visits 3,2,1,0 then bounces.
        // After 4 updates: B rendered 0, advanced to 1 (forward).
        // Head A: 0,1,2,3 → bounced to 2 after 4 updates.
        // index 0 should be off (both heads have moved away from it).
        let mut effect = KnightRiderEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RGB8::default(); 4];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[0], BLACK, "index 0 should be off: head B bounced");
    }

    // ── tail direction ───────────────────────────────────────────────────────

    #[test]
    fn test_tail_a_follows_forward_direction() {
        // n=12, tail_length=3, decay=255 (no decay for easy comparison).
        // After 4 updates: head A at 4 (forward), tail at 3, 2, 1.
        // Head B after 4 updates: at 7 (backward), tail at 8, 9, 10.
        // index 5 is untouched by either head.
        let mut effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(3)
            .with_decay(255);
        let mut buffer = [RGB8::default(); 12];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[4], RED, "head A at 4");
        assert!(buffer[3].r > 0, "tail A at 3 should be lit");
        assert!(buffer[2].r > 0, "tail A at 2 should be lit");
        assert!(buffer[1].r > 0, "tail A at 1 should be lit");
        assert_eq!(buffer[5], BLACK, "index 5 should be off (ahead of head A)");
    }

    #[test]
    fn test_tail_b_follows_backward_direction() {
        // n=12, tail_length=3, decay=255.
        // After 4 updates: head B at 7 (backward), tail at 8, 9, 10.
        // Head A after 4 updates: at 4 (forward), tail at 3, 2, 1.
        // index 6 is between the two heads and untouched.
        let mut effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(3)
            .with_decay(255);
        let mut buffer = [RGB8::default(); 12];
        for _ in 0..4 {
            effect.update(&mut buffer).unwrap();
        }
        effect.current(&mut buffer).unwrap();
        assert_eq!(buffer[7], RED, "head B at 7");
        assert!(buffer[8].r > 0, "tail B at 8 should be lit");
        assert!(buffer[9].r > 0, "tail B at 9 should be lit");
        assert!(buffer[10].r > 0, "tail B at 10 should be lit");
        assert_eq!(buffer[6], BLACK, "index 6 should be off (ahead of head B)");
    }

    // ── oversized buffer ─────────────────────────────────────────────────────

    #[test]
    fn test_oversized_buffer_accepted() {
        // Buffers larger than num_leds should be accepted;
        // only buffer[0..num_leds] is written.
        let effect = KnightRiderEffect::new(4)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0);
        let mut buffer = [RED; 8];
        assert!(effect.current(&mut buffer).is_ok());
        // First 4 LEDs were cleared and redrawn by current().
        assert_eq!(buffer[0], RED, "head A at index 0");
        assert_eq!(buffer[3], RED, "head B at index 3");
        // Bytes beyond num_leds are untouched by current().
        assert_eq!(buffer[4], RED, "beyond num_leds should be untouched");
    }

    // ── current / update contract ────────────────────────────────────────────

    #[test]
    fn test_current_does_not_advance() {
        let effect = KnightRiderEffect::new(8).unwrap();
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();
        assert_eq!(buf1, buf2, "current() must not change state");
    }

    // ── reset ────────────────────────────────────────────────────────────────

    #[test]
    fn test_reset_restores_initial_state() {
        let mut effect = KnightRiderEffect::new(8).unwrap().with_tail_length(0);

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
        let mut effect = KnightRiderEffect::new(12)
            .unwrap()
            .with_color(RED)
            .with_tail_length(0)
            .with_speed(3)
            .unwrap();

        let mut buffer = [RGB8::default(); 12];
        for _ in 0..3 {
            effect.update(&mut buffer).unwrap();
        }

        let mut before = [RGB8::default(); 12];
        effect.current(&mut before).unwrap();
        let mut lit_before = [usize::MAX; 2];
        let mut lit_before_count = 0;
        for (i, led) in before.iter().enumerate() {
            if *led != BLACK {
                lit_before[lit_before_count] = i;
                lit_before_count += 1;
            }
        }
        assert_eq!(
            lit_before_count, 2,
            "expected exactly two lit head positions"
        );

        effect.set_color(RGB8::new(0, 0, 255));

        let mut after = [RGB8::default(); 12];
        effect.current(&mut after).unwrap();
        let mut lit_after = [usize::MAX; 2];
        let mut lit_after_count = 0;
        for (i, led) in after.iter().enumerate() {
            if *led != BLACK {
                lit_after[lit_after_count] = i;
                lit_after_count += 1;
            }
        }
        assert_eq!(
            lit_after_count, 2,
            "expected exactly two lit head positions"
        );

        assert_eq!(
            lit_before, lit_after,
            "lit positions must be unchanged after set_color"
        );
    }

    // ── trait object dispatch ─────────────────────────────────────────────────

    #[test]
    fn test_trait_object_update() {
        let mut effect = KnightRiderEffect::new(8)
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
            "KnightRider should advance between trait-object updates"
        );
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = KnightRiderEffect::new(8).unwrap().with_tail_length(0);
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
}

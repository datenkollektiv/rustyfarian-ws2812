//! Twinkle / sparkle animation effect for LED rings.
//!
//! Random LEDs flash to peak brightness and decay independently,
//! producing an ambient starfield or glitter effect.
//! Uses a built-in xorshift32 PRNG — no external dependency required.

use crate::effect::{validate_buffer, validate_num_leds, Effect, EffectError, MAX_LEDS};
use crate::util::scale_brightness;
use rgb::RGB8;

/// Default PRNG seed. Non-zero; produces a pleasant default sparkle pattern.
const DEFAULT_SEED: u32 = 0x1234_5678;

/// xorshift32 — a fast, no-std compatible PRNG with period 2^32 - 1.
///
/// If called with 0, xorshift32 returns 0 and the state becomes permanently stuck.
/// Always seed with a non-zero value; `debug_assert!` catches misuse in debug builds.
fn xorshift32(mut x: u32) -> u32 {
    debug_assert!(x != 0, "xorshift32 state must be non-zero");
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

/// An ambient twinkle / sparkle effect.
///
/// Random LEDs flash to `max_brightness` and then decay independently each
/// tick, creating a starfield or glitter appearance.
/// All sparkles share the same base `color`; brightness is modulated per LED.
///
/// A built-in xorshift32 PRNG drives both which LED fires and whether a new
/// sparkle is triggered each tick. Seed it with [`with_seed`](TwinkleEffect::with_seed)
/// for reproducible sequences or accept the default for typical use.
///
/// # Example
///
/// ```
/// use ferriswheel::{TwinkleEffect, Effect};
/// use ferriswheel::RGB8;
///
/// let mut twinkle = TwinkleEffect::new(12).unwrap()
///     .with_color(RGB8::new(150, 180, 255))
///     .with_spawn_chance(40)
///     .with_decay(15);
/// let mut buffer = [RGB8::default(); 12];
///
/// twinkle.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TwinkleEffect {
    num_leds: usize,
    color: RGB8,
    /// Per-LED brightness (0 = off, `max_brightness` = fully on).
    /// Only indices `0..num_leds` are used; the rest are always 0.
    /// This field occupies `MAX_LEDS` bytes (256 bytes) in the struct.
    brightness: [u8; MAX_LEDS],
    max_brightness: u8,
    /// Per-tick brightness reduction applied to every lit LED.
    decay: u8,
    /// Probability of lighting a new LED each tick.
    /// `0` = never spawn. `255` = always trigger one spawn event per tick
    /// (may hit the same LED repeatedly). Values 1–254 give a `spawn_chance / 256` probability.
    spawn_chance: u8,
    /// Current PRNG state (advances each tick).
    rng_state: u32,
    /// Seed stored at construction / `with_seed` time; restored by `reset()`.
    initial_seed: u32,
}

impl TwinkleEffect {
    /// Creates a new twinkle effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Color: white (255, 255, 255)
    /// - Max brightness: 200
    /// - Decay: 20 (~10 frames to fade out at 50 ms/frame)
    /// - Spawn chance: 40 (~16 % per tick)
    /// - Seed: `0x1234_5678`
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;
        Ok(Self {
            num_leds,
            color: RGB8::new(255, 255, 255),
            brightness: [0u8; MAX_LEDS],
            max_brightness: 200,
            decay: 20,
            spawn_chance: 40,
            rng_state: DEFAULT_SEED,
            initial_seed: DEFAULT_SEED,
        })
    }

    /// Sets the sparkle color.
    pub fn with_color(mut self, color: RGB8) -> Self {
        self.color = color;
        self
    }

    /// Sets the maximum brightness a newly spawned LED reaches (0–255).
    pub fn with_max_brightness(mut self, max_brightness: u8) -> Self {
        self.max_brightness = max_brightness;
        self
    }

    /// Sets the per-tick brightness decay applied to every lit LED.
    ///
    /// Higher values produce a shorter, sharper flash; lower values produce a
    /// longer, softer glow. `0` = LEDs stay on indefinitely once lit.
    pub fn with_decay(mut self, decay: u8) -> Self {
        self.decay = decay;
        self
    }

    /// Sets the per-tick probability of lighting a new LED.
    ///
    /// `0` = never spawn. `255` = always trigger one spawn event per tick
    /// (the same LED may be picked repeatedly by the PRNG).
    /// Values 1–254 give a `spawn_chance / 256` probability.
    pub fn with_spawn_chance(mut self, spawn_chance: u8) -> Self {
        self.spawn_chance = spawn_chance;
        self
    }

    /// Seeds the PRNG for reproducible sparkle sequences.
    ///
    /// A seed of `0` is silently promoted to `1` (xorshift32 breaks at 0).
    /// The same seed is restored by [`reset`](TwinkleEffect::reset).
    pub fn with_seed(mut self, seed: u32) -> Self {
        let s = seed.max(1);
        self.rng_state = s;
        self.initial_seed = s;
        self
    }

    /// Updates the sparkle color without resetting brightness state.
    pub fn set_color(&mut self, color: RGB8) {
        self.color = color;
    }

    /// Updates the spawn chance without resetting brightness state.
    pub fn set_spawn_chance(&mut self, spawn_chance: u8) {
        self.spawn_chance = spawn_chance;
    }

    /// Updates the decay rate without resetting brightness state.
    pub fn set_decay(&mut self, decay: u8) {
        self.decay = decay;
    }

    /// Updates the max brightness without resetting brightness state.
    pub fn set_max_brightness(&mut self, max_brightness: u8) {
        self.max_brightness = max_brightness;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current sparkle state without advancing.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        for (i, led) in buffer.iter_mut().enumerate().take(self.num_leds) {
            *led = scale_brightness(self.color, self.brightness[i]);
        }
        Ok(())
    }

    /// Decays all LEDs, maybe spawns a new sparkle, then fills the buffer.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        // Validate before any state mutation.
        validate_buffer(buffer, self.num_leds)?;

        // Decay all active LEDs.
        for b in self.brightness[..self.num_leds].iter_mut() {
            *b = b.saturating_sub(self.decay);
        }

        // Advance PRNG and maybe light a random LED.
        self.rng_state = xorshift32(self.rng_state);
        let should_spawn = self.spawn_chance == 255
            || (self.spawn_chance > 0 && ((self.rng_state & 0xFF) as u8) < self.spawn_chance);

        if should_spawn {
            self.rng_state = xorshift32(self.rng_state);
            let idx = (self.rng_state as usize) % self.num_leds;
            self.brightness[idx] = self.max_brightness;
        }

        self.current(buffer)
    }

    /// Resets all LEDs to black and restores the PRNG to its initial seed.
    pub fn reset(&mut self) {
        self.brightness = [0u8; MAX_LEDS];
        self.rng_state = self.initial_seed;
    }
}

impl Effect for TwinkleEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        TwinkleEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        TwinkleEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        TwinkleEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(TwinkleEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = TwinkleEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert!(matches!(
            TwinkleEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = TwinkleEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        assert_eq!(
            effect.current(&mut buffer).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8,
            }
        );
    }

    #[test]
    fn test_all_leds_start_dark() {
        let effect = TwinkleEffect::new(12).unwrap();
        let mut buffer = [RGB8::new(1, 1, 1); 12]; // pre-fill with non-black
        effect.current(&mut buffer).unwrap();
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(*led, RGB8::default(), "LED {} should be black on init", i);
        }
    }

    #[test]
    fn test_spawn_chance_zero_never_lights() {
        // With spawn_chance=0, no LED is ever lit regardless of updates.
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_spawn_chance(0)
            .with_decay(0);
        let mut buffer = [RGB8::default(); 12];
        for _ in 0..50 {
            effect.update(&mut buffer).unwrap();
        }
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(
                *led,
                RGB8::default(),
                "LED {} should stay dark with spawn_chance=0",
                i
            );
        }
    }

    #[test]
    fn test_spawn_chance_255_always_lights() {
        // With spawn_chance=255, one spawn event is guaranteed each tick.
        // After one update on a dark ring, at least one LED must be non-black.
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_max_brightness(200);
        let mut buffer = [RGB8::default(); 12];
        effect.update(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "at least one LED should be lit after spawn_chance=255 update"
        );
    }

    #[test]
    fn test_decay_reduces_brightness() {
        // Spawn one sparkle, then switch to no-spawn with decay and verify it dims.
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_spawn_chance(255)
            .with_decay(0)
            .with_max_brightness(200);
        let mut buffer = [RGB8::default(); 12];

        // First update: spawn a sparkle at max_brightness, no decay.
        effect.update(&mut buffer).unwrap();
        let lit_pos = buffer
            .iter()
            .position(|led| led.r > 0)
            .expect("one LED should be lit with spawn_chance=255");
        let brightness_before = buffer[lit_pos].r;

        // Switch off spawning, apply decay.
        effect.set_spawn_chance(0);
        effect.set_decay(50);
        effect.update(&mut buffer).unwrap();

        assert!(
            buffer[lit_pos].r < brightness_before,
            "brightness should decrease after applying decay"
        );
    }

    #[test]
    fn test_decay_does_not_underflow() {
        // A brightness of 1 with a decay of 255 must go to 0, not wrap.
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_max_brightness(1);
        let mut buffer = [RGB8::default(); 12];

        effect.update(&mut buffer).unwrap();

        effect.set_spawn_chance(0);
        effect.set_decay(255);
        for _ in 0..10 {
            effect.update(&mut buffer).unwrap();
        }
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(
                *led,
                RGB8::default(),
                "LED {} should be black (no underflow)",
                i
            );
        }
    }

    #[test]
    fn test_max_brightness_ceiling() {
        // No LED should ever exceed max_brightness.
        let max_b = 80u8;
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_color(RGB8::new(255, 255, 255))
            .with_spawn_chance(255)
            .with_decay(0)
            .with_max_brightness(max_b);
        let mut buffer = [RGB8::default(); 12];

        for _ in 0..20 {
            effect.update(&mut buffer).unwrap();
        }

        // scale_brightness(white, max_b) = max_b for r/g/b channels
        for (i, led) in buffer.iter().enumerate() {
            assert!(
                led.r <= max_b,
                "LED {} brightness {} exceeds max_brightness {}",
                i,
                led.r,
                max_b
            );
        }
    }

    #[test]
    fn test_reset_clears_all_leds() {
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0);
        let mut buffer = [RGB8::default(); 12];

        for _ in 0..10 {
            effect.update(&mut buffer).unwrap();
        }
        // At least some LEDs should be lit now.
        assert!(buffer.iter().any(|led| *led != RGB8::default()));

        effect.reset();
        effect.current(&mut buffer).unwrap();
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(
                *led,
                RGB8::default(),
                "LED {} should be black after reset",
                i
            );
        }
    }

    #[test]
    fn test_reset_restores_rng_sequence() {
        // Two effects with the same seed should produce identical output after reset.
        let mut effect = TwinkleEffect::new(8)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_seed(0xABCD_1234);
        let mut buffer1 = [RGB8::default(); 8];
        let mut buffer2 = [RGB8::default(); 8];

        // Run several updates to advance the PRNG.
        for _ in 0..5 {
            effect.update(&mut buffer1).unwrap();
        }

        // Reset and run the same number of updates.
        effect.reset();
        for _ in 0..5 {
            effect.update(&mut buffer2).unwrap();
        }

        assert_eq!(
            buffer1, buffer2,
            "identical sequence should replay after reset"
        );
    }

    #[test]
    fn test_set_color_does_not_reset_brightness() {
        // set_color() must not clear existing brightness state.
        let mut effect = TwinkleEffect::new(8)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_color(RGB8::new(255, 0, 0));
        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();

        // A lit LED should have non-zero red channel.
        let red_sum_before: u32 = buffer.iter().map(|led| led.r as u32).sum();
        assert!(red_sum_before > 0, "at least one LED should be lit");

        // Change color; brightness state must be preserved.
        effect.set_color(RGB8::new(0, 0, 255));
        effect.set_spawn_chance(0);
        effect.current(&mut buffer).unwrap();

        // Lit LEDs now show in the blue channel with the same relative brightness.
        let blue_sum: u32 = buffer.iter().map(|led| led.b as u32).sum();
        assert!(blue_sum > 0, "brightness state should survive set_color");
    }

    #[test]
    fn test_current_does_not_advance() {
        let mut effect = TwinkleEffect::new(8)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0);
        let mut buffer = [RGB8::default(); 8];

        effect.update(&mut buffer).unwrap(); // advance to a non-trivial state

        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();

        assert_eq!(buf1, buf2, "current() must not change state");
    }

    #[test]
    fn test_update_changes_output_over_time() {
        // Fixed seed + spawn_chance=255 makes the PRNG sequence fully deterministic.
        // After 12 updates on a 12-LED ring, xorshift32 (period 2^32−1) will pick
        // more than one distinct LED index, so the final state must differ from
        // the state after the very first update.
        let mut effect = TwinkleEffect::new(12)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_seed(1);
        let mut buf = [RGB8::default(); 12];

        effect.update(&mut buf).unwrap();
        let after_first = buf;

        for _ in 0..11 {
            effect.update(&mut buf).unwrap();
        }

        assert_ne!(
            buf, after_first,
            "state must evolve after 12 updates with spawn_chance=255"
        );
    }

    #[test]
    fn test_trait_object_update() {
        let mut effect = TwinkleEffect::new(8)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0);

        let effect_ref: &mut dyn Effect = &mut effect;
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect_ref.update(&mut buf1).unwrap();
        effect_ref.update(&mut buf2).unwrap();

        // With spawn_chance=255 and decay=0, each tick triggers a spawn event
        // (may pick the same LED, but with the default seed the two spawns differ);
        assert_ne!(
            buf1, buf2,
            "output should advance between trait-object updates"
        );
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = TwinkleEffect::new(8)
            .unwrap()
            .with_spawn_chance(255)
            .with_decay(0)
            .with_seed(42);
        let mut buf_before = [RGB8::default(); 8];
        let mut buf_after = [RGB8::default(); 8];

        let effect_ref: &mut dyn Effect = &mut effect;
        effect_ref.update(&mut buf_before).unwrap();
        effect_ref.reset();
        effect_ref.update(&mut buf_after).unwrap();

        assert_eq!(
            buf_before, buf_after,
            "trait reset should replay the same sequence"
        );
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = TwinkleEffect::new(4).unwrap();
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

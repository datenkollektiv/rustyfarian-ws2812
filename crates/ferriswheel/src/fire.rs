//! Fire / flame animation effect for LED rings.
//!
//! A heat simulation: the base of the ring ignites randomly, heat diffuses
//! upward toward the tip, and each LED's temperature maps to a fire colour
//! gradient (black → dark red → orange → yellow).

use crate::effect::{validate_buffer, validate_num_leds, Effect, EffectError, MAX_LEDS};
use crate::util::{xorshift32, DEFAULT_SEED};
use rgb::RGB8;

/// Maps a heat value (0–255) to a fire colour.
///
/// Four gradient stops:
/// - `0`   → black         (0, 0, 0)
/// - `85`  → dark red      (180, 0, 0)
/// - `170` → orange-yellow (255, 200, 0)
/// - `255` → bright yellow (255, 255, 0)
///
/// The top end stays in the red/orange/yellow family — no blue channel,
/// so peak heat reads as a vivid yellow rather than white.
///
/// The total RGB brightness is weakly monotone — it never decreases as heat
/// increases. The function is `pub(crate)` so the test module can exercise it
/// directly.
pub(crate) fn fire_color(heat: u8) -> RGB8 {
    let t = heat as u16;
    if t < 85 {
        // black → dark red
        RGB8::new((t * 180 / 84) as u8, 0, 0)
    } else if t < 170 {
        // dark red → orange-yellow
        let s = t - 85;
        RGB8::new((180 + s * 75 / 84) as u8, (s * 200 / 84) as u8, 0)
    } else {
        // orange-yellow → bright yellow (no blue)
        let s = t - 170;
        RGB8::new(255, (200 + s * 55 / 85) as u8, 0)
    }
}

/// A fire / flame animation effect.
///
/// Simulates upward-propagating heat on an LED strip or ring.
/// LEDs at the base (index 0) ignite randomly each tick; heat diffuses toward
/// the tip (index `num_leds − 1`) and cools as it rises, producing a
/// naturalistic flickering flame.
///
/// Each LED's temperature is mapped through a black → dark red → orange →
/// yellow gradient. The simulation is driven by a built-in xorshift32 PRNG —
/// seed it with [`with_seed`](FireEffect::with_seed) for deterministic
/// sequences.
///
/// The maximum number of LEDs is [`MAX_LEDS`](crate::effect::MAX_LEDS).
///
/// # Diffusion modes
///
/// - **Linear** (default): heat at indices 0 and 1 acts as a fixed base anchor
///   — those cells are not touched by the diffusion step, so a small ignition
///   zone keeps producing fresh heat that propagates upward toward the tip.
/// - **Circular** (via [`with_wrap`](FireEffect::with_wrap)): heat[n-1] feeds
///   back into heat[0], producing a symmetric ring fire with no cold seam.
///
/// # Example
///
/// ```
/// use ferriswheel::{FireEffect, Effect};
/// use ferriswheel::RGB8;
///
/// let mut fire = FireEffect::new(12).unwrap()
///     .with_cooling(55)
///     .with_sparking(120);
/// let mut buffer = [RGB8::default(); 12];
///
/// fire.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FireEffect {
    num_leds: usize,
    /// Per-LED heat (0 = cold/black, 255 = peak/bright yellow).
    /// Only indices `0..num_leds` are active; the rest are always 0.
    heat: [u8; MAX_LEDS],
    /// Cooling rate per tick. Higher = shorter, cooler flames.
    cooling: u8,
    /// Per-tick probability of a new base spark (0 = never, 255 = always).
    sparking: u8,
    /// Current PRNG state (advances each tick).
    rng_state: u32,
    /// Seed stored at construction / `with_seed` time; restored by `reset()`.
    initial_seed: u32,
    /// When true, heat diffusion wraps around: `heat[n-1]` feeds back into
    /// `heat[0]`, producing a symmetric ring fire with no cold seam.
    wrap: bool,
    /// Number of LEDs at the base eligible for spark ignition.
    /// Default: `num_leds.min(3)`. Clamped to `1..=num_leds`.
    base_range: usize,
}

impl FireEffect {
    /// Creates a new fire effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Cooling: 55 (moderate cooling, medium-height flames)
    /// - Sparking: 120 (~47 % chance of a base spark per tick)
    /// - Seed: `0x1234_5678`
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;
        Ok(Self {
            num_leds,
            heat: [0u8; MAX_LEDS],
            cooling: 55,
            sparking: 120,
            rng_state: DEFAULT_SEED,
            initial_seed: DEFAULT_SEED,
            wrap: false,
            base_range: num_leds.min(3),
        })
    }

    /// Sets the cooling rate (0–255).
    ///
    /// Higher values cool LEDs faster, producing shorter, more erratic flames.
    /// Lower values retain heat longer, giving taller, steadier flames.
    pub fn with_cooling(mut self, cooling: u8) -> Self {
        self.cooling = cooling;
        self
    }

    /// Sets the per-tick spark probability at the base (0–255).
    ///
    /// `0` = no new sparks (existing flames fade out).
    /// `255` = a spark is guaranteed every tick.
    /// Values 1–254 give a `sparking / 256` probability.
    pub fn with_sparking(mut self, sparking: u8) -> Self {
        self.sparking = sparking;
        self
    }

    /// Enables or disables circular heat diffusion.
    ///
    /// When `true`, heat wraps around from the tip (index `n-1`) back to the
    /// base (index 0), producing a symmetric ring fire with no cold seam.
    /// When `false` (default), heat diffuses linearly from base to tip.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Sets the number of base LEDs eligible for spark ignition.
    ///
    /// Default: `num_leds.min(3)` — fine for small rings but too narrow for
    /// long strips (60+ LEDs). A wider range (e.g. `(n / 10).max(3)`) produces
    /// a more natural flame base on longer strips.
    ///
    /// The value is clamped to `1..=num_leds`.
    pub fn with_base_range(mut self, range: usize) -> Self {
        self.base_range = range.clamp(1, self.num_leds);
        self
    }

    /// Seeds the PRNG for reproducible flame sequences.
    ///
    /// A seed of `0` is promoted to `1` (xorshift32 requires non-zero state).
    /// The same seed is restored by [`reset`](FireEffect::reset).
    pub fn with_seed(mut self, seed: u32) -> Self {
        let s = seed.max(1);
        self.rng_state = s;
        self.initial_seed = s;
        self
    }

    /// Updates the cooling rate without resetting heat state.
    pub fn set_cooling(&mut self, cooling: u8) {
        self.cooling = cooling;
    }

    /// Updates the sparking probability without resetting heat state.
    pub fn set_sparking(&mut self, sparking: u8) {
        self.sparking = sparking;
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Returns the current cooling rate.
    pub fn cooling(&self) -> u8 {
        self.cooling
    }

    /// Returns the current sparking probability.
    pub fn sparking(&self) -> u8 {
        self.sparking
    }

    /// Advances the PRNG and returns the low byte of the new state.
    fn rng_byte(&mut self) -> u8 {
        self.rng_state = xorshift32(self.rng_state);
        self.rng_state as u8
    }

    /// Fills the buffer from the current heat state without advancing the simulation.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        for (led, heat) in buffer.iter_mut().zip(self.heat[..self.num_leds].iter()) {
            *led = fire_color(*heat);
        }
        Ok(())
    }

    /// Advances the fire simulation one step and fills the buffer.
    ///
    /// Each tick runs four stages:
    /// 1. Every cell cools by a random amount proportional to `cooling`.
    /// 2. Heat diffuses upward (base index 0 → tip index `num_leds − 1`).
    /// 3. A new base spark is maybe added, controlled by `sparking`.
    /// 4. Each LED's heat is mapped to a fire colour and written to `buffer`.
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;
        let n = self.num_leds;

        // Stage 1: cool every cell slightly.
        // Scale by ring size so medium `cooling` values work across all sizes.
        // The +2 ensures cooling_max >= 2 regardless of `cooling` or `n`.
        let cooling_max = ((self.cooling as u16 * 10 / n as u16) + 2).min(255) as u8;
        for i in 0..n {
            let cool = self.rng_byte() % cooling_max;
            self.heat[i] = self.heat[i].saturating_sub(cool);
        }

        // Stage 2: diffuse heat upward (toward higher indices).
        // Weighted average (a + a + b) / 3 gives double weight to the nearer
        // neighbour — the same convention used in the FastLED fire simulation.
        if self.wrap && n >= 2 {
            // Circular: every index averages its two predecessors (wrapping).
            // A snapshot avoids read-after-write: computing heat[0] reads
            // heat[n-1], which is also modified in this pass. Only the active
            // prefix is copied — the unused tail is irrelevant.
            let mut snapshot = [0u8; MAX_LEDS];
            snapshot[..n].copy_from_slice(&self.heat[..n]);
            for i in (0..n).rev() {
                let a = snapshot[(i + n - 1) % n] as u16;
                let b = snapshot[(i + n - 2) % n] as u16;
                self.heat[i] = ((a + a + b) / 3) as u8;
            }
        } else if n >= 3 {
            // Linear: indices 0 and 1 are "base" anchors, not diffused.
            // Iterate from the tip downward so each read uses un-mutated values.
            for i in (2..n).rev() {
                let a = self.heat[i - 1] as u16;
                let b = self.heat[i - 2] as u16;
                self.heat[i] = ((a + a + b) / 3) as u8;
            }
        }

        // Stage 3: randomly ignite the base.
        if self.sparking == 255 || (self.sparking > 0 && self.rng_byte() < self.sparking) {
            // Choose y in [0, base_range) without modulo bias.
            // Draw from 0..=255 and reject values in the incomplete top bucket.
            // base_range is upheld at >= 1 by `new` and `with_base_range`; the
            // assert guards `256 / range` from a division-by-zero panic.
            debug_assert!(self.base_range >= 1, "base_range invariant: must be >= 1");
            let range = self.base_range as u16;
            let limit = (256 / range) * range;
            let y = loop {
                let r = self.rng_byte() as u16;
                if r < limit {
                    break (r % range) as usize;
                }
            };
            let boost = self.rng_byte().saturating_add(100);
            self.heat[y] = self.heat[y].saturating_add(boost);
        }

        // Stage 4: map heat to colour.
        for (led, heat) in buffer.iter_mut().zip(self.heat[..n].iter()) {
            *led = fire_color(*heat);
        }

        Ok(())
    }

    /// Resets all heat to zero and restores the PRNG to its initial seed.
    pub fn reset(&mut self) {
        self.heat = [0u8; MAX_LEDS];
        self.rng_state = self.initial_seed;
    }
}

impl Effect for FireEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        FireEffect::update(self, buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        FireEffect::current(self, buffer)
    }

    fn reset(&mut self) {
        FireEffect::reset(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::MAX_LEDS;

    // ── constructor ──────────────────────────────────────────────────────────

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(FireEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = FireEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_new_with_too_many_leds_returns_error() {
        assert!(matches!(
            FireEffect::new(MAX_LEDS + 1).unwrap_err(),
            EffectError::TooManyLeds { .. }
        ));
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = FireEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        assert_eq!(
            effect.current(&mut buffer).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8,
            }
        );
    }

    // ── initial state ────────────────────────────────────────────────────────

    #[test]
    fn test_initial_state_all_black() {
        let effect = FireEffect::new(12).unwrap();
        let mut buffer = [RGB8::new(1, 1, 1); 12]; // pre-fill with non-black
        effect.current(&mut buffer).unwrap();
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(*led, RGB8::default(), "LED {i} should be black on init");
        }
    }

    // ── sparking behaviour ───────────────────────────────────────────────────

    #[test]
    fn test_sparking_zero_never_ignites() {
        // No sparks and no initial heat — LEDs must stay black forever.
        let mut effect = FireEffect::new(12).unwrap().with_sparking(0);
        let mut buffer = [RGB8::default(); 12];
        for _ in 0..50 {
            effect.update(&mut buffer).unwrap();
        }
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(
                *led,
                RGB8::default(),
                "LED {i} should stay black with sparking=0"
            );
        }
    }

    #[test]
    fn test_sparking_255_lights_base() {
        // Guaranteed spark every tick — at least one LED must be non-black after one update.
        let mut effect = FireEffect::new(12).unwrap().with_sparking(255);
        let mut buffer = [RGB8::default(); 12];
        effect.update(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "at least one LED should be lit with sparking=255"
        );
    }

    // ── current / update contract ────────────────────────────────────────────

    #[test]
    fn test_current_does_not_advance() {
        let mut effect = FireEffect::new(8).unwrap().with_sparking(255);
        let mut buffer = [RGB8::default(); 8];
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect.current(&mut buf1).unwrap();
        effect.current(&mut buf2).unwrap();
        assert_eq!(buf1, buf2, "current() must not change state");
    }

    #[test]
    fn test_update_changes_state_over_time() {
        // With sparking=255 the output must evolve between two consecutive updates.
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0);
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect.update(&mut buf1).unwrap();
        // Run several more ticks so diffusion spreads the initial heat.
        for _ in 0..5 {
            effect.update(&mut buf2).unwrap();
        }
        assert_ne!(buf1, buf2, "state must evolve over multiple updates");
    }

    // ── reset ────────────────────────────────────────────────────────────────

    #[test]
    fn test_reset_clears_all_leds() {
        let mut effect = FireEffect::new(12).unwrap().with_sparking(255);
        let mut buffer = [RGB8::default(); 12];
        for _ in 0..10 {
            effect.update(&mut buffer).unwrap();
        }
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "some LEDs should be lit before reset"
        );
        effect.reset();
        effect.current(&mut buffer).unwrap();
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(*led, RGB8::default(), "LED {i} should be black after reset");
        }
    }

    #[test]
    fn test_reset_restores_rng_sequence() {
        // Two runs from the same seed must produce byte-identical output.
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_sparking(255)
            .with_seed(0xDEAD_BEEF);
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        for _ in 0..5 {
            effect.update(&mut buf1).unwrap();
        }
        effect.reset();
        for _ in 0..5 {
            effect.update(&mut buf2).unwrap();
        }
        assert_eq!(
            buf1, buf2,
            "same seed must replay an identical sequence after reset"
        );
    }

    // ── live setters ─────────────────────────────────────────────────────────

    #[test]
    fn test_set_cooling_does_not_reset_heat() {
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0);
        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "heat should exist after sparking=255 update"
        );
        effect.set_cooling(10);
        effect.current(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "heat should survive set_cooling"
        );
    }

    #[test]
    fn test_set_sparking_does_not_reset_heat() {
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0);
        let mut buffer = [RGB8::default(); 8];
        effect.update(&mut buffer).unwrap();
        effect.set_sparking(0);
        effect.current(&mut buffer).unwrap();
        assert!(
            buffer.iter().any(|led| *led != RGB8::default()),
            "heat should survive set_sparking"
        );
    }

    // ── fire_color gradient ──────────────────────────────────────────────────

    #[test]
    fn test_fire_color_black_at_zero() {
        assert_eq!(fire_color(0), RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_fire_color_yellow_at_max() {
        assert_eq!(fire_color(255), RGB8::new(255, 255, 0));
    }

    #[test]
    fn test_fire_color_gradient_midpoints() {
        let c85 = fire_color(85);
        assert_eq!(c85, RGB8::new(180, 0, 0), "heat=85 should be dark red");

        let c170 = fire_color(170);
        assert_eq!(
            c170,
            RGB8::new(255, 200, 0),
            "heat=170 should be orange-yellow"
        );
    }

    #[test]
    fn test_fire_color_brightness_monotone() {
        // Total RGB brightness must never decrease as heat increases.
        let mut prev_sum: u32 = 0;
        for heat in 0u8..=255 {
            let c = fire_color(heat);
            let sum = c.r as u32 + c.g as u32 + c.b as u32;
            assert!(
                sum >= prev_sum,
                "brightness decreased at heat={heat}: prev={prev_sum}, now={sum}"
            );
            prev_sum = sum;
        }
    }

    // ── deterministic snapshot ───────────────────────────────────────────────

    #[test]
    fn test_snapshot_fixed_seed() {
        // Regression guard: any change to the gradient or PRNG that shifts
        // output will be caught here. Values were captured from the first run
        // with seed=0xABCD_1234, n=4, sparking=255, cooling=0, 3 ticks.
        let mut effect = FireEffect::new(4)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0)
            .with_seed(0xABCD_1234);
        let mut buf = [RGB8::default(); 4];
        for _ in 0..3 {
            effect.update(&mut buf).unwrap();
        }
        let expected = [
            RGB8::new(0, 0, 0),
            RGB8::new(255, 255, 0),
            RGB8::new(255, 255, 0),
            RGB8::new(255, 218, 0),
        ];
        assert_eq!(
            buf, expected,
            "snapshot mismatch — gradient or PRNG changed"
        );
    }

    // ── live getters ──────────────────────────────────────────────────────────

    #[test]
    fn test_cooling_getter() {
        let effect = FireEffect::new(8).unwrap().with_cooling(42);
        assert_eq!(effect.cooling(), 42);
    }

    #[test]
    fn test_sparking_getter() {
        let effect = FireEffect::new(8).unwrap().with_sparking(200);
        assert_eq!(effect.sparking(), 200);
    }

    // ── edge cases ───────────────────────────────────────────────────────────

    #[test]
    fn test_single_led_ring() {
        // n=1: no diffusion step; only cooling and sparking run.
        let mut effect = FireEffect::new(1)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0);
        let mut buffer = [RGB8::default(); 1];
        effect.update(&mut buffer).unwrap();
        assert_ne!(
            buffer[0],
            RGB8::default(),
            "single-LED ring should light after sparking=255 update"
        );
    }

    // ── trait object dispatch ─────────────────────────────────────────────────

    #[test]
    fn test_trait_object_update() {
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0);
        let effect_ref: &mut dyn Effect = &mut effect;
        let mut buf1 = [RGB8::default(); 8];
        let mut buf2 = [RGB8::default(); 8];
        effect_ref.update(&mut buf1).unwrap();
        for _ in 0..5 {
            effect_ref.update(&mut buf2).unwrap();
        }
        // After 1 vs 6 updates, heat has diffused further; output must differ.
        assert_ne!(buf1, buf2, "state must evolve between trait-object updates");
    }

    #[test]
    fn test_trait_reset_path() {
        let mut effect = FireEffect::new(8).unwrap().with_sparking(255).with_seed(42);
        let mut buf_before = [RGB8::default(); 8];
        let mut buf_after = [RGB8::default(); 8];
        let effect_ref: &mut dyn Effect = &mut effect;
        effect_ref.update(&mut buf_before).unwrap();
        effect_ref.reset();
        effect_ref.update(&mut buf_after).unwrap();
        assert_eq!(
            buf_before, buf_after,
            "trait reset must replay the same sequence"
        );
    }

    // ── wrap-around diffusion ───────────────────────────────────────────────

    #[test]
    fn test_wrap_default_is_false() {
        let effect = FireEffect::new(12).unwrap();
        assert!(!effect.wrap, "wrap should default to false");
    }

    #[test]
    fn test_wrap_enabled_propagates_tip_to_base() {
        // Direct test of the property: with wrap enabled, heat at heat[n-1]
        // must influence heat[0] after one diffusion step. With wrap disabled,
        // heat[0] is anchored and can only change via cooling/sparking.
        // We disable both so any change to heat[0] comes from diffusion alone.
        const N: usize = 8;
        let mut wrap_effect = FireEffect::new(N)
            .unwrap()
            .with_sparking(0)
            .with_cooling(0)
            .with_wrap(true);
        let mut linear_effect = FireEffect::new(N)
            .unwrap()
            .with_sparking(0)
            .with_cooling(0)
            .with_wrap(false);

        // Inject heat at the tip directly; both base and middle stay cold.
        wrap_effect.heat[N - 1] = 240;
        linear_effect.heat[N - 1] = 240;

        let mut wrap_buf = [RGB8::default(); N];
        let mut linear_buf = [RGB8::default(); N];
        wrap_effect.update(&mut wrap_buf).unwrap();
        linear_effect.update(&mut linear_buf).unwrap();

        // Wrap mode: heat[0] = (heat[n-1]*2 + heat[n-2]) / 3 ≈ 160.
        // Cooling has a +2 floor that may shave 0–1 off heat[n-1] first, so
        // we assert the property — heat[0] picks up substantial heat from the
        // tip — rather than a brittle exact value.
        assert!(
            wrap_effect.heat[0] > 100,
            "wrap mode must propagate heat[n-1] into heat[0]; got {}",
            wrap_effect.heat[0]
        );
        // Linear mode: heat[0] is a base anchor — diffusion never touches it,
        // sparking is off, and cooling on a zero cell saturates at 0.
        assert_eq!(
            linear_effect.heat[0], 0,
            "linear mode must leave heat[0] anchored at zero"
        );
    }

    #[test]
    fn test_wrap_single_led_no_panic() {
        let mut effect = FireEffect::new(1)
            .unwrap()
            .with_sparking(255)
            .with_cooling(0)
            .with_wrap(true);
        let mut buffer = [RGB8::default(); 1];
        // n=1 doesn't run the diffusion step (wrap path requires n >= 2);
        // only cooling and sparking run. Must not panic.
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }
    }

    #[test]
    fn test_wrap_two_leds_no_panic() {
        let mut effect = FireEffect::new(2)
            .unwrap()
            .with_sparking(255)
            .with_wrap(true);
        let mut buffer = [RGB8::default(); 2];
        for _ in 0..5 {
            effect.update(&mut buffer).unwrap();
        }
    }

    // ── base range parameterisation ──────────────────────────────────────────

    #[test]
    fn test_base_range_default_small_ring() {
        let effect = FireEffect::new(12).unwrap();
        assert_eq!(
            effect.base_range, 3,
            "default for n=12 should be min(12,3)=3"
        );
    }

    #[test]
    fn test_base_range_default_tiny_ring() {
        let effect = FireEffect::new(2).unwrap();
        assert_eq!(effect.base_range, 2, "default for n=2 should be min(2,3)=2");
    }

    #[test]
    fn test_base_range_default_single_led() {
        let effect = FireEffect::new(1).unwrap();
        assert_eq!(effect.base_range, 1, "default for n=1 should be 1");
    }

    #[test]
    fn test_with_base_range_sets_value() {
        let effect = FireEffect::new(12).unwrap().with_base_range(6);
        assert_eq!(effect.base_range, 6);
    }

    #[test]
    fn test_with_base_range_clamps_to_num_leds() {
        let effect = FireEffect::new(8).unwrap().with_base_range(20);
        assert_eq!(effect.base_range, 8, "should clamp to num_leds");
    }

    #[test]
    fn test_with_base_range_clamps_zero_to_one() {
        let effect = FireEffect::new(8).unwrap().with_base_range(0);
        assert_eq!(effect.base_range, 1, "should clamp 0 to 1");
    }

    #[test]
    fn test_wide_base_range_ignites_beyond_index_2() {
        // With base_range=8 and sparking=255, sparks can land at indices 3..7.
        // Run enough ticks that at least one spark lands beyond index 2.
        let mut effect = FireEffect::new(8)
            .unwrap()
            .with_base_range(8)
            .with_sparking(255)
            .with_cooling(0);
        let mut buffer = [RGB8::default(); 8];
        let mut saw_high_index = false;
        for _ in 0..50 {
            effect.update(&mut buffer).unwrap();
            // Check if any LED at index 3+ is non-black (heat was sparked there).
            if buffer[3..].iter().any(|led| *led != RGB8::default()) {
                saw_high_index = true;
                break;
            }
        }
        assert!(
            saw_high_index,
            "with base_range=8, sparks should reach indices beyond 2"
        );
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = FireEffect::new(4).unwrap();
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

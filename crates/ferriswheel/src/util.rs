//! Pure utility functions for LED effects.
//!
//! These helpers are used by multiple effects and are useful for custom effects too.

use rgb::RGB8;

/// 256-entry sine lookup table.
///
/// Maps a phase angle (0–255) to amplitude (0–255).
/// Phase 0 = 0, peak (~255) at phase ~113–115, descends to 0 around phase 230,
/// then stays 0 through phase 255 (half-wave rectified).
/// Values from rustyfarian-knob, tested on hardware.
#[rustfmt::skip]
const SINE_TABLE: [u8; 256] = [
      0,   3,   6,   9,  12,  16,  19,  22,  25,  28,  31,  34,  37,  40,  44,  47,
     50,  53,  56,  59,  62,  65,  68,  71,  74,  77,  80,  83,  86,  89,  92,  95,
     98, 100, 103, 106, 109, 112, 115, 117, 120, 123, 126, 128, 131, 134, 136, 139,
    142, 144, 147, 149, 152, 154, 157, 159, 162, 164, 167, 169, 171, 174, 176, 178,
    181, 183, 185, 187, 189, 192, 194, 196, 198, 200, 202, 204, 206, 207, 209, 211,
    213, 215, 216, 218, 220, 221, 223, 225, 226, 228, 229, 231, 232, 234, 235, 236,
    238, 239, 240, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 253,
    254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 253,
    253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 240, 239, 238, 236,
    235, 234, 232, 231, 229, 228, 226, 225, 223, 221, 220, 218, 216, 215, 213, 211,
    209, 207, 206, 204, 202, 200, 198, 196, 194, 192, 189, 187, 185, 183, 181, 178,
    176, 174, 171, 169, 167, 164, 162, 159, 157, 154, 152, 149, 147, 144, 142, 139,
    136, 134, 131, 128, 126, 123, 120, 117, 115, 112, 109, 106, 103, 100,  98,  95,
     92,  89,  86,  83,  80,  77,  74,  71,  68,  65,  62,  59,  56,  53,  50,  47,
     44,  40,  37,  34,  31,  28,  25,  22,  19,  16,  12,   9,   6,   3,   0,   0,
      0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
];

/// Returns a sine-wave value for the given phase.
///
/// The phase maps a full cycle (0–255) to an output amplitude (0–255).
/// Values rise from 0 at phase 0 to a peak of 255 around phase 113–115,
/// then fall back toward 0 around phase 230, staying at 0 through phase 255
/// (half-wave rectified).
///
/// This is useful for breathing/pulsing effects.
pub fn sine_wave(phase: u8) -> u8 {
    SINE_TABLE[phase as usize]
}

/// Returns a full symmetric sine-wave value for the given phase.
///
/// Maps a full cycle (0–255) to an output amplitude (0–255) with no plateau
/// at zero. The wave rises from 0 at phase 0 to a peak of 255 at phase 128,
/// then falls symmetrically back to 0 at phase 255.
///
/// Unlike [`sine_wave`], which is half-wave rectified and stays at 0 for
/// roughly a quarter of the cycle, `sine_full` gives a smooth continuous
/// oscillation — useful for breathing effects where the brightness should
/// rise and fall without any pause at the floor.
///
/// # Example
///
/// ```
/// use ferriswheel::sine_full;
///
/// assert_eq!(sine_full(0), 0);
/// assert_eq!(sine_full(128), 255);
/// assert_eq!(sine_full(255), 0);
/// ```
pub fn sine_full(phase: u8) -> u8 {
    // Use only the strictly rising window of SINE_TABLE: indices 0..=113.
    // Index 113 is the first entry that reaches 255 (the peak); entries beyond
    // it are plateau (255) or falling, so stopping here keeps the window monotone.
    //
    // Both half-cycles are mapped onto this 114-entry window:
    //   rising  (phase   0..=127): index = phase * 114 / 128  →  0..=113
    //   falling (phase 128..=255): index = 113 - (phase-128) * 114 / 128  →  113..=0
    //
    // 128  = half-cycle length (256 / 2)
    // 114  = number of table entries in the window (indices 0 through 113, inclusive),
    //        chosen so that phase=127 maps to index 113 and phase=255 maps back to 0.
    if phase < 128 {
        sine_wave((phase as u16 * 114 / 128) as u8)
    } else {
        // (phase - 128) is 0..=127; scaled by 114/128 it stays within 0..=113,
        // so the subtraction from 113 is always non-negative.
        let offset = ((phase - 128) as u16 * 114 / 128) as u8;
        debug_assert!(offset <= 113, "offset {offset} exceeds window bound 113");
        sine_wave(113u8 - offset)
    }
}

/// Scales a single color channel by a brightness factor (0–255).
///
/// Uses integer math: `(channel * brightness) / 255`.
pub fn scale_brightness(color: RGB8, brightness: u8) -> RGB8 {
    let b = brightness as u16;
    RGB8::new(
        ((color.r as u16 * b) / 255) as u8,
        ((color.g as u16 * b) / 255) as u8,
        ((color.b as u16 * b) / 255) as u8,
    )
}

/// Linearly interpolates between two colors.
///
/// `t` ranges from 0 (returns `a`) to 255 (returns `b`).
pub fn lerp_color(a: RGB8, b: RGB8, t: u8) -> RGB8 {
    let t16 = t as u16;
    let inv = 255 - t16;
    RGB8::new(
        ((a.r as u16 * inv + b.r as u16 * t16) / 255) as u8,
        ((a.g as u16 * inv + b.g as u16 * t16) / 255) as u8,
        ((a.b as u16 * inv + b.b as u16 * t16) / 255) as u8,
    )
}

/// Fills all elements of `buffer` with the given color.
pub fn fill_solid(buffer: &mut [RGB8], color: RGB8) {
    for pixel in buffer.iter_mut() {
        *pixel = color;
    }
}

/// Clamps a requested tail length so it cannot reach or exceed `num_leds`.
///
/// The maximum valid tail is `num_leds - 1`; requesting more than that would let
/// the tail overwrite the head LED.  `num_leds.saturating_sub(1)` is used so
/// that `num_leds = 0` returns 0 rather than wrapping.
pub(crate) fn clamp_tail_length(tail: u8, num_leds: usize) -> u8 {
    tail.min(num_leds.saturating_sub(1).min(u8::MAX as usize) as u8)
}

/// Scales a pre-computed sine value into the `[min_brightness, max_brightness]` range.
///
/// Used by brightness-oscillating effects (e.g. [`PulseEffect`], [`BreatheEffect`]) to
/// convert a raw sine-table output into a final brightness value.  The caller is
/// responsible for obtaining `sine_val` from the appropriate table function
/// ([`sine_wave`] or [`sine_full`]).
///
/// If `min_brightness > max_brightness` the two are swapped automatically, so the
/// result is always well-defined.
///
/// [`PulseEffect`]: crate::PulseEffect
/// [`BreatheEffect`]: crate::BreatheEffect
pub(crate) fn brightness_from_sine(sine_val: u8, min_brightness: u8, max_brightness: u8) -> u8 {
    let lo = min_brightness.min(max_brightness) as u16;
    let hi = min_brightness.max(max_brightness) as u16;
    (lo + (sine_val as u16 * (hi - lo)) / 255) as u8
}

/// Default PRNG seed shared by effects that embed an xorshift32 generator.
///
/// The value is non-zero (required by xorshift32) and was chosen to produce
/// a pleasant default sparkle / flame pattern on first use.
pub(crate) const DEFAULT_SEED: u32 = 0x1234_5678;

/// xorshift32 — a fast, `no_std`-compatible PRNG with period 2^32 − 1.
///
/// State must be non-zero; a `debug_assert!` catches misuse in debug builds.
pub(crate) fn xorshift32(mut x: u32) -> u32 {
    debug_assert!(x != 0, "xorshift32 state must be non-zero");
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    x
}

/// Advances a scanner head one step using reflection arithmetic.
///
/// Returns the new `(position, forward)` pair. Large `speed` values are
/// handled by clamping the reflected position rather than wrapping or panicking.
pub(crate) fn scanner_bounce(
    position: u8,
    forward: bool,
    speed: u8,
    num_leds: usize,
) -> (u8, bool) {
    let n = num_leds as isize;
    let mut pos = position as isize;
    let step = speed as isize;
    let mut fwd = forward;

    if fwd {
        pos += step;
        if pos >= n {
            // Reflect off the top end; clamp to 0 in case step > 2*(n-1).
            pos = (2 * (n - 1) - pos).max(0);
            fwd = false;
        }
    } else {
        pos -= step;
        if pos < 0 {
            // Reflect off the bottom end; clamp to n-1 in case step > 2*(n-1).
            pos = (-pos).min(n - 1);
            fwd = true;
        }
    }

    (pos as u8, fwd)
}

/// Draws a scanner head and its exponentially-decaying tail into `buffer`.
pub(crate) fn draw_scanner_head(
    buffer: &mut [RGB8],
    num_leds: usize,
    head: usize,
    forward: bool,
    color: RGB8,
    tail_length: u8,
    decay: u8,
) {
    buffer[head] = color;

    let effective_tail = (tail_length as usize).min(num_leds - 1);
    let mut brightness: u16 = 255;
    for i in 1..=effective_tail {
        brightness = brightness * decay as u16 / 255;
        if brightness == 0 {
            break;
        }
        let tail_idx = if forward {
            // Moving toward higher indices: tail is behind at lower indices.
            match head.checked_sub(i) {
                Some(idx) => idx,
                None => break,
            }
        } else {
            // Moving toward lower indices: tail is behind at higher indices.
            let idx = head + i;
            if idx >= num_leds {
                break;
            }
            idx
        };
        buffer[tail_idx] = scale_brightness(color, brightness as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_wave_zero_at_phase_0() {
        assert_eq!(sine_wave(0), 0);
    }

    #[test]
    fn test_sine_wave_peak_at_phase_64() {
        assert_eq!(sine_wave(64), 181);
    }

    #[test]
    fn test_sine_wave_max_at_quarter() {
        let peak = (113..=115).map(sine_wave).max().unwrap();
        assert_eq!(peak, 255);
    }

    #[test]
    fn test_sine_wave_descending_at_phase_128() {
        // Phase 128 is past the peak, value should be less than the peak
        assert!(sine_wave(128) < 255);
        assert!(sine_wave(128) > 200);
    }

    #[test]
    fn test_sine_wave_second_half_zero() {
        for phase in 240..=255 {
            assert_eq!(sine_wave(phase), 0, "phase {} should be 0", phase);
        }
    }

    #[test]
    fn test_sine_wave_monotonic_rise() {
        // Table should rise monotonically from 0 to the peak
        for i in 0..113 {
            assert!(
                sine_wave(i) <= sine_wave(i + 1),
                "sine_wave({}) = {} > sine_wave({}) = {}",
                i,
                sine_wave(i),
                i + 1,
                sine_wave(i + 1)
            );
        }
    }

    #[test]
    fn test_sine_wave_monotonic_fall() {
        // Table should fall monotonically from peak to tail
        for i in 115..239 {
            assert!(
                sine_wave(i) >= sine_wave(i + 1),
                "sine_wave({}) = {} < sine_wave({}) = {}",
                i,
                sine_wave(i),
                i + 1,
                sine_wave(i + 1)
            );
        }
    }

    #[test]
    fn test_sine_full_zero_at_start_and_end() {
        assert_eq!(sine_full(0), 0);
        assert_eq!(sine_full(255), 0);
    }

    #[test]
    fn test_sine_full_peak_at_midpoint() {
        assert_eq!(sine_full(128), 255);
    }

    #[test]
    fn test_sine_full_symmetric() {
        // The curve should be roughly symmetric around the peak at phase 128.
        // Allow ±1 tolerance for integer rounding across the rising/falling mapping.
        for offset in 1u8..=60 {
            let rise = sine_full(128u8.saturating_sub(offset));
            let fall = sine_full(128u8.saturating_add(offset));
            let diff = (rise as i16 - fall as i16).unsigned_abs();
            assert!(
                diff <= 1,
                "sine_full({}) = {} vs sine_full({}) = {}, diff {}",
                128 - offset,
                rise,
                128 + offset as u16,
                fall,
                diff
            );
        }
    }

    #[test]
    fn test_sine_full_no_extended_plateau() {
        // Count how many phases give a zero output.
        // PulseEffect's half-wave sine has ~26 zero phases; sine_full should have at most 3.
        let zero_count = (0u16..=255).filter(|&p| sine_full(p as u8) == 0).count();
        assert!(
            zero_count <= 3,
            "expected at most 3 zero phases, got {}",
            zero_count
        );
    }

    #[test]
    fn test_sine_full_monotonic_rise() {
        // Should be non-decreasing from phase 0 to the peak at 128.
        for i in 1u8..128 {
            let prev = sine_full(i - 1);
            let curr = sine_full(i);
            assert!(
                curr >= prev,
                "sine_full({}) = {} < sine_full({}) = {}",
                i,
                curr,
                i - 1,
                prev
            );
        }
    }

    #[test]
    fn test_sine_full_monotonic_fall() {
        // Should be non-increasing from the peak at 128 to phase 255.
        for i in 129u8..=255 {
            let prev = sine_full(i - 1);
            let curr = sine_full(i);
            assert!(
                curr <= prev,
                "sine_full({}) = {} > sine_full({}) = {}",
                i,
                curr,
                i - 1,
                prev
            );
        }
    }

    #[test]
    fn test_scale_brightness_full() {
        let color = RGB8::new(100, 200, 50);
        let result = scale_brightness(color, 255);
        assert_eq!(result, color);
    }

    #[test]
    fn test_scale_brightness_zero() {
        let color = RGB8::new(100, 200, 50);
        let result = scale_brightness(color, 0);
        assert_eq!(result, RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_scale_brightness_half() {
        let color = RGB8::new(200, 100, 50);
        let result = scale_brightness(color, 128);
        assert!(result.r > 90 && result.r < 110);
        assert!(result.g > 45 && result.g < 55);
        assert!(result.b > 20 && result.b < 30);
    }

    #[test]
    fn test_lerp_color_at_zero() {
        let a = RGB8::new(255, 0, 0);
        let b = RGB8::new(0, 255, 0);
        let result = lerp_color(a, b, 0);
        assert_eq!(result, a);
    }

    #[test]
    fn test_lerp_color_at_max() {
        let a = RGB8::new(255, 0, 0);
        let b = RGB8::new(0, 255, 0);
        let result = lerp_color(a, b, 255);
        assert_eq!(result, b);
    }

    #[test]
    fn test_lerp_color_at_midpoint() {
        let a = RGB8::new(0, 0, 0);
        let b = RGB8::new(200, 100, 50);
        let result = lerp_color(a, b, 128);
        assert!(result.r > 90 && result.r < 110);
        assert!(result.g > 45 && result.g < 55);
        assert!(result.b > 20 && result.b < 30);
    }

    #[test]
    fn test_fill_solid() {
        let mut buffer = [RGB8::default(); 5];
        let color = RGB8::new(10, 20, 30);
        fill_solid(&mut buffer, color);
        for pixel in &buffer {
            assert_eq!(*pixel, color);
        }
    }

    #[test]
    fn test_fill_solid_empty_buffer() {
        let mut buffer: [RGB8; 0] = [];
        fill_solid(&mut buffer, RGB8::new(10, 20, 30));
        // Should not panic
    }

    #[test]
    fn test_scanner_bounce_reflects_at_top_boundary() {
        assert_eq!(scanner_bounce(19, true, 1, 20), (18, false));
    }

    #[test]
    fn test_scanner_bounce_reflects_at_bottom_boundary() {
        assert_eq!(scanner_bounce(0, false, 1, 20), (1, true));
    }

    #[test]
    fn test_scanner_bounce_clamps_large_steps() {
        assert_eq!(scanner_bounce(5, true, 50, 20), (0, false));
        assert_eq!(scanner_bounce(10, false, 255, 20), (19, true));
    }

    #[test]
    fn test_draw_scanner_head_forward_trails_toward_lower_indices() {
        let red = RGB8::new(255, 0, 0);
        let mut buffer = [RGB8::default(); 10];
        draw_scanner_head(&mut buffer, 10, 4, true, red, 3, 255);
        assert_eq!(buffer[4], red);
        assert_eq!(buffer[3], red);
        assert_eq!(buffer[2], red);
        assert_eq!(buffer[1], red);
        assert_eq!(buffer[0], RGB8::default());
        assert_eq!(buffer[5], RGB8::default());
    }

    #[test]
    fn test_draw_scanner_head_backward_trails_toward_higher_indices() {
        let blue = RGB8::new(0, 0, 255);
        let mut buffer = [RGB8::default(); 10];
        draw_scanner_head(&mut buffer, 10, 5, false, blue, 3, 255);
        assert_eq!(buffer[5], blue);
        assert_eq!(buffer[6], blue);
        assert_eq!(buffer[7], blue);
        assert_eq!(buffer[8], blue);
        assert_eq!(buffer[4], RGB8::default());
        assert_eq!(buffer[9], RGB8::default());
    }

    #[test]
    fn test_draw_scanner_head_clamps_at_strip_boundary_and_applies_decay() {
        let white = RGB8::new(255, 255, 255);
        let mut buffer = [RGB8::default(); 10];
        draw_scanner_head(&mut buffer, 10, 2, true, white, 5, 128);
        assert_eq!(buffer[2], white);
        assert_eq!(buffer[1], scale_brightness(white, 128));
        assert_eq!(buffer[0], scale_brightness(white, 64));
        assert_eq!(buffer[3], RGB8::default());
    }

    #[test]
    fn test_clamp_tail_length_clamps_to_num_leds_minus_one() {
        assert_eq!(clamp_tail_length(255, 4), 3);
        assert_eq!(clamp_tail_length(10, 5), 4);
    }

    #[test]
    fn test_clamp_tail_length_zero_leds() {
        assert_eq!(clamp_tail_length(0, 0), 0);
        assert_eq!(clamp_tail_length(5, 0), 0);
    }

    #[test]
    fn test_clamp_tail_length_within_range_unchanged() {
        assert_eq!(clamp_tail_length(2, 10), 2);
        assert_eq!(clamp_tail_length(0, 5), 0);
    }

    #[test]
    fn test_brightness_from_sine_endpoints() {
        assert_eq!(brightness_from_sine(0, 10, 200), 10);
        assert_eq!(brightness_from_sine(255, 10, 200), 200);
    }

    #[test]
    fn test_brightness_from_sine_full_range() {
        assert_eq!(brightness_from_sine(0, 0, 255), 0);
        assert_eq!(brightness_from_sine(255, 0, 255), 255);
        assert_eq!(brightness_from_sine(128, 0, 255), 128);
    }

    #[test]
    fn test_brightness_from_sine_inverted_range() {
        // Inverted min/max should produce the same result as the canonical order.
        assert_eq!(
            brightness_from_sine(0, 200, 10),
            brightness_from_sine(0, 10, 200)
        );
        assert_eq!(
            brightness_from_sine(255, 200, 10),
            brightness_from_sine(255, 10, 200)
        );
    }

    #[test]
    fn test_xorshift32_is_deterministic() {
        assert_eq!(xorshift32(DEFAULT_SEED), xorshift32(DEFAULT_SEED));
    }

    #[test]
    fn test_xorshift32_produces_nonzero() {
        let mut state = DEFAULT_SEED;
        for _ in 0..100 {
            state = xorshift32(state);
            assert_ne!(state, 0);
        }
    }

    #[test]
    fn test_xorshift32_advances_state() {
        let a = xorshift32(DEFAULT_SEED);
        let b = xorshift32(a);
        assert_ne!(a, b);
        assert_ne!(a, DEFAULT_SEED);
    }
}

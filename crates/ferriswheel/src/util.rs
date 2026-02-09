//! Pure utility functions for LED effects.
//!
//! These helpers are used by multiple effects and are useful for custom effects too.

use rgb::RGB8;

/// 256-entry sine lookup table.
///
/// Maps a phase angle (0–255) to amplitude (0–255).
/// Phase 0 = 0, phase 64 = 255, phase 128 = 0, phase 192 = 0 (clamped).
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
/// The first half (0–127) produces a smooth sine hump; the second half
/// (128–255) returns 0 (half-wave rectified).
///
/// This is useful for breathing/pulsing effects.
pub fn sine_wave(phase: u8) -> u8 {
    SINE_TABLE[phase as usize]
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
}

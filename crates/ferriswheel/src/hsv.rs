//! Pure integer-math HSV to RGB conversion.
//!
//! This module provides a `no_std` compatible, float-free implementation
//! of HSV to RGB color conversion suitable for embedded systems.

use rgb::RGB8;

/// Converts HSV color values to RGB.
///
/// Uses integer-only arithmetic (16-bit intermediates) for embedded compatibility.
///
/// # Arguments
///
/// * `hue` - Color hue (0-255, wraps around: 0=red, 85≈green, 170≈blue)
/// * `saturation` - Color saturation (0=grayscale, 255=full color)
/// * `value` - Brightness value (0=black, 255=full brightness)
///
/// # Example
///
/// ```
/// use ferriswheel::hsv_to_rgb;
/// use rgb::RGB8;
///
/// // Pure red
/// let red = hsv_to_rgb(0, 255, 255);
/// assert_eq!(red, RGB8::new(255, 0, 0));
///
/// // 50% gray
/// let gray = hsv_to_rgb(0, 0, 128);
/// assert_eq!(gray, RGB8::new(128, 128, 128));
/// ```
pub fn hsv_to_rgb(hue: u8, saturation: u8, value: u8) -> RGB8 {
    // Handle grayscale case (saturation = 0)
    if saturation == 0 {
        return RGB8::new(value, value, value);
    }

    // Scale hue to 0-1535 range (256 * 6 - 1) for six color sectors
    // Each sector spans ~256 values (0-255, 256-511, etc.)
    let h = hue as u16 * 6;
    let sector = (h >> 8) as u8; // 0-5
    let fraction = (h & 0xFF) as u8; // 0-255 within a sector

    let s = saturation as u16;
    let v = value as u16;

    // Calculate color components using 16-bit math
    // p = value * (1 - saturation)
    let p = ((v * (255 - s)) / 255) as u8;
    // q = value * (1 - saturation * fraction)
    let q = ((v * (255 - (s * fraction as u16) / 255)) / 255) as u8;
    // t = value * (1 - saturation * (1 - fraction))
    let t = ((v * (255 - (s * (255 - fraction as u16)) / 255)) / 255) as u8;

    match sector {
        0 => RGB8::new(value, t, p), // Red to Yellow
        1 => RGB8::new(q, value, p), // Yellow to Green
        2 => RGB8::new(p, value, t), // Green to Cyan
        3 => RGB8::new(p, q, value), // Cyan to Blue
        4 => RGB8::new(t, p, value), // Blue to Magenta
        _ => RGB8::new(value, p, q), // Magenta to Red (sector 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_red_at_hue_0() {
        let color = hsv_to_rgb(0, 255, 255);
        assert_eq!(color, RGB8::new(255, 0, 0));
    }

    #[test]
    fn test_green_at_hue_85() {
        // Hue 85 is approximately 1/3 of 255, should be green
        let color = hsv_to_rgb(85, 255, 255);
        // Green should be dominant
        assert!(color.g > color.r);
        assert!(color.g > color.b);
        assert_eq!(color.g, 255);
    }

    #[test]
    fn test_blue_at_hue_170() {
        // Hue 170 is approximately 2/3 of 255, should be blue
        let color = hsv_to_rgb(170, 255, 255);
        // Blue should be dominant
        assert!(color.b > color.r);
        assert!(color.b > color.g);
        assert_eq!(color.b, 255);
    }

    #[test]
    fn test_white_with_zero_saturation() {
        let color = hsv_to_rgb(0, 0, 255);
        assert_eq!(color, RGB8::new(255, 255, 255));
    }

    #[test]
    fn test_gray_with_zero_saturation() {
        let color = hsv_to_rgb(0, 0, 128);
        assert_eq!(color, RGB8::new(128, 128, 128));
    }

    #[test]
    fn test_black_with_zero_value() {
        let color = hsv_to_rgb(0, 255, 0);
        assert_eq!(color, RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_black_with_any_hue_zero_value() {
        // Any hue with value=0 should be black
        for hue in [0, 42, 85, 128, 170, 213, 255] {
            let color = hsv_to_rgb(hue, 255, 0);
            assert_eq!(color, RGB8::new(0, 0, 0), "hue {} should be black", hue);
        }
    }

    #[test]
    fn test_half_brightness_red() {
        let color = hsv_to_rgb(0, 255, 128);
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    }

    #[test]
    fn test_half_saturation_red() {
        // Half-saturation means mixing with white
        let color = hsv_to_rgb(0, 128, 255);
        // Red should still be 255
        assert_eq!(color.r, 255);
        // Green and blue should be about half (white mixed in)
        assert!(color.g > 100 && color.g < 140);
        assert!(color.b > 100 && color.b < 140);
    }

    #[test]
    fn test_hue_wraps_around() {
        // Hue 255 should be close to red (hue 0)
        let color_255 = hsv_to_rgb(255, 255, 255);
        let color_0 = hsv_to_rgb(0, 255, 255);
        // Both should be predominantly red
        assert!(color_255.r > 200);
        assert_eq!(color_0.r, 255);
    }
}

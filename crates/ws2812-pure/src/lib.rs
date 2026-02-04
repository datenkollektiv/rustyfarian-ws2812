#![cfg_attr(not(test), no_std)]

//! Pure Rust WS2812 color utilities.
//!
//! This crate provides hardware-independent color conversion and bit manipulation
//! utilities for WS2812 (NeoPixel) LEDs. It has no ESP or embedded dependencies,
//! making it fully testable on any platform.

use rgb::RGB8;

/// Converts RGB to GRB u32 format (WS2812 color order).
///
/// WS2812 LEDs expect color data in GRB order, not RGB.
/// This function packs the color into a 24-bit value with:
/// - Bits 23-16: Green
/// - Bits 15-8: Red
/// - Bits 7-0: Blue
///
/// # Example
///
/// ```
/// use ws2812_pure::rgb_to_grb;
/// use rgb::RGB8;
///
/// let red = RGB8::new(255, 0, 0);
/// assert_eq!(rgb_to_grb(red), 0x00FF00); // Green=0, Red=255, Blue=0
/// ```
pub fn rgb_to_grb(rgb: RGB8) -> u32 {
    ((rgb.g as u32) << 16) | ((rgb.r as u32) << 8) | rgb.b as u32
}

/// Extracts bit values from a 24-bit color for WS2812 transmission.
///
/// Returns an array of 24 booleans representing each bit, MSB first.
/// This is the order required by WS2812 protocol.
///
/// # Example
///
/// ```
/// use ws2812_pure::color_to_bits;
///
/// let bits = color_to_bits(0b101010101010101010101010);
/// assert_eq!(bits[0], true);  // MSB
/// assert_eq!(bits[1], false);
/// assert_eq!(bits[23], false); // LSB
/// ```
pub fn color_to_bits(color: u32) -> [bool; 24] {
    let mut bits = [false; 24];
    for i in (0..24).rev() {
        bits[23 - i] = (color >> i) & 1 != 0;
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_grb_red() {
        let red = RGB8::new(255, 0, 0);
        // GRB format: G=0x00, R=0xFF, B=0x00 -> 0x00FF00
        assert_eq!(rgb_to_grb(red), 0x00FF00);
    }

    #[test]
    fn test_rgb_to_grb_green() {
        let green = RGB8::new(0, 255, 0);
        // GRB format: G=0xFF, R=0x00, B=0x00 -> 0xFF0000
        assert_eq!(rgb_to_grb(green), 0xFF0000);
    }

    #[test]
    fn test_rgb_to_grb_blue() {
        let blue = RGB8::new(0, 0, 255);
        // GRB format: G=0x00, R=0x00, B=0xFF -> 0x0000FF
        assert_eq!(rgb_to_grb(blue), 0x0000FF);
    }

    #[test]
    fn test_rgb_to_grb_white() {
        let white = RGB8::new(255, 255, 255);
        // GRB format: G=0xFF, R=0xFF, B=0xFF -> 0xFFFFFF
        assert_eq!(rgb_to_grb(white), 0xFFFFFF);
    }

    #[test]
    fn test_rgb_to_grb_black() {
        let black = RGB8::new(0, 0, 0);
        assert_eq!(rgb_to_grb(black), 0x000000);
    }

    #[test]
    fn test_rgb_to_grb_mixed() {
        let color = RGB8::new(0x12, 0x34, 0x56);
        // GRB format: G=0x34, R=0x12, B=0x56 -> 0x341256
        assert_eq!(rgb_to_grb(color), 0x341256);
    }

    #[test]
    fn test_color_to_bits_all_ones() {
        let bits = color_to_bits(0xFFFFFF);
        assert!(bits.iter().all(|&b| b));
    }

    #[test]
    fn test_color_to_bits_all_zeros() {
        let bits = color_to_bits(0x000000);
        assert!(bits.iter().all(|&b| !b));
    }

    #[test]
    fn test_color_to_bits_alternating() {
        // 0xAAAAAA = 10101010 10101010 10101010
        let bits = color_to_bits(0xAAAAAA);
        for i in 0..24 {
            assert_eq!(bits[i], i % 2 == 0, "bit {} should be {}", i, i % 2 == 0);
        }
    }

    #[test]
    fn test_color_to_bits_msb_first() {
        // 0x800000 = 1 followed by 23 zeros
        let bits = color_to_bits(0x800000);
        assert!(bits[0], "MSB should be set");
        assert!(bits[1..].iter().all(|&b| !b), "all other bits should be 0");
    }

    #[test]
    fn test_color_to_bits_lsb() {
        // 0x000001 = 23 zeros followed by 1
        let bits = color_to_bits(0x000001);
        assert!(bits[23], "LSB should be set");
        assert!(bits[..23].iter().all(|&b| !b), "all other bits should be 0");
    }
}

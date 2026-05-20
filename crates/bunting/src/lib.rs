#![cfg_attr(not(test), no_std)]

//! Pure Rust WS2812 color utilities.
//!
//! This crate provides hardware-independent color conversion and bit manipulation
//! utilities for WS2812 (NeoPixel) LEDs. It has no ESP or embedded dependencies,
//! making it fully testable on any platform.
//!
//! ## SPI Pre-rendering
//!
//! [`prerender_spi`] encodes `&[RGB8]` into a byte buffer suitable for SPI-based
//! WS2812 transmission (4 SPI bits per WS2812 data bit, 12 bytes per LED).
//! The encoding is byte-for-byte compatible with
//! [`ws2812-spi`](https://crates.io/crates/ws2812-spi) v0.5.1's prerendered module.
//!
//! ## Grid primitives
//!
//! The [`grid`] module provides `GridLayout`, `GridBuffer`, and gamma/brightness
//! helpers for addressing WS2812 LEDs wired as a rectangular matrix.

pub mod grid;

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
/// use bunting::rgb_to_grb;
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
/// use bunting::color_to_bits;
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

/// Error returned by [`prerender_spi`] when the output buffer is too small.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpiEncodeError {
    BufferTooSmall,
}

impl core::fmt::Display for SpiEncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "SPI output buffer too small"),
        }
    }
}

/// Returns the number of SPI data bytes required to encode `num_leds` LEDs.
///
/// This covers pixel data only — it does **not** include reset bytes.
/// Each LED requires 12 SPI bytes (24 WS2812 bits × 4 SPI bits each ÷ 8).
pub const fn spi_data_len(num_leds: usize) -> usize {
    num_leds * 12
}

/// Minimum reset bytes for 2 MHz SPI clock.
///
/// Append this many `0x00` bytes after LED data for single-transaction use.
/// At 2 MHz: 280 µs × 2 Mbit/s ÷ 8 = 70 bytes. 80 provides margin.
pub const SPI_RESET_BYTES_2MHZ: usize = 80;

/// SPI lookup table: maps each 2-bit WS2812 pair to an SPI byte.
///
/// Index by `(grb_value >> shift) & 0b11` for each 2-bit pair (MSB-first).
///
/// | Pair | Byte   | WS2812 meaning              |
/// |------|--------|-----------------------------|
/// | `00` | `0x88` | bit0=low (`1000`), bit1=low  |
/// | `01` | `0x8E` | bit0=low, bit1=high (`1110`) |
/// | `10` | `0xE8` | bit0=high, bit1=low          |
/// | `11` | `0xEE` | bit0=high, bit1=high         |
const SPI_PATTERNS: [u8; 4] = [0x88, 0x8E, 0xE8, 0xEE];

/// Pre-renders `colors` into a WS2812-compatible SPI byte buffer.
///
/// Encodes each [`RGB8`] pixel in GRB order using 4 SPI bits per WS2812 data bit,
/// producing 12 bytes per LED. The buffer must be at least [`spi_data_len`]`(colors.len())`
/// bytes long; excess bytes are left untouched.
///
/// Reset bytes are **not** appended — the caller can either send a separate
/// SPI transaction of zeros or pre-zero the tail of an oversized buffer.
/// For correct WS2812 reset at 2 MHz, transmit at least [`SPI_RESET_BYTES_2MHZ`]
/// trailing `0x00` bytes after pixel data.
///
/// # Errors
///
/// Returns [`SpiEncodeError::BufferTooSmall`] if `buf.len() < spi_data_len(colors.len())`.
///
/// # Example
///
/// ```
/// use bunting::prerender_spi;
/// use rgb::RGB8;
///
/// let colors = [RGB8::new(0, 0, 0)]; // black
/// let mut buf = [0u8; 12];
/// prerender_spi(&colors, &mut buf).unwrap();
/// assert!(buf.iter().all(|&b| b == 0x88));
/// ```
pub fn prerender_spi(colors: &[RGB8], buf: &mut [u8]) -> Result<(), SpiEncodeError> {
    let required = spi_data_len(colors.len());
    if buf.len() < required {
        return Err(SpiEncodeError::BufferTooSmall);
    }
    let mut pos = 0;
    for &rgb in colors {
        let grb = rgb_to_grb(rgb);
        for shift in (0..24).step_by(2).rev() {
            let pair = ((grb >> shift) & 0b11) as usize;
            buf[pos] = SPI_PATTERNS[pair];
            pos += 1;
        }
    }
    Ok(())
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

    // --- SPI prerender tests ---

    #[test]
    fn spi_data_len_zero() {
        assert_eq!(spi_data_len(0), 0);
    }

    #[test]
    fn spi_data_len_one() {
        assert_eq!(spi_data_len(1), 12);
    }

    #[test]
    fn spi_data_len_twelve() {
        assert_eq!(spi_data_len(12), 144);
    }

    #[test]
    fn prerender_spi_black() {
        let colors = [RGB8::new(0, 0, 0)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        assert!(buf.iter().all(|&b| b == 0x88), "all-zero bits → 0x88");
    }

    #[test]
    fn prerender_spi_white() {
        let colors = [RGB8::new(255, 255, 255)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        assert!(buf.iter().all(|&b| b == 0xEE), "all-one bits → 0xEE");
    }

    #[test]
    fn prerender_spi_red() {
        // RGB(255,0,0) → GRB = 0x00FF00
        // G=0x00: 00 00 00 00 → [0x88, 0x88, 0x88, 0x88]
        // R=0xFF: 11 11 11 11 → [0xEE, 0xEE, 0xEE, 0xEE]
        // B=0x00: 00 00 00 00 → [0x88, 0x88, 0x88, 0x88]
        let colors = [RGB8::new(255, 0, 0)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        let expected = [
            0x88, 0x88, 0x88, 0x88, // G=0x00
            0xEE, 0xEE, 0xEE, 0xEE, // R=0xFF
            0x88, 0x88, 0x88, 0x88, // B=0x00
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn prerender_spi_green() {
        // RGB(0,255,0) → GRB = 0xFF0000
        let colors = [RGB8::new(0, 255, 0)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        let expected = [
            0xEE, 0xEE, 0xEE, 0xEE, // G=0xFF
            0x88, 0x88, 0x88, 0x88, // R=0x00
            0x88, 0x88, 0x88, 0x88, // B=0x00
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn prerender_spi_blue() {
        // RGB(0,0,255) → GRB = 0x0000FF
        let colors = [RGB8::new(0, 0, 255)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        let expected = [
            0x88, 0x88, 0x88, 0x88, // G=0x00
            0x88, 0x88, 0x88, 0x88, // R=0x00
            0xEE, 0xEE, 0xEE, 0xEE, // B=0xFF
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn prerender_spi_mixed() {
        // RGB(0x12, 0x34, 0x56) → GRB = 0x341256
        // G=0x34 = 0011_0100 → pairs: 00 11 01 00 → [0x88, 0xEE, 0x8E, 0x88]
        // R=0x12 = 0001_0010 → pairs: 00 01 00 10 → [0x88, 0x8E, 0x88, 0xE8]
        // B=0x56 = 0101_0110 → pairs: 01 01 01 10 → [0x8E, 0x8E, 0x8E, 0xE8]
        let colors = [RGB8::new(0x12, 0x34, 0x56)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        let expected = [
            0x88, 0xEE, 0x8E, 0x88, // G=0x34
            0x88, 0x8E, 0x88, 0xE8, // R=0x12
            0x8E, 0x8E, 0x8E, 0xE8, // B=0x56
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn prerender_spi_buffer_too_small() {
        let colors = [RGB8::new(0, 0, 0)];
        let mut buf = [0u8; 11];
        assert_eq!(
            prerender_spi(&colors, &mut buf),
            Err(SpiEncodeError::BufferTooSmall)
        );
    }

    #[test]
    fn prerender_spi_exact_buffer() {
        let colors = [RGB8::new(0, 0, 0)];
        let mut buf = [0u8; 12];
        assert!(prerender_spi(&colors, &mut buf).is_ok());
    }

    #[test]
    fn prerender_spi_empty() {
        let colors: &[RGB8] = &[];
        let mut buf: [u8; 0] = [];
        assert!(prerender_spi(colors, &mut buf).is_ok());
    }

    #[test]
    fn prerender_spi_multiple_leds() {
        let colors = [
            RGB8::new(255, 0, 0),
            RGB8::new(0, 255, 0),
            RGB8::new(0, 0, 255),
        ];
        let mut buf = [0u8; 36];
        prerender_spi(&colors, &mut buf).unwrap();
        assert_eq!(buf.len(), 36);
        // First LED (red): G=0, R=FF, B=0
        assert_eq!(&buf[0..4], &[0x88, 0x88, 0x88, 0x88]);
        assert_eq!(&buf[4..8], &[0xEE, 0xEE, 0xEE, 0xEE]);
        assert_eq!(&buf[8..12], &[0x88, 0x88, 0x88, 0x88]);
        // Last LED (blue): G=0, R=0, B=FF
        assert_eq!(&buf[24..28], &[0x88, 0x88, 0x88, 0x88]);
        assert_eq!(&buf[28..32], &[0x88, 0x88, 0x88, 0x88]);
        assert_eq!(&buf[32..36], &[0xEE, 0xEE, 0xEE, 0xEE]);
    }

    #[test]
    fn prerender_spi_oversized_buffer() {
        let colors = [RGB8::new(0, 0, 0)];
        let mut buf = [0xFFu8; 20];
        prerender_spi(&colors, &mut buf).unwrap();
        // First 12 bytes are encoded
        assert!(buf[..12].iter().all(|&b| b == 0x88));
        // Remaining bytes untouched
        assert!(buf[12..].iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_max_leds_encoding() {
        // 256 LEDs is the practical maximum used throughout the workspace
        const MAX_LEDS: usize = 256;
        let pixels: Vec<RGB8> = (0..MAX_LEDS)
            .map(|i| {
                RGB8::new(
                    (i % 256) as u8,
                    ((i * 2) % 256) as u8,
                    ((i * 3) % 256) as u8,
                )
            })
            .collect();
        let mut buf = vec![0u8; spi_data_len(MAX_LEDS)];
        prerender_spi(&pixels, &mut buf).unwrap();
        // The output buffer must be fully populated (no zeros left from a partial encode)
        // Verify by checking the buffer length matches the required size exactly
        assert_eq!(buf.len(), spi_data_len(MAX_LEDS));
        // Every byte must be one of the four valid SPI patterns
        for &b in &buf {
            assert!(
                b == 0x88 || b == 0x8E || b == 0xE8 || b == 0xEE,
                "unexpected SPI byte: 0x{:02X}",
                b
            );
        }
    }

    #[test]
    fn test_rgb_to_grb_color_to_bits_composition() {
        // RGB(255, 0, 0) in GRB order: G=0, R=255, B=0 → 0x00FF00
        // Bits: 00000000 11111111 00000000
        // Bit layout (MSB first):
        //   bits[0..8]  = G byte = 0x00 → all false
        //   bits[8..16] = R byte = 0xFF → all true
        //   bits[16..24]= B byte = 0x00 → all false
        let red = RGB8::new(255, 0, 0);
        let grb = rgb_to_grb(red);
        assert_eq!(grb, 0x00FF00, "rgb_to_grb(red) must be 0x00FF00");

        let bits = color_to_bits(grb);

        // Green byte (bits 0..8): all false — because green channel of red is 0
        for i in 0..8 {
            assert!(
                !bits[i],
                "bit {} (green byte) should be false for pure red (G=0)",
                i
            );
        }
        // Red byte (bits 8..16): all true — because red channel is 255
        for i in 8..16 {
            assert!(
                bits[i],
                "bit {} (red byte) should be true for pure red (R=255)",
                i
            );
        }
        // Blue byte (bits 16..24): all false — because blue channel of red is 0
        for i in 16..24 {
            assert!(
                !bits[i],
                "bit {} (blue byte) should be false for pure red (B=0)",
                i
            );
        }
    }

    #[test]
    fn prerender_spi_conformance_ws2812_spi() {
        // Reference buffer produced by ws2812-spi v0.5.1's write_byte algorithm:
        //   for each color byte, extract 2-bit pairs MSB-first, index into
        //   [0x88, 0x8E, 0xE8, 0xEE].
        // Input: RGB(0xCA, 0xFE, 0x42) → GRB bytes: 0xFE, 0xCA, 0x42
        //   G=0xFE (11 11 11 10): [0xEE, 0xEE, 0xEE, 0xE8]
        //   R=0xCA (11 00 10 10): [0xEE, 0x88, 0xE8, 0xE8]
        //   B=0x42 (01 00 00 10): [0x8E, 0x88, 0x88, 0xE8]
        let colors = [RGB8::new(0xCA, 0xFE, 0x42)];
        let mut buf = [0u8; 12];
        prerender_spi(&colors, &mut buf).unwrap();
        #[rustfmt::skip]
        let expected: [u8; 12] = [
            0xEE, 0xEE, 0xEE, 0xE8, // G=0xFE
            0xEE, 0x88, 0xE8, 0xE8, // R=0xCA
            0x8E, 0x88, 0x88, 0xE8, // B=0x42
        ];
        assert_eq!(
            buf, expected,
            "must match ws2812-spi v0.5.1 write_byte output"
        );
    }
}

//! Grid coordinate mapping, gamma correction, and brightness scaling.
//!
//! Pure-logic primitives for addressing WS2812 LEDs wired as a rectangular
//! grid. The types in this module have no hardware dependencies and are
//! fully testable on any host.
//!
//! # Example
//!
//! ```
//! use rgb::RGB8;
//! use ws2812_pure::grid::{GridBuffer, GridLayout};
//!
//! // 8×8 grid, LEDs wired column-major bottom-to-top.
//! let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::ColumnMajorBottomUp);
//! grid.set_brightness(64);
//! grid.set_pixel(0, 0, RGB8::new(255, 0, 0));
//!
//! // Hand the raw buffer to any WS2812 driver.
//! let _slice: &[RGB8] = grid.as_slice();
//! ```
//!
//! # Layout coverage
//!
//! Only the two layouts actually exercised by hardware in this workspace are
//! provided today: [`GridLayout::RowMajor`] and
//! [`GridLayout::ColumnMajorBottomUp`]. Many physical WS2812 matrices use
//! serpentine (zig-zag) wiring; future variants such as
//! `RowMajorSerpentine` or `ColumnMajorSerpentine` may be added when a real
//! board needs them. The enum is non-trivial to extend without a breaking
//! change — callers that need an unsupported layout should compute indices
//! manually for now.

use rgb::RGB8;

// ─── Gamma correction ───────────────────────────────────────────────────────

/// Gamma correction LUT for γ ≈ 2.0.
///
/// Computed at compile time as `i * i / 255` for all 256 entries. This is a
/// cheap integer approximation that produces perceptually smoother brightness
/// ramps than a linear mapping, without requiring floating-point or a
/// precomputed high-precision table.
pub const GAMMA_2_0: [u8; 256] = {
    let mut table = [0u8; 256];
    let mut i: usize = 0;
    while i < 256 {
        table[i] = (i * i / 255) as u8;
        i += 1;
    }
    table
};

/// Apply brightness scaling and a gamma LUT to an RGB value.
///
/// `brightness` is 0–255. Each channel is scaled by `brightness / 255` (floor
/// division — no rounding), then looked up in `gamma`. The "brightness, then
/// gamma" ordering is intentional: it matches the typical embedded LED
/// pipeline where brightness is a linear-light dimming knob and gamma
/// converts the result to the LED driver's perceptual curve. Reversing the
/// order would gamma-correct the dimmed value a second time.
pub fn apply_brightness_gamma(rgb: RGB8, brightness: u8, gamma: &[u8; 256]) -> RGB8 {
    let scale = |v: u8| -> u8 {
        let scaled = (v as u16 * brightness as u16 / 255) as u8;
        gamma[scaled as usize]
    };
    RGB8 {
        r: scale(rgb.r),
        g: scale(rgb.g),
        b: scale(rgb.b),
    }
}

// ─── GridLayout ─────────────────────────────────────────────────────────────

/// Physical LED wiring layout for a rectangular grid.
///
/// Determines how `(x, y)` coordinates map to linear LED indices along the
/// WS2812 data chain. `(0, 0)` is always the top-left of the logical grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLayout {
    /// Row-major, left-to-right, top-to-bottom.
    ///
    /// `index = y * width + x`
    RowMajor,
    /// Column-major, bottom-to-top (no serpentine).
    ///
    /// The LED strip enters from the bottom-left and runs upward in columns.
    /// Every column runs bottom-to-top; columns are visited left-to-right.
    ///
    /// `index = x * height + (height - 1 - y)`
    ColumnMajorBottomUp,
}

impl GridLayout {
    /// Convert `(x, y)` to a linear LED index.
    ///
    /// `x` is the column (0 = left), `y` is the row (0 = top). `width` and
    /// `height` describe the grid dimensions. Callers are responsible for
    /// bounds-checking coordinates before calling.
    pub const fn to_index(self, x: usize, y: usize, width: usize, height: usize) -> usize {
        match self {
            GridLayout::RowMajor => y * width + x,
            GridLayout::ColumnMajorBottomUp => x * height + (height - 1 - y),
        }
    }
}

// ─── GridBuffer ─────────────────────────────────────────────────────────────

/// Fixed-size grid pixel buffer with brightness + gamma scaling.
///
/// Holds `W * H` [`RGB8`] pixels laid out according to the configured
/// [`GridLayout`]. `N` must equal `W * H` — this is checked with a const
/// assertion in [`new`](Self::new), so a mismatched instantiation fails to
/// compile when used in a `const` context (and panics on the first construction
/// at runtime otherwise).
///
/// The buffer applies [`GAMMA_2_0`] and the current brightness to every
/// pixel **as it is written** — values are baked, not stored separately.
/// This means [`set_brightness`](Self::set_brightness) only affects pixels
/// written after the call; previously written pixels keep their already-baked
/// values. Callers who want global dimming with retroactive effect should
/// re-render the whole buffer (e.g. [`fill`](Self::fill) followed by
/// per-pixel writes) after changing brightness, or apply brightness at
/// flush time using [`apply_brightness_gamma`] against an unscaled source
/// buffer.
///
/// The raw buffer returned by [`as_slice`](Self::as_slice) can be passed
/// directly to any WS2812 driver.
///
/// Callers who need a different gamma curve can use
/// [`apply_brightness_gamma`] with their own LUT and manage a
/// `[RGB8; N]` array themselves.
#[derive(Debug, Clone, Copy)]
pub struct GridBuffer<const W: usize, const H: usize, const N: usize> {
    pixels: [RGB8; N],
    layout: GridLayout,
    brightness: u8,
}

impl<const W: usize, const H: usize, const N: usize> GridBuffer<W, H, N> {
    /// Create a new empty grid buffer with the given layout.
    ///
    /// Initial brightness is 255 (full). All pixels start black.
    ///
    /// # Compile-time / runtime errors
    ///
    /// Asserts that `N == W * H`. In a `const` context this fails to compile
    /// with a "evaluation of constant value failed" error; in a non-`const`
    /// context the assertion panics on first call.
    pub const fn new(layout: GridLayout) -> Self {
        assert!(N == W * H, "N must equal W * H");
        Self {
            pixels: [RGB8 { r: 0, g: 0, b: 0 }; N],
            layout,
            brightness: 255,
        }
    }

    /// Set the global brightness level (0–255).
    pub fn set_brightness(&mut self, level: u8) {
        self.brightness = level;
    }

    /// Returns the current brightness level.
    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Fill every pixel with the given colour (brightness + gamma applied).
    pub fn fill(&mut self, color: RGB8) {
        let c = apply_brightness_gamma(color, self.brightness, &GAMMA_2_0);
        self.pixels.fill(c);
    }

    /// Set the pixel at `(x, y)` (brightness + gamma applied).
    ///
    /// Out-of-bounds coordinates are silently ignored.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: RGB8) {
        if x >= W || y >= H {
            return;
        }
        let idx = self.layout.to_index(x, y, W, H);
        self.pixels[idx] = apply_brightness_gamma(color, self.brightness, &GAMMA_2_0);
    }

    /// Returns the raw pixel buffer for passing to a WS2812 driver.
    pub fn as_slice(&self) -> &[RGB8] {
        &self.pixels
    }

    /// Grid width (compile-time constant).
    pub const fn width(&self) -> usize {
        W
    }

    /// Grid height (compile-time constant).
    pub const fn height(&self) -> usize {
        H
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Gamma LUT ───────────────────────────────────────────────────────

    #[test]
    fn gamma_lut_endpoints() {
        assert_eq!(GAMMA_2_0[0], 0);
        assert_eq!(GAMMA_2_0[255], 255);
    }

    #[test]
    fn gamma_lut_known_midpoints() {
        // 128 * 128 / 255 = 64
        assert_eq!(GAMMA_2_0[128], 64);
        // 64 * 64 / 255 = 16
        assert_eq!(GAMMA_2_0[64], 16);
    }

    #[test]
    fn gamma_lut_monotonic() {
        for i in 1..256 {
            assert!(
                GAMMA_2_0[i] >= GAMMA_2_0[i - 1],
                "gamma LUT must be non-decreasing at index {}",
                i
            );
        }
    }

    // ── apply_brightness_gamma ──────────────────────────────────────────

    #[test]
    fn full_brightness_applies_gamma_only() {
        let rgb = RGB8::new(255, 128, 64);
        let out = apply_brightness_gamma(rgb, 255, &GAMMA_2_0);
        assert_eq!(out.r, GAMMA_2_0[255]);
        assert_eq!(out.g, GAMMA_2_0[128]);
        assert_eq!(out.b, GAMMA_2_0[64]);
    }

    #[test]
    fn zero_brightness_returns_black() {
        let rgb = RGB8::new(255, 255, 255);
        let out = apply_brightness_gamma(rgb, 0, &GAMMA_2_0);
        assert_eq!(out, RGB8::new(0, 0, 0));
    }

    #[test]
    fn identity_lut_halves_at_half_brightness() {
        // Identity LUT: gamma[i] == i
        let identity: [u8; 256] = {
            let mut lut = [0u8; 256];
            let mut i = 0;
            while i < 256 {
                lut[i] = i as u8;
                i += 1;
            }
            lut
        };
        let rgb = RGB8::new(200, 100, 50);
        let out = apply_brightness_gamma(rgb, 128, &identity);
        // 200 * 128 / 255 = 100, 100 * 128 / 255 = 50, 50 * 128 / 255 = 25
        assert_eq!(out, RGB8::new(100, 50, 25));
    }

    // ── GridLayout ──────────────────────────────────────────────────────

    #[test]
    fn row_major_origin() {
        assert_eq!(GridLayout::RowMajor.to_index(0, 0, 8, 8), 0);
    }

    #[test]
    fn row_major_end() {
        assert_eq!(GridLayout::RowMajor.to_index(7, 7, 8, 8), 63);
    }

    #[test]
    fn row_major_sequential() {
        // (1, 0) should be index 1, (0, 1) should be index 8 in an 8-wide grid.
        assert_eq!(GridLayout::RowMajor.to_index(1, 0, 8, 8), 1);
        assert_eq!(GridLayout::RowMajor.to_index(0, 1, 8, 8), 8);
    }

    #[test]
    fn column_major_bottom_up_bottom_left_is_zero() {
        // (x=0, y=7) is the bottom-left corner → LED index 0.
        assert_eq!(GridLayout::ColumnMajorBottomUp.to_index(0, 7, 8, 8), 0);
    }

    #[test]
    fn column_major_bottom_up_top_left_is_seven() {
        // (x=0, y=0) is the top of the first column → index 7.
        assert_eq!(GridLayout::ColumnMajorBottomUp.to_index(0, 0, 8, 8), 7);
    }

    #[test]
    fn column_major_bottom_up_top_right_is_last() {
        // (x=7, y=0) is the top-right corner → last LED index 63.
        assert_eq!(GridLayout::ColumnMajorBottomUp.to_index(7, 0, 8, 8), 63);
    }

    #[test]
    fn column_major_bottom_up_matches_firmware_formula() {
        // The original formula was `x * 8 + (7 - y)` for an 8x8 grid.
        for x in 0..8 {
            for y in 0..8 {
                let expected = x * 8 + (7 - y);
                let actual = GridLayout::ColumnMajorBottomUp.to_index(x, y, 8, 8);
                assert_eq!(actual, expected, "mismatch at ({}, {})", x, y);
            }
        }
    }

    // ── GridBuffer ──────────────────────────────────────────────────────

    #[test]
    fn new_starts_black() {
        let grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        assert!(grid.as_slice().iter().all(|p| *p == RGB8::new(0, 0, 0)));
        assert_eq!(grid.brightness(), 255);
    }

    #[test]
    fn dimensions_are_compile_time_constants() {
        let grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        assert_eq!(grid.width(), 8);
        assert_eq!(grid.height(), 8);
        assert_eq!(grid.as_slice().len(), 64);
    }

    #[test]
    fn brightness_round_trip() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        grid.set_brightness(42);
        assert_eq!(grid.brightness(), 42);
    }

    #[test]
    fn fill_paints_every_pixel_with_gamma_scaled_colour() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        grid.set_brightness(255);
        grid.fill(RGB8::new(128, 128, 128));
        // At brightness 255, fill should produce gamma[128] = 64 on each channel.
        let expected = RGB8::new(64, 64, 64);
        assert!(grid.as_slice().iter().all(|p| *p == expected));
    }

    #[test]
    fn fill_at_zero_brightness_is_black() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        grid.set_brightness(0);
        grid.fill(RGB8::new(255, 255, 255));
        assert!(grid.as_slice().iter().all(|p| *p == RGB8::new(0, 0, 0)));
    }

    #[test]
    fn set_pixel_writes_at_layout_index() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::ColumnMajorBottomUp);
        grid.set_brightness(255);
        // (0, 7) → ColumnMajorBottomUp index 0
        grid.set_pixel(0, 7, RGB8::new(255, 0, 0));
        let expected = RGB8::new(GAMMA_2_0[255], 0, 0);
        assert_eq!(grid.as_slice()[0], expected);
        // Other pixels remain black
        assert_eq!(grid.as_slice()[1], RGB8::new(0, 0, 0));
    }

    #[test]
    fn set_pixel_row_major_layout() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        grid.set_brightness(255);
        // (3, 2) in row-major → index 2 * 8 + 3 = 19
        grid.set_pixel(3, 2, RGB8::new(0, 255, 0));
        let expected = RGB8::new(0, GAMMA_2_0[255], 0);
        assert_eq!(grid.as_slice()[19], expected);
    }

    #[test]
    fn set_pixel_out_of_bounds_is_noop() {
        let mut grid: GridBuffer<8, 8, 64> = GridBuffer::new(GridLayout::RowMajor);
        grid.set_pixel(8, 0, RGB8::new(255, 0, 0));
        grid.set_pixel(0, 8, RGB8::new(0, 255, 0));
        grid.set_pixel(100, 100, RGB8::new(0, 0, 255));
        assert!(grid.as_slice().iter().all(|p| *p == RGB8::new(0, 0, 0)));
    }

    #[test]
    fn non_square_grid() {
        // 16×4 grid, 64 pixels.
        let mut grid: GridBuffer<16, 4, 64> = GridBuffer::new(GridLayout::RowMajor);
        assert_eq!(grid.width(), 16);
        assert_eq!(grid.height(), 4);
        grid.set_pixel(15, 3, RGB8::new(255, 0, 0));
        // Last index: 3 * 16 + 15 = 63
        let expected = RGB8::new(GAMMA_2_0[255], 0, 0);
        assert_eq!(grid.as_slice()[63], expected);
    }
}

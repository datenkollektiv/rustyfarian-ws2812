//! A three-color palette for themed LED effects.

use rgb::RGB8;

/// A three-color theme palette.
///
/// Provides primary, secondary, and accent colors for effects that
/// need a coordinated color scheme (e.g., [`SectionEffect`](crate::SectionEffect)).
///
/// Currently [`SectionEffect`](crate::SectionEffect) uses only `primary` for rendering.
/// The `secondary` and `accent` fields are available for future rendering
/// modes such as gradients or alternating patterns within a section.
///
/// # Example
///
/// ```
/// use ferriswheel::ColorPalette;
/// use rgb::RGB8;
///
/// let palette = ColorPalette::new(
///     RGB8::new(255, 0, 0),
///     RGB8::new(0, 255, 0),
///     RGB8::new(0, 0, 255),
/// );
/// assert_eq!(palette.primary, RGB8::new(255, 0, 0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorPalette {
    /// The dominant color.
    pub primary: RGB8,
    /// The supporting color.
    pub secondary: RGB8,
    /// The highlight color.
    pub accent: RGB8,
}

impl ColorPalette {
    /// Creates a new palette with the given colors.
    pub fn new(primary: RGB8, secondary: RGB8, accent: RGB8) -> Self {
        Self {
            primary,
            secondary,
            accent,
        }
    }

    /// Creates a monochromatic palette where all three colors are the same.
    pub fn mono(color: RGB8) -> Self {
        Self {
            primary: color,
            secondary: color,
            accent: color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stores_fields() {
        let p = RGB8::new(255, 0, 0);
        let s = RGB8::new(0, 255, 0);
        let a = RGB8::new(0, 0, 255);
        let palette = ColorPalette::new(p, s, a);
        assert_eq!(palette.primary, p);
        assert_eq!(palette.secondary, s);
        assert_eq!(palette.accent, a);
    }

    #[test]
    fn test_mono_sets_all_equal() {
        let color = RGB8::new(42, 100, 200);
        let palette = ColorPalette::mono(color);
        assert_eq!(palette.primary, color);
        assert_eq!(palette.secondary, color);
        assert_eq!(palette.accent, color);
    }

    #[test]
    fn test_copy_clone() {
        let palette = ColorPalette::mono(RGB8::new(10, 20, 30));
        let copied = palette;
        let cloned = palette.clone();
        assert_eq!(palette, copied);
        assert_eq!(palette, cloned);
    }

    #[test]
    fn test_partial_eq() {
        let a = ColorPalette::new(RGB8::new(1, 2, 3), RGB8::new(4, 5, 6), RGB8::new(7, 8, 9));
        let b = ColorPalette::new(RGB8::new(1, 2, 3), RGB8::new(4, 5, 6), RGB8::new(7, 8, 9));
        let c = ColorPalette::new(RGB8::new(9, 8, 7), RGB8::new(4, 5, 6), RGB8::new(7, 8, 9));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}

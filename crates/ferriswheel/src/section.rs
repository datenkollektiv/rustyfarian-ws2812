//! Weighted color-section effect for LED rings.
//!
//! Splits an LED ring into colored sections, each sized proportionally
//! to its weight.
//! Sections are driven externally via [`set_sections`](SectionEffect::set_sections).

use crate::effect::{validate_buffer, validate_num_leds, Effect, EffectError};
use crate::palette::ColorPalette;
use crate::util::fill_solid;
use rgb::RGB8;

/// Maximum number of sections supported by [`SectionEffect`].
pub const MAX_SECTIONS: usize = 8;

/// An effect that divides the ring into weighted color sections.
///
/// Each section is a [`ColorPalette`] paired with a weight.
/// LEDs are distributed proportionally across sections based on their weights.
/// The primary color of each palette is used for rendering.
///
/// Like [`ProgressEffect`](crate::ProgressEffect), this effect is externally
/// driven — `update()` renders the current state without advancing animation.
///
/// # Example
///
/// ```
/// use ferriswheel::{SectionEffect, ColorPalette, Effect};
/// use rgb::RGB8;
///
/// let mut effect = SectionEffect::new(12).unwrap();
/// let red = ColorPalette::mono(RGB8::new(255, 0, 0));
/// let blue = ColorPalette::mono(RGB8::new(0, 0, 255));
/// effect.set_sections(&[(red, 1), (blue, 1)]).unwrap();
///
/// let mut buffer = [RGB8::default(); 12];
/// effect.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct SectionEffect {
    num_leds: usize,
    sections: [(ColorPalette, u8); MAX_SECTIONS],
    count: usize,
}

impl SectionEffect {
    /// Creates a new section effect for the specified number of LEDs.
    ///
    /// Starts with no sections (ring is dark).
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        let default_palette = ColorPalette::mono(RGB8::default());
        Ok(Self {
            num_leds,
            sections: [(default_palette, 0); MAX_SECTIONS],
            count: 0,
        })
    }

    /// Sets the active sections.
    ///
    /// Each entry is a `(ColorPalette, weight)` pair.
    /// Weights determine proportional LED distribution.
    /// If all weights are zero, sections are treated as equal weight.
    /// The last section absorbs any rounding remainder from integer division.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::TooManySections` if `sections.len()` exceeds `MAX_SECTIONS`.
    pub fn set_sections(&mut self, sections: &[(ColorPalette, u8)]) -> Result<(), EffectError> {
        if sections.len() > MAX_SECTIONS {
            return Err(EffectError::TooManySections {
                requested: sections.len(),
                max: MAX_SECTIONS,
            });
        }

        for (i, &entry) in sections.iter().enumerate() {
            self.sections[i] = entry;
        }
        self.count = sections.len();

        Ok(())
    }

    /// Removes all sections (ring goes dark on next render).
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Returns the number of active sections.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current section layout.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        if self.count == 0 {
            fill_solid(&mut buffer[..self.num_leds], RGB8::default());
            return Ok(());
        }

        let total_weight: u32 = self.sections[..self.count]
            .iter()
            .map(|&(_, w)| w as u32)
            .sum();

        // If all weights are zero, treat each section as weight 1
        let (effective_weights, effective_total) = if total_weight == 0 {
            ([1u32; MAX_SECTIONS], self.count as u32)
        } else {
            let mut weights = [0u32; MAX_SECTIONS];
            for (i, &(_, w)) in self.sections[..self.count].iter().enumerate() {
                weights[i] = w as u32;
            }
            (weights, total_weight)
        };

        let mut led_idx = 0;
        for (i, (&weight, &(palette, _))) in effective_weights[..self.count]
            .iter()
            .zip(self.sections[..self.count].iter())
            .enumerate()
        {
            let leds_for_section = if i == self.count - 1 {
                // Last section absorbs rounding remainder
                self.num_leds - led_idx
            } else {
                (weight * self.num_leds as u32 / effective_total) as usize
            };

            for led in buffer[led_idx..led_idx + leds_for_section].iter_mut() {
                *led = palette.primary;
            }
            led_idx += leds_for_section;
        }

        Ok(())
    }

    /// Renders the current sections (same as `current` — sections are externally driven).
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)
    }

    /// Resets the effect by clearing all sections.
    pub fn reset(&mut self) {
        self.clear();
    }
}

impl Effect for SectionEffect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.update(buffer)
    }

    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red_palette() -> ColorPalette {
        ColorPalette::mono(RGB8::new(255, 0, 0))
    }

    fn blue_palette() -> ColorPalette {
        ColorPalette::mono(RGB8::new(0, 0, 255))
    }

    fn green_palette() -> ColorPalette {
        ColorPalette::mono(RGB8::new(0, 255, 0))
    }

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(SectionEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = SectionEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
        assert_eq!(effect.count(), 0);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = SectionEffect::new(12).unwrap();
        let mut buffer = [RGB8::default(); 8];
        assert_eq!(
            effect.current(&mut buffer).unwrap_err(),
            EffectError::BufferTooSmall {
                required: 12,
                actual: 8
            }
        );
    }

    #[test]
    fn test_too_many_sections_returns_error() {
        let mut effect = SectionEffect::new(12).unwrap();
        let sections: Vec<(ColorPalette, u8)> =
            (0..MAX_SECTIONS + 1).map(|_| (red_palette(), 1)).collect();
        assert_eq!(
            effect.set_sections(&sections).unwrap_err(),
            EffectError::TooManySections {
                requested: MAX_SECTIONS + 1,
                max: MAX_SECTIONS
            }
        );
    }

    #[test]
    fn test_empty_sections_all_dark() {
        let effect = SectionEffect::new(8).unwrap();
        let mut buffer = [RGB8::new(99, 99, 99); 8];
        effect.current(&mut buffer).unwrap();
        for led in buffer.iter().take(8) {
            assert_eq!(*led, RGB8::default());
        }
    }

    #[test]
    fn test_single_section_fills_entire_ring() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect.set_sections(&[(red_palette(), 1)]).unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        for (i, led) in buffer.iter().enumerate().take(8) {
            assert_eq!(*led, RGB8::new(255, 0, 0), "LED {} should be red", i);
        }
    }

    #[test]
    fn test_two_equal_weight_sections_split_evenly() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect
            .set_sections(&[(red_palette(), 1), (blue_palette(), 1)])
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        for (i, led) in buffer.iter().enumerate().take(4) {
            assert_eq!(*led, RGB8::new(255, 0, 0), "LED {} should be red", i);
        }
        for i in 4..8 {
            assert_eq!(buffer[i], RGB8::new(0, 0, 255), "LED {} should be blue", i);
        }
    }

    #[test]
    fn test_weighted_sections_proportional() {
        let mut effect = SectionEffect::new(12).unwrap();
        // Weight 1:2:1 → should give 3:6:3
        effect
            .set_sections(&[
                (red_palette(), 1),
                (green_palette(), 2),
                (blue_palette(), 1),
            ])
            .unwrap();

        let mut buffer = [RGB8::default(); 12];
        effect.current(&mut buffer).unwrap();

        let red_count = buffer
            .iter()
            .filter(|led| led.r == 255 && led.g == 0)
            .count();
        let green_count = buffer
            .iter()
            .filter(|led| led.g == 255 && led.r == 0)
            .count();
        let blue_count = buffer
            .iter()
            .filter(|led| led.b == 255 && led.r == 0 && led.g == 0)
            .count();

        assert_eq!(red_count, 3, "red section should have 3 LEDs");
        assert_eq!(green_count, 6, "green section should have 6 LEDs");
        assert_eq!(blue_count, 3, "blue section should have 3 LEDs");
    }

    #[test]
    fn test_last_section_absorbs_rounding_remainder() {
        let mut effect = SectionEffect::new(10).unwrap();
        // Weight 1:1:1 → 10/3 = 3 per section, last gets 10 - 6 = 4
        effect
            .set_sections(&[
                (red_palette(), 1),
                (green_palette(), 1),
                (blue_palette(), 1),
            ])
            .unwrap();

        let mut buffer = [RGB8::default(); 10];
        effect.current(&mut buffer).unwrap();

        let red_count = buffer
            .iter()
            .filter(|led| led.r == 255 && led.g == 0)
            .count();
        let green_count = buffer
            .iter()
            .filter(|led| led.g == 255 && led.r == 0)
            .count();
        let blue_count = buffer
            .iter()
            .filter(|led| led.b == 255 && led.r == 0 && led.g == 0)
            .count();

        assert_eq!(red_count, 3);
        assert_eq!(green_count, 3);
        assert_eq!(blue_count, 4, "last section absorbs remainder");
    }

    #[test]
    fn test_zero_weights_treated_as_equal() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect
            .set_sections(&[(red_palette(), 0), (blue_palette(), 0)])
            .unwrap();

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        let red_count = buffer.iter().filter(|led| led.r == 255).count();
        let blue_count = buffer.iter().filter(|led| led.b == 255).count();
        assert_eq!(red_count, 4);
        assert_eq!(blue_count, 4);
    }

    #[test]
    fn test_clear_makes_ring_dark() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect.set_sections(&[(red_palette(), 1)]).unwrap();
        effect.clear();

        let mut buffer = [RGB8::new(99, 99, 99); 8];
        effect.current(&mut buffer).unwrap();
        for led in buffer.iter().take(8) {
            assert_eq!(*led, RGB8::default());
        }
    }

    #[test]
    fn test_reset_makes_ring_dark() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect.set_sections(&[(red_palette(), 1)]).unwrap();
        effect.reset();

        assert_eq!(effect.count(), 0);

        let mut buffer = [RGB8::new(99, 99, 99); 8];
        effect.current(&mut buffer).unwrap();
        for led in buffer.iter().take(8) {
            assert_eq!(*led, RGB8::default());
        }
    }

    #[test]
    fn test_count_getter() {
        let mut effect = SectionEffect::new(8).unwrap();
        assert_eq!(effect.count(), 0);
        effect
            .set_sections(&[(red_palette(), 1), (blue_palette(), 2)])
            .unwrap();
        assert_eq!(effect.count(), 2);
        effect.clear();
        assert_eq!(effect.count(), 0);
    }

    #[test]
    fn test_update_same_as_current() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect
            .set_sections(&[(red_palette(), 1), (blue_palette(), 1)])
            .unwrap();

        let mut buf_current = [RGB8::default(); 8];
        let mut buf_update = [RGB8::default(); 8];
        effect.current(&mut buf_current).unwrap();
        effect.update(&mut buf_update).unwrap();
        assert_eq!(buf_current, buf_update);
    }

    #[test]
    fn test_trait_object_usage() {
        let mut effect = SectionEffect::new(8).unwrap();
        effect.set_sections(&[(red_palette(), 1)]).unwrap();

        let effect_ref: &dyn Effect = &effect;
        let mut buffer = [RGB8::default(); 8];
        effect_ref.current(&mut buffer).unwrap();

        for led in buffer.iter().take(8) {
            assert_eq!(*led, RGB8::new(255, 0, 0));
        }
    }

    #[test]
    fn test_max_sections_allowed() {
        let mut effect = SectionEffect::new(16).unwrap();
        let sections: Vec<(ColorPalette, u8)> =
            (0..MAX_SECTIONS).map(|_| (red_palette(), 1)).collect();
        assert!(effect.set_sections(&sections).is_ok());
        assert_eq!(effect.count(), MAX_SECTIONS);
    }
}

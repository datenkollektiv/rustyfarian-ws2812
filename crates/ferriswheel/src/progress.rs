//! Proportional ring fill effect for LED rings.
//!
//! Fills the ring proportionally based on a progress value (0–255).
//! Supports partial LED blending for smooth transitions.

use crate::effect::{validate_buffer, validate_num_leds, Effect, EffectError};
use crate::util::lerp_color;
use rgb::RGB8;

/// A progress indicator effect that fills the ring proportionally.
///
/// Progress is set externally via [`set_progress`](ProgressEffect::set_progress).
/// Calling `update()` renders the current progress without advancing any animation.
///
/// # Example
///
/// ```
/// use ferriswheel::{ProgressEffect, Effect};
/// use ferriswheel::RGB8;
///
/// let mut progress = ProgressEffect::new(12).unwrap()
///     .with_fill_color(RGB8::new(0, 255, 0));
/// let mut buffer = [RGB8::default(); 12];
///
/// progress.set_progress(128);
/// progress.update(&mut buffer).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct ProgressEffect {
    num_leds: usize,
    fill_color: RGB8,
    empty_color: RGB8,
    progress: u8,
}

impl ProgressEffect {
    /// Creates a new progress effect for the specified number of LEDs.
    ///
    /// # Errors
    ///
    /// Returns `EffectError::ZeroLeds` if `num_leds` is 0.
    /// Returns `EffectError::TooManyLeds` if `num_leds` exceeds `MAX_LEDS`.
    ///
    /// # Default Configuration
    ///
    /// - Fill color: green (0, 255, 0)
    /// - Empty color: off (0, 0, 0)
    /// - Progress: 0
    pub fn new(num_leds: usize) -> Result<Self, EffectError> {
        validate_num_leds(num_leds)?;

        Ok(Self {
            num_leds,
            fill_color: RGB8::new(0, 255, 0),
            empty_color: RGB8::new(0, 0, 0),
            progress: 0,
        })
    }

    /// Sets the color of filled LEDs.
    pub fn with_fill_color(mut self, color: RGB8) -> Self {
        self.fill_color = color;
        self
    }

    /// Sets the color of empty (unfilled) LEDs.
    pub fn with_empty_color(mut self, color: RGB8) -> Self {
        self.empty_color = color;
        self
    }

    /// Sets the current progress (0–255, mapping to 0%–100%).
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress;
    }

    /// Returns the current progress value.
    pub fn progress(&self) -> u8 {
        self.progress
    }

    /// Returns the number of LEDs this effect is configured for.
    pub fn num_leds(&self) -> usize {
        self.num_leds
    }

    /// Fills the buffer with the current progress state without changing it.
    pub fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        validate_buffer(buffer, self.num_leds)?;

        let n = self.num_leds;

        // Scale progress (0–255) to LED-space (0–num_leds*256)
        // This gives sub-LED resolution for partial fill using 0–255 as the fraction.
        let fill_256 = self.progress as u32 * n as u32 * 256 / 255;
        let full_leds = (fill_256 / 256) as usize;
        let fractional = (fill_256 % 256) as u8;

        for (i, led) in buffer.iter_mut().take(n).enumerate() {
            if i < full_leds {
                *led = self.fill_color;
            } else if i == full_leds && full_leds < n {
                // Partial LED: blend between empty and fill based on fraction
                *led = lerp_color(self.empty_color, self.fill_color, fractional);
            } else {
                *led = self.empty_color;
            }
        }

        Ok(())
    }

    /// Renders the current progress (same as `current` — progress is externally driven).
    pub fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError> {
        self.current(buffer)
    }

    /// Resets progress to 0.
    pub fn reset(&mut self) {
        self.progress = 0;
    }
}

impl Effect for ProgressEffect {
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

    #[test]
    fn test_new_with_zero_leds_returns_error() {
        assert_eq!(ProgressEffect::new(0).unwrap_err(), EffectError::ZeroLeds);
    }

    #[test]
    fn test_new_with_valid_leds_succeeds() {
        let effect = ProgressEffect::new(12).unwrap();
        assert_eq!(effect.num_leds(), 12);
    }

    #[test]
    fn test_buffer_too_small_returns_error() {
        let effect = ProgressEffect::new(12).unwrap();
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
    fn test_zero_progress_all_empty() {
        let effect = ProgressEffect::new(8)
            .unwrap()
            .with_fill_color(RGB8::new(0, 255, 0))
            .with_empty_color(RGB8::new(0, 0, 0));

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        for (i, led) in buffer.iter().enumerate().take(8) {
            assert_eq!(*led, RGB8::new(0, 0, 0), "LED {} should be empty", i);
        }
    }

    #[test]
    fn test_full_progress_all_filled() {
        let mut effect = ProgressEffect::new(8)
            .unwrap()
            .with_fill_color(RGB8::new(0, 255, 0));

        effect.set_progress(255);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        for (i, led) in buffer.iter().enumerate().take(8) {
            assert_eq!(*led, RGB8::new(0, 255, 0), "LED {} should be filled", i);
        }
    }

    #[test]
    fn test_half_progress() {
        let mut effect = ProgressEffect::new(8)
            .unwrap()
            .with_fill_color(RGB8::new(255, 0, 0))
            .with_empty_color(RGB8::new(0, 0, 0));

        effect.set_progress(128);

        let mut buffer = [RGB8::default(); 8];
        effect.current(&mut buffer).unwrap();

        // With 128/255 progress on 8 LEDs, roughly 4 should be filled
        let filled_count = buffer.iter().take(8).filter(|led| led.r > 128).count();
        assert!(
            filled_count >= 3 && filled_count <= 5,
            "about half should be filled, got {}",
            filled_count
        );
    }

    #[test]
    fn test_partial_led_blending() {
        let mut effect = ProgressEffect::new(4)
            .unwrap()
            .with_fill_color(RGB8::new(255, 0, 0))
            .with_empty_color(RGB8::new(0, 0, 0));

        // Set progress so that roughly 1.5 LEDs should be filled
        // 1.5/4 * 255 ≈ 96
        effect.set_progress(96);

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        // First LED should be fully filled
        assert_eq!(buffer[0], RGB8::new(255, 0, 0));
        // The partial LED should have intermediate brightness
        // (not fully filled, not fully empty)
        let partial = buffer[1];
        assert!(
            partial.r > 0 && partial.r < 255,
            "partial LED should be blended, got r={}",
            partial.r
        );
        // Remaining LEDs should be empty
        assert_eq!(buffer[2], RGB8::new(0, 0, 0));
        assert_eq!(buffer[3], RGB8::new(0, 0, 0));
    }

    #[test]
    fn test_custom_empty_color() {
        let effect = ProgressEffect::new(4)
            .unwrap()
            .with_fill_color(RGB8::new(0, 255, 0))
            .with_empty_color(RGB8::new(10, 10, 10));

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        // At 0 progress, all LEDs should show the empty color
        for led in buffer.iter().take(4) {
            assert_eq!(*led, RGB8::new(10, 10, 10));
        }
    }

    #[test]
    fn test_reset_clears_progress() {
        let mut effect = ProgressEffect::new(4).unwrap();
        effect.set_progress(200);
        assert_eq!(effect.progress(), 200);

        effect.reset();
        assert_eq!(effect.progress(), 0);
    }

    #[test]
    fn test_progress_getter() {
        let mut effect = ProgressEffect::new(4).unwrap();
        assert_eq!(effect.progress(), 0);
        effect.set_progress(42);
        assert_eq!(effect.progress(), 42);
    }

    #[test]
    fn test_trait_object_usage() {
        let mut effect = ProgressEffect::new(4)
            .unwrap()
            .with_fill_color(RGB8::new(255, 0, 0));
        effect.set_progress(200);

        let effect_ref: &dyn Effect = &effect;
        let mut buffer = [RGB8::default(); 4];
        effect_ref.current(&mut buffer).unwrap();

        // Most LEDs should be filled
        let filled = buffer.iter().filter(|led| led.r > 128).count();
        assert!(filled >= 2, "most LEDs should be filled at progress 200");
    }

    // --- Tests for the fractional-byte fix (fill_256 formula) ---

    /// At progress=254 with n=1, the single LED is a partial fill: fractional=254,
    /// so lerp(empty, fill, 254) produces a near-full but distinct colour from fill_color.
    /// This exercises the partial-LED blending path at the top of the progress range.
    #[test]
    fn test_progress_254_single_led_is_nearly_full() {
        let fill = RGB8::new(255, 0, 0);
        let empty = RGB8::new(0, 0, 0);
        let mut effect = ProgressEffect::new(1)
            .unwrap()
            .with_fill_color(fill)
            .with_empty_color(empty);

        effect.set_progress(254);

        let mut buffer = [RGB8::default(); 1];
        effect.current(&mut buffer).unwrap();

        // fill_256 = 254*1*256/255 = 65024/255 = 254 (integer), full_leds=0, fractional=254
        // lerp(black, red, 254): r = (0*1 + 255*254)/255 = 64770/255 = 254
        // The LED is nearly full (254 out of 255) but not exactly fill_color
        assert!(
            buffer[0].r >= 250 && buffer[0].r < 255,
            "at progress=254 with 1 LED, red channel should be nearly full (250..254), got {}",
            buffer[0].r
        );
        assert_ne!(
            buffer[0], fill,
            "at progress=254 with 1 LED, should not yet be exactly fill_color"
        );
        assert_ne!(
            buffer[0], empty,
            "at progress=254 with 1 LED, should not be empty"
        );
    }

    /// At progress=255 with n=1, full_leds==1, so LED 0 is filled directly.
    /// Verifies no partial-blend LED appears at the maximum boundary for a 1-LED ring.
    /// The existing test_full_progress_all_filled only uses n=8; this covers n=1.
    #[test]
    fn test_full_progress_single_led_is_filled() {
        let fill = RGB8::new(0, 128, 255);
        let mut effect = ProgressEffect::new(1)
            .unwrap()
            .with_fill_color(fill)
            .with_empty_color(RGB8::new(0, 0, 0));

        effect.set_progress(255);

        let mut buffer = [RGB8::default(); 1];
        effect.current(&mut buffer).unwrap();

        // fill_256 = 255*1*256/255 = 256, full_leds = 1, fractional = 0
        // LED 0: i < full_leds (0 < 1) → fill_color, no partial-blend LED
        assert_eq!(
            buffer[0], fill,
            "single LED should be fully filled at progress=255"
        );
    }

    /// At progress=255, fill_256 = 255*n*256/255 = n*256, so full_leds=n and fractional=0.
    /// The partial-LED branch (i==full_leds && full_leds < n) is never entered,
    /// meaning no blended LED appears at the boundary. Tests n=4 which differs from the
    /// existing test_full_progress_all_filled (n=8).
    #[test]
    fn test_full_progress_no_partial_blend_led() {
        let fill = RGB8::new(255, 0, 0);
        let empty = RGB8::new(0, 0, 0);
        let mut effect = ProgressEffect::new(4)
            .unwrap()
            .with_fill_color(fill)
            .with_empty_color(empty);

        effect.set_progress(255);

        let mut buffer = [RGB8::default(); 4];
        effect.current(&mut buffer).unwrap();

        // fill_256 = 255*4*256/255 = 1024, full_leds=4, fractional=0
        // The partial-LED branch (i==full_leds && full_leds < n) is never entered
        // because full_leds==n. Every LED must be exactly fill_color.
        for (i, led) in buffer.iter().enumerate() {
            assert_eq!(
                *led, fill,
                "LED {} should be fill_color at progress=255, not blended",
                i
            );
        }
    }

    #[test]
    fn test_oversized_buffer_accepted() {
        let sentinel = RGB8::new(0xDE, 0xAD, 0xFF);
        let effect = ProgressEffect::new(4).unwrap();
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

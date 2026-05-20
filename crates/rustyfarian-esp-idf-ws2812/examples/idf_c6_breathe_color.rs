//! ESP32-C6 Breathe Color Example (ESP-IDF)
//!
//! Runs a [`BreatheEffect`] on a 12-LED WS2812B ring, cycling through the
//! full hue wheel each frame using [`BreatheEffect::set_color`].
//!
//! This demonstrates that [`set_color`] updates the hue *without resetting
//! the breathing phase* — the brightness envelope continues smoothly while
//! the color shifts gradually around the spectrum.
//!
//! ## Comparison with [`idf_c6_effects`](idf_c6_effects)
//!
//! [`PulseEffect`] uses a half-wave sine: brightness rises to peak, falls to
//! zero, and pauses at zero for roughly a quarter of the cycle.
//! [`BreatheEffect`] uses a full symmetric sine with no pause at the floor —
//! the brightness oscillates continuously in and out.
//!
//! ## Components
//!
//! - ESP32-C6 development board
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C6            WS2812B ring
//! ────────────        ────────────
//! GPIO18 ──[330 Ω]──► DIN
//! GND   ──────────► GND
//! 3V3   ──────────► VCC
//! ```
//!
//! **Power note:** 3.3 V logic is sufficient for a small ring at low brightness.
//! For full brightness or longer strips, use 5 V VCC and a 3.3 V→5 V level shifter
//! (e.g., 74AHCT125) on the data line.
//!
//! ## Build
//!
//! ```sh
//! just build-example idf-ws2812 idf_c6_breathe_color
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c6_breathe_color
//! ```

use ferriswheel::{hsv_to_rgb, BreatheEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = Ws2812Rmt::new(peripherals.pins.gpio18)?;
    let mut effect = BreatheEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(hsv_to_rgb(0, 255, 255))
        .with_max_brightness(180)
        .with_speed(2)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];
    // Tracks the current hue (0–255 maps the full color wheel).
    // Increments by 1 each frame (~50 ms), completing one rotation in ~12.8 s.
    let mut hue: u8 = 0;

    loop {
        // EffectError does not implement std::error::Error, so ? is unavailable;
        // unwrap() panics on failure, which ESP-IDF logs to the serial console.
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors)?;
        thread::sleep(Duration::from_millis(50));

        // Advance hue and update color without resetting the breathing phase.
        hue = hue.wrapping_add(1);
        effect.set_color(hsv_to_rgb(hue, 255, 255));
    }
}

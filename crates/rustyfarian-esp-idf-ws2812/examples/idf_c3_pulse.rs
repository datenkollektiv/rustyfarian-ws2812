//! ESP32-C3 Blue Pulse Example (ESP-IDF)
//!
//! Runs a [`PulseEffect`] (blue) from `ferriswheel` on a 12-LED WS2812B ring
//! using the ESP-IDF RMT peripheral on GPIO4.
//!
//! This is the ESP-IDF counterpart of `hal_c3_pulse.rs`.
//! Both drive the same ring with the same effect, frame period and pin, so they
//! differ only in the driver and its runtime.
//!
//! ## Components
//!
//! - ESP32-C3 development board
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C3            WS2812B ring
//! ────────────        ────────────
//! GPIO4 ──[330 Ω]──► DIN
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
//! just build-example idf-ws2812 idf_c3_pulse
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c3_pulse
//! ```

use ferriswheel::PulseEffect;
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use std::thread;
use std::time::Duration;

const NUM_LEDS: usize = 12;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    let mut ws = Ws2812Rmt::new(peripherals.pins.gpio4)?;
    let mut effect = PulseEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(0, 0, 255))
        .with_max_brightness(64)
        .with_speed(3)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        // The buffer is exactly NUM_LEDS long and the effect was built for the
        // same count, so update() cannot fail; fail loudly if that stops holding.
        // `?` is unavailable here because EffectError does not implement Error.
        effect
            .update(&mut colors)
            .expect("buffer length matches the effect's LED count");
        ws.set_pixels_slice(&colors)?;
        thread::sleep(Duration::from_millis(50));
    }
}

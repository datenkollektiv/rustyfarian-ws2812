//! ESP32-C6 Knight Rider Example (ESP-IDF)
//!
//! Runs a [`KnightRiderEffect`] on a 12-LED WS2812B ring — two light-blue
//! scanner heads sweep toward each other from opposite ends, cross in the
//! middle, and each reverses independently, using the ESP-IDF RMT peripheral
//! on GPIO18.
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
//! just build-example idf-ws2812 idf_c6_knight_rider
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c6_knight_rider
//! ```

use ferriswheel::KnightRiderEffect;
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = Ws2812Rmt::new(peripherals.pins.gpio18)?;
    let mut effect = KnightRiderEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(0, 80, 180));
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors)?;
        thread::sleep(Duration::from_millis(30));
    }
}

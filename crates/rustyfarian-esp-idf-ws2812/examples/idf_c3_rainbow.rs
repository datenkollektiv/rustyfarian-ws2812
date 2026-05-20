//! ESP32-C3 Rainbow Example (ESP-IDF)
//!
//! Runs a [`RainbowEffect`] from `ferriswheel` on a 12-LED WS2812B ring
//! using the ESP-IDF RMT peripheral on GPIO4.
//!
//! This uses the `rustyfarian-esp-idf-ws2812` driver (ESP-IDF, `std`) as a
//! known-good hardware baseline.
//! Use it to verify wiring and LED behaviour before testing the bare-metal driver.
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
//! just build-example idf-ws2812 idf_c3_rainbow
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c3_rainbow
//! ```

use ferriswheel::{Direction, RainbowEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = Ws2812Rmt::new(peripherals.pins.gpio4)?;
    let mut effect = RainbowEffect::new(NUM_LEDS)
        .unwrap()
        .with_brightness(32)
        .with_speed(2)
        .unwrap()
        .with_direction(Direction::Clockwise);
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut colors).ok();
        ws.set_pixels_slice(&colors)?;
        thread::sleep(Duration::from_millis(50));
    }
}

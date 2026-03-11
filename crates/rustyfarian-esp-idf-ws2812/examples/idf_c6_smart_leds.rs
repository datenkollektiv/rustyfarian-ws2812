//! ESP32-C6 Smart LEDs Pipeline Example (ESP-IDF)
//!
//! Demonstrates the full animation pipeline on a 12-LED WS2812B ring:
//!
//! ```text
//! ferriswheel (RainbowEffect)
//!     └─► [RGB8; 12]
//!         └─► SmartLedsWrite::write()
//!             └─► WS2812RMT (ESP-IDF RMT peripheral)
//!                 └─► LEDs
//! ```
//!
//! [`RainbowEffect`] from `ferriswheel` fills a color buffer each frame.
//! [`SmartLedsWrite::write`] accepts any iterator of colors, so you can
//! insert post-processing adapters from the `smart-leds` ecosystem between
//! the animation and the hardware — without touching either layer:
//!
//! ```text
//! ws.write(colors.iter().copied())                         // raw
//! ws.write(brightness(colors.iter().copied(), 64))         // 25% brightness
//! ws.write(gamma(brightness(colors.iter().copied(), 128))) // + gamma correction
//! ```
//!
//! All adapters are zero-allocation iterator wrappers from the `smart-leds` crate.
//!
//! See also [`idf_c6_rainbow`](idf_c6_rainbow) for the `set_pixels_slice` alternative
//! without the `SmartLedsWrite` pipeline.
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
//! just build-example idf-ws2812 idf_c6_smart_leds
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c6_smart_leds
//! ```

use ferriswheel::{Direction, RainbowEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use smart_leds_trait::SmartLedsWrite;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = WS2812RMT::new(peripherals.pins.gpio18)?;
    let mut effect = RainbowEffect::new(NUM_LEDS)
        .unwrap()
        .with_speed(2)
        .unwrap()
        .with_direction(Direction::Clockwise);
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        // Advance the animation and fill the color buffer.
        effect.update(&mut colors).ok();

        // Drive the ring via SmartLedsWrite::write().
        // The iterator interface lets you insert adapters here without
        // changing the effect or the driver — for example, to run at 25% brightness:
        ws.write(smart_leds::brightness(colors.iter().copied(), 64))?;

        thread::sleep(Duration::from_millis(50));
    }
}

//! ESP32-S3 Rainbow Example (ESP-IDF)
//!
//! Runs a [`RainbowEffect`] from `ferriswheel` on the 5-LED WS2812 ring of the
//! Elecrow CrowPanel 1.28" rotary knob using the ESP-IDF RMT peripheral on GPIO48.
//!
//! This uses the `rustyfarian-esp-idf-ws2812` driver (ESP-IDF, `std`) as a
//! known-good hardware baseline.
//! Use it to verify LED behaviour before testing the bare-metal driver.
//!
//! This example also exists so that `just run hal_s3_*` can obtain the ESP-IDF v5.3.3
//! bootloader for the ESP32-S3 (`scripts/ensure-bootloader.sh` builds it from here).
//!
//! ## Components
//!
//! - Elecrow CrowPanel 1.28" rotary knob (ESP32-S3)
//!
//! ## Wiring
//!
//! The 5-LED WS2812 ring is onboard, with its data line on GPIO48.
//! No external wiring or data-line resistor is needed.
//!
//! Some ESP32-S3 DevKit revisions carry a single onboard WS2812 on GPIO48; check your board's schematic.
//! A DevKit has only one LED, so adjust `NUM_LEDS` accordingly.
//!
//! ## Build
//!
//! ```sh
//! just build-example idf-ws2812 idf_s3_rainbow
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_s3_rainbow
//! ```

use ferriswheel::{Direction, RainbowEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 5;
    let mut ws = Ws2812Rmt::new(peripherals.pins.gpio48)?;
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

//! ESP32-C6 Twinkle / Sparkle Example (ESP-IDF)
//!
//! Runs a [`TwinkleEffect`] on a 12-LED WS2812B ring — a blue-white starfield
//! where random LEDs flash to peak brightness and fade independently, using the
//! ESP-IDF RMT peripheral on GPIO18.
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
//! just build-example idf-ws2812 idf_c6_twinkle
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c6_twinkle
//! ```

use ferriswheel::{Effect, TwinkleEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = WS2812RMT::new(peripherals.pins.gpio18, peripherals.rmt.channel0)?;
    let mut effect = TwinkleEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(150, 180, 255))
        .with_spawn_chance(40)
        .with_decay(15)
        .with_max_brightness(200);
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect
            .update(&mut colors)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        ws.set_pixels_slice(&colors)?;
        thread::sleep(Duration::from_millis(50));
    }
}

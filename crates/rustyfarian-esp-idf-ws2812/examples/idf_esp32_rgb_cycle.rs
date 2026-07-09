//! ESP32 (WROOM) Discrete RGB LED Example (ESP-IDF)
//!
//! Cycles a plain **common-anode RGB LED** through its eight on/off colours
//! using the [`RgbGpioLed`](pennant::RgbGpioLed) adapter from `pennant`, driving
//! three ordinary GPIOs — this is **not** a WS2812/addressable LED and does not
//! use the RMT peripheral.
//!
//! Use it to verify the `RgbGpioLed` per-channel and polarity logic on real
//! hardware: each named colour should light the matching channels. If red and
//! blue look swapped, or "off" is fully lit, your pin mapping or [`Polarity`]
//! is wrong.
//!
//! ## Board / wiring
//!
//! Defaults to the **Cheap Yellow Display** (ESP32-2432S028R) onboard RGB LED,
//! which needs no external parts:
//!
//! ```text
//! ESP32-2432S028R      onboard RGB LED (common-anode, active-low)
//! ───────────────      ────────────────────────────────────────
//! GPIO4  ───────────►  R   (LOW = lit)
//! GPIO16 ───────────►  G   (LOW = lit)
//! GPIO17 ───────────►  B   (LOW = lit)
//! ```
//!
//! For a different board, change the `gpioN` fields where the three
//! [`PinDriver`]s are created, and set `POLARITY` to
//! [`Polarity::ActiveHigh`] for a common-cathode LED.
//!
//! ## Build
//!
//! ```sh
//! just build-example idf-ws2812 idf_esp32_rgb_cycle
//! ```
//!
//! ## Run (flash + serial monitor)
//!
//! ```sh
//! just run idf_esp32_rgb_cycle
//! ```

use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use pennant::{Polarity, RgbGpioLed, StatusLed};
use rgb::RGB8;
use std::thread;
use std::time::Duration;

/// Wiring polarity. The Cheap Yellow Display's onboard RGB LED is common-anode
/// (active-low): a channel lights when its pin is driven LOW. Use
/// [`Polarity::ActiveHigh`] for a common-cathode LED.
const POLARITY: Polarity = Polarity::ActiveLow;

/// How long each colour is shown.
const STEP: Duration = Duration::from_millis(800);

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;

    // Pin selection — Cheap Yellow Display (ESP32-2432S028R) onboard RGB LED.
    // Swap the `gpioN` fields here to match a different board's wiring.
    let r = PinDriver::output(peripherals.pins.gpio4)?;
    let g = PinDriver::output(peripherals.pins.gpio16)?;
    let b = PinDriver::output(peripherals.pins.gpio17)?;

    let mut led = RgbGpioLed::new(r, g, b).with_polarity(POLARITY);

    // The eight colours a per-channel on/off RGB LED can show. Full-scale channel
    // values map cleanly to "on"; zero maps to "off".
    let colors: [(&str, RGB8); 8] = [
        ("off", RGB8::new(0, 0, 0)),
        ("red", RGB8::new(255, 0, 0)),
        ("green", RGB8::new(0, 255, 0)),
        ("blue", RGB8::new(0, 0, 255)),
        ("yellow", RGB8::new(255, 255, 0)),
        ("cyan", RGB8::new(0, 255, 255)),
        ("magenta", RGB8::new(255, 0, 255)),
        ("white", RGB8::new(255, 255, 255)),
    ];

    println!("RgbGpioLed colour cycle starting (polarity: {POLARITY:?})");

    loop {
        for (name, color) in colors {
            println!("-> {name}");
            led.set_color(color)?;
            thread::sleep(STEP);
        }
    }
}

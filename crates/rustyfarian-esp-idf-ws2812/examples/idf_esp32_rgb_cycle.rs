//! ESP32 (WROOM) Discrete RGB LED Example (ESP-IDF)
//!
//! Cycles a plain **discrete RGB LED** through its eight on/off colours using the
//! [`RgbGpioLed`](pennant::RgbGpioLed) adapter from `pennant`, driving three
//! ordinary GPIOs — this is **not** a WS2812/addressable LED and does not use the
//! RMT peripheral.
//!
//! Use it to verify the `RgbGpioLed` per-channel and polarity logic on real
//! hardware: each named colour should light the matching channels. If red and
//! blue look swapped, or "off" is fully lit, your pin mapping or [`Polarity`]
//! is wrong.
//!
//! ## Board / wiring
//!
//! Uses the **Cheap Yellow Display** (ESP32-2432S028R) RGB *GPIO numbers*, but the
//! default `POLARITY` is [`Polarity::ActiveHigh`] — it expects an **external
//! common-cathode** LED wired to those pins (common leg to GND), as verified on
//! hardware. It does **not** drive the CYD's common-anode onboard LED by default:
//!
//! ```text
//! ESP32                discrete RGB LED (common-cathode, active-high)
//! ─────                ────────────────────────────────────────────
//! GPIO4  ───────────►  R   (HIGH = lit)
//! GPIO16 ───────────►  G   (HIGH = lit)
//! GPIO17 ───────────►  B   (HIGH = lit)
//! ```
//!
//! For the CYD's *onboard* RGB LED (common-anode), set `POLARITY` to
//! [`Polarity::ActiveLow`]. For different pins, change the `gpioN` fields where
//! the three [`PinDriver`]s are created.
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

/// Wiring polarity. The default [`Polarity::ActiveHigh`] matches a common-cathode
/// LED (common leg to GND), lit when its pin is driven HIGH — verified on hardware.
/// Use [`Polarity::ActiveLow`] for a common-anode LED such as the Cheap Yellow
/// Display's onboard LED.
const POLARITY: Polarity = Polarity::ActiveHigh;

/// How long each colour is shown.
const STEP: Duration = Duration::from_millis(800);

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;

    // Cheap Yellow Display (ESP32-2432S028R) RGB GPIO numbers. With the default
    // ActiveHigh polarity these drive an external common-cathode LED; for the CYD's
    // common-anode onboard LED, set POLARITY = ActiveLow above.
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

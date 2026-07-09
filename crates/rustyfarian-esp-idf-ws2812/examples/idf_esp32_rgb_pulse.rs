//! ESP32 (WROOM) Discrete RGB LED — smooth PWM Pulse (ESP-IDF)
//!
//! Fades a plain **discrete RGB LED** in and out with [`PulseEffect`] driven
//! through the [`RgbPwmLed`](pennant::RgbPwmLed) adapter from `pennant`, using
//! three LEDC PWM channels — this is **not** a WS2812/addressable LED and does
//! not use the RMT peripheral.
//!
//! Where [`idf_esp32_rgb_cycle`](../idf_esp32_rgb_cycle) shows the on/off
//! [`RgbGpioLed`](pennant::RgbGpioLed) stepping through eight fixed colours, this
//! example uses PWM so the same effect renders as a **smooth brightness fade**
//! rather than a blink — the payoff of driving the channels with a duty cycle.
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
//! GPIO4  ──[LEDC ch0]►  R   (higher duty = brighter)
//! GPIO16 ──[LEDC ch1]►  G
//! GPIO17 ──[LEDC ch2]►  B
//! ```
//!
//! For the CYD's *onboard* RGB LED (common-anode), set `POLARITY` to
//! [`Polarity::ActiveLow`] (which inverts the duty). For different pins, change
//! the `gpioN` pins where the three [`LedcDriver`]s are created.
//!
//! ## Build
//!
//! ```sh
//! just build-example idf-ws2812 idf_esp32_rgb_pulse
//! ```
//!
//! ## Run (flash + serial monitor)
//!
//! ```sh
//! just run idf_esp32_rgb_pulse
//! ```

use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::*; // FromValueType, for `5.kHz()`
use pennant::{Polarity, PulseEffect, RgbPwmLed, StatusLed};
use std::thread;
use std::time::Duration;

/// Wiring polarity. The default [`Polarity::ActiveHigh`] matches a common-cathode
/// LED (common leg to GND), brightest when its pin is driven HIGH — verified on
/// hardware. Use [`Polarity::ActiveLow`] for a common-anode LED such as the Cheap
/// Yellow Display's onboard LED.
const POLARITY: Polarity = Polarity::ActiveHigh;

/// Base colour the pulse fades in and out. Change the channels to taste; any
/// non-zero channel participates in the fade.
const BASE_COLOR: (u8, u8, u8) = (0, 0, 255); // blue

/// Delay between animation frames.
const FRAME: Duration = Duration::from_millis(20);

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;

    // One 5 kHz timer shared by all three channels. LEDC selects a PWM resolution
    // for this frequency; RgbPwmLed scales each 8-bit colour component to whatever
    // duty range the channel reports (see `SetDutyCycle`), so the exact resolution
    // is not relied upon for correctness.
    let timer = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::new().frequency(5.kHz().into()),
    )?;

    // Cheap Yellow Display (ESP32-2432S028R) RGB GPIO numbers. With the default
    // ActiveHigh polarity these drive an external common-cathode LED; for the CYD's
    // common-anode onboard LED, set POLARITY = ActiveLow above.
    // Swap the `gpioN` pins here to match a different board's wiring.
    let r = LedcDriver::new(peripherals.ledc.channel0, &timer, peripherals.pins.gpio4)?;
    let g = LedcDriver::new(peripherals.ledc.channel1, &timer, peripherals.pins.gpio16)?;
    let b = LedcDriver::new(peripherals.ledc.channel2, &timer, peripherals.pins.gpio17)?;

    let mut led = RgbPwmLed::new(r, g, b).with_polarity(POLARITY);

    // Full-range fade so the PWM brightness is obvious. PulseEffect::new() uses a
    // dim 2..30 status range; here we sweep the whole 0..255 span.
    // PulseEffectError is no_std and does not implement std::error::Error, so it
    // cannot flow through `?` into anyhow — unwrap the compile-time-valid range.
    let mut pulse = PulseEffect::with_range(0, 255, 3).expect("0 < 255 and step 3 > 0");

    println!("RgbPwmLed pulse starting (base {BASE_COLOR:?}, polarity: {POLARITY:?})");

    loop {
        let color = pulse.update(BASE_COLOR);
        led.set_color(color)?;
        thread::sleep(FRAME);
    }
}

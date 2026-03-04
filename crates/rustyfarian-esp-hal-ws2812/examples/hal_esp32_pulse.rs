//! ESP32 (WROOM) Blue Pulse Example
//!
//! Runs a [`PulseEffect`] (blue) from `ferriswheel` on a 12-LED WS2812B ring
//! using the ESP32 RMT peripheral on GPIO4.
//!
//! See also `hal_c3_pulse.rs` and `hal_c6_pulse.rs`.
//!
//! ## Components
//!
//! - ESP32-WROOM-32 (or any ESP32 module)
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-WROOM-32       WS2812B ring
//! ─────────────        ────────────
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
//! just build-example hal-ws2812 hal_esp32_pulse
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_esp32_pulse
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    gpio::Level,
    main,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
};
use ferriswheel::PulseEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

// Minimal panic handler — replace with `panic-halt` or `panic-probe` for your application.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let config = TxChannelConfig::default()
        .with_clk_divider(RMT_CLK_DIV)
        .with_idle_output_level(Level::Low)
        .with_idle_output(true)
        .with_carrier_modulation(false);
    let channel = rmt
        .channel0
        .configure_tx(peripherals.GPIO4, config)
        .unwrap();

    let mut ws = Ws2812Rmt::<N>::new(channel);
    let mut effect = PulseEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(0, 0, 255))
        .with_max_brightness(64)
        .with_speed(3)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();

    loop {
        effect.update(&mut colors).ok();
        ws.set_pixels_slice(&colors).ok();
        delay.delay_millis(50u32);
    }
}

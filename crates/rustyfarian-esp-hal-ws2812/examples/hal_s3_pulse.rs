//! ESP32-S3 Blue Pulse Example
//!
//! Runs a [`PulseEffect`] (blue) from `ferriswheel` on the 5-LED WS2812 ring
//! of the Elecrow CrowPanel 1.28" rotary knob using the ESP32-S3 RMT peripheral on GPIO48.
//!
//! See also `hal_esp32_pulse.rs`, `hal_c3_pulse.rs`, and `hal_c6_pulse.rs`.
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
//! just build-example hal-ws2812 hal_s3_pulse
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_s3_pulse
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

const NUM_LEDS: usize = 5;
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
        .configure_tx(&config)
        .unwrap()
        .with_pin(peripherals.GPIO48);

    let mut ws = Ws2812Rmt::<_, N>::new(channel);
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

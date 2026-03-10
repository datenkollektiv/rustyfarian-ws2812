//! ESP32-C6 Rainbow Comet Example (esp-hal, bare-metal)
//!
//! Runs a [`RainbowCometEffect`] on a 12-LED WS2812B ring — a comet whose
//! tail cycles through the color wheel, using the ESP32-C6 RMT peripheral
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
//! ESP32-C6             WS2812B ring
//! ─────────────        ────────────
//! GPIO18 ──[330 Ω]──► DIN
//! GND    ──────────► GND
//! 3V3    ──────────► VCC
//! ```
//!
//! **Power note:** 3.3 V logic is sufficient for a small ring at low brightness.
//! For full brightness or longer strips, use 5 V VCC and a 3.3 V→5 V level shifter
//! (e.g., 74AHCT125) on the data line.
//!
//! ## Build
//!
//! ```sh
//! just build-example hal-ws2812 hal_c6_rainbow_comet
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_rainbow_comet
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
use esp_println::println;
use ferriswheel::RainbowCometEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
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
        .configure_tx(peripherals.GPIO18, config)
        .unwrap();

    let mut ws = Ws2812Rmt::<N>::new(channel);
    let mut effect = RainbowCometEffect::new(NUM_LEDS)
        .unwrap()
        .with_hue(0)
        .with_hue_step(16);
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();

    loop {
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors).unwrap();
        delay.delay_millis(30u32);
    }
}

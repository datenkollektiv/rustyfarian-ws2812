//! ESP32-C6 Fire Example (esp-hal, bare-metal)
//!
//! Runs a [`FireEffect`] on a 12-LED WS2812B ring — a flickering flame that
//! sparks at the base (index 0), diffuses heat upward toward the tip, and
//! maps each LED's temperature through a black → dark red → orange → white
//! gradient, using the ESP32-C6 RMT peripheral on GPIO18.
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
//! just build-example hal-ws2812 hal_c6_fire
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_fire
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
use ferriswheel::FireEffect;
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
    let mut effect = FireEffect::new(NUM_LEDS)
        .unwrap()
        .with_cooling(55)
        .with_sparking(120);
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();

    loop {
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors).unwrap();
        delay.delay_millis(50u32);
    }
}

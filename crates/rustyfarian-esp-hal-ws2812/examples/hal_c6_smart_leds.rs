//! ESP32-C6 Smart LEDs Pipeline Example
//!
//! Demonstrates the full animation pipeline on a 12-LED WS2812B ring:
//!
//! ```text
//! ferriswheel (RainbowEffect)
//!     └─► [RGB8; 12]
//!         └─► SmartLedsWrite::write()
//!             └─► Ws2812Rmt (RMT peripheral)
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
//! See also [`hal_c6_pulse`](hal_c6_pulse) for the `set_pixels_slice` alternative
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
//! just build-example hal-ws2812 hal_c6_smart_leds
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_smart_leds
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
use ferriswheel::{Direction, RainbowEffect};
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};
use smart_leds_trait::SmartLedsWrite;

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
    let mut effect = RainbowEffect::new(NUM_LEDS)
        .unwrap()
        .with_speed(2)
        .unwrap()
        .with_direction(Direction::Clockwise);
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();

    loop {
        // Advance the animation and fill the color buffer.
        effect.update(&mut colors).ok();

        // Drive the ring via SmartLedsWrite::write().
        // The iterator interface lets you insert adapters here without
        // changing the effect or the driver — for example, to run at 25% brightness:
        ws.write(smart_leds::brightness(colors.iter().copied(), 64))
            .ok();

        delay.delay_millis(50u32);
    }
}

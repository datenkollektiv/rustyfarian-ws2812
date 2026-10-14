//! ESP32-S3 Async Pulse Example (esp-hal + Embassy, bare-metal)
//!
//! Pulses the first LED of the Elecrow CrowPanel 1.28" knob's WS2812 ring in blue via the
//! async RMT driver; [`AsyncStatusLed`] drives one status LED (`hal_s3_pulse` pulses all five).
//! The animation loop uses [`embassy_time::Timer`] to yield between frames.
//!
//! See also `hal_esp32_pulse_async.rs`, `hal_c3_pulse_async.rs`, and `hal_c6_pulse_async.rs`.
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
//! just build-example hal-ws2812 hal_s3_pulse_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_s3_pulse_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    gpio::Level,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
    timer::timg::TimerGroup,
};
use pennant::AsyncStatusLed;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 5;
const N: usize = buffer_size(NUM_LEDS);

fn scale(color: RGB8, brightness: u8) -> RGB8 {
    RGB8::new(
        ((color.r as u16 * brightness as u16) / 255) as u8,
        ((color.g as u16 * brightness as u16) / 255) as u8,
        ((color.b as u16 * brightness as u16) / 255) as u8,
    )
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();
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

    let base_color = RGB8::new(0, 0, 255);
    let mut brightness: u8 = 0;
    let mut increasing = true;

    loop {
        AsyncStatusLed::set_color(&mut ws, scale(base_color, brightness))
            .await
            .unwrap();

        if increasing {
            if brightness >= 64 {
                increasing = false;
            } else {
                brightness = brightness.saturating_add(2);
            }
        } else if brightness == 0 {
            increasing = true;
        } else {
            brightness = brightness.saturating_sub(2);
        }

        Timer::after_millis(30).await;
    }
}

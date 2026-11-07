//! ESP32-S3 Async Pulse Example (esp-hal + Embassy, bare-metal)
//!
//! Runs a [`PulseEffect`] (blue) from `ferriswheel` on all five LEDs of the Elecrow CrowPanel
//! 1.28" knob's WS2812 ring via the async RMT driver; the async twin of `hal_s3_pulse`.
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
//! just build-example hal_s3_pulse_async
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
use esp_hal::{rmt::Rmt, time::Rate, timer::timg::TimerGroup};
use ferriswheel::PulseEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt};

const NUM_LEDS: usize = 5;
const N: usize = buffer_size(NUM_LEDS);

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
    let mut ws = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO48).unwrap();

    let mut effect = PulseEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(0, 0, 255))
        .with_max_brightness(64)
        .with_speed(3)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut colors).ok();
        ws.set_pixels_slice(&colors).await.ok();
        Timer::after_millis(50).await;
    }
}

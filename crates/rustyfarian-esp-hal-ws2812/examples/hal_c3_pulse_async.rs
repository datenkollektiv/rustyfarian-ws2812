//! ESP32-C3 Async Pulse Example (esp-hal + Embassy, bare-metal)
//!
//! Runs a [`PulseEffect`] (blue) from `ferriswheel` on all LEDs of a 12-LED WS2812B ring with the
//! async RMT driver; the async twin of `hal_c3_pulse`.
//! The animation loop uses [`embassy_time::Timer`] to yield between frames.
//!
//! See also `hal_c6_pulse_async.rs` and `hal_esp32_pulse_async.rs`.
//!
//! ## Components
//!
//! - ESP32-C3 development board
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C3            WS2812B ring
//! ────────────        ────────────
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
//! just build-example hal_c3_pulse_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c3_pulse_async
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

const NUM_LEDS: usize = 12;
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
    let mut ws = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO4).unwrap();

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

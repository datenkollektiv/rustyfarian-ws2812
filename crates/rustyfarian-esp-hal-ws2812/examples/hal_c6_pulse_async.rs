//! ESP32-C6 Async Pulse Example (esp-hal + Embassy, bare-metal)
//!
//! Pulses the first LED of a 12-LED WS2812B ring in blue with the async RMT driver;
//! [`AsyncStatusLed`] drives a single status LED; the ring is cleared once at start, so the other LEDs stay dark.
//! The animation loop uses [`embassy_time::Timer`] to yield between frames.
//!
//! This example demonstrates using `AsyncStatusLed` from `pennant` with the
//! async HAL driver — the same pattern as blocking `StatusLed`, but with `.await`.
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
//! just build-example hal_c6_pulse_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_pulse_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{rmt::Rmt, time::Rate, timer::timg::TimerGroup};
use esp_println::println;
use pennant::AsyncStatusLed;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

fn scale(color: RGB8, brightness: u8) -> RGB8 {
    RGB8::new(
        ((color.r as u16 * brightness as u16) / 255) as u8,
        ((color.g as u16 * brightness as u16) / 255) as u8,
        ((color.b as u16 * brightness as u16) / 255) as u8,
    )
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
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
    println!("RTOS started, RMT configured");

    let mut ws = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO18).unwrap();

    // Clear the whole ring once. `AsyncStatusLed` below drives only the first LED, and the
    // others would keep whatever the previous firmware latched.
    ws.set_pixels_slice(&[RGB8::default(); NUM_LEDS])
        .await
        .unwrap();

    let base_color = RGB8::new(0, 0, 255);
    let mut brightness: u8 = 0;
    let mut increasing = true;

    println!("Entering async pulse loop (AsyncStatusLed)");
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

//! ESP32 (WROOM) Async Pulse Example (esp-hal + Embassy, bare-metal)
//!
//! Runs a simple blue pulse on a 12-LED WS2812B ring using the async RMT driver
//! and the [`AsyncStatusLed`] trait.
//! The animation loop uses [`embassy_time::Timer`] to yield between frames.
//!
//! See also `hal_c3_pulse_async.rs` and `hal_c6_pulse_async.rs`.
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
//! just build-example hal-ws2812 hal_esp32_pulse_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_esp32_pulse_async
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
use led_effects::AsyncStatusLed;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

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
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    // `esp_rtos::start` takes a second `SoftwareInterrupt<'static, 0>` argument only on
    // RISC-V targets (it is gated by `#[cfg(riscv)]` in esp-rtos 0.2). On Xtensa LX6
    // (the original ESP32) the parameter does not exist, so this single-arg call is
    // correct here and the C3/C6 examples deliberately differ.
    esp_rtos::start(timg0.timer0);

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
        .with_pin(peripherals.GPIO4);

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

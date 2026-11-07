//! ESP32-C3 Colours Example — the boring bench check
//!
//! Lights the whole 12-LED WS2812B ring in one solid colour at a time, slowly:
//! red, green, blue, white, off, 3 s each, forever, and prints every change over serial.
//! It exists for the hardware bench: one colour, one question, at human speed.
//! A red/green swap, a dead pixel or a flaky data line shows up here before any effect runs.
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
//! The colours are deliberately dim (32/255 per channel, white 16/16/16), so a ring
//! on 3.3 V shows them cleanly and the eye can tell them apart.
//!
//! ## Serial output
//!
//! ```text
//! hal_c3_colors: 12 LEDs on GPIO4, 3000 ms per colour
//! cycle 1: showing RED on 12 LEDs
//! cycle 1: showing GREEN on 12 LEDs
//! ```
//!
//! ## Build
//!
//! ```sh
//! just build-example hal_c3_colors
//! ```
//!
//! ## Run (flash and monitor)
//!
//! ```sh
//! just run hal_c3_colors
//! ```
//!
//! Or let the bench drive it: `just bench rustyfarian-ws2812 c3 colors` from the `rustyfarian-bench` checkout.

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{delay::Delay, main, rmt::Rmt, time::Rate};
use esp_println::println;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);
const HOLD_MS: u32 = 3000;
const STEPS: [(&str, RGB8); 5] = [
    ("RED", RGB8::new(32, 0, 0)),
    ("GREEN", RGB8::new(0, 32, 0)),
    ("BLUE", RGB8::new(0, 0, 32)),
    ("WHITE", RGB8::new(16, 16, 16)),
    ("OFF", RGB8::new(0, 0, 0)),
];

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut ws = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO4).unwrap();
    let delay = Delay::new();

    println!("hal_c3_colors: {NUM_LEDS} LEDs on GPIO4, {HOLD_MS} ms per colour");

    let mut cycle: u32 = 0;
    loop {
        cycle += 1;
        for (name, color) in STEPS {
            match ws.set_pixels_slice(&[color; NUM_LEDS]) {
                Ok(()) => println!("cycle {cycle}: showing {name} on {NUM_LEDS} LEDs"),
                Err(e) => println!("cycle {cycle}: {name} transmit error: {e} FAIL"),
            }
            delay.delay_millis(HOLD_MS);
        }
    }
}

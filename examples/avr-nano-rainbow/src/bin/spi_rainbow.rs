//! Arduino Nano Rainbow — SPI prerendered backend (DIAGNOSTIC / COMPARISON ONLY).
//!
//! ⚠️ **This binary is not the recommended demo.** It exists for comparison and
//! diagnosis. Many WS2812 strips render this path as stable white-ish output
//! (chip's "0/1" decision threshold lands near the encoding's `T0H = 500 ns`)
//! even though the bit-bang backend renders the *same strip* correctly.
//! See [ADR 007](../../../../docs/adr/007-avr-ws2812-driver-strategy.md) for the full story.
//!
//! For a known-good hardware demo, flash `--bin avr-nano-rainbow` (the default
//! binary, which uses [`Ws2812BitBang`]) or `--bin bitbang_demo`. From the
//! workspace root: `just flash-avr-example` or `just flash-avr-bitbang-demo`.
//!
//! [`Ws2812BitBang`]: rustyfarian_avr_ws2812::Ws2812BitBang
//!
//! ## Wiring
//!
//! ```text
//! Arduino Nano             WS2812 Ring
//! ────────────             ───────────
//! 5V  ─────────────────── VDD
//! GND ─────────────────── GND
//! D11 ── [330 ohm] ────── DIN
//! ```
//!
//! ## Build and flash from the example directory
//!
//! ```sh
//! cargo +nightly-2025-04-27 run --release -Z build-std=core --bin spi_rainbow
//! ```
//!
//! Or from the workspace root: `just flash-avr-spi-rainbow`.

#![no_std]
#![no_main]

use arduino_hal::spi;
use ferriswheel::{Direction, RainbowEffect};
use panic_halt as _;
use rgb::RGB8;
use rustyfarian_avr_ws2812::{spi_buffer_size, Ws2812Spi};

const NUM_LEDS: usize = 12;
const BUF_SIZE: usize = spi_buffer_size(NUM_LEDS);

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let (spi, _cs) = arduino_hal::Spi::new(
        dp.SPI,
        pins.d13.into_output(),        // SCK
        pins.d11.into_output(),        // MOSI — data to WS2812
        pins.d12.into_pull_up_input(), // MISO — unused
        pins.d10.into_output(),        // SS — must be output for master mode
        spi::Settings {
            data_order: spi::DataOrder::MostSignificantFirst,
            clock: spi::SerialClockRate::OscfOver8, // 16 MHz / 8 = 2 MHz
            mode: embedded_hal::spi::MODE_0,
        },
    );

    let mut ws: Ws2812Spi<_, BUF_SIZE> = Ws2812Spi::new(spi);

    let mut effect = RainbowEffect::new(NUM_LEDS)
        .unwrap()
        .with_brightness(32) // ~12% — safe for USB power
        .with_speed(2)
        .unwrap()
        .with_direction(Direction::Clockwise);

    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut colors).ok();

        // SPI backend requires the caller to wrap `write` in a critical section.
        avr_device::interrupt::free(|_| {
            ws.write(&colors).unwrap();
        });

        arduino_hal::delay_ms(50);
    }
}

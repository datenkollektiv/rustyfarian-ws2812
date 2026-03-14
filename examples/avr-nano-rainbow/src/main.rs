//! Arduino Nano Rainbow — Bit-Bang Backend (default demo).
//!
//! Runs a [`RainbowEffect`] from `ferriswheel` on a 12-LED WS2812 ring using the
//! cycle-counted inline-`asm!` bit-bang driver from `rustyfarian-avr-ws2812`
//! (the recommended AVR backend per [ADR 007]). Drives PB3 / Arduino D11.
//!
//! [ADR 007]: ../../../docs/adr/007-avr-ws2812-driver-strategy.md
//! [`RainbowEffect`]: ferriswheel::RainbowEffect
//!
//! See `bin/spi_rainbow.rs` for the SPI-prerendered comparison (diagnostic only —
//! often shows white-ish output on real hardware) and `bin/bitbang_spike.rs` for the
//! frozen low-level reference.
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
//! ## Build and flash
//!
//! From this directory:
//!
//! ```sh
//! cargo +nightly-2025-04-27 run --release -Z build-std=core
//! ```
//!
//! Or from the workspace root: `just flash-avr-example`.

#![no_std]
#![no_main]

use ferriswheel::{Direction, RainbowEffect};
use panic_halt as _;
use rgb::RGB8;
use rustyfarian_avr_ws2812::{ports, Ws2812BitBang};

const NUM_LEDS: usize = 12;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // The const generics describe which AVR pin to drive:
    //   - First const  (`{ ports::PORTB }`): the port-register address. Use
    //     `ports::PORTB` for D8–D13, `ports::PORTC` for A0–A6, `ports::PORTD`
    //     for D0–D7 on Arduino Uno/Nano (ATmega328P).
    //   - Second const (`3`): the pin's bit number within that port. `3` here =
    //     bit 3 of PORTB = PB3 = Arduino D11. To target e.g. D6 (PD6), change
    //     to `Ws2812BitBang<_, { ports::PORTD }, 6>` and pass `pins.d6` below.
    //
    // The driver also owns the configured output pin so DDR stays set for the
    // driver's lifetime.
    let mut driver: Ws2812BitBang<_, { ports::PORTB }, 3> =
        Ws2812BitBang::new(pins.d11.into_output());

    let mut effect = RainbowEffect::new(NUM_LEDS)
        .unwrap()
        .with_brightness(32) // ~12% — safe for USB power
        .with_speed(2)
        .unwrap()
        .with_direction(Direction::Clockwise);

    let mut colors = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut colors).ok();
        // Bit-bang driver wraps the asm in `interrupt::free` internally —
        // no critical-section boilerplate needed at the call site.
        driver.write(&colors).ok();
        arduino_hal::delay_ms(50);
    }
}

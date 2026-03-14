//! Production bit-bang driver demo: `Ws2812BitBang` driving `ferriswheel::PulseEffect`.
//!
//! This is the recommended bit-bang reference for ATmega328P. It uses the
//! [`rustyfarian_avr_ws2812::Ws2812BitBang`] driver — a thin wrapper around the
//! cycle-counted asm validated in `bin/bitbang_spike.rs` — exposed via const
//! generics over port-register address and pin bit.
//!
//! See [ADR 007] and `docs/features/avr-bitbang-driver.md`.
//!
//! [ADR 007]: ../../../../docs/adr/007-avr-ws2812-driver-strategy.md
//!
//! Wiring: D11 → 330 Ω → DIN, 5 V supply with shared GND (same as the spike).
//!
//! Build and flash from `examples/avr-nano-rainbow/`:
//!
//! ```sh
//! cargo +nightly-2025-04-27 run --release -Z build-std=core --bin bitbang_demo
//! ```
//!
//! Or via just from the workspace root: `just flash-avr-bitbang-demo`.
//!
//! Pass criteria:
//! - All `NUM_LEDS` glow smooth red, breathing on a sine curve (identical to the spike)
//! - No flicker, no other colors, no chain leakage

#![no_std]
#![no_main]

use ferriswheel::PulseEffect;
use panic_halt as _;
use rgb::RGB8;
use rustyfarian_avr_ws2812::{ports, Ws2812BitBang};

const NUM_LEDS: usize = 10;

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
    // Ownership of the configured output pin is transferred to the driver,
    // tying GPIO direction to the driver's lifetime.
    let mut driver: Ws2812BitBang<_, { ports::PORTB }, 3> =
        Ws2812BitBang::new(pins.d11.into_output());

    let mut effect = PulseEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(RGB8::new(255, 0, 0))
        .with_min_brightness(0)
        .with_max_brightness(32)
        .with_speed(2)
        .unwrap();

    let mut buf = [RGB8::default(); NUM_LEDS];

    loop {
        effect.update(&mut buf).ok();
        driver.write(&buf).ok();
        arduino_hal::delay_ms(20);
    }
}

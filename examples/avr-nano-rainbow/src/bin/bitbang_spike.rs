//! Track B Spike: Cycle-counted bit-banged WS2812 on PB3 (Arduino D11).
//!
//! **Status:** SPIKE — bit-bang approach hardware-validated 2026-05-04 with a
//! triangle-ramp red breath. Now driving `ferriswheel::PulseEffect` to validate
//! the full effect-pipeline → asm-send integration before generalising the asm
//! into `rustyfarian-avr-ws2812` as the `bitbang` backend.
//!
//! See `docs/features/avr-bitbang-driver.md`.
//!
//! Pass criteria:
//! - All `NUM_LEDS` glow smooth red (not white), breathing on a sine curve
//! - No flicker, no other colors, no chain leakage
//!
//! Wiring: same as the SPI example — D11 → 330 Ω → DIN, 5 V supply with shared GND.
//!
//! Build and flash from `examples/avr-nano-rainbow/`:
//!
//! ```sh
//! cargo +nightly-2025-04-27 run --release -Z build-std=core --bin bitbang_spike
//! ```
//!
//! ## Timing budget at 16 MHz (1 cycle = 62.5 ns)
//!
//! Adapted from Adafruit_NeoPixel's proven ATmega328P @ 16 MHz numbers:
//!
//! | Bit | T_H        | T_L        | Total |
//! |:----|:-----------|:-----------|:------|
//! | "0" | 4 cycles   | 16 cycles  | 20 cy |
//! | "1" | 13 cycles  | 7 cycles   | 20 cy |
//!
//! Per-bit Rust-loop overhead (shift + branch, ~3-5 cycles) extends T_L between
//! bits but stays well below the WS2812B reset threshold of ~50 µs.
//! Cycle-accurate timing is a follow-up; this spike validates the approach.

#![no_std]
#![no_main]
#![feature(asm_experimental_arch)]

use core::arch::asm;
use ferriswheel::PulseEffect;
use panic_halt as _;
use rgb::RGB8;

const NUM_LEDS: usize = 10;

// PB3 = Arduino D11. PORTB I/O address = 0x05 (low I/O space, sbi/cbi work).
// We hardcode both for this spike — generalisation comes when the routine
// moves into the driver crate.

/// Send one byte MSB-first to PB3 with cycle-counted asm.
///
/// SAFETY: caller MUST disable global interrupts for the duration of a frame
/// to keep timing within WS2812 spec.
#[inline(always)]
unsafe fn ws2812_send_byte(byte: u8) {
    let mut b = byte;
    for _ in 0..8 {
        if (b & 0x80) != 0 {
            // bit = 1: T1H = 13 cycles, T1L = 7 cycles, total 20
            asm!(
                "sbi 0x05, 3", // 2 cy — pin HIGH
                "nop", "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop",  // 11 cy padding
                "cbi 0x05, 3", // 2 cy — pin LOW (T_H complete)
                "nop", "nop", "nop", "nop", "nop",  // 5 cy padding
                options(nostack, preserves_flags),
            );
        } else {
            // bit = 0: T0H = 4 cycles, T0L = 16 cycles, total 20
            asm!(
                "sbi 0x05, 3", // 2 cy — pin HIGH
                "nop", "nop",  // 2 cy padding
                "cbi 0x05, 3", // 2 cy — pin LOW (T_H complete)
                "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop",         // 14 cy padding
                options(nostack, preserves_flags),
            );
        }
        b <<= 1;
    }
}

/// Send a buffer of `RGB8` pixels to the chain (GRB byte order on the wire).
///
/// Wraps the per-byte asm in a `interrupt::free` critical section for the whole
/// frame so timing stays within WS2812 spec.
unsafe fn ws2812_send_pixels(pixels: &[RGB8]) {
    avr_device::interrupt::free(|_| {
        for pixel in pixels {
            ws2812_send_byte(pixel.g);
            ws2812_send_byte(pixel.r);
            ws2812_send_byte(pixel.b);
        }
    });
    // Pin is low at exit; the >50 µs gap before the next call latches the frame.
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Configure D11 (PB3) as output via arduino-hal's safe API.
    // The pin is held by `_data_pin` — dropping it would re-configure as input.
    let _data_pin = pins.d11.into_output();

    // Sine-curve red breath via ferriswheel's PulseEffect.
    // max_brightness = 32 keeps the strip dim enough for USB power (~10% of full).
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

        unsafe {
            ws2812_send_pixels(&buf);
        }

        arduino_hal::delay_ms(20);
    }
}

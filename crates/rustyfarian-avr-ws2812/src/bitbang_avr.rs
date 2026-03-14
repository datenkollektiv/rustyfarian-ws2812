//! AVR-specific cycle-counted bit-bang assembly for WS2812 timing.
//!
//! This module is gated `#[cfg(target_arch = "avr")]` and pulls in `core::arch::asm`
//! via the `asm_experimental_arch` feature (enabled in the crate root).
//!
//! Timing budget at 16 MHz `F_CPU` (1 cycle = 62.5 ns), matching `Adafruit_NeoPixel`'s
//! proven ATmega328P pattern:
//!
//! | Bit | T_H        | T_L        | Total     |
//! |:----|:-----------|:-----------|:----------|
//! | 0   | 4 cycles   | 16 cycles  | 20 cycles |
//! | 1   | 13 cycles  | 7 cycles   | 20 cycles |
//!
//! `sbi` and `cbi` are 2-cycle, single-instruction port-bit operations that work on
//! lower I/O space (addresses 0x00–0x1F). All three ATmega328P GPIO ports (PORTB at 0x05,
//! PORTC at 0x08, PORTD at 0x0B) are in this range.
//!
//! `PORT_ADDR` and `PIN_BIT` are substituted as `const` operands at compile time, so the
//! generated assembly is identical to the spike's hardcoded `sbi 0x05, 3` form — no
//! runtime port-pointer indirection, same 2-cycle path.

use core::arch::asm;

/// Send one byte MSB-first to `PORT_ADDR` bit `PIN_BIT` with cycle-counted asm.
///
/// # Timing model
///
/// Each bit's high pulse and low pulse are **cycle-counted inside the `asm!` block**:
/// the 20-cycle bit cell (T_H + T_L) is exact at 16 MHz `F_CPU`.
/// The Rust `for _ in 0..8` loop and bit-test branch sit *between* the asm blocks,
/// so a few extra cycles of pin-low slack accumulate between adjacent bits and
/// between adjacent bytes.
///
/// At `opt-level = "s"` with LTO this loop overhead is roughly 3–5 CPU cycles
/// (≈ 200–300 ns at 16 MHz) — well under the WS2812 chip's > 50 µs reset/latch
/// threshold, so the strip never inadvertently resets mid-frame. The slack only
/// extends `T_L`, which the WS2812B accepts for several µs without affecting bit
/// interpretation. Hardware-validated end-to-end on the production demo binary
/// (`examples/avr-nano-rainbow/src/bin/bitbang_demo.rs`).
///
/// If a future use case (e.g. very long chains, looser-tolerance clones) shows
/// artefacts attributable to inter-bit slack, the follow-up is to move the byte
/// loop into a single `asm!` block using the Adafruit "head20" pattern with a
/// `next` register to balance both branches at exactly 20 cycles per bit. See
/// `docs/features/avr-bitbang-driver.md` "head20 single-block asm optimisation".
///
/// # Safety
///
/// - Caller MUST disable global interrupts for the duration of a frame
///   (the public [`Ws2812BitBang::write`] wrapper does this via `interrupt::free`).
/// - `PORT_ADDR` MUST be a port register address in low I/O space (0x00–0x1F)
///   so `sbi`/`cbi` can reach it; enforced at compile time by the public type.
/// - `PIN_BIT` MUST be 0–7; enforced at compile time by the public type.
/// - The pin MUST already be configured as output by the caller.
///
/// [`Ws2812BitBang::write`]: crate::Ws2812BitBang::write
#[inline(always)]
pub(crate) unsafe fn send_byte<const PORT_ADDR: u8, const PIN_BIT: u8>(byte: u8) {
    let mut b = byte;
    for _ in 0..8 {
        if (b & 0x80) != 0 {
            // bit = 1: T1H = 13 cycles, T1L = 7 cycles, total 20
            asm!(
                "sbi {p}, {n}",      // 2 cy — pin HIGH
                "nop", "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop",   // 11 cy padding
                "cbi {p}, {n}",      // 2 cy — pin LOW (T_H complete)
                "nop", "nop", "nop", "nop", "nop",   // 5 cy padding
                p = const PORT_ADDR,
                n = const PIN_BIT,
                options(nostack, preserves_flags),
            );
        } else {
            // bit = 0: T0H = 4 cycles, T0L = 16 cycles, total 20
            asm!(
                "sbi {p}, {n}",      // 2 cy — pin HIGH
                "nop", "nop",        // 2 cy padding
                "cbi {p}, {n}",      // 2 cy — pin LOW (T_H complete)
                "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop", "nop",
                "nop", "nop", "nop", "nop",          // 14 cy padding
                p = const PORT_ADDR,
                n = const PIN_BIT,
                options(nostack, preserves_flags),
            );
        }
        b <<= 1;
    }
}

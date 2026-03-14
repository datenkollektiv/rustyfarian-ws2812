//! Host-runnable tests for the bit-bang backend.
//!
//! These tests cannot validate cycle-accurate timing (that requires AVR hardware
//! or an instruction-set simulator). They cover:
//!
//! - The public `ports` constants match the ATmega328P datasheet I/O addresses.
//! - Const-generic instantiation compiles for representative `(PORT, BIT)` pairs.
//! - Public types implement the expected traits.
//! - `SmartLedsWrite` adapter forwards to the inner `write` (the AVR write is a
//!   no-op on host, but the trait machinery and stack-buffer adapter are exercised).
//!
//! Run via `just test-avr-all-features`.

#![cfg(feature = "bitbang")]

use rgb::RGB8;
use rustyfarian_avr_ws2812::{ports, BitBangError, Ws2812BitBang};

#[test]
fn port_constants_match_datasheet() {
    // ATmega328P I/O register addresses (low I/O space, sbi/cbi reachable).
    assert_eq!(ports::PORTB, 0x05);
    assert_eq!(ports::PORTC, 0x08);
    assert_eq!(ports::PORTD, 0x0B);
}

#[test]
fn instantiate_for_each_port() {
    // Driver type compiles for any in-range (port, bit) combination.
    // Use `()` as a stand-in pin type — the host stub never reads it.
    let _b: Ws2812BitBang<(), { ports::PORTB }, 3> = Ws2812BitBang::new(());
    let _c: Ws2812BitBang<(), { ports::PORTC }, 0> = Ws2812BitBang::new(());
    let _d: Ws2812BitBang<(), { ports::PORTD }, 7> = Ws2812BitBang::new(());
}

#[test]
fn write_returns_ok_on_host_stub() {
    // On non-AVR targets, `write` is a no-op that returns Ok.
    let mut driver: Ws2812BitBang<(), { ports::PORTB }, 3> = Ws2812BitBang::new(());
    let pixels = [RGB8::new(8, 0, 0); 10];
    assert!(driver.write(&pixels).is_ok());
}

#[test]
fn release_returns_inner_pin() {
    // `release` consumes the driver and yields the wrapped pin token.
    let driver: Ws2812BitBang<u32, { ports::PORTB }, 3> = Ws2812BitBang::new(42u32);
    assert_eq!(driver.release(), 42);
}

#[test]
fn error_implements_traits() {
    // Sanity-check Debug/Display/Eq derives so library users can compare and log.
    let e = BitBangError::Infallible;
    assert_eq!(e, BitBangError::Infallible);
    let _ = format!("{e:?}");
    let _ = format!("{e}");
}

#[cfg(feature = "smart-leds-trait")]
#[test]
fn smart_leds_write_adapter_compiles_and_runs() {
    use smart_leds_trait::SmartLedsWrite;

    let mut driver: Ws2812BitBang<(), { ports::PORTB }, 3> = Ws2812BitBang::new(());
    let pixels = vec![RGB8::new(1, 2, 3); 5];

    // Iterator-based path: typical `smart-leds` consumer call site.
    SmartLedsWrite::write(&mut driver, pixels.iter().copied()).unwrap();
}

#[cfg(feature = "smart-leds-trait")]
#[test]
fn smart_leds_write_truncates_oversize_input() {
    use smart_leds_trait::SmartLedsWrite;

    // Adapter buffers up to MAX_LEDS (256). Feeding more should not panic.
    let mut driver: Ws2812BitBang<(), { ports::PORTB }, 3> = Ws2812BitBang::new(());
    let oversize = (0..300u16).map(|i| RGB8::new(i as u8, 0, 0));
    SmartLedsWrite::write(&mut driver, oversize).unwrap();
}

//! Cycle-counted inline-`asm!` bit-bang WS2812 driver for ATmega328P @ 16 MHz.
//!
//! See [ADR 007] for the rationale. The SPI prerendered backend ([`Ws2812Spi`])
//! relies on chip tolerance for `T0H ≈ 500 ns` and `T1H ≈ 1500 ns`; this backend
//! produces in-spec timing on every strip and is the recommended AVR driver.
//!
//! [ADR 007]: https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/docs/adr/007-avr-ws2812-driver-strategy.md
//! [`Ws2812Spi`]: crate::Ws2812Spi
//!
//! # Constraints
//!
//! - **MCU:** ATmega328P only.
//!   The cycle counts are tuned for 16 MHz `F_CPU`. Other AVR variants and clock
//!   rates are a follow-up.
//! - **Pin:** any pin on PORTB / PORTC / PORTD. All three ports lie in the
//!   `sbi`/`cbi` low I/O-address range (0x00–0x1F) on the ATmega328P.
//!   See [`ports`] for the constants.
//! - **Interrupts:** disabled internally for the duration of each `write` call,
//!   via `avr_device::interrupt::free`. Timing is mandatory, not opportunistic.
//!
//! # Example
//!
//! ```ignore
//! use arduino_hal::pins;
//! use rustyfarian_avr_ws2812::{ports, Ws2812BitBang};
//! use rgb::RGB8;
//!
//! let dp = arduino_hal::Peripherals::take().unwrap();
//! let pins = pins!(dp);
//!
//! let pin = pins.d11.into_output();
//! let mut driver: Ws2812BitBang<_, { ports::PORTB }, 3> = Ws2812BitBang::new(pin);
//!
//! let colors = [RGB8::new(8, 0, 0); 10]; // dim red across 10 LEDs
//! driver.write(&colors).ok();
//! ```

use core::fmt;
use rgb::RGB8;

#[cfg(target_arch = "avr")]
use crate::bitbang_avr::send_byte;

/// AVR I/O addresses for the ATmega328P GPIO ports in low I/O space.
///
/// Pass one of these as the `PORT_ADDR` const generic of [`Ws2812BitBang`].
pub mod ports {
    /// PORTB register address. Pins PB0–PB7 (Arduino Uno/Nano D8–D13 + crystal pins).
    pub const PORTB: u8 = 0x05;
    /// PORTC register address. Pins PC0–PC6 (Arduino Uno/Nano A0–A6).
    pub const PORTC: u8 = 0x08;
    /// PORTD register address. Pins PD0–PD7 (Arduino Uno/Nano D0–D7).
    pub const PORTD: u8 = 0x0B;
}

/// Errors returned by [`Ws2812BitBang::write`].
///
/// The current implementation is infallible — the variant is reserved so adding
/// recoverable error cases later (e.g. timeout on a future async variant) does
/// not break the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBangError {
    /// Reserved. The current bit-bang `write` cannot fail.
    #[doc(hidden)]
    Infallible,
}

impl fmt::Display for BitBangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infallible => write!(f, "bit-bang infallible (reserved variant)"),
        }
    }
}

/// WS2812 driver using cycle-counted inline assembly bit-bang.
///
/// `PORT_ADDR` is the AVR port register address in low I/O space (use one of
/// the constants in [`ports`]); `PIN_BIT` is the bit number 0–7.
/// The driver owns the configured output pin `P` so the GPIO direction is
/// tied to the driver's lifetime, mirroring [`Ws2812Spi`]'s ownership model.
///
/// # Target Architecture
///
/// This driver only emits real WS2812 timing on AVR.
/// On non-AVR builds (e.g. `cargo test` on the host) the type is still defined
/// and [`write`](Self::write) is a no-op that returns `Ok(())` — see that method's
/// docs for the rationale and the "compiled for the wrong target" caveat.
///
/// # Compile-time validation
///
/// `PORT_ADDR` must be in the AVR low I/O address space (`0x00..=0x1F`) so the
/// `sbi`/`cbi` instructions can reach it; `PIN_BIT` must be `0..=7`.
/// Out-of-range values fail to compile with a clear `assert!` message at the
/// user's call site — there is no way to construct a misaligned driver instance.
///
/// Out-of-range port address (sbi/cbi can't reach it):
///
/// ```compile_fail
/// use rustyfarian_avr_ws2812::Ws2812BitBang;
/// let _: Ws2812BitBang<(), 0x22, 3> = Ws2812BitBang::new(());
/// ```
///
/// Out-of-range pin bit:
///
/// ```compile_fail
/// use rustyfarian_avr_ws2812::Ws2812BitBang;
/// use rustyfarian_avr_ws2812::ports::PORTB;
/// let _: Ws2812BitBang<(), { PORTB }, 9> = Ws2812BitBang::new(());
/// ```
///
/// [`Ws2812Spi`]: crate::Ws2812Spi
pub struct Ws2812BitBang<P, const PORT_ADDR: u8, const PIN_BIT: u8> {
    _pin: P,
}

impl<P, const PORT_ADDR: u8, const PIN_BIT: u8> Ws2812BitBang<P, PORT_ADDR, PIN_BIT> {
    /// Compile-time guard for the const generics.
    ///
    /// Evaluating this constant fails the build during monomorphisation if
    /// `PORT_ADDR` is outside the AVR low I/O space or `PIN_BIT` is > 7.
    /// Referenced in [`new`](Self::new) to force evaluation at every call site.
    const VALIDATE: () = {
        assert!(
            PORT_ADDR <= 0x1F,
            "Ws2812BitBang: PORT_ADDR must be in AVR low I/O space (0x00..=0x1F) so sbi/cbi can reach it",
        );
        assert!(
            PIN_BIT <= 7,
            "Ws2812BitBang: PIN_BIT must be a valid AVR pin bit number (0..=7)",
        );
    };

    /// Wrap an already-configured output pin in a bit-bang driver.
    ///
    /// The caller MUST configure the pin as output before constructing the driver
    /// (typically via `pins.dN.into_output()` from `arduino-hal`).
    /// The driver only writes to the PORT register, never to DDR.
    ///
    /// `PORT_ADDR` and `PIN_BIT` are validated at compile time — see the
    /// [type-level docs](Self#compile-time-validation).
    pub const fn new(pin: P) -> Self {
        // Force monomorphisation-time evaluation of `VALIDATE`. The `let _` keeps
        // the reference live in `const fn` context.
        let _ = Self::VALIDATE;
        Self { _pin: pin }
    }

    /// Send a buffer of `RGB8` colors to the LED chain (GRB byte order on the wire).
    ///
    /// On AVR (`target_arch = "avr"`) this wraps the asm loop in
    /// `avr_device::interrupt::free(..)` so global interrupts stay disabled for the
    /// duration of the frame — required for cycle-accurate WS2812 timing.
    ///
    /// # Host (non-AVR) behavior
    ///
    /// On any non-AVR target this is a deliberate no-op that returns `Ok(())`.
    /// The host stub exists so the public API surface (`new`, `release`,
    /// `SmartLedsWrite`, the const-generic guards) can be exercised by host unit
    /// tests without an AVR toolchain.
    ///
    /// **Caveat:** if you accidentally build for a non-AVR target (e.g. you forget
    /// `--target avr-none` or omit `[build] target = "avr-none"` in `.cargo/config.toml`),
    /// `write` will silently succeed and *no LEDs will light*. Always double-check the
    /// build target when LEDs unexpectedly stay dark — `cargo build --release` from a
    /// host shell with no AVR target configured produces a host-architecture binary that
    /// cannot drive real hardware.
    #[allow(unused_variables)] // `colors` is consumed by the AVR cfg branch only.
    pub fn write(&mut self, colors: &[RGB8]) -> Result<(), BitBangError> {
        #[cfg(target_arch = "avr")]
        {
            avr_device::interrupt::free(|_| {
                for pixel in colors {
                    // SAFETY: PORT_ADDR is a const-generic compile-time port address;
                    // the caller has configured the pin as output; `interrupt::free`
                    // guarantees the asm timing.
                    unsafe {
                        send_byte::<PORT_ADDR, PIN_BIT>(pixel.g);
                        send_byte::<PORT_ADDR, PIN_BIT>(pixel.r);
                        send_byte::<PORT_ADDR, PIN_BIT>(pixel.b);
                    }
                }
            });
            // Pin is low at exit; the >50 µs gap before the next call latches the frame.
        }
        Ok(())
    }

    /// Consume the driver and return the wrapped pin so the caller can repurpose it.
    pub fn release(self) -> P {
        self._pin
    }
}

/// `SmartLedsWrite` adapter for [`Ws2812BitBang`].
///
/// Provides ecosystem parity with [`smart-leds`] consumers (sister ESP drivers
/// implement the same trait). Internally, the iterator is collected into a
/// stack-allocated `[RGB8; 256]` and forwarded to [`Ws2812BitBang::write`].
///
/// # Stack cost
///
/// **Each call uses ≈ 768 bytes of stack** (`256 × size_of::<RGB8>()`).
/// On ATmega328P (2 KB SRAM total) this is a meaningful fraction of available
/// memory — depending on what else is on the call stack, it can leave little
/// headroom for arrays, `core::fmt` machinery, or interrupt-context frames.
///
/// If you don't need the iterator-based ergonomics, prefer the inherent
/// [`Ws2812BitBang::write`] which takes a `&[RGB8]` directly and **allocates no
/// adapter buffer**: pass a `[RGB8; NUM_LEDS]` you already own (e.g. the one a
/// `ferriswheel::Effect` writes into).
///
/// # Truncation
///
/// Iterators with more than `256` items are silently truncated to the first
/// 256 colors — the cap matches [`ferriswheel::effect::MAX_LEDS`]. This avoids
/// dynamic allocation while staying within the workspace's pure-logic contract.
/// If you need longer chains, use the inherent `write` with a slice of any length.
///
/// [`smart-leds`]: https://crates.io/crates/smart-leds
/// [`ferriswheel::effect::MAX_LEDS`]: https://docs.rs/ferriswheel/latest/ferriswheel/effect/constant.MAX_LEDS.html
#[cfg(feature = "smart-leds-trait")]
impl<P, const PORT_ADDR: u8, const PIN_BIT: u8> smart_leds_trait::SmartLedsWrite
    for Ws2812BitBang<P, PORT_ADDR, PIN_BIT>
{
    type Error = BitBangError;
    type Color = RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // 256 × 3 bytes = 768 bytes on stack — see the impl docs above.
        let mut buf = [RGB8::default(); 256];
        let mut count = 0usize;
        for color in iterator {
            if count >= buf.len() {
                break;
            }
            buf[count] = color.into();
            count += 1;
        }
        Self::write(self, &buf[..count])
    }
}

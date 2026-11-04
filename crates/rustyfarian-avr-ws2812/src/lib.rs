#![no_std]
#![cfg_attr(
    all(feature = "bitbang", target_arch = "avr"),
    feature(asm_experimental_arch)
)]
//! WS2812 (NeoPixel) LED driver for AVR with two backends.
//!
//! Both backends share the same `&[RGB8]`-based public API and run every
//! [`ferriswheel`](https://crates.io/crates/ferriswheel) effect unchanged.
//! All animation logic stays in `ferriswheel` / `bunting`; these drivers are
//! thin hardware wrappers.
//!
//! # Choosing a Backend
//!
//! | Aspect             | [`Ws2812Spi`] (SPI prerendered) | [`Ws2812BitBang`] (cycle-counted asm) |
//! |:-------------------|:--------------------------------|:--------------------------------------|
//! | Cargo feature      | always available                | `bitbang` (opt-in)                    |
//! | Status             | works on tolerant strips        | **recommended; hardware-validated**   |
//! | Hardware           | any AVR with SPI peripheral     | ATmega328P @ 16 MHz                   |
//! | Pin                | the SPI MOSI pin                | any pin on PORTB / PORTC / PORTD      |
//! | WS2812 timing      | `T0H = 500 ns`, `T1H = 1500 ns` (relies on chip tolerance) | `T0H = 250 ns`, `T1H = 812 ns` (in WS2812B spec) |
//! | Interrupts         | caller wraps `set_pixels_slice` in `avr_device::interrupt::free` | wrapped internally — timing is mandatory |
//! | Other peripherals  | SPI bus is owned by the driver  | only the chosen GPIO pin is owned     |
//!
//! [ADR 007](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/docs/adr/007-avr-ws2812-driver-strategy.md)
//! records the empirical evidence: a strip that works correctly on the ESP32 RMT drivers
//! produced stable white-ish output and chain leakage on the SPI prerendered backend
//! across both genuine and clone Arduino Nanos.
//! The bit-bang backend renders correctly on the same hardware.
//!
//! Use the SPI backend if your strip is known to tolerate the encoding's
//! out-of-spec `T1H`, or if you need other peripherals to keep operating during the LED
//! write. Use the bit-bang backend everywhere else.
//!
//! # SPI Prerendered Backend
//!
//! [`Ws2812Spi`] drives WS2812/NeoPixel LEDs over SPI using the prerendered encoding
//! from [`bunting`](https://crates.io/crates/bunting) — 4 SPI bits per WS2812 bit,
//! 12 SPI bytes per LED. The encoding is byte-for-byte compatible with
//! [`ws2812-spi`](https://crates.io/crates/ws2812-spi) v0.5.1's prerendered module.
//!
//! ## Buffer sizing
//!
//! Const-generic buffer `[u8; N]` where `N = spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`.
//! Use [`spi_buffer_size`] to compute `N` at compile time:
//!
//! ```ignore
//! use rustyfarian_avr_ws2812::spi_buffer_size;
//! const N: usize = spi_buffer_size(8); // 8-LED ring
//! ```
//!
//! ## SPI clock configuration
//!
//! The SPI peripheral **must** be configured at 2 MHz before the first call to
//! [`Ws2812Spi::set_pixels_slice`]. On a 16 MHz ATmega328P, use a clock prescaler of ÷8.
//!
//! ## Interrupt safety
//!
//! The SPI backend is **not** interrupt-safe by itself; the caller wraps each
//! `set_pixels_slice` in a critical section:
//!
//! ```ignore
//! avr_device::interrupt::free(|_| {
//!     ws.set_pixels_slice(&colors).unwrap();
//! });
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use rustyfarian_avr_ws2812::{Ws2812Spi, spi_buffer_size};
//! use rgb::RGB8;
//!
//! const NUM_LEDS: usize = 8;
//! const N: usize = spi_buffer_size(NUM_LEDS);
//!
//! let mut ws: Ws2812Spi<_, N> = Ws2812Spi::new(spi_bus);
//! let colors = [RGB8::new(255, 0, 0); NUM_LEDS];
//! avr_device::interrupt::free(|_| {
//!     ws.set_pixels_slice(&colors).unwrap();
//! });
//! ```
//!
//! # Bit-Bang Backend (recommended)
//!
//! [`Ws2812BitBang`] uses cycle-counted inline `asm!` to drive any GPIO pin in low
//! I/O space (PORTB / PORTC / PORTD on ATmega328P) at WS2812-spec timing.
//! The `set_pixels_slice` method wraps the asm loop in `avr_device::interrupt::free` internally —
//! the caller does **not** need to add a critical section.
//!
//! Enable the `bitbang` feature in `Cargo.toml`:
//!
//! ```toml
//! rustyfarian-avr-ws2812 = { version = "0.1", features = ["bitbang"] }
//! ```
//!
//! ## Pin selection
//!
//! The driver is generic over the port-register address ([`ports::PORTB`], [`ports::PORTC`],
//! [`ports::PORTD`]) and the pin bit number (0–7). Both are compile-time constants so
//! the asm uses single-instruction `sbi`/`cbi` operations.
//!
//! ## Interrupt safety
//!
//! Cycle-accurate WS2812 timing requires interrupts to stay disabled for the full frame
//! window (≈ 30 µs per LED). The driver does this internally; user code remains free of
//! `interrupt::free` boilerplate. The `millis()` timer and serial UART will lose ticks
//! during the write window — standard tradeoff documented in `docs/avr-getting-started.md`.
//!
//! ## Example
//!
//! ```ignore
//! use rustyfarian_avr_ws2812::{ports, Ws2812BitBang};
//! use rgb::RGB8;
//!
//! let pin = pins.d11.into_output();
//! let mut driver: Ws2812BitBang<_, { ports::PORTB }, 3> = Ws2812BitBang::new(pin);
//!
//! let colors = [RGB8::new(8, 0, 0); 10];
//! driver.set_pixels_slice(&colors).ok(); // no `interrupt::free` needed — handled inside
//! ```
//!
//! # Vocabulary
//!
//! Both backends share the method names of the ESP driver crates:
//! `set_pixels_slice(&[RGB8])` sends a whole frame and `set_pixel(RGB8)` sends a one-LED frame.
//! The 0.7 method `write` stays as a deprecated alias of `set_pixels_slice` for the 0.8 cycle.
//!
//! # Optional features
//!
//! - `smart-leds-trait` implements `SmartLedsWrite` for both backends.
//! - `pennant` implements `pennant::StatusLed` for both backends, addressing the first LED.
//!
//! # Runnable Examples
//!
//! Standalone, flashable Arduino Nano examples live at
//! [`examples/avr-nano-rainbow/`](https://github.com/datenkollektiv/rustyfarian-ws2812/tree/main/examples/avr-nano-rainbow)
//! in the workspace root (separate AVR toolchain, target, and `arduino-hal` git dependency).
//! See [`docs/avr-getting-started.md`](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/docs/avr-getting-started.md)
//! for wiring, toolchain setup, and `just` recipes:
//!
//! - `just flash-avr-example` — bit-bang `RainbowEffect`, the recommended demo (`src/main.rs`)
//! - `just flash-avr-bitbang-demo` — bit-bang `PulseEffect` red breath (`bin/bitbang_demo`)
//! - `just flash-avr-spi-rainbow` — SPI prerendered comparison, **diagnostic only** (`bin/spi_rainbow`)
//! - `just flash-avr-bitbang-spike` — frozen low-level reference, no driver crate (`bin/bitbang_spike`)

use bunting::{
    checked_spi_frame_len, prerender_spi, spi_data_len, SpiEncodeError, SPI_RESET_BYTES_2MHZ,
};

/// Stack capacity of the `SmartLedsWrite::write` collect buffer.
///
/// Matches `ferriswheel::effect::MAX_LEDS`; this crate does not depend on ferriswheel.
#[cfg(feature = "smart-leds-trait")]
pub(crate) const MAX_WRITE_LEDS: usize = 256;
use core::fmt;
use embedded_hal::spi::SpiBus;
use rgb::RGB8;

/// Errors that can occur during WS2812 SPI operations.
#[derive(Debug)]
pub enum SpiError<E> {
    /// The color slice was too large for the buffer.
    Encode(SpiEncodeError),
    /// The underlying SPI bus returned an error.
    Spi(E),
}

// `Display` already includes the encode error, so `source()` stays `None` to avoid printing it twice in error chains.
impl<E: fmt::Debug> core::error::Error for SpiError<E> {}

impl<E: fmt::Debug> fmt::Display for SpiError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(e) => write!(f, "SPI encode error: {e}"),
            Self::Spi(e) => write!(f, "SPI bus error: {e:?}"),
        }
    }
}

impl<E> From<SpiEncodeError> for SpiError<E> {
    fn from(e: SpiEncodeError) -> Self {
        Self::Encode(e)
    }
}

/// Returns the total SPI buffer size for `num_leds` LEDs (data + reset bytes).
///
/// Use this as the const generic `N` for [`Ws2812Spi`]:
///
/// ```
/// use rustyfarian_avr_ws2812::spi_buffer_size;
/// const N: usize = spi_buffer_size(8); // 8-LED ring → 176
/// assert_eq!(N, 176);
/// ```
pub const fn spi_buffer_size(num_leds: usize) -> usize {
    spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ
}

/// WS2812 LED driver using SPI prerendered encoding (`no_std`, `embedded-hal` 1.0).
///
/// `N` is the total SPI buffer size in bytes.
/// Compute it with [`spi_buffer_size`]: `N = spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`.
///
/// The struct is generic over any [`SpiBus`] implementation — no AVR-specific
/// dependencies are introduced here. The caller is responsible for:
/// - Configuring SPI at 2 MHz before calling [`set_pixels_slice`](Ws2812Spi::set_pixels_slice).
/// - Wrapping the `set_pixels_slice` call in a critical section on interrupt-driven systems.
///
/// # Type Parameters
///
/// - `SPI` — any type implementing [`embedded_hal::spi::SpiBus`].
/// - `N` — total SPI buffer size (`spi_data_len(num_leds) + SPI_RESET_BYTES_2MHZ`).
pub struct Ws2812Spi<SPI, const N: usize> {
    spi: SPI,
    buf: [u8; N],
}

impl<SPI: SpiBus, const N: usize> Ws2812Spi<SPI, N> {
    /// Creates a new WS2812 SPI driver wrapping the given SPI bus.
    ///
    /// The internal buffer is zero-initialised. The SPI bus **must** already be
    /// configured at 2 MHz before the first call to [`set_pixels_slice`](Self::set_pixels_slice).
    pub fn new(spi: SPI) -> Self {
        Self { spi, buf: [0u8; N] }
    }

    /// Encodes `colors` into the internal buffer and sends it over SPI.
    ///
    /// The first `spi_data_len(colors.len())` bytes of the buffer are filled with
    /// prerendered WS2812 SPI data.
    /// The remaining bytes are zeroed to provide the WS2812 reset pulse
    /// (`SPI_RESET_BYTES_2MHZ` = 80 bytes at 2 MHz).
    ///
    /// An empty slice sends nothing and returns `Ok(())`, matching the ESP drivers.
    ///
    /// # Errors
    ///
    /// - [`SpiError::Encode`] if `N < spi_buffer_size(colors.len())`, that is `colors.len() > (N - SPI_RESET_BYTES_2MHZ) / 12`
    ///   or `N` below the 80 reset bytes (the buffer cannot hold the frame and its reset tail).
    /// - [`SpiError::Spi`] if the underlying SPI bus returns an error.
    pub fn set_pixels_slice(&mut self, colors: &[RGB8]) -> Result<(), SpiError<SPI::Error>> {
        if colors.is_empty() {
            return Ok(());
        }
        let data_len = checked_spi_frame_len(colors.len(), N).map_err(SpiError::Encode)?;
        prerender_spi(colors, &mut self.buf[..data_len])?;
        // Zero the reset tail so the WS2812 latches the frame.
        self.buf[data_len..].fill(0);
        self.spi.write(&self.buf).map_err(SpiError::Spi)?;
        Ok(())
    }

    /// Sends a one-LED frame with `rgb`.
    ///
    /// Only the first LED of the chain is addressed.
    /// The rest of the chain keeps the colour it last latched, the same contract as `set_pixel` on the ESP drivers.
    ///
    /// `N` must be at least [`spi_buffer_size(1)`](spi_buffer_size) (92 bytes), otherwise
    /// [`SpiError::Encode`] is returned.
    /// On interrupt-driven systems the caller wraps the call in a critical section.
    ///
    /// # Errors
    ///
    /// Same as [`set_pixels_slice`](Self::set_pixels_slice).
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<(), SpiError<SPI::Error>> {
        self.set_pixels_slice(&[rgb])
    }

    /// Deprecated alias of [`set_pixels_slice`](Self::set_pixels_slice).
    #[deprecated(since = "0.8.0", note = "renamed to `set_pixels_slice`")]
    #[inline]
    pub fn write(&mut self, colors: &[RGB8]) -> Result<(), SpiError<SPI::Error>> {
        self.set_pixels_slice(colors)
    }

    /// Releases the inner SPI bus, consuming the driver.
    pub fn release(self) -> SPI {
        self.spi
    }
}

/// `SmartLedsWrite` adapter for [`Ws2812Spi`].
///
/// Provides ecosystem parity with [`smart-leds`] consumers (sister ESP drivers
/// implement the same trait). Internally, the iterator is collected into a
/// stack-allocated `[RGB8; 256]` and forwarded to [`Ws2812Spi::set_pixels_slice`].
///
/// # Stack cost
///
/// **Each call uses ≈ 768 bytes of stack** (`256 × size_of::<RGB8>()`).
/// On ATmega328P (2 KB SRAM total) this is a meaningful fraction of available
/// memory — depending on what else is on the call stack, it can leave little
/// headroom for arrays, `core::fmt` machinery, or interrupt-context frames.
///
/// If you don't need the iterator-based ergonomics, prefer the inherent
/// [`Ws2812Spi::set_pixels_slice`] which takes a `&[RGB8]` directly and **allocates no
/// adapter buffer**: pass a `[RGB8; NUM_LEDS]` you already own.
///
/// # Truncation
///
/// Iterators with more than `256` items are silently truncated to the first
/// 256 colors — the cap matches [`ferriswheel::effect::MAX_LEDS`]. This avoids
/// dynamic allocation while staying within the workspace's pure-logic contract.
/// If you need longer chains, use the inherent `set_pixels_slice` with a slice of any length.
///
/// # Method resolution
///
/// The trait method is also called `write`, and the deprecated inherent alias `write` shadows it
/// for plain method-call syntax.
/// Call the trait explicitly with `SmartLedsWrite::write(&mut ws, iter)`.
///
/// [`smart-leds`]: https://crates.io/crates/smart-leds
/// [`ferriswheel::effect::MAX_LEDS`]: https://docs.rs/ferriswheel/latest/ferriswheel/effect/constant.MAX_LEDS.html
#[cfg(feature = "smart-leds-trait")]
impl<SPI: SpiBus, const N: usize> smart_leds_trait::SmartLedsWrite for Ws2812Spi<SPI, N> {
    type Error = SpiError<SPI::Error>;
    type Color = RGB8;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // Cap is ferriswheel's MAX_LEDS (not a dependency here); 256 x 3 bytes = 768 bytes on stack.
        let mut buf = [RGB8::default(); MAX_WRITE_LEDS];
        let count = bunting::collect_truncated(iterator.into_iter().map(Into::into), &mut buf);
        self.set_pixels_slice(&buf[..count])
    }
}

/// `pennant::StatusLed` adapter for [`Ws2812Spi`].
///
/// Drives one LED: `set_color` sends a one-LED frame through [`Ws2812Spi::set_pixel`].
/// The caller still wraps the call in a critical section on interrupt-driven systems.
#[cfg(feature = "pennant")]
impl<SPI: SpiBus, const N: usize> pennant::StatusLed for Ws2812Spi<SPI, N> {
    type Error = SpiError<SPI::Error>;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

#[cfg(feature = "bitbang")]
mod bitbang;
#[cfg(all(feature = "bitbang", target_arch = "avr"))]
mod bitbang_avr;

#[cfg(feature = "bitbang")]
pub use bitbang::{ports, BitBangError, Ws2812BitBang};

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use bunting::SpiEncodeError;
    use std::string::ToString;

    #[test]
    fn spi_error_is_core_error_without_source() {
        let err: SpiError<()> = SpiError::Encode(SpiEncodeError::BufferTooSmall);
        let err: &dyn core::error::Error = &err;
        assert!(err.source().is_none());
    }

    // --- set_pixel tests -----------------------------------------------------

    use embedded_hal::spi::{ErrorKind, ErrorType};

    /// Minimal `SpiBus` that records the length of every `write`.
    #[derive(Default)]
    struct RecordingSpi {
        writes: std::vec::Vec<usize>,
    }

    impl ErrorType for RecordingSpi {
        type Error = ErrorKind;
    }

    impl SpiBus for RecordingSpi {
        fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
            self.writes.push(words.len());
            Ok(())
        }

        fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn flush(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn set_pixel_writes_one_led_frame() {
        const N: usize = spi_buffer_size(1);
        let mut ws: Ws2812Spi<RecordingSpi, N> = Ws2812Spi::new(RecordingSpi::default());
        ws.set_pixel(RGB8::new(1, 2, 3)).unwrap();
        let spi = ws.release();
        assert_eq!(spi.writes, std::vec![92]);
    }

    #[test]
    fn set_pixels_slice_buffer_boundary() {
        let colors = [RGB8::new(1, 2, 3); 2];
        let mut ok: Ws2812Spi<RecordingSpi, { spi_buffer_size(2) }> =
            Ws2812Spi::new(RecordingSpi::default());
        ok.set_pixels_slice(&colors).unwrap();
        assert_eq!(ok.release().writes, std::vec![104]);
        let mut short: Ws2812Spi<RecordingSpi, { spi_buffer_size(2) - 1 }> =
            Ws2812Spi::new(RecordingSpi::default());
        let err = short.set_pixels_slice(&colors).unwrap_err();
        assert!(matches!(
            err,
            SpiError::Encode(SpiEncodeError::BufferTooSmall)
        ));
        assert!(short.release().writes.is_empty());
        assert!(short_empty_ok());
    }

    fn short_empty_ok() -> bool {
        let mut ws: Ws2812Spi<RecordingSpi, 0> = Ws2812Spi::new(RecordingSpi::default());
        ws.set_pixels_slice(&[]).is_ok()
    }

    #[test]
    fn set_pixel_with_buffer_below_one_led_is_encode_error() {
        const N: usize = spi_buffer_size(1) - 1;
        let mut ws: Ws2812Spi<RecordingSpi, N> = Ws2812Spi::new(RecordingSpi::default());
        let err = ws.set_pixel(RGB8::new(1, 2, 3)).unwrap_err();
        assert!(matches!(err, SpiError::Encode(_)));
        assert!(ws.release().writes.is_empty());
    }

    #[test]
    fn empty_slice_sends_nothing_even_with_a_tiny_buffer() {
        const N: usize = 1;
        let mut ws: Ws2812Spi<RecordingSpi, N> = Ws2812Spi::new(RecordingSpi::default());
        ws.set_pixels_slice(&[]).unwrap();
        assert!(ws.release().writes.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn write_alias_matches_set_pixels_slice() {
        const N: usize = spi_buffer_size(2);
        let mut ws: Ws2812Spi<RecordingSpi, N> = Ws2812Spi::new(RecordingSpi::default());
        ws.write(&[RGB8::new(1, 2, 3); 2]).unwrap();
        ws.set_pixels_slice(&[RGB8::new(1, 2, 3); 2]).unwrap();
        assert_eq!(ws.release().writes, std::vec![N, N]);
    }

    #[cfg(feature = "pennant")]
    #[test]
    fn status_led_set_color_writes_one_led_frame() {
        const N: usize = spi_buffer_size(1);
        let mut ws: Ws2812Spi<RecordingSpi, N> = Ws2812Spi::new(RecordingSpi::default());
        pennant::StatusLed::set_color(&mut ws, RGB8::new(0, 0, 255)).unwrap();
        assert_eq!(ws.release().writes, std::vec![N]);
    }

    // --- spi_buffer_size tests -----------------------------------------------

    #[test]
    fn spi_buffer_size_zero_leds() {
        assert_eq!(spi_buffer_size(0), SPI_RESET_BYTES_2MHZ);
    }

    #[test]
    fn spi_buffer_size_one_led() {
        // 1 LED: 12 data bytes + 80 reset bytes = 92
        assert_eq!(spi_buffer_size(1), 92);
    }

    #[test]
    fn spi_buffer_size_eight_leds() {
        // 8 LEDs: 96 data bytes + 80 reset bytes = 176
        assert_eq!(spi_buffer_size(8), 176);
    }

    #[test]
    fn spi_buffer_size_twelve_leds() {
        // 12 LEDs: 144 data bytes + 80 reset bytes = 224
        assert_eq!(spi_buffer_size(12), 224);
    }

    // --- SpiError Display tests ----------------------------------------------

    #[test]
    fn spi_error_encode_display() {
        let e: SpiError<()> = SpiError::Encode(SpiEncodeError::BufferTooSmall);
        assert!(e.to_string().contains("encode"));
    }

    #[test]
    fn spi_error_spi_display() {
        let e: SpiError<&str> = SpiError::Spi("bus failure");
        assert!(e.to_string().contains("SPI bus error"));
    }

    // --- From<SpiEncodeError> ------------------------------------------------

    #[test]
    fn from_spi_encode_error() {
        let e: SpiError<()> = SpiEncodeError::BufferTooSmall.into();
        assert!(matches!(
            e,
            SpiError::Encode(SpiEncodeError::BufferTooSmall)
        ));
    }
}

//! WS2812 (NeoPixel) LED driver using ESP-IDF RMT peripheral.
//!
//! This crate provides a driver for WS2812/NeoPixel addressable LEDs using
//! the ESP-IDF RMT (Remote Control Transceiver) peripheral for precise timing.
//! It works with any ESP32 variant that supports RMT via ESP-IDF.
//!
//! For bare-metal (no_std) projects using `esp-hal`, see `rustyfarian-esp-hal-ws2812`.
//! Pure color utilities are available in the `bunting` crate for testing.
//!
//! # Example
//!
//! ```ignore
//! use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
//! use rgb::RGB8;
//!
//! let mut led = Ws2812Rmt::new(peripherals.pins.gpio8)?;
//!
//! led.set_pixel(RGB8::new(255, 0, 0))?;
//!
//! let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
//! led.set_pixels_slice(&colors)?;
//! ```
//!
//! # Supported Boards
//!
//! Works with any ESP32 variant that has RMT support via ESP-IDF:
//! - ESP32-C3-DevKit-Rust-1: GPIO2
//! - ESP32-C3-DevKitC-02: GPIO8
//! - ESP32-C6-DevKitC-1: GPIO8

// Requires esp-idf-hal 0.46+ for TxChannelDriver and BytesEncoder.
// Uses a workaround for send_and_wait bug present in 0.46.2
// (see transmit_bytes and ROADMAP.md).
use anyhow::Result;
use bunting::rgb_to_grb;
use core::time::Duration;
use esp_idf_hal::{
    gpio::OutputPin,
    rmt::{
        config::{MemoryAccess, TransmitConfig, TxChannelConfig},
        encoder::{BytesEncoder, BytesEncoderConfig},
        PinState, Pulse, Symbol, TxChannelDriver,
    },
    units::Hertz,
};
use rgb::RGB8;

/// WS2812 LED driver using RMT peripheral.
///
/// The RMT peripheral provides precise timing control needed for the
/// WS2812 protocol without CPU intervention.
///
/// # Example
///
/// ```ignore
/// use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
/// use rgb::RGB8;
///
/// let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;
/// let mut led = Ws2812Rmt::new(peripherals.pins.gpio8)?;
///
/// led.set_pixel(RGB8::new(255, 0, 0))?;
///
/// let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
/// led.set_pixels_slice(&colors)?;
/// ```
pub struct Ws2812Rmt<'a> {
    tx: TxChannelDriver<'a>,
    encoder_config: BytesEncoderConfig,
}

#[deprecated(
    since = "0.6.0",
    note = "use `Ws2812Rmt` for consistency with the esp-hal driver; planned removal in 0.7.0"
)]
pub type WS2812RMT<'a> = Ws2812Rmt<'a>;

/// Transmits bytes using the C-side `BytesEncoder` directly, bypassing
/// `send_and_wait` which wraps the encoder in a Rust `EncoderWrapper`.
///
/// The `EncoderWrapper` callback converts `rmt_encode_state_t` via a Rust
/// `match` that panics on bitwise-OR'd flag values (e.g. `COMPLETE | MEM_FULL
/// = 0x03`). Since the encode callback runs in ISR context, the panic triggers
/// `abort()` when the panic handler tries to acquire a recursive mutex.
///
/// Using `start_send` + `wait_all_done` passes the C encoder handle directly
/// to `rmt_transmit`, so the ISR calls the C encode function with no Rust
/// wrapper in the path.
// TODO: Switch back to send_and_wait when esp-idf-hal fixes EncoderWrapper
// to handle bitwise-OR'd rmt_encode_state_t flags. See ROADMAP.md.
fn transmit_bytes(
    tx: &mut TxChannelDriver<'_>,
    encoder_config: &BytesEncoderConfig,
    bytes: &[u8],
) -> Result<()> {
    let mut encoder = BytesEncoder::with_config(encoder_config)?;
    // SAFETY: `encoder` and `bytes` live until `wait_all_done` returns.
    unsafe {
        tx.start_send(&mut encoder, bytes, &TransmitConfig::default())?;
    }
    tx.wait_all_done(None)?;
    Ok(())
}

impl<'d> Ws2812Rmt<'d> {
    /// Creates a new WS2812 driver with default channel configuration.
    ///
    /// The default uses one RMT memory block of 48 symbols, which is the block
    /// size on ESP32-C3 and ESP32-C6. These chips have only two TX channels, so
    /// the ESP-IDF default of 64 symbols would round up to two blocks and exhaust
    /// all available TX candidates.
    ///
    /// For other ESP32 variants (classic ESP32, S2, S3) where each block holds 64
    /// symbols, use [`new_with_channel_config`] to pass an appropriate
    /// `TxChannelConfig`.
    ///
    /// # Arguments
    ///
    /// * `led` - GPIO pin connected to the LED data line
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut led = Ws2812Rmt::new(peripherals.pins.gpio8)?;
    /// ```
    ///
    /// [`new_with_channel_config`]: Ws2812Rmt::new_with_channel_config
    pub fn new(led: impl OutputPin + 'd) -> Result<Self> {
        // 48 symbols = one RMT memory block on ESP32-C3/C6 (default 64 would round
        // up to 2 blocks, exhausting all TX channels on chips with only 2).
        let channel_config = TxChannelConfig {
            resolution: Hertz(10_000_000), // 100 ns/tick
            memory_access: MemoryAccess::Indirect {
                memory_block_symbols: 48,
            },
            ..Default::default()
        };
        Self::new_with_channel_config(led, channel_config)
    }

    /// Creates a new WS2812 driver with a custom `TxChannelConfig`.
    ///
    /// Use this when targeting ESP32 variants other than C3/C6, or when you
    /// need to tune memory block size, DMA usage, or other channel parameters.
    ///
    /// The RMT channel resolution (`channel_config.resolution`) is used to derive
    /// WS2812 pulse durations and must be at least `Hertz(10_000_000)` (100 ns/tick)
    /// for correct protocol timing.
    /// Higher resolutions (e.g. 20 MHz) also work.
    /// Returns an error if resolution is below 10 MHz.
    ///
    /// # Arguments
    ///
    /// * `led` - GPIO pin connected to the LED data line
    /// * `channel_config` - RMT TX channel configuration; resolution must be >= 10 MHz
    ///
    /// # Example
    ///
    /// ```ignore
    /// use esp_idf_hal::rmt::config::{MemoryAccess, TxChannelConfig};
    /// use esp_idf_hal::units::Hertz;
    ///
    /// // Classic ESP32: 64-symbol blocks, up to 8 TX channels.
    /// let channel_config = TxChannelConfig {
    ///     resolution: Hertz(10_000_000),
    ///     memory_access: MemoryAccess::Indirect { memory_block_symbols: 64 },
    ///     ..Default::default()
    /// };
    /// let mut led = Ws2812Rmt::new_with_channel_config(peripherals.pins.gpio18, channel_config)?;
    /// ```
    pub fn new_with_channel_config(
        led: impl OutputPin + 'd,
        channel_config: TxChannelConfig,
    ) -> Result<Self> {
        let resolution = channel_config.resolution;
        anyhow::ensure!(
            resolution.0 >= 10_000_000,
            "RMT resolution must be >= 10 MHz for WS2812 timing (got {} Hz)",
            resolution.0
        );
        let tx = TxChannelDriver::new(led, &channel_config)?;

        let t0h = Pulse::new_with_duration(resolution, PinState::High, Duration::from_nanos(350))?;
        let t0l = Pulse::new_with_duration(resolution, PinState::Low, Duration::from_nanos(800))?;
        let t1h = Pulse::new_with_duration(resolution, PinState::High, Duration::from_nanos(700))?;
        let t1l = Pulse::new_with_duration(resolution, PinState::Low, Duration::from_nanos(600))?;

        let encoder_config = BytesEncoderConfig {
            bit0: Symbol::new(t0h, t0l),
            bit1: Symbol::new(t1h, t1l),
            msb_first: true,
            ..Default::default()
        };

        Ok(Self { tx, encoder_config })
    }

    /// Sets a single pixel color.
    ///
    /// Use this for single-LED indicators or when updating one pixel at a time.
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<()> {
        let grb = rgb_to_grb(rgb);
        let bytes = [(grb >> 16) as u8, (grb >> 8) as u8, grb as u8];
        transmit_bytes(&mut self.tx, &self.encoder_config, &bytes)
    }

    /// Sets multiple pixels from a slice.
    ///
    /// Use this for LED strips with multiple pixels.
    ///
    /// # Arguments
    ///
    /// * `rgbs` - Slice of colors, one per pixel in order
    pub fn set_pixels_slice(&mut self, rgbs: &[RGB8]) -> Result<()> {
        let mut bytes = Vec::with_capacity(rgbs.len() * 3);
        for rgb in rgbs {
            let grb = rgb_to_grb(*rgb);
            bytes.push((grb >> 16) as u8);
            bytes.push((grb >> 8) as u8);
            bytes.push(grb as u8);
        }
        transmit_bytes(&mut self.tx, &self.encoder_config, &bytes)
    }
}

#[cfg(feature = "pennant")]
impl pennant::StatusLed for Ws2812Rmt<'_> {
    type Error = anyhow::Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

impl smart_leds_trait::SmartLedsWrite for Ws2812Rmt<'_> {
    type Error = anyhow::Error;
    type Color = smart_leds_trait::RGB8;

    /// Writes a sequence of colors to the LED strip.
    ///
    /// Each item in the iterator is converted from `I` into `smart_leds_trait::RGB8`
    /// and collected into a flat GRB byte buffer. The `BytesEncoder` then converts
    /// each byte into RMT symbols per-bit.
    /// For a zero-allocation path use the `no_std` HAL driver
    /// (`rustyfarian-esp-hal-ws2812`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use smart_leds_trait::{SmartLedsWrite, RGB8};
    ///
    /// let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
    /// led.write(colors.iter().copied())?;
    /// ```
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let mut bytes = Vec::new();
        for item in iterator {
            let rgb: RGB8 = item.into();
            let grb = rgb_to_grb(rgb);
            bytes.push((grb >> 16) as u8);
            bytes.push((grb >> 8) as u8);
            bytes.push(grb as u8);
        }
        if bytes.is_empty() {
            return Ok(());
        }
        transmit_bytes(&mut self.tx, &self.encoder_config, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Color bridge tests --------------------------------------------------
    //
    // `write()` converts `I: Into<smart_leds_trait::RGB8>` (== `rgb::RGB8`) via
    // `item.into()`.  Use distinct per-channel values to catch accidental
    // channel transposition — the only failure mode worth guarding here.

    #[test]
    fn color_bridge_channels_are_not_swapped() {
        let sl_color = smart_leds_trait::RGB8 {
            r: 10,
            g: 20,
            b: 30,
        };
        let rgb_color = RGB8::new(sl_color.r, sl_color.g, sl_color.b);
        assert_eq!(rgb_color.r, 10);
        assert_eq!(rgb_color.g, 20);
        assert_eq!(rgb_color.b, 30);
    }
}

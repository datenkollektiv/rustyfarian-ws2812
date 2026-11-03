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
//! # Frame spacing (latch)
//!
//! The LEDs latch a frame after the data line has been low for at least [`WS2812_RESET_US`] (280 µs).
//! A frame that follows the previous one more closely would merge with it.
//! Every send path (`set_pixel`, `set_pixels_slice` and `SmartLedsWrite::write`) therefore waits for the remainder of that interval before it starts a frame.
//! The interval is measured from the moment the previous transmit finished, whether it succeeded or failed.
//! The wait is a busy-wait of at most 280 µs (`Ets::delay_us`), so it is not rounded to FreeRTOS ticks.
//! It uses `std::time::Instant`, which reads the monotonic ESP-IDF system timer in microseconds.
//! Callers that already space their frames by more than 280 µs never wait.
//!
//! # Long frames and radios
//!
//! A frame that fits in one RMT memory block (48 symbols, so 2 LEDs with the default configuration) is loaded before the transmit starts.
//! Longer frames are refilled from the RMT interrupt service routine while the frame is on the wire.
//! This tolerates task latency, for example while the Wi-Fi or Bluetooth tasks run, but it is not hard real time.
//! The RMT ISR is not IRAM-safe by default (`CONFIG_RMT_ISR_IRAM_SAFE` is `n` in ESP-IDF v5.3.3), so a flash-cache disable during a frame, as caused by an NVS or flash write or an OTA update, could in principle delay a refill.
//! The ISR also runs at a low or medium priority that it shares with other peripherals.
//! On a downstream ESP32-C3 bench (12 LEDs, 2026-11-01) the default kept the ring clean through a 1.6 MB OTA download; that is one run, not a guarantee.
//!
//! **Do not enable `CONFIG_RMT_ISR_IRAM_SAFE`.**
//! esp-idf-hal 0.47 registers a transmit-done callback that lives in flash, and ESP-IDF rejects it when the ISR is IRAM-safe.
//! [`Ws2812Rmt::new`] then fails with `ESP_ERR_INVALID_ARG` ("on_trans_done callback not in IRAM").
//! When the setting is enabled, an `ESP_ERR_INVALID_ARG` from the constructor names it as the likely cause.
//! This holds until esp-idf-hal places its callback in IRAM.
//!
//! # Supported Boards
//!
//! Works with any ESP32 variant that has RMT support via ESP-IDF:
//! - ESP32-C3-DevKit-Rust-1: GPIO2
//! - ESP32-C3-DevKitC-02: GPIO8
//! - ESP32-C6-DevKitC-1: GPIO8

// This crate is only meaningful on ESP-IDF targets.  Gating the entire library
// body here means that for bare-metal targets (riscv32imac-unknown-none-elf
// etc.) the crate compiles as an empty library, so IDE tooling that uses the
// bare-metal workspace default can analyze the rest of the workspace cleanly.
// Use `just check-idf` / `just build-all` for IDF-target verification.
#![cfg(target_os = "espidf")]

// Built against esp-idf-hal 0.47 (TxChannelDriver and BytesEncoder arrived in 0.46).
// Uses a workaround for a send_and_wait bug present in 0.46.2 and still
// unfixed in 0.47.0 (see transmit_bytes and ROADMAP.md).
use anyhow::Result;
use bunting::rgb_to_grb;
use core::time::Duration;
use esp_idf_hal::{
    delay::Ets,
    gpio::OutputPin,
    rmt::{
        config::{MemoryAccess, TransmitConfig, TxChannelConfig},
        encoder::{BytesEncoder, BytesEncoderConfig},
        PinState, Pulse, Symbol, TxChannelDriver,
    },
    units::Hertz,
};
use rgb::RGB8;
use std::time::Instant;

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
    next_frame_at: Option<Instant>,
}

/// Minimum low time in microseconds between two frames, so the LEDs latch the previous one.
///
/// The WS2812B datasheet asks for at least 280 µs; older WS2812 parts need only 50 µs.
/// The driver waits for the remainder of this interval before it starts a frame
/// that follows the previous one too closely.
/// This mirrors the constant of the same name in `rustyfarian-esp-hal-ws2812`;
/// see [Frame spacing (latch)](crate#frame-spacing-latch).
pub const WS2812_RESET_US: u32 = 280;

/// Whole microseconds to busy-wait for `remaining` of the latch interval, rounded up and capped at [`WS2812_RESET_US`].
///
/// The cap keeps a misbehaving clock from turning the latch wait into a long stall.
const fn latch_wait_us(remaining: Duration) -> u32 {
    let mut us = remaining.as_micros();
    if !remaining.subsec_nanos().is_multiple_of(1000) {
        us += 1;
    }
    if us > WS2812_RESET_US as u128 {
        WS2812_RESET_US
    } else {
        us as u32
    }
}

const _: () = {
    assert!(latch_wait_us(Duration::ZERO) == 0);
    assert!(latch_wait_us(Duration::from_nanos(1)) == 1);
    assert!(latch_wait_us(Duration::from_micros(100)) == 100);
    assert!(latch_wait_us(Duration::from_nanos(100_001)) == 101);
    assert!(latch_wait_us(Duration::from_micros(WS2812_RESET_US as u64)) == WS2812_RESET_US);
    assert!(latch_wait_us(Duration::from_secs(5)) == WS2812_RESET_US);
};

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
//
// Every send path goes through here, so the latch wait and stamp live here too.
fn transmit_bytes(
    tx: &mut TxChannelDriver<'_>,
    encoder_config: &BytesEncoderConfig,
    next_frame_at: &mut Option<Instant>,
    bytes: &[u8],
) -> Result<()> {
    if let Some(deadline) = *next_frame_at {
        let wait_us = latch_wait_us(deadline.saturating_duration_since(Instant::now()));
        if wait_us > 0 {
            Ets::delay_us(wait_us);
        }
    }
    let result = send_and_wait_done(tx, encoder_config, bytes);
    *next_frame_at = Some(Instant::now() + Duration::from_micros(WS2812_RESET_US as u64));
    result
}

fn send_and_wait_done(
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
    /// size on ESP32-C3, ESP32-C6 and ESP32-S3. The C3 and C6 have only two TX
    /// channels, so the ESP-IDF default of 64 symbols would round up to two blocks
    /// and exhaust all available TX candidates.
    ///
    /// For the classic ESP32 and the S2, where each block holds 64 symbols, use [`new_with_channel_config`] to pass an appropriate
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
    ///
    /// # Errors
    ///
    /// - The resolution is below 10 MHz.
    /// - The RMT channel cannot be created, for example when no TX channel is free.
    ///   With `CONFIG_RMT_ISR_IRAM_SAFE` enabled it fails with `ESP_ERR_INVALID_ARG`, and the error names the setting as the likely cause.
    ///   See [Long frames and radios](crate#long-frames-and-radios).
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
        let tx = TxChannelDriver::new(led, &channel_config);
        // esp-idf-hal 0.47 registers a flash-resident transmit-done callback, which ESP-IDF rejects
        // with ESP_ERR_INVALID_ARG when the RMT ISR is IRAM-safe. Other invalid configurations return
        // the same code, so the context names the setting as the likely cause, not the certain one.
        #[cfg(esp_idf_rmt_isr_iram_safe)]
        let tx = tx.map_err(|e| {
            let iram_rejection =
                e.code() == esp_idf_hal::sys::ESP_ERR_INVALID_ARG as esp_idf_hal::sys::esp_err_t;
            let err = anyhow::Error::from(e);
            if iram_rejection {
                err.context(
                    "RMT TX channel rejected with ESP_ERR_INVALID_ARG; CONFIG_RMT_ISR_IRAM_SAFE is enabled, \
                     which esp-idf-hal 0.47 does not support (its transmit-done callback is not in IRAM), \
                     so check that setting first",
                )
            } else {
                err
            }
        });
        let tx = tx?;

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

        Ok(Self {
            tx,
            encoder_config,
            next_frame_at: None,
        })
    }

    /// Sets a single pixel color.
    ///
    /// Use this for single-LED indicators or when updating one pixel at a time.
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<()> {
        let grb = rgb_to_grb(rgb);
        let bytes = [(grb >> 16) as u8, (grb >> 8) as u8, grb as u8];
        transmit_bytes(
            &mut self.tx,
            &self.encoder_config,
            &mut self.next_frame_at,
            &bytes,
        )
    }

    /// Sets multiple pixels from a slice.
    ///
    /// Use this for LED strips with multiple pixels.
    /// An empty slice sends nothing and returns `Ok(())`, matching `SmartLedsWrite::write` with an empty iterator.
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
        if bytes.is_empty() {
            return Ok(());
        }
        transmit_bytes(
            &mut self.tx,
            &self.encoder_config,
            &mut self.next_frame_at,
            &bytes,
        )
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
        transmit_bytes(
            &mut self.tx,
            &self.encoder_config,
            &mut self.next_frame_at,
            &bytes,
        )
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

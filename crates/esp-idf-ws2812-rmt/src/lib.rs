//! WS2812 (NeoPixel) LED driver using ESP-IDF RMT peripheral.
//!
//! This crate provides a driver for WS2812/NeoPixel addressable LEDs using
//! the ESP-IDF RMT (Remote Control Transceiver) peripheral for precise timing.
//! It works with any ESP32 variant that supports RMT via ESP-IDF.
//!
//! For bare-metal (no_std) projects using `esp-hal`, see `esp-hal-ws2812-rmt`.
//! Pure color utilities are available in the `ws2812-pure` crate for testing.
//!
//! # Example
//!
//! ```ignore
//! use esp_idf_ws2812_rmt::WS2812RMT;
//! use rgb::RGB8;
//!
//! let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
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

use anyhow::Result;
use core::time::Duration;
use esp_idf_hal::{
    gpio::OutputPin,
    peripheral::Peripheral,
    rmt::{
        config::TransmitConfig, FixedLengthSignal, PinState, Pulse, RmtChannel, TxRmtDriver,
        VariableLengthSignal,
    },
};
use rgb::RGB8;
use ws2812_pure::rgb_to_grb;

/// WS2812 LED driver using RMT peripheral.
///
/// The RMT peripheral provides precise timing control needed for the
/// WS2812 protocol without CPU intervention.
pub struct WS2812RMT<'a> {
    tx_rtm_driver: TxRmtDriver<'a>,
}

impl<'d> WS2812RMT<'d> {
    /// Creates a new WS2812 driver.
    ///
    /// # Arguments
    ///
    /// * `led` - GPIO pin connected to the LED data line
    /// * `channel` - RMT channel to use for transmission
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut led = WS2812RMT::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    /// ```
    pub fn new(
        led: impl Peripheral<P = impl OutputPin> + 'd,
        channel: impl Peripheral<P = impl RmtChannel> + 'd,
    ) -> Result<Self> {
        let config = TransmitConfig::new().clock_divider(2);
        let tx = TxRmtDriver::new(channel, led, &config)?;
        Ok(Self { tx_rtm_driver: tx })
    }

    /// Creates the WS2812 timing pulses for 0 and 1 bits.
    fn create_pulses(&mut self) -> Result<(Pulse, Pulse, Pulse, Pulse)> {
        let ticks_hz = self.tx_rtm_driver.counter_clock()?;
        let t0h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(350))?;
        let t0l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(800))?;
        let t1h = Pulse::new_with_duration(ticks_hz, PinState::High, &ns(700))?;
        let t1l = Pulse::new_with_duration(ticks_hz, PinState::Low, &ns(600))?;
        Ok((t0h, t0l, t1h, t1l))
    }

    /// Sets a single pixel color.
    ///
    /// Use this for single-LED indicators or when updating one pixel at a time.
    pub fn set_pixel(&mut self, rgb: RGB8) -> Result<()> {
        let color = rgb_to_grb(rgb);
        let (t0h, t0l, t1h, t1l) = self.create_pulses()?;
        let mut signal = FixedLengthSignal::<24>::new();
        Self::encode_color_bits(color, &mut signal, 0, t0h, t0l, t1h, t1l)?;
        self.tx_rtm_driver.start_blocking(&signal)?;
        Ok(())
    }

    /// Encodes a 24-bit color value into RMT pulses (MSB first).
    fn encode_color_bits(
        color: u32,
        signal: &mut FixedLengthSignal<24>,
        offset: usize,
        t0h: Pulse,
        t0l: Pulse,
        t1h: Pulse,
        t1l: Pulse,
    ) -> Result<()> {
        for i in (0..24).rev() {
            let bit = (color >> i) & 1 != 0;
            let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            signal.set(offset + (23 - i as usize), &(high_pulse, low_pulse))?;
        }
        Ok(())
    }

    /// Sets multiple pixels from a slice.
    ///
    /// Use this for LED strips with multiple pixels.
    ///
    /// # Arguments
    ///
    /// * `rgbs` - Slice of colors, one per pixel in order
    pub fn set_pixels_slice(&mut self, rgbs: &[RGB8]) -> Result<()> {
        let (t0h, t0l, t1h, t1l) = self.create_pulses()?;
        let mut signal = VariableLengthSignal::new();
        for rgb in rgbs {
            let pulses = Self::color_to_pulses(*rgb, t0h, t0l, t1h, t1l);
            signal.push(&pulses)?;
        }
        self.tx_rtm_driver.start_blocking(&signal)?;
        Ok(())
    }

    /// Converts a color to individual pulses (no allocation, returns an array).
    fn color_to_pulses(rgb: RGB8, t0h: Pulse, t0l: Pulse, t1h: Pulse, t1l: Pulse) -> [Pulse; 48] {
        let color = rgb_to_grb(rgb);
        let mut pulses = [t0h; 48]; // Initialize with dummy values
        for i in (0..24).rev() {
            let bit = (color >> i) & 1 != 0;
            let (high, low) = if bit { (t1h, t1l) } else { (t0h, t0l) };
            let idx = (23 - i) * 2;
            pulses[idx] = high;
            pulses[idx + 1] = low;
        }
        pulses
    }
}

fn ns(nanos: u64) -> Duration {
    Duration::from_nanos(nanos)
}

#[cfg(feature = "led-effects")]
impl led_effects::StatusLed for WS2812RMT<'_> {
    type Error = anyhow::Error;

    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error> {
        self.set_pixel(color)
    }
}

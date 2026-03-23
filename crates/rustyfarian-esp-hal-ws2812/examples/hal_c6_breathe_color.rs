//! ESP32-C6 Breathe Color Example
//!
//! Runs a [`BreatheEffect`] on a 12-LED WS2812B ring, cycling through the
//! full hue wheel each frame using [`BreatheEffect::set_color`].
//!
//! This demonstrates that [`set_color`] updates the hue *without resetting
//! the breathing phase* — the brightness envelope continues smoothly while
//! the color shifts gradually around the spectrum.
//!
//! ## Comparison with [`hal_c6_pulse`](hal_c6_pulse)
//!
//! [`PulseEffect`] uses a half-wave sine: brightness rises to peak, falls to
//! zero, and pauses at zero for roughly a quarter of the cycle.
//! [`BreatheEffect`] uses a full symmetric sine with no pause at the floor —
//! the brightness oscillates continuously in and out.
//!
//! ## Components
//!
//! - ESP32-C6 development board
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C6             WS2812B ring
//! ─────────────        ────────────
//! GPIO18 ──[330 Ω]──► DIN
//! GND    ──────────► GND
//! 3V3    ──────────► VCC
//! ```
//!
//! **Power note:** 3.3 V logic is sufficient for a small ring at low brightness.
//! For full brightness or longer strips, use 5 V VCC and a 3.3 V→5 V level shifter
//! (e.g., 74AHCT125) on the data line.
//!
//! ## Build
//!
//! ```sh
//! just build-example hal-ws2812 hal_c6_breathe_color
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_breathe_color
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use esp_hal::{
    delay::Delay,
    gpio::Level,
    main,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
};
use esp_println::println;
use ferriswheel::{hsv_to_rgb, BreatheEffect};
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let config = TxChannelConfig::default()
        .with_clk_divider(RMT_CLK_DIV)
        .with_idle_output_level(Level::Low)
        .with_idle_output(true)
        .with_carrier_modulation(false);
    let channel = rmt
        .channel0
        .configure_tx(peripherals.GPIO18, config)
        .unwrap();

    let mut ws = Ws2812Rmt::<_, N>::new(channel);
    let mut effect = BreatheEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(hsv_to_rgb(0, 255, 255))
        .with_max_brightness(180)
        .with_speed(2)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();
    // Tracks the current hue (0–255 maps the full color wheel).
    // Increments by 1 each frame (~50 ms), completing one rotation in ~12.8 s.
    let mut hue: u8 = 0;

    loop {
        // unwrap() is appropriate in examples: any failure panics into the
        // printing panic handler above, so the cause appears on JTAG serial.
        effect.update(&mut colors).unwrap();
        ws.set_pixels_slice(&colors).unwrap();
        delay.delay_millis(50u32);

        // Advance hue and update color without resetting the breathing phase.
        hue = hue.wrapping_add(1);
        effect.set_color(hsv_to_rgb(hue, 255, 255));
    }
}

//! ESP32-C6 Multi-Effect Sequencer (ESP-IDF)
//!
//! Cycles through three animations stored as `Vec<Box<dyn Effect>>`, spending
//! roughly five seconds on each before advancing to the next.
//!
//! Sequence:
//!   1. [`RainbowEffect`]  — rotating hue wheel, clockwise
//!   2. [`BreatheEffect`]  — smooth green breathing, full symmetric sine
//!   3. [`PulseEffect`]    — blue heartbeat, half-wave with floor pause
//!
//! The BreatheEffect → PulseEffect transition makes the floor difference
//! visible on hardware: `BreatheEffect` has no pause at black; `PulseEffect`
//! pauses for roughly a quarter of its cycle.
//!
//! This example shows why the [`Effect`] trait exists as a runtime dispatch
//! boundary: the main loop and driver code are identical regardless of which
//! animation is active.
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
//! ESP32-C6            WS2812B ring
//! ────────────        ────────────
//! GPIO18 ──[330 Ω]──► DIN
//! GND   ──────────► GND
//! 3V3   ──────────► VCC
//! ```
//!
//! **Power note:** 3.3 V logic is sufficient for a small ring at low brightness.
//! For full brightness or longer strips, use 5 V VCC and a 3.3 V→5 V level shifter
//! (e.g., 74AHCT125) on the data line.
//!
//! ## Build
//!
//! ```sh
//! just build-example idf-ws2812 idf_c6_effects
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash idf_c6_effects
//! ```

use ferriswheel::{BreatheEffect, Direction, Effect, PulseEffect, RainbowEffect};
use rgb::RGB8;
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;

    const NUM_LEDS: usize = 12;
    let mut ws = WS2812RMT::new(peripherals.pins.gpio18, peripherals.rmt.channel0)?;
    let mut colors = [RGB8::default(); NUM_LEDS];

    let mut effects: Vec<Box<dyn Effect>> = vec![
        Box::new(
            RainbowEffect::new(NUM_LEDS)
                .unwrap()
                .with_brightness(64)
                .with_speed(3)
                .unwrap()
                .with_direction(Direction::Clockwise),
        ),
        Box::new(
            BreatheEffect::new(NUM_LEDS)
                .unwrap()
                .with_color(RGB8::new(0, 200, 80))
                .with_max_brightness(180)
                .with_speed(3)
                .unwrap(),
        ),
        Box::new(
            PulseEffect::new(NUM_LEDS)
                .unwrap()
                .with_color(RGB8::new(0, 0, 255))
                .with_max_brightness(64)
                .with_speed(5)
                .unwrap(),
        ),
    ];

    // Each effect runs for ~5 s (100 frames × 50 ms).
    const FRAMES_PER_EFFECT: usize = 100;
    let mut idx = 0;

    loop {
        for _ in 0..FRAMES_PER_EFFECT {
            // EffectError does not implement std::error::Error, so ? is unavailable;
            // unwrap() panics on failure, which ESP-IDF logs to the serial console.
            effects[idx].update(&mut colors).unwrap();
            ws.set_pixels_slice(&colors)?;
            thread::sleep(Duration::from_millis(50));
        }
        // Advance to the next effect and reset it so it always starts from phase 0.
        idx = (idx + 1) % effects.len();
        effects[idx].reset();
    }
}

//! ESP32-C6 Onboard LED Pulse Example — GPIO8 regression guard
//!
//! Runs a [`PulseEffect`] (blue) on the **single onboard SK68XXMINI RGB LED**
//! wired to GPIO8 on the ESP32-C6-DevKitC-1. No external hardware required.
//!
//! ## Why this example exists
//!
//! GPIO8 has a documented failure history. Under `esp-hal 1.0.0` the blocking
//! [`Ws2812Rmt::set_pixels_slice`] call hung forever inside `txn.wait()` on the
//! first transmit when the pin was GPIO8 — but only on that pin; GPIO18 was
//! unaffected. The hang was resolved by the `esp-hal 1.0.0 → 1.1.0` upgrade
//! through RMT-internal changes upstream, and **no workaround exists in this
//! driver**. Nothing in our code would stop the bug returning.
//!
//! Every other `hal_c6_*` example targets GPIO18, so without this example the
//! GPIO8 path has no standing coverage and each retest needs a hand-edited pin.
//! Keep this example so the regression check is a single command.
//!
//! See `docs/project-lore.md` and
//! `docs/features/esp-hal-stack-upgrade-august-2026-v1.md`.
//!
//! ## Components
//!
//! - ESP32-C6-DevKitC-1 (or any C6 board with an addressable LED on GPIO8)
//! - Nothing else — do **not** attach an external ring
//!
//! ## Expected behaviour
//!
//! The onboard LED pulses blue, smoothly, indefinitely. Anything else is a
//! failure worth investigating:
//!
//! - **Hangs immediately, LED dark** — the GPIO8 transmit regression is back.
//!   The first `set_pixels_slice` never returns. Do not ship the esp-hal bump.
//! - **Pulses green or red instead of blue** — colour-channel order regression.
//! - **Flickers or stutters** — RMT timing regression.
//!
//! ## Build
//!
//! ```sh
//! just build-example hal-ws2812 hal_c6_onboard_pulse
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_onboard_pulse
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
use ferriswheel::PulseEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

/// The onboard SK68XXMINI is a single addressable LED.
const NUM_LEDS: usize = 1;
const N: usize = buffer_size(NUM_LEDS);

/// Blue — deliberately a pure single channel. A red or green pulse instead of blue
/// indicates a colour-order (GRB/RGB) regression rather than a timing one.
const PULSE_COLOR: RGB8 = RGB8::new(0, 0, 255);
/// Kept low: the onboard LED sits directly beside the USB connector and is bright.
const MAX_BRIGHTNESS: u8 = 64;
/// Slow enough that a human can see a stutter or a dropped frame.
const PULSE_SPEED: u8 = 3;
/// ~20 fps — matches the sibling `hal_c6_pulse` example so the two are comparable by eye.
const FRAME_DELAY_MS: u32 = 50;

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
        .configure_tx(&config)
        .unwrap()
        .with_pin(peripherals.GPIO8);

    let mut ws = Ws2812Rmt::<_, N>::new(channel);
    let mut effect = PulseEffect::new(NUM_LEDS)
        .unwrap()
        .with_color(PULSE_COLOR)
        .with_max_brightness(MAX_BRIGHTNESS)
        .with_speed(PULSE_SPEED)
        .unwrap();
    let mut colors = [RGB8::default(); NUM_LEDS];
    let delay = Delay::new();

    // Printed before the first transmit: if the GPIO8 hang regresses, this line
    // is the last output seen and pins the fault to set_pixels_slice.
    println!("hal_c6_onboard_pulse: starting RMT transmit loop on GPIO8");

    loop {
        // Fail loudly. This example is a regression guard, so a dark LED must never be
        // mistakable for success — `.ok()` here would turn a real transmit failure into a
        // silent no-op indistinguishable from a hang. The printing panic handler above
        // puts the cause on the serial monitor.
        //
        // Note this deliberately differs from the sibling demo examples, which use `.ok()`:
        // ignoring a dropped frame is reasonable in a demo, not in a guard.
        //
        // Panicking (rather than logging and continuing) also avoids a 20 Hz error spam:
        // once a blocking transmit fails, the RMT channel is consumed and every subsequent
        // call returns `Error::Transmit` forever. See `Ws2812Rmt` docs.
        effect
            .update(&mut colors)
            .expect("PulseEffect::update failed — effect state is invalid");
        ws.set_pixels_slice(&colors)
            .expect("set_pixels_slice failed — RMT transmit regression on GPIO8");
        delay.delay_millis(FRAME_DELAY_MS);
    }
}

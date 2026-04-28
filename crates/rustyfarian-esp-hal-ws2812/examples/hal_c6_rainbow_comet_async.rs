//! ESP32-C6 Rainbow Comet Example — Async (esp-hal + Embassy, bare-metal)
//!
//! Runs a [`RainbowCometEffect`] on a 12-LED WS2812B ring using the async RMT driver.
//! The animation loop uses [`embassy_time::Timer`] to yield to the executor between
//! frames, allowing other Embassy tasks to run during the 30 ms inter-frame delay.
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
//! just build-example hal-ws2812 hal_c6_rainbow_comet_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_rainbow_comet_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    gpio::Level,
    interrupt::software::SoftwareInterruptControl,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_println::println;
use ferriswheel::RainbowCometEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialise the RTOS scheduler so that embassy-time's Timer works.
    // `esp_rtos::start()` sets up the hardware timer that drives the embassy-time
    // clock; it must be called before the first `.await` that uses a timer.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_ints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();
    let config = TxChannelConfig::default()
        .with_clk_divider(RMT_CLK_DIV)
        .with_idle_output_level(Level::Low)
        .with_idle_output(true)
        .with_carrier_modulation(false);
    let channel = rmt
        .channel0
        .configure_tx(&config)
        .unwrap()
        .with_pin(peripherals.GPIO18);

    println!("RTOS started, RMT configured");

    let mut ws = Ws2812Rmt::<_, N>::new(channel);
    let mut effect = RainbowCometEffect::new(NUM_LEDS)
        .unwrap()
        .with_hue(0)
        .with_hue_step(16);
    let mut colors = [RGB8::default(); NUM_LEDS];

    println!("Entering animation loop");
    let mut frame: u32 = 0;
    // Animation loop.
    //
    // Each iteration:
    //   1. `set_pixels_slice().await` — yields to the executor while the RMT peripheral
    //      transmits pulses (~360 µs for 12 LEDs). Other tasks run during this window.
    //   2. `Timer::after_millis(30).await` — yields for ~30 ms. Other tasks run here too.
    //
    // Frame timing note: `Timer::after_millis` specifies the *minimum* delay — if the
    // executor is busy (e.g., processing a Wi-Fi event) when the timer fires, the actual
    // inter-frame gap will be slightly longer. For LED animations this is imperceptible,
    // but high-precision timing applications should account for the jitter or use a
    // fixed-period ticker instead of a one-shot timer.
    loop {
        if frame < 5 || frame % 100 == 0 {
            println!("frame {}", frame);
        }
        effect.update(&mut colors).unwrap();
        if frame == 0 {
            println!("effect.update OK, calling set_pixels_slice");
        }
        ws.set_pixels_slice(&colors).await.unwrap();
        if frame == 0 {
            println!("set_pixels_slice OK, calling Timer::after_millis");
        }
        Timer::after_millis(30).await;
        if frame == 0 {
            println!("Timer OK, loop continues");
        }
        frame = frame.wrapping_add(1);
    }
}

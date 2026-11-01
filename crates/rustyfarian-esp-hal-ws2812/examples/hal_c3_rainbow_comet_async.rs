//! ESP32-C3 Rainbow Comet Example — Async (esp-hal + Embassy, bare-metal)
//!
//! Runs a [`RainbowCometEffect`] continuously on a 12-LED WS2812B ring through the async RMT driver,
//! and logs transmit statistics once per second over serial.
//!
//! This is the workload of a typical async LED clock: every 30 ms the whole ring is re-sent.
//! A 12-LED frame is 289 pulse codes, more than one ESP32-C3 channel holds (48),
//! so every frame takes the critical-section path with interrupts masked for about 0.4 ms.
//! Watch the ring for flicker, wrong colours or a jumping comet, and the log for errors and outliers.
//!
//! ## Serial output (one line per second)
//!
//! ```text
//! comet: 33 frames, 0 errors, send min 405 / mean 410 / max 418 us, loop max 31 ms
//! ```
//!
//! - `errors` counts transmits that returned an error; it must stay 0.
//! - `send` is the duration of `set_pixels_slice`; it should stay close to 0.4 ms.
//! - `loop max` is the longest frame-to-frame interval; about 31 ms is expected.
//!
//! ## Components
//!
//! - ESP32-C3 development board, connected over USB (native USB-Serial-JTAG or a USB-UART bridge)
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C3            WS2812B ring
//! ────────────        ────────────
//! GPIO4 ──[330 Ω]──► DIN
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
//! just build-example hal-ws2812 hal_c3_rainbow_comet_async
//! ```
//!
//! ## Run (flash and monitor)
//!
//! ```sh
//! just run hal_c3_rainbow_comet_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    gpio::Level,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::{Instant, Rate},
    timer::timg::TimerGroup,
};
use esp_println::println;
use ferriswheel::RainbowCometEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);
const FRAME_DELAY_MS: u64 = 30;
const REPORT_INTERVAL_US: u64 = 1_000_000;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

/// Per-second transmit statistics.
struct Stats {
    frames: u32,
    errors: u32,
    send_min_us: u64,
    send_max_us: u64,
    send_total_us: u64,
    loop_max_us: u64,
}

impl Stats {
    const fn new() -> Self {
        Self {
            frames: 0,
            errors: 0,
            send_min_us: u64::MAX,
            send_max_us: 0,
            send_total_us: 0,
            loop_max_us: 0,
        }
    }

    fn record_send(&mut self, send_us: u64, ok: bool) {
        self.frames += 1;
        self.errors += u32::from(!ok);
        self.send_min_us = self.send_min_us.min(send_us);
        self.send_max_us = self.send_max_us.max(send_us);
        self.send_total_us += send_us;
    }

    fn report(&self) {
        let mean = self.send_total_us / u64::from(self.frames.max(1));
        println!(
            "comet: {} frames, {} errors, send min {} / mean {} / max {} us, loop max {} ms",
            self.frames,
            self.errors,
            self.send_min_us,
            mean,
            self.send_max_us,
            self.loop_max_us / 1000
        );
    }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

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
        .with_pin(peripherals.GPIO4);

    let mut ws = Ws2812Rmt::<_, N>::new(channel);
    let mut effect = RainbowCometEffect::new(NUM_LEDS)
        .unwrap()
        .with_hue(0)
        .with_hue_step(16);
    let mut colors = [RGB8::default(); NUM_LEDS];

    println!(
        "hal_c3_rainbow_comet_async: {NUM_LEDS} LEDs on GPIO4, frame every {FRAME_DELAY_MS} ms"
    );

    let mut stats = Stats::new();
    let mut report_start = Instant::now();
    let mut last_loop = Instant::now();

    loop {
        let loop_us = last_loop.elapsed().as_micros();
        last_loop = Instant::now();
        if stats.frames > 0 {
            stats.loop_max_us = stats.loop_max_us.max(loop_us);
        }

        let updated = effect.update(&mut colors);
        let send_start = Instant::now();
        let sent = ws.set_pixels_slice(&colors).await;
        stats.record_send(
            send_start.elapsed().as_micros(),
            updated.is_ok() && sent.is_ok(),
        );
        if let Err(e) = sent {
            println!("comet: transmit error: {e}");
        }

        if report_start.elapsed().as_micros() >= REPORT_INTERVAL_US {
            stats.report();
            stats = Stats::new();
            report_start = Instant::now();
        }

        Timer::after_millis(FRAME_DELAY_MS).await;
    }
}

//! ESP32-C6 Multi-Task Example — Async (esp-hal + Embassy, bare-metal)
//!
//! Demonstrates Embassy multi-task architecture: a render task drives the LED ring
//! while a separate button task cycles through effects.
//! The two tasks communicate via a static [`embassy_sync::signal::Signal`].
//!
//! ## Components
//!
//! - ESP32-C6 development board
//! - WS2812B LED ring, 12 LEDs
//! - 300–500 Ω resistor (data line protection)
//! - Momentary push button on GPIO9 (active low, using internal pull-up)
//!
//! ## Wiring
//!
//! ```text
//! ESP32-C6             WS2812B ring
//! ─────────────        ────────────
//! GPIO18 ──[330 Ω]──► DIN
//! GND    ──────────► GND
//! 3V3    ──────────► VCC
//!
//! GPIO9  ──[button]──► GND   (BOOT button on most C6 dev boards)
//! ```
//!
//! ## Build
//!
//! ```sh
//! just build-example hal-ws2812 hal_c6_multitask_async
//! ```
//!
//! ## Flash
//!
//! ```sh
//! just flash hal_c6_multitask_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Pull},
    interrupt::software::SoftwareInterruptControl,
    rmt::{Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
    timer::timg::TimerGroup,
    Async,
};
use esp_println::println;
use ferriswheel::{BreatheEffect, EffectError, MeteorEffect, RainbowCometEffect, SpinnerEffect};
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);
const NUM_EFFECTS: u8 = 4;
const FRAME_DELAY_MS: u64 = 30;
const DEBOUNCE_MS: u64 = 200;

/// Effect selection signal — written by button task, read by render task.
///
/// Both tasks run on the same `#[esp_rtos::main]` thread-mode executor, so
/// `NoopRawMutex` is sufficient and zero-cost. Switch to `CriticalSectionRawMutex`
/// only if the signal would be written from an ISR or a different executor.
static EFFECT_SIGNAL: Signal<NoopRawMutex, u8> = Signal::new();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

/// Cycles through effects on each button press (GPIO9, active low).
///
/// Debounce strategy: after a falling edge, wait `DEBOUNCE_MS` for contact bounce
/// to settle before accepting the press, then wait for the rising edge (button
/// release) before re-arming. This filters both press-side bounce and avoids
/// auto-repeating on a held button.
#[embassy_executor::task]
async fn button_task(mut button: Input<'static>) {
    let mut current: u8 = 0;
    println!("button_task: ready, press BOOT button to cycle effects");
    loop {
        button.wait_for_falling_edge().await;
        Timer::after_millis(DEBOUNCE_MS).await;
        current = (current + 1) % NUM_EFFECTS;
        println!("button_task: switching to effect {}", current);
        EFFECT_SIGNAL.signal(current);
        button.wait_for_rising_edge().await;
    }
}

/// Renders the active effect to the LED ring at ~30 fps.
#[embassy_executor::task]
async fn render_task(mut ws: Ws2812Rmt<'static, Async, N>) {
    let mut colors = [RGB8::default(); NUM_LEDS];
    let mut effect_id: u8 = 0;

    let mut rainbow = RainbowCometEffect::new(NUM_LEDS).unwrap().with_hue_step(16);
    let mut meteor = MeteorEffect::new(NUM_LEDS).unwrap();
    let mut breathe = BreatheEffect::new(NUM_LEDS).unwrap();
    let mut spinner = SpinnerEffect::new(NUM_LEDS).unwrap();

    println!("render_task: starting with effect 0 (rainbow comet)");

    loop {
        // Check for effect change (non-blocking).
        // `try_take` returns Some and resets the signal, or None if no new value.
        if let Some(new_id) = EFFECT_SIGNAL.try_take() {
            if new_id != effect_id {
                effect_id = new_id;
                reset_effect(
                    effect_id,
                    &mut rainbow,
                    &mut meteor,
                    &mut breathe,
                    &mut spinner,
                );
                println!("render_task: switched to effect {}", effect_id);
            }
        }

        let result = update_effect(
            effect_id,
            &mut colors,
            &mut rainbow,
            &mut meteor,
            &mut breathe,
            &mut spinner,
        );

        if let Err(e) = result {
            println!("render_task: effect error: {:?}", e);
        }

        ws.set_pixels_slice(&colors).await.unwrap();
        Timer::after_millis(FRAME_DELAY_MS).await;
    }
}

fn update_effect(
    id: u8,
    buf: &mut [RGB8],
    rainbow: &mut RainbowCometEffect,
    meteor: &mut MeteorEffect,
    breathe: &mut BreatheEffect,
    spinner: &mut SpinnerEffect,
) -> Result<(), EffectError> {
    match id {
        0 => rainbow.update(buf),
        1 => meteor.update(buf),
        2 => breathe.update(buf),
        3 => spinner.update(buf),
        _ => rainbow.update(buf),
    }
}

fn reset_effect(
    id: u8,
    rainbow: &mut RainbowCometEffect,
    meteor: &mut MeteorEffect,
    breathe: &mut BreatheEffect,
    spinner: &mut SpinnerEffect,
) {
    match id {
        0 => rainbow.reset(),
        1 => meteor.reset(),
        2 => breathe.reset(),
        3 => spinner.reset(),
        _ => rainbow.reset(),
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialise the RTOS scheduler so that embassy-time's Timer works.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_ints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    // Configure RMT for WS2812 in async mode.
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
        .configure_tx(peripherals.GPIO18, config)
        .unwrap();

    // Configure button on GPIO9 (BOOT button on most C6 dev boards).
    let button_config = InputConfig::default().with_pull(Pull::Up);
    let button = Input::new(peripherals.GPIO9, button_config);

    let ws: Ws2812Rmt<'_, Async, N> = Ws2812Rmt::new(channel);

    println!("Peripherals configured, spawning tasks");

    spawner.spawn(button_task(button)).unwrap();
    spawner.spawn(render_task(ws)).unwrap();

    println!("Tasks spawned, main parking");

    // Main task parks — all work happens in spawned tasks.
    loop {
        Timer::after_millis(1000).await;
    }
}

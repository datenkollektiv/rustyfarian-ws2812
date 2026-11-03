//! ESP32-C3 Rainbow Comet Under Load — Async (esp-hal + Embassy, bare-metal)
//!
//! Runs the 12-LED rainbow comet of `hal_c3_rainbow_comet_async` while two stressors
//! stand in for what a radio stack does to firmware, and logs once per second over serial.
//!
//! - **Interrupt load:** a TIMG1 timer interrupt every 100 µs does a little work and records
//!   the gap since its previous run. The largest gap shows how long interrupts were held off,
//!   which is what the 12-LED critical-section frames cost every other interrupt.
//! - **CPU load:** a second Embassy task computes in 3 ms chunks without yielding,
//!   like a network stack or a parser hogging the executor.
//!
//! The example cycles through four modes, 5 s each:
//!
//! | Mode               | Interrupts | Comet | CPU hog | Question it answers                          |
//! |:-------------------|:-----------|:------|:--------|:---------------------------------------------|
//! | `irq`              | on         | off   | off     | Baseline interrupt gap without LED frames    |
//! | `irq+comet`        | on         | on    | off     | Interrupt gap added by 12-LED frames         |
//! | `irq+comet+hog`    | on         | on    | on      | Does the ring stay clean under both loads?   |
//! | `comet+hog`        | off        | on    | on      | Executor contention alone                    |
//!
//! This does not reproduce a real radio: its interrupt priorities, internal critical sections
//! and flash activity are not simulated; radio tests belong to downstream firmware.
//!
//! ## Serial output (one line per second, shape only)
//!
//! ```text
//! irq+comet: 33 frames, 0 errors, send max 430 us, loop max 30 ms | irq 9990 ticks, max gap 520 us | hog 0 chunks
//! ```
//!
//! ## Components and wiring
//!
//! Same as `hal_c3_rainbow_comet_async`: an ESP32-C3 board and a 12-LED WS2812B ring on GPIO4
//! through a 300–500 Ω resistor.
//!
//! ## Run (flash and monitor)
//!
//! ```sh
//! just run hal_c3_comet_under_load_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use core::{
    cell::{Cell, RefCell},
    hint::black_box,
    sync::atomic::{AtomicBool, Ordering},
};

use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    rmt::Rmt,
    time::{Duration, Instant, Rate},
    timer::{timg::TimerGroup, PeriodicTimer},
    Blocking,
};
use esp_println::println;
use ferriswheel::RainbowCometEffect;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Ws2812Rmt};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);
const FRAME_DELAY_MS: u64 = 30;
const TICK_PERIOD_US: u64 = 100;
const HOG_CHUNK_US: u64 = 3_000;
const MODE_SECONDS: u32 = 5;

/// One load configuration of the test cycle.
struct Mode {
    name: &'static str,
    irq: bool,
    comet: bool,
    hog: bool,
    evict: bool,
}

const MODES: [Mode; 6] = [
    Mode {
        name: "irq",
        irq: true,
        comet: false,
        hog: false,
        evict: false,
    },
    Mode {
        name: "irq+comet",
        irq: true,
        comet: true,
        hog: false,
        evict: false,
    },
    Mode {
        name: "irq+comet+hog",
        irq: true,
        comet: true,
        hog: true,
        evict: false,
    },
    Mode {
        name: "comet+hog",
        irq: false,
        comet: true,
        hog: true,
        evict: false,
    },
    Mode {
        name: "comet",
        irq: false,
        comet: true,
        hog: false,
        evict: false,
    },
    Mode {
        name: "comet+evict",
        irq: false,
        comet: true,
        hog: false,
        evict: true,
    },
];

/// Flash-resident data swept before each frame in `evict` modes, twice the size of the C3 cache.
static EVICT_DATA: [u32; EVICT_WORDS] = evict_data();
const EVICT_WORDS: usize = 8192;
/// Words per 32-byte cache line.
const EVICT_STRIDE: usize = 8;

const fn evict_data() -> [u32; EVICT_WORDS] {
    let mut data = [0u32; EVICT_WORDS];
    let mut i = 0;
    while i < EVICT_WORDS {
        data[i] = (i as u32).wrapping_mul(2_654_435_761) | 1;
        i += 1;
    }
    data
}

/// Reads one word per cache line of `EVICT_DATA`, so the next frame starts with a cold cache,
/// as it would after a radio stack has run between frames.
fn evict_cache() -> u32 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < EVICT_WORDS {
        sum = sum.wrapping_add(unsafe { core::ptr::read_volatile(&EVICT_DATA[i]) });
        i += EVICT_STRIDE;
    }
    black_box(sum)
}

/// Interrupt-side statistics, updated by the timer interrupt.
#[derive(Clone, Copy)]
struct IrqStats {
    ticks: u32,
    max_gap_us: u64,
    last_us: Option<u64>,
}

impl IrqStats {
    const fn new() -> Self {
        Self {
            ticks: 0,
            max_gap_us: 0,
            last_us: None,
        }
    }
}

static TICK_TIMER: Mutex<RefCell<Option<PeriodicTimer<'static, Blocking>>>> =
    Mutex::new(RefCell::new(None));
static IRQ_STATS: Mutex<RefCell<IrqStats>> = Mutex::new(RefCell::new(IrqStats::new()));
static HOG_CHUNKS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static IRQ_ON: AtomicBool = AtomicBool::new(false);
static HOG_ON: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

/// Timer interrupt: records the gap since the previous run and does a little work.
#[esp_hal::handler]
fn on_tick() {
    let now_us = Instant::now().duration_since_epoch().as_micros();
    critical_section::with(|cs| {
        if let Some(timer) = TICK_TIMER.borrow_ref_mut(cs).as_mut() {
            timer.clear_interrupt();
        }
        let mut stats = IRQ_STATS.borrow_ref_mut(cs);
        if !IRQ_ON.load(Ordering::Relaxed) {
            stats.last_us = None;
            return;
        }
        if let Some(last_us) = stats.last_us {
            stats.max_gap_us = stats.max_gap_us.max(now_us.saturating_sub(last_us));
        }
        stats.last_us = Some(now_us);
        stats.ticks += 1;
    });
    let mut x: u32 = now_us as u32;
    for _ in 0..64 {
        x = black_box(x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223));
    }
}

/// CPU hog: computes for `HOG_CHUNK_US` without yielding, then yields briefly.
#[embassy_executor::task]
async fn hog_task() {
    loop {
        if HOG_ON.load(Ordering::Relaxed) {
            let start = Instant::now();
            let mut x: u32 = 1;
            while start.elapsed().as_micros() < HOG_CHUNK_US {
                x = black_box(x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223));
            }
            critical_section::with(|cs| {
                let chunks = HOG_CHUNKS.borrow(cs);
                chunks.set(chunks.get() + 1);
            });
        }
        Timer::after_micros(100).await;
    }
}

/// Per-second statistics of the comet loop.
struct FrameStats {
    frames: u32,
    errors: u32,
    send_max_us: u64,
    loop_max_us: u64,
}

impl FrameStats {
    const fn new() -> Self {
        Self {
            frames: 0,
            errors: 0,
            send_max_us: 0,
            loop_max_us: 0,
        }
    }
}

fn apply(mode: &Mode) {
    IRQ_ON.store(mode.irq, Ordering::Relaxed);
    HOG_ON.store(mode.hog, Ordering::Relaxed);
    println!("mode -> {}", mode.name);
}

fn report(mode: &Mode, frames: &FrameStats) {
    let irq = critical_section::with(|cs| {
        let mut stats = IRQ_STATS.borrow_ref_mut(cs);
        let snapshot = *stats;
        stats.ticks = 0;
        stats.max_gap_us = 0;
        snapshot
    });
    let hog = critical_section::with(|cs| HOG_CHUNKS.borrow(cs).replace(0));
    println!(
        "{}: {} frames, {} errors, send max {} us, loop max {} ms | irq {} ticks, max gap {} us | hog {} chunks",
        mode.name,
        frames.frames,
        frames.errors,
        frames.send_max_us,
        frames.loop_max_us / 1000,
        irq.ticks,
        irq.max_gap_us,
        hog
    );
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let mut tick_timer = PeriodicTimer::new(timg1.timer0);
    tick_timer.set_interrupt_handler(on_tick);
    tick_timer.listen();
    tick_timer
        .start(Duration::from_micros(TICK_PERIOD_US))
        .unwrap();
    critical_section::with(|cs| TICK_TIMER.borrow_ref_mut(cs).replace(tick_timer));

    spawner.spawn(hog_task().expect("hog_task spawn token"));

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();
    let mut ws = Ws2812Rmt::<_, N>::from_channel_creator(rmt.channel0, peripherals.GPIO4).unwrap();
    let mut effect = RainbowCometEffect::new(NUM_LEDS)
        .unwrap()
        .with_hue(0)
        .with_hue_step(16);
    let mut colors = [RGB8::default(); NUM_LEDS];

    println!(
        "hal_c3_comet_under_load_async: {NUM_LEDS} LEDs on GPIO4, tick {TICK_PERIOD_US} us, hog chunk {HOG_CHUNK_US} us"
    );

    let mut mode_index = 0;
    let mut seconds_in_mode = 0;
    apply(&MODES[mode_index]);

    let mut frames = FrameStats::new();
    let mut second_start = Instant::now();
    let mut last_loop = Instant::now();

    loop {
        let mode = &MODES[mode_index];
        let loop_us = last_loop.elapsed().as_micros();
        last_loop = Instant::now();

        if mode.evict {
            evict_cache();
        }
        if mode.comet {
            if frames.frames > 0 {
                frames.loop_max_us = frames.loop_max_us.max(loop_us);
            }
            let updated = effect.update(&mut colors);
            let send_start = Instant::now();
            let sent = ws.set_pixels_slice(&colors).await;
            frames.send_max_us = frames.send_max_us.max(send_start.elapsed().as_micros());
            frames.frames += 1;
            frames.errors += u32::from(updated.is_err() || sent.is_err());
            if let Err(e) = sent {
                println!("{}: transmit error: {e}", mode.name);
            }
        }

        if second_start.elapsed().as_micros() >= 1_000_000 {
            report(mode, &frames);
            frames = FrameStats::new();
            second_start = Instant::now();
            seconds_in_mode += 1;
            if seconds_in_mode >= MODE_SECONDS {
                seconds_in_mode = 0;
                mode_index = (mode_index + 1) % MODES.len();
                apply(&MODES[mode_index]);
            }
        }

        Timer::after_millis(FRAME_DELAY_MS).await;
    }
}

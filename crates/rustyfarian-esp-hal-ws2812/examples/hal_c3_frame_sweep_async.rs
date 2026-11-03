//! ESP32-C3 Async Frame Sweep Example (esp-hal + Embassy, bare-metal)
//!
//! Self-reporting validation fixture for the long-frame async release gate in `maintenance-plan.md`.
//! It sends a fixed sequence of frame sizes and transition patterns through the async RMT driver,
//! checks what software can check, prints a verdict per cycle over USB serial,
//! and shows the verdict on the ring.
//!
//! Each cycle runs five phases, separated by 1 s pauses so a logic-analyser capture is easy to segment:
//!
//! 1. Solid frames of 1, 2, 3, 12 and 64 LEDs; each transmit must take at least the wire time
//!    (24 bits × 1.1 µs per LED, rounded up), otherwise the frame was cut short.
//!    Frames that fit in one channel's RAM (1 LED on the C3) must also include the 280 µs encoded latch.
//! 2. Ten back-to-back frames at 1, 12 and 64 LEDs; the mean period must include the 280 µs latch.
//!    The 1-LED case sends the encoded latch (await path): if the RMT did not hold the encoded low half before the end,
//!    the period would be only about 30–100 µs.
//! 3. Alternating 1-LED and 64-LED frames (short ↔ long); the mean pair period must include two latches.
//! 4. A 12-LED frame under `LongFramePolicy::Reject` must fail with `Error::FrameTooLong`,
//!    then a 1-LED frame must succeed.
//! 5. Cancellation: a 1-LED frame is cancelled by a 10 µs timeout, then a 1-LED frame is timed;
//!    it must take at least the forced 280 µs latch wait plus its own frame minimum (wire time and encoded latch).
//!    If the timeout does not fire, the step is skipped without failing.
//!
//! The cycle ends with the verdict on the ring for 1 s: all LEDs dim green for PASS, dim red for FAIL.
//! Failures are logged and counted, never panicked on, so the log keeps running.
//!
//! ## What the log cannot tell you
//!
//! Software only sees timing and errors.
//! Whether the LEDs latched the right colours (no garbling, no stale bits, latch low ≥ 280 µs on the wire)
//! needs your eyes on the ring or a logic analyser on the data line.
//!
//! ## Serial output (one cycle, shape only)
//!
//! ```text
//! cycle 1: start
//! phase 1:  1 LEDs    420 us (minimum 309 us) OK
//! phase 1: 64 LEDs   1980 us (minimum 1844 us) OK
//! phase 2:  1 LEDs back-to-back, mean period 330 us (minimum 308 us) OK
//! phase 2: 12 LEDs back-to-back, mean period 668 us (minimum 625 us) OK
//! phase 3: 1 <-> 64 LEDs, mean pair period 2510 us (minimum 2432 us) OK
//! phase 4: 12 LEDs under Reject -> Err(FrameTooLong) in 4 us OK
//! phase 4: 1 LED after reject -> Ok OK
//! phase 5: cancelled (TimeoutError), next 1 LED took 625 us (minimum 589 us) OK
//! cycle 1: PASS (0 failures); ring shows the verdict
//! ```
//!
//! ## Components
//!
//! - ESP32-C3 development board, connected over USB (native USB-Serial-JTAG or a USB-UART bridge)
//! - WS2812B LED ring, 64 LEDs (short frames drive only the first 1, 2, 3 or 12 LEDs)
//! - 300–500 Ω resistor (data line protection)
//! - Logic analyser on the data line (optional, for the waveform part of the gate)
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
//! **Masked-time ceiling:** the 64-LED phases mask interrupts for about 2 ms under the default `CriticalSection` policy,
//! which is above the default 1 ms nominal-masked-time ceiling and would return `Error::FrameTooLong`.
//! The example lifts the ceiling with `with_max_nominal_masked_us(None)`.
//!
//! **Power note:** colours are kept dim so 64 LEDs stay within USB current limits.
//!
//! ## Build
//!
//! ```sh
//! just build-example hal_c3_frame_sweep_async
//! ```
//!
//! ## Run (flash and monitor)
//!
//! ```sh
//! just run hal_c3_frame_sweep_async
//! ```

#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_time::{with_timeout, Duration, Timer};
use esp_hal::{
    rmt::Rmt,
    time::{Instant, Rate},
    timer::timg::TimerGroup,
    Async,
};
use esp_println::println;
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{buffer_size, Error, LongFramePolicy, Ws2812Rmt, WS2812_RESET_US};

const NUM_LEDS: usize = 64;
const N: usize = buffer_size(NUM_LEDS);
const LATCH_US: u64 = WS2812_RESET_US as u64;

const RED: RGB8 = RGB8::new(32, 0, 0);
const GREEN: RGB8 = RGB8::new(0, 32, 0);
const BLUE: RGB8 = RGB8::new(0, 0, 32);
const WHITE: RGB8 = RGB8::new(16, 16, 16);

type Driver<'d> = Ws2812Rmt<'d, Async, N>;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    loop {}
}

/// Shortest possible time on the wire for `leds` LEDs: 24 bits of at least 1.1 µs each (the `0` code, T0H + T0L), rounded up.
const fn wire_minimum_us(leds: usize) -> u64 {
    (leds as u64 * 24 * 11).div_ceil(10)
}

/// Shortest possible transmit time for one frame of `leds` LEDs.
///
/// Frames that fit in one channel's RAM take the async path, which carries the latch in the
/// signal, so their transmit includes the 280 µs low time; longer frames do not.
const fn frame_minimum_us(leds: usize) -> u64 {
    if leds * 24 + 1 <= esp_hal::rmt::CHANNEL_RAM_SIZE {
        wire_minimum_us(leds) + LATCH_US
    } else {
        wire_minimum_us(leds)
    }
}

fn verdict(ok: bool) -> &'static str {
    if ok {
        "OK"
    } else {
        "FAIL"
    }
}

/// Fills the first `len` entries of `frame` with `color` and sends them.
async fn show(
    ws: &mut Driver<'_>,
    frame: &mut [RGB8; NUM_LEDS],
    len: usize,
    color: RGB8,
) -> Result<(), Error> {
    frame[..len].fill(color);
    ws.set_pixels_slice(&frame[..len]).await
}

/// Sends one frame and returns how long the transmit took, in microseconds.
async fn timed_show(
    ws: &mut Driver<'_>,
    frame: &mut [RGB8; NUM_LEDS],
    len: usize,
    color: RGB8,
) -> Result<u64, Error> {
    let start = Instant::now();
    show(ws, frame, len, color).await?;
    Ok(start.elapsed().as_micros())
}

/// Sends `count` back-to-back frames of `len` LEDs in alternating colours.
///
/// Returns the mean period of frames 2 to `count` in microseconds.
/// The first frame follows a pause, so it carries no latch wait and is not timed.
async fn back_to_back(
    ws: &mut Driver<'_>,
    frame: &mut [RGB8; NUM_LEDS],
    len: usize,
    colors: [RGB8; 2],
    count: u32,
) -> Result<u64, Error> {
    show(ws, frame, len, colors[0]).await?;
    let start = Instant::now();
    for i in 1..count {
        show(ws, frame, len, colors[(i % 2) as usize]).await?;
    }
    Ok(start.elapsed().as_micros() / u64::from(count - 1))
}

/// Sends `pairs` pairs of a 1-LED and a 64-LED frame without delay.
///
/// Returns the mean pair period of pairs 2 to `pairs` in microseconds.
async fn alternate(
    ws: &mut Driver<'_>,
    frame: &mut [RGB8; NUM_LEDS],
    pairs: u32,
) -> Result<u64, Error> {
    show(ws, frame, 1, GREEN).await?;
    show(ws, frame, NUM_LEDS, BLUE).await?;
    let start = Instant::now();
    for _ in 1..pairs {
        show(ws, frame, 1, GREEN).await?;
        show(ws, frame, NUM_LEDS, BLUE).await?;
    }
    Ok(start.elapsed().as_micros() / u64::from(pairs - 1))
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))
        .unwrap()
        .into_async();
    let mut ws = Driver::from_channel_creator(rmt.channel0, peripherals.GPIO4)
        .unwrap()
        .with_max_nominal_masked_us(None);
    let mut frame = [RGB8::new(0, 0, 0); NUM_LEDS];
    let mut cycle: u32 = 0;

    println!("hal_c3_frame_sweep_async: {NUM_LEDS} LEDs on GPIO4, latch {LATCH_US} us");

    loop {
        cycle += 1;
        let mut failures: u32 = 0;
        println!("cycle {cycle}: start");

        for (len, color) in [
            (1, RED),
            (2, GREEN),
            (3, BLUE),
            (12, WHITE),
            (NUM_LEDS, RED),
        ] {
            let minimum = frame_minimum_us(len);
            match timed_show(&mut ws, &mut frame, len, color).await {
                Ok(us) => {
                    let ok = us >= minimum;
                    failures += u32::from(!ok);
                    println!(
                        "phase 1: {len:>2} LEDs {us:>6} us (minimum {minimum} us) {}",
                        verdict(ok)
                    );
                }
                Err(e) => {
                    failures += 1;
                    println!("phase 1: {len:>2} LEDs error: {e} FAIL");
                }
            }
            Timer::after_millis(100).await;
        }
        Timer::after_millis(1000).await;

        for (len, colors) in [
            (1, [GREEN, BLUE]),
            (12, [GREEN, BLUE]),
            (NUM_LEDS, [RED, WHITE]),
        ] {
            let minimum = wire_minimum_us(len) + LATCH_US;
            match back_to_back(&mut ws, &mut frame, len, colors, 10).await {
                Ok(period) => {
                    let ok = period >= minimum;
                    failures += u32::from(!ok);
                    println!(
                        "phase 2: {len:>2} LEDs back-to-back, mean period {period} us (minimum {minimum} us) {}",
                        verdict(ok)
                    );
                }
                Err(e) => {
                    failures += 1;
                    println!("phase 2: {len:>2} LEDs back-to-back error: {e} FAIL");
                }
            }
        }
        Timer::after_millis(1000).await;

        let minimum = wire_minimum_us(1) + wire_minimum_us(NUM_LEDS) + 2 * LATCH_US;
        match alternate(&mut ws, &mut frame, 8).await {
            Ok(period) => {
                let ok = period >= minimum;
                failures += u32::from(!ok);
                println!(
                    "phase 3: 1 <-> {NUM_LEDS} LEDs, mean pair period {period} us (minimum {minimum} us) {}",
                    verdict(ok)
                );
            }
            Err(e) => {
                failures += 1;
                println!("phase 3: 1 <-> {NUM_LEDS} LEDs error: {e} FAIL");
            }
        }
        Timer::after_millis(1000).await;

        ws = ws.with_long_frame_policy(LongFramePolicy::Reject);
        let start = Instant::now();
        let rejected = show(&mut ws, &mut frame, 12, RED).await;
        let reject_us = start.elapsed().as_micros();
        let ok = matches!(rejected, Err(Error::FrameTooLong));
        failures += u32::from(!ok);
        println!(
            "phase 4: 12 LEDs under Reject -> {rejected:?} in {reject_us} us {}",
            verdict(ok)
        );
        let after = show(&mut ws, &mut frame, 1, GREEN).await;
        failures += u32::from(after.is_err());
        println!(
            "phase 4: 1 LED after reject -> {after:?} {}",
            verdict(after.is_ok())
        );
        ws = ws.with_long_frame_policy(LongFramePolicy::CriticalSection);
        Timer::after_millis(1000).await;

        let cancelled =
            with_timeout(Duration::from_micros(10), show(&mut ws, &mut frame, 1, RED)).await;
        let start = Instant::now();
        let next = show(&mut ws, &mut frame, 1, GREEN).await;
        let next_us = start.elapsed().as_micros();
        match (&cancelled, &next) {
            (Err(_), Ok(())) => {
                let ok = next_us >= LATCH_US + frame_minimum_us(1);
                failures += u32::from(!ok);
                println!(
                    "phase 5: cancelled (TimeoutError), next 1 LED took {next_us} us (minimum {} us) {}",
                    LATCH_US + frame_minimum_us(1),
                    verdict(ok)
                );
            }
            (Ok(_), Ok(())) => println!("phase 5: not cancelled, skipped"),
            _ => {
                failures += 1;
                println!("phase 5: unexpected result {cancelled:?} / {next:?} FAIL");
            }
        }
        Timer::after_millis(1000).await;

        let passed = failures == 0;
        let shown = show(
            &mut ws,
            &mut frame,
            NUM_LEDS,
            if passed { GREEN } else { RED },
        )
        .await;
        println!(
            "cycle {cycle}: {} ({failures} failures); ring shows the verdict{}",
            if passed { "PASS" } else { "FAIL" },
            if shown.is_ok() {
                ""
            } else {
                " (verdict frame failed to send)"
            }
        );
        Timer::after_millis(1000).await;
    }
}

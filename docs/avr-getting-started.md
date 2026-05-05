# AVR Getting Started — Flash and Run on ATmega328P

How to wire, build, flash, and run `rustyfarian-avr-ws2812` on an Arduino Nano or Uno
with a WS2812 LED ring.

This guide covers:
- Physical wiring between the Arduino and the WS2812 ring
- Toolchain setup (Rust nightly, GNU AVR toolchain, ravedude)
- Building and flashing a minimal rainbow example

For the SPI encoding internals, see [ws2812-spi-prerendered-encoding.md](ws2812-spi-prerendered-encoding.md).
For the ecosystem research behind this driver, see [research-avr-ws2812.md](research-avr-ws2812.md).

---

## Quick Start

If you have a 5 V Arduino Nano with a new bootloader, a 12-LED WS2812 ring is wired to D11, and the toolchain installed:

```sh
just setup-avr
just flash-avr-example
```

The first command pins `nightly-2025-04-27` and adds `rust-src`; the second builds the `avr-nano-rainbow` example with `-Z build-std=core` and flashes it via `ravedude`.

If anything below "first light" is unfamiliar — wiring, bootloader variants, AVR toolchain, the `build-std` merge gotcha — read the full guide.

---

## Wiring

### Pin connections

| WS2812 pad | Connect to                                      | Notes                                   |
|:-----------|:------------------------------------------------|:----------------------------------------|
| VDD / +5V  | Arduino 5V                                      | Use external supply for full brightness |
| GND        | Arduino GND                                     | Common ground with Arduino              |
| DIN        | Arduino D11 (PB3/MOSI) via 330-470 ohm resistor | Data input to first LED                 |

The SPI MOSI pin (PB3 / Arduino D11) carries the prerendered WS2812 data stream.
SCK (D13) and SS (D10) are consumed by the SPI peripheral but not connected to anything.

### Wiring diagram

```text
Arduino Nano             WS2812 Ring
────────────             ───────────
5V  ─────────────────── VDD (+5V)
GND ─────────────────── GND
D11 ── [330 ohm] ────── DIN
                    |
                   [100uF] (across VDD/GND, close to ring)
```

### Data line resistor

Place a **330-470 ohm resistor** in series between D11 and the WS2812 DIN pad.
This reduces ringing on the data line.
Some WS2812 ring breakout boards include this resistor on-board — check before adding a second one.

### Decoupling capacitor

Place a **100-1000 uF electrolytic capacitor** (minimum 6.3 V rating) across VDD and GND,
as close to the WS2812 ring as possible.
This absorbs inrush current when the first frame is sent.
100 uF is sufficient for a 12-LED ring.

### Level shifting

The Arduino Nano (ATmega328P) runs at **5 V logic**.
WS2812 LEDs powered from 5 V have a logic-high threshold of 3.5 V (70% of VDD).
The ATmega328P GPIO outputs ~5 V, which exceeds this threshold.
**No level shifter is needed** for the classic 5 V Arduino Nano or Uno.

For 3.3 V boards (Arduino Pro Mini 3.3 V, Nano 33 IoT), add a **74AHCT125** or **74HCT245** level shifter.

If a 5 V Nano/Uno still flickers, the usual causes are long data wires, a missing series resistor or decoupling capacitor, or brownout from undersized power — fix those before reaching for a level shifter.

### Power budget

Each WS2812 LED draws up to 60 mA at full RGB brightness (20 mA per channel).
A 12-LED ring at full white: 12 x 60 mA = 720 mA.
The Arduino 5 V pin via USB can supply ~400-500 mA — not enough for full brightness.

For hardware testing, keep brightness below ~30% when powered from USB.
For full-brightness use, power the ring from an external 5 V supply with GND tied to Arduino GND.

---

## Prerequisites

### GNU AVR toolchain

macOS:

```sh
brew tap osx-cross/avr
brew install avr-gcc avrdude
```

Debian/Ubuntu:

```sh
sudo apt install gcc-avr binutils-avr avr-libc avrdude
```

### Rust AVR nightly

```sh
just setup-avr
```

This installs `nightly-2025-04-27` and the `rust-src` component needed for `-Z build-std=core`.

### ravedude (flash tool)

```sh
cargo +stable install --locked ravedude
```

`ravedude` wraps `avrdude` and integrates with `cargo run` for one-command build-and-flash.

---

## Project Setup

An AVR example binary needs its own crate (separate from the workspace) because:
- It requires `avr-none` as the build target (the workspace defaults to ESP32)
- It needs `arduino-hal` which is a git dependency (not on crates.io)
- It requires a pinned nightly via `rust-toolchain.toml`

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "nightly-2025-04-27"
components = ["rust-src"]
profile = "minimal"
```

### `.cargo/config.toml`

```toml
[build]
target = "avr-none"

[target.'cfg(target_arch = "avr")']
rustflags = ["-C", "target-cpu=atmega328p"]
runner = "ravedude"
```

Board and baud rate live in `Ravedude.toml` (single source of truth) so the runner stays simple.

**Why no `[unstable] build-std = ["core"]` here?**
This repository's workspace root `.cargo/config.toml` sets `build-std = ["std", "panic_abort"]` for ESP-IDF builds.
Cargo merges `build-std` arrays from parent configs, so adding `["core"]` here would produce the merged value `["std", "panic_abort", "core"]` — which fails on AVR (no `std`).
Instead, pass `-Z build-std=core` on the command line (the `just build-avr-example` and `just flash-avr-example` recipes already do this).
If you're consuming `rustyfarian-avr-ws2812` from a standalone project (no ESP-IDF workspace above it), you can safely add `[unstable] build-std = ["core"]` to your local config and skip the `-Z` flag.

### `Ravedude.toml` (next to Cargo.toml)

For Arduino Nano boards manufactured after January 2018 (new bootloader / Optiboot):

```toml
[general]
board = "nano-new"
serial-baudrate = 115200
open-console = false
```

For older Nano boards (ATmegaBOOT bootloader), use:

```toml
[general]
board = "nano"
serial-baudrate = 57600
open-console = false
```

### `Cargo.toml`

```toml
[package]
name = "avr-ws2812-example"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
panic-halt = "1.0"
avr-device = "0.8.1"
embedded-hal = "1.0"
rgb = "0.8"

[dependencies.arduino-hal]
git = "https://github.com/Rahix/avr-hal"
features = ["arduino-nano"]

[dependencies.rustyfarian-avr-ws2812]
path = "../../crates/rustyfarian-avr-ws2812"

[dependencies.bunting]
path = "../../crates/bunting"

[dependencies.ferriswheel]
path = "../../crates/ferriswheel"

[profile.dev]
panic = "abort"
opt-level = "s"

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
```

Use `features = ["arduino-uno"]` for an Arduino Uno instead of Nano.

---

## Build

From the workspace root:

```sh
just build-avr-example
```

Or directly from the example crate (with the same effect):

```sh
cd examples/avr-nano-rainbow
cargo +nightly-2025-04-27 build --release -Z build-std=core
```

The `--release` flag is **mandatory** for WS2812 work on AVR — debug builds produce slower code that can cause SPI timing violations.
The `-Z build-std=core` flag is required because the example's `.cargo/config.toml` deliberately does not set `build-std` (see the warning in [`.cargo/config.toml`](#cargoconfigtoml) above).

The compiled ELF lands at `examples/avr-nano-rainbow/target/avr-none/release/avr-nano-rainbow.elf`.

---

## Flash

### With ravedude (recommended)

From the workspace root:

```sh
just flash-avr-example
```

Or directly from the example crate:

```sh
cd examples/avr-nano-rainbow
cargo +nightly-2025-04-27 run --release -Z build-std=core
```

`ravedude` reads the board config from `Ravedude.toml`, converts the ELF to Intel HEX, and flashes via the Arduino bootloader.

To specify the serial port explicitly (useful when auto-detection fails):

```sh
RAVEDUDE_PORT=/dev/tty.usbserial-1410 just flash-avr-example
```

### Manual flash with avrdude

If not using ravedude, first convert ELF to HEX:

```sh
avr-objcopy -O ihex -R .eeprom \
    target/avr-none/release/<crate-name>.elf \
    target/avr-none/release/<crate-name>.hex
```

Then flash via the Arduino bootloader:

```sh
avrdude -v -p atmega328p -c arduino \
    -P /dev/ttyUSB0 -b 57600 \
    -U flash:w:target/avr-none/release/<crate-name>.hex:i
```

For Nano new bootloader, use `-b 115200`.

Via USBasp programmer (no serial port needed):

```sh
avrdude -v -p atmega328p -c usbasp \
    -U flash:w:target/avr-none/release/<crate-name>.hex:i
```

---

## SPI Configuration Reference

On a 16 MHz ATmega328P, the hardware SPI divisors are:

| Variant         | Divisor | Frequency |
|:----------------|:--------|:----------|
| `OscfOver2`     | /2      | 8 MHz     |
| `OscfOver4`     | /4      | 4 MHz     |
| **`OscfOver8`** | **/8**  | **2 MHz** |
| `OscfOver16`    | /16     | 1 MHz     |

**`OscfOver8` (2 MHz)** is the only standard divisor within the WS2812 acceptable range of 2-3.8 MHz.

SPI mode: `MODE_0` (CPOL=0, CPHA=0), data order: MSB first.

The SPI constructor consumes four pins even though only MOSI carries data:

```rust
let (spi, _cs) = arduino_hal::Spi::new(
    dp.SPI,
    pins.d13.into_output(),          // SCK
    pins.d11.into_output(),          // MOSI — data to WS2812
    pins.d12.into_pull_up_input(),   // MISO — unused but required
    pins.d10.into_output(),          // SS — must be output for master mode
    spi::Settings {
        data_order: spi::DataOrder::MostSignificantFirst,
        clock: spi::SerialClockRate::OscfOver8,
        mode: embedded_hal::spi::MODE_0,
    },
);
```

---

## Interrupt Safety

WS2812 requires an uninterrupted data stream for the entire frame.
On AVR, wrap the `write` call in a critical section:

```rust
avr_device::interrupt::free(|_| {
    ws.write(&colors).unwrap();
});
```

For a 12-LED ring at 2 MHz: `224 bytes x 8 bits / 2,000,000 Hz = 896 us` interrupt-free.
The Arduino `millis()` timer loses ticks during this window, but for LED animations this is acceptable.

---

## Flashing the Arduino Bootloader

If the ATmega328P has no bootloader (blank chip, corrupted bootloader, or Chinese clone with wrong/missing bootloader),
serial-based flashing (`ravedude`, `avrdude -c arduino`) will not work.
You need an ISP programmer to write the bootloader and set the correct fuses.

### What you need

An ISP (In-System Programming) programmer — any of:

- **USBasp** — cheap, dedicated programmer (~$3-5)
- **Another Arduino** — running the "Arduino as ISP" sketch (built into the Arduino IDE)
- **AVRISP mkII** — Atmel's official programmer

### ISP wiring

Connect the programmer to the ATmega328P's ICSP header (the 2x3 pin header on the Nano/Uno):

| ICSP pin | Signal | Arduino pin |
|:---------|:-------|:------------|
| 1        | MISO   | D12         |
| 2        | VCC    | 5V          |
| 3        | SCK    | D13         |
| 4        | MOSI   | D11         |
| 5        | RESET  | RST         |
| 6        | GND    | GND         |

Most ICSP cables have a notch or dot marking pin 1.

### Using another Arduino as ISP

Upload the "ArduinoISP" sketch to the *programmer* Arduino first:

1. Open the Arduino IDE
2. File → Examples → 11.ArduinoISP → ArduinoISP
3. Upload to the programmer Arduino
4. Wire the programmer Arduino to the target board's ICSP header:
   - Programmer D10 → Target RESET
   - Programmer D11 → Target D11 (MOSI)
   - Programmer D12 → Target D12 (MISO)
   - Programmer D13 → Target D13 (SCK)
   - Programmer 5V → Target 5V
   - Programmer GND → Target GND
5. Place a **10 uF capacitor** between RESET and GND on the *programmer* Arduino (prevents it from resetting during programming)

### Checking if a bootloader is present

Try reading the chip signature and fuses — this works even without a bootloader:

```sh
avrdude -v -p atmega328p -c usbasp
```

Or with Arduino as ISP (replace port as needed):

```sh
avrdude -v -p atmega328p -c stk500v1 -P /dev/tty.usbserial-1410 -b 19200
```

If this succeeds and prints the device signature (`0x1e 0x95 0x0f` for ATmega328P), the chip is alive.
If serial flashing (`-c arduino`) fails but ISP works, the bootloader is missing or corrupted.

### Flashing the bootloader with avrdude

The Arduino Nano (old bootloader) uses ATmegaBOOT at 57600 baud.
The Arduino Nano (new bootloader) and Uno use Optiboot at 115200 baud.

**Option A — Via the Arduino IDE** (easiest):

1. Tools → Board → "Arduino Nano" (or "Arduino Uno")
2. Tools → Processor → "ATmega328P" (or "ATmega328P (Old Bootloader)" for old Nano)
3. Tools → Programmer → select your programmer (USBasp, "Arduino as ISP", etc.)
4. Tools → Burn Bootloader

This writes the correct bootloader and sets all fuses automatically.

**Option B — Manual avrdude commands** (if you don't have the Arduino IDE):

First, locate the bootloader hex file.
On macOS with Arduino IDE installed:

```sh
ls ~/Library/Arduino15/packages/arduino/hardware/avr/*/bootloaders/optiboot/
```

Or download from the [Arduino GitHub repository](https://github.com/arduino/ArduinoCore-avr/tree/master/bootloaders).

Set fuses and flash the Optiboot bootloader (Nano new / Uno):

```sh
avrdude -v -p atmega328p -c usbasp \
    -U lfuse:w:0xFF:m \
    -U hfuse:w:0xDE:m \
    -U efuse:w:0xFD:m \
    -U flash:w:optiboot_atmega328.hex:i
```

For the old Nano bootloader (ATmegaBOOT, 57600 baud):

```sh
avrdude -v -p atmega328p -c usbasp \
    -U lfuse:w:0xFF:m \
    -U hfuse:w:0xDA:m \
    -U efuse:w:0xFD:m \
    -U flash:w:ATmegaBOOT_168_atmega328.hex:i
```

Replace `-c usbasp` with `-c stk500v1 -P /dev/tty.usbserial-1410 -b 19200` if using Arduino as ISP.

### Fuse reference

| Fuse  | Old Nano (ATmegaBOOT) | New Nano / Uno (Optiboot) | Notes                                           |
|:------|:----------------------|:--------------------------|:------------------------------------------------|
| lfuse | `0xFF`                | `0xFF`                    | External 16 MHz crystal, slow rising power      |
| hfuse | `0xDA`                | `0xDE`                    | `DA` = 2 KB bootloader; `DE` = 512 B bootloader |
| efuse | `0xFD`                | `0xFD`                    | BOD at 2.7 V                                    |

The key difference: old Nano reserves 2 KB for the larger ATmegaBOOT bootloader (`hfuse=0xDA`),
while Optiboot fits in 512 bytes (`hfuse=0xDE`), leaving more flash for your application.

### After flashing the bootloader

Verify the bootloader works by attempting a serial connection:

```sh
avrdude -v -p atmega328p -c arduino -P /dev/tty.usbserial-1410 -b 115200
```

Use `-b 57600` for the old bootloader.
If this succeeds, serial flashing (ravedude, `just flash-avr-example`) will work normally.

### Chinese Nano clones

Many cheap Nano clones use the **CH340** USB-to-serial chip instead of FTDI.
On macOS, install the CH340 driver if the board doesn't appear as a serial port:

```sh
brew install --cask wch-ch34x-usb-serial-driver
```

These clones sometimes ship with the old bootloader (57600 baud) or no bootloader at all.
If in doubt, flash Optiboot and use `nano-new` / 115200 baud going forward.

---

## Troubleshooting

| Symptom                                                           | Likely cause                                                               | Fix                                                                                                                                                                  |
|:------------------------------------------------------------------|:---------------------------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| LEDs don't light up                                               | Wiring issue: DIN not connected to D11                                     | Check MOSI pin (PB3/D11)                                                                                                                                             |
| First LED lights, rest are wrong colors                           | Missing series resistor causing signal ringing                             | Add 330 ohm resistor on DIN                                                                                                                                          |
| LEDs flicker or show random colors                                | Interrupts corrupting the data stream                                      | Wrap `write()` in `avr_device::interrupt::free`                                                                                                                      |
| `cargo build` fails with "can't find crate for core"              | Wrong toolchain or missing `build-std`                                     | Ensure `rust-toolchain.toml` is in place and pass `-Z build-std=core` (or use `just build-avr-example`)                                                              |
| `error: can't find crate for 'std'` (or build-std mentions `std`) | Workspace root config merged `build-std = ["std", ...]` into the AVR build | Pass `-Z build-std=core` explicitly via `just build-avr-example` / `just flash-avr-example`; do not set `[unstable] build-std` in the example's `.cargo/config.toml` |
| `avrdude: stk500_recv(): programmer is not responding`            | Wrong serial port, baud rate, or missing bootloader                        | Check port, try both 57600 and 115200; if neither works, flash the bootloader via ISP (see above)                                                                    |
| `avrdude: stk500_getsync() attempt N of 10: not in sync`          | Bootloader missing, corrupted, or wrong baud rate                          | Flash the bootloader via ISP programmer (see "Flashing the Arduino Bootloader" section)                                                                              |
| Binary too large for flash                                        | Debug build or too many dependencies                                       | Use `--release` and `opt-level = "s"`                                                                                                                                |

---

*Last updated: May 2026*

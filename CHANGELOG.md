# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `rustyfarian-esp-hal-ws2812`: `smart_leds_trait::SmartLedsWrite` trait implementation with zero-allocation buffer-draining iterator support; error type is crate's `Error` enum, color type is `smart_leds_trait::RGB8`
- `rustyfarian-esp-idf-ws2812`: `smart_leds_trait::SmartLedsWrite` trait implementation enabling use within the `smart-leds` ecosystem; error type is `anyhow::Error`, color type is `smart_leds_trait::RGB8`
- `README.md`: IDF troubleshooting tip documenting `just clean-idf` for stale `sdkconfig.defaults` cache
- `scripts/build-example.sh`, `scripts/run-example.sh`: progress output (`Building <example> for <target>...` / `Flashing <example> with bootloader <path>...`) so users can see what the script is doing at each step
- `rustyfarian-esp-hal-ws2812`: `esp32` feature and `hal_esp32_pulse` bare-metal example for the ESP32-WROOM-32 (Xtensa LX6), using GPIO4 and the `xtensa-esp32-none-elf` target
- `.cargo/config.toml`: `[target.xtensa-esp32-none-elf]` entry with `-Tlinkall.x` linker flag for bare-metal ESP32 builds
- `justfile`: `build-example-esp32-hal` alias for `just build-example hal-ws2812 hal_esp32_pulse`
- `rustyfarian-esp-hal-ws2812`: full WS2812 RMT driver for ESP32-C6 (`esp-hal 1.0.0`, bare-metal `no_std`); const-generic buffer `N = num_leds * 24 + 1`, `buffer_size()` const helper, `RMT_CLK_DIV` constant, `esp32c3`/`esp32c6`/`unstable`/`rt` features
- `hal_c3_rainbow`, `hal_c3_pulse`, `hal_c6_rainbow`, `hal_c6_pulse` examples: complete 2×2 matrix of `RainbowEffect` and `PulseEffect` for ESP32-C3 (GPIO4) and ESP32-C6 (GPIO18) using the bare-metal esp-hal driver
- `idf_c3_rainbow`, `idf_c3_pulse`, `idf_c6_rainbow`, `idf_c6_pulse` examples: matching 2×2 matrix using the ESP-IDF driver as a known-good hardware baseline; C3 examples use GPIO4, C6 examples use GPIO18
- `idf_esp32_rainbow` and `idf_esp32_pulse` examples: IDF examples for Adafruit Feather ESP32 V2 (Xtensa); onboard NeoPixel on GPIO0, power enable on GPIO2 (`NEOPIXEL_I2C_POWER`); uses `cargo +esp` and `xtensa-esp32-espidf` target
- `just build-example-c6` alias for `just build-example hal-ws2812 hal_c6_rainbow`
- `just build-example-esp32` alias for `just build-example idf-ws2812 idf_esp32_rainbow`
- `[target.xtensa-esp32-espidf]` added to `.cargo/config.toml` with `ldproxy` linker for Xtensa ESP32 IDF builds
- `scripts/build-example.sh`, `scripts/run-example.sh`, `scripts/ensure-bootloader.sh`: `esp32` chip added alongside `c3` and `c6`; maps to `xtensa-esp32-espidf` target with `MCU=esp32`
- `build.rs` in `rustyfarian-esp-idf-ws2812`: re-emits ESP-IDF link args via `embuild::espidf::sysenv::output()` so downstream examples link without their own `build.rs`
- `just build-example <crate> <example>` and `just run-example <crate> <example>`: universal recipes; driver and chip auto-detected from the `{driver}_{chip}_{name}` naming convention
- `just ensure-bootloader <chip>`: builds the IDF example cache on demand so the v5.3.3 bootloader is always available for both IDF and bare-metal flashing
- `just flash` auto-detects the driver crate from the example name prefix (`hal_*` → `hal-ws2812`, `idf_*` → `idf-ws2812`)
- `just check-hal` and `just clippy-hal` recipes for the bare-metal crate (no ESP-IDF toolchain required)
- `[target.riscv32imc-esp-espidf]` in `.cargo/config.toml` with `ldproxy` linker for ESP32-C3 IDF builds
- `-Tlinkall.x` linker flag for `riscv32imc-unknown-none-elf` and `riscv32imac-unknown-none-elf` targets in `.cargo/config.toml`

### Removed

- HAL rainbow examples (`hal_c3_rainbow`, `hal_c6_rainbow`) and IDF pulse examples (`idf_c3_pulse`, `idf_c6_pulse`, `idf_esp32_pulse`) removed to keep one example per driver per chip:
  HAL crate uses `PulseEffect`; IDF crate uses `RainbowEffect`

### Changed

- `README.md`: examples table updated to match actual example set — removed stale entries (`hal_c3_rainbow`, `hal_c6_rainbow`, `idf_c3_pulse`, `idf_c6_pulse`, `idf_esp32_pulse`), added `hal_esp32_pulse`; example `just run` command updated to `hal_c6_pulse`
- `rustyfarian-esp-hal-ws2812/Cargo.toml`: added comment on `esp-println` dev-dependency clarifying it is used only in `hal_c6_pulse` and the `esp32c6` feature selection is intentional
- `rustyfarian-esp-hal-ws2812`: chip and `unstable` feature selection moved from the workspace root into the driver crate's own `[features]` (`esp32c6`, `unstable`);
  the workspace only pins the version now, making the crate self-describing and easier to extend for future chips

### Fixed

- `scripts/run-example.sh`, `scripts/ensure-bootloader.sh`: replaced fragile `ls -t … | head -1` bootloader lookup with a bash array glob that exits with a clear error listing all candidates when multiple `esp-idf-sys-*` build directories exist, preventing silent selection of the wrong bootloader after a `cargo update` or dependency bump

- `hal_c3_rainbow` example: `with_idle_output(false)` corrected to `with_idle_output(true)` to match the C6 example and satisfy the WS2812 reset condition (pin must be actively driven LOW between frames)

- `hal_c3_rainbow` and `hal_c6_rainbow` examples: added `esp-bootloader-esp-idf` dependency and `esp_app_desc!()` invocation so the IDF v5.3.3 bootloader reads a valid app descriptor instead of garbage bytes at that offset;
  fixes `boot_comm: Image requires efuse blk rev >= vX.Y, but chip is v0.3` boot loop
- `rustyfarian-esp-hal-ws2812/Cargo.toml`: `esp32c6` and `esp32c3` features now forward to `esp-bootloader-esp-idf` chip features so the app descriptor is populated with the correct MMU page size for each target

- `scripts/build-example.sh` and `scripts/run-example.sh`: HAL bare-metal examples now built with `--release` as strongly recommended by esp-hal; binary path updated from `debug/` to `release/`
- `scripts/build-example.sh` and `scripts/run-example.sh`: `pkg` is now derived from the example name prefix, and `crate_alias` is validated against the derived value — mismatched combinations (e.g. `hal-ws2812` alias with an `idf_*` example) now fail immediately with a clear message instead of silently using the wrong package
- `scripts/run-example.sh`: added a defensive check after HAL bootloader lookup — exits with a clear error and remediation hint if `$bl` is empty after `ensure-bootloader.sh`, rather than passing `--bootloader ""` to espflash
- `scripts/ensure-bootloader.sh` and `scripts/run-example.sh`: use `ls -t` to select the most recently modified `esp-idf-sys` build directory, avoiding stale artifacts when multiple build hashes coexist
- `idf_c3_rainbow` example: `NUM_LEDS` corrected from `24` to `12` to match the 12-LED ring stated in the wiring documentation
- `docs/key-insights.md`: tightened five insight descriptions to be less absolute and avoid future misinterpretation:
  - link-arg propagation insight reworded around "final crate" rather than blanket "does not propagate"
  - `sdkconfig.defaults` insight notes the special case applies only when the file is introduced for the first time
  - C3/C6 target-mismatch insight softened from "compiles but wrong" to "may compile far enough to produce an invalid image"
  - espflash bootloader insight now includes an explicit "re-verify if upgrading espflash or ESP-IDF" guardrail
  - stack-overflow explanation rephrased to avoid the imprecise "compiler does not reuse stack frames"
- `hal_c3_rainbow` and `hal_c6_rainbow` examples: inline comment added above `#[panic_handler]` noting it is minimal and should be replaced for real applications

- `sdkconfig.defaults` added to workspace root with `CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192`:
  the default 3584-byte main task stack overflows in debug builds when `set_pixels_slice`
  iterates over 12 LEDs, because `color_to_pulses` returns `[Pulse; 48]` (~192 bytes) on
  the stack per LED and the unoptimized compiler does not reuse stack frames across iterations

## [0.2.0] - 2026-03-01

### Fixed

- `PulseEffect::new()` doc comment now correctly states the default `min_brightness` is `2`, not `0`
- `SpinnerEffect` tail brightness calculation now uses `u16` arithmetic with `.max(1)` instead of a separate zero-floor branch, removing a redundant `let mut`

### Added

- `NoLed` stub in `led-effects`: a zero-size `StatusLed` implementor with `type Error = Infallible` for use when no physical LED is present
- `RainbowEffect::with_hue_offset(u8)` builder for setting the initial hue offset
- `RainbowEffect::set_hue_offset(&mut self, u8)` for live hue adjustment without resetting the rotation cycle
- `PulseEffect::set_color(&mut self, RGB8)` for changing color without resetting the breathing phase
- `SpinnerEffect::set_color(&mut self, RGB8)` for changing color without resetting the spinner position
- `ChaseEffect::set_color(&mut self, RGB8)` for changing color without resetting the chase position
- `FlashEffect::set_color(&mut self, RGB8)` for changing color without resetting the duty-cycle counter

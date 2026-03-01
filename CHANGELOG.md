# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed

- `PulseEffect::new()` doc comment now correctly states the default `min_brightness` is `2`, not `0`
- `SpinnerEffect` tail brightness calculation now uses `u16` arithmetic with `.max(1)` instead of a separate zero-floor branch, removing a redundant `let mut`

### Added

- `RainbowEffect::with_hue_offset(u8)` builder for setting the initial hue offset
- `RainbowEffect::set_hue_offset(&mut self, u8)` for live hue adjustment without resetting the rotation cycle
- `PulseEffect::set_color(&mut self, RGB8)` for changing color without resetting the breathing phase
- `SpinnerEffect::set_color(&mut self, RGB8)` for changing color without resetting the spinner position
- `ChaseEffect::set_color(&mut self, RGB8)` for changing color without resetting the chase position
- `FlashEffect::set_color(&mut self, RGB8)` for changing color without resetting the duty-cycle counter

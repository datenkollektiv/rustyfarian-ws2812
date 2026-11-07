# Bench

Guided, rerunnable hardware checks, driven by [`rustyfarian-bench`](https://github.com/datenkollektiv/rustyfarian-bench).

| Board                        | Directory        |
|:-----------------------------|:-----------------|
| CrowPanel 1.28" HMI (S3)     | [s3/](s3/)       |
| ESP32-C3 + 12-LED ring       | [c3/](c3/)       |
| ESP32-C6 + 12-LED ring       | [c6/](c6/)       |
| ESP32-WROOM-32 + 12-LED ring | [esp32/](esp32/) |
| Arduino Nano + 12-LED ring   | [avr/](avr/)     |

The Nano is not detected (espflash cannot identify an AVR), so `just bench avr` always needs `--port`.

## Running a check

From this repo's root; `just bench` runs `rustyfarian-bench` v0.2.0 from GitHub, or from a local checkout if `RUSTYFARIAN_BENCH` points at one:

```sh
just bench s3
just bench c3
```

One board per command; it runs every check for that board in turn.
More targets:

```sh
just bench s3 pulse
just bench hal_s3_pulse_async --no-build
just bench c3 --port /dev/cu.usbmodemXXXX
just bench-list
just bench-detect
```

`s3 pulse` runs one check (hal, then offers the async twin); pin `--port` while another board is attached.
From the bench checkout instead: `just bench rustyfarian-ws2812 s3`.

The runner builds, waits for the board, flashes, resets into the app, captures serial output and guides each step.

> Note: The first run on a board takes longer: the hal examples are flashed with the ESP-IDF v5.3.3 bootloader, because the one bundled with espflash 4.x refuses them.
> `just flash` automatically builds `idf_<board>_rainbow` once to produce it.

## Layout

- `bench.toml` — build/flash commands and the boards.
- `<board>/<check>.toml` — one scenario: the examples it runs and the shared steps.
- `<board>/logs/` — captured serial output per run (gitignored).
- `<board>/results.jsonl` — one JSON line per example run (gitignored).

## What is different from an input bench

Every board starts with the same two checks, `pulse` and `rainbow` (order set by `checks` in `bench.toml`).
On the ESP boards `pulse` covers esp-hal (blocking and async) and `rainbow` covers ESP-IDF, since the plain rainbow exists only as an ESP-IDF example; each board README names the tier per check.
Those examples print nothing, so verdicts are `confirm` steps for the operator plus `duration` steps that fail on a crash.
The other C3 fixtures log over serial, so their checks are `expect` steps like an input bench.
The frame sweep self-reports `PASS`/`FAIL` per cycle; the comet reports an error count every second.
Visual checks get their own slow fixture (`hal_c3_colors`: one solid colour at a time, 3 s each); effect patterns are too fast to judge by eye.
Pin `--port` when another board is attached: detection probes every candidate port with espflash, which resets whatever is on it.

# ws2812-spi Prerendered Module — Encoding Deep Dive

Authoritative reference for the exact SPI bit encoding used by `ws2812-spi` v0.5.1's prerendered module,
confirming all parameters needed to implement a compatible `prerender_spi` function in `bunting`.

> **Note on `docs/research-avr-ws2812.md`**:
> An earlier revision of that document stated an incorrect buffer-sizing formula (`12 * num_leds + 20`).
> It has since been corrected to the baseline `12 * num_leds` with 140 bytes added per reset-related feature flag — consistent with Q3 below.

---

## Summary

All five questions are answered with exact values sourced from the `ws2812-spi` v0.5.1 source code
at `smart-leds-rs/ws2812-spi-rs` on GitHub (commit fetched 2026-03-13).

---

## Q1 — How many SPI bits per WS2812 data bit?

**4 SPI bits per WS2812 data bit.**

Each WS2812 color byte is processed four bits at a time by `write_byte`.
The function loops four times, each iteration extracting the top two bits of the current byte
and emitting one SPI byte (8 SPI bits) that encodes two WS2812 data bits.
This gives 4 SPI bytes per color byte (32 SPI bits for 8 WS2812 data bits), meaning
**4 SPI bits per WS2812 data bit** on average across the encoding.

The existing research doc's description of "3 SPI bits" is incorrect for `ws2812-spi`.
The 3-bits-per-WS2812-bit variant exists in some other implementations but is not what this crate uses.

---

## Q2 — Exact SPI bit patterns for WS2812 `1` and `0`

### Encoding table

The `write_byte` function uses this lookup table for all four two-bit combinations:

```rust
let patterns = [0b1000_1000, 0b1000_1110, 0b11101000, 0b11101110];
```

Indexed by the two WS2812 data bits extracted from the high two bits of each byte:

| WS2812 bits (pair) | Pattern byte  | Binary     | SPI signal meaning       |
|:-------------------|:--------------|:-----------|:-------------------------|
| `00`               | `0b1000_1000` | `10001000` | bit0=`1000`, bit1=`1000` |
| `01`               | `0b1000_1110` | `10001110` | bit0=`1000`, bit1=`1110` |
| `10`               | `0b1110_1000` | `11101000` | bit0=`1110`, bit1=`1000` |
| `11`               | `0b1110_1110` | `11101110` | bit0=`1110`, bit1=`1110` |

The per-bit patterns, reading each 4-bit nibble:

| WS2812 data bit | 4-bit SPI pattern | Decimal | Meaning                            |
|:----------------|:------------------|:--------|:-----------------------------------|
| `0`             | `1000`            | `0x8`   | High for 1 clock, low for 3 clocks |
| `1`             | `1110`            | `0xE`   | High for 3 clocks, low for 1 clock |

At 2–3.8 MHz SPI clock and 4 SPI clocks per WS2812 bit (period = 1000–500 ns):
- WS2812 `0`: T_high ≈ 250–500 ns, T_low ≈ 750–1500 ns — meets T0H ≤ 500 ns, T0L ≥ 850 ns requirement.
- WS2812 `1`: T_high ≈ 750–1500 ns, T1L ≥ 200 ns — meets T1H ≥ 700 ns requirement.

### How bits are extracted

```rust
fn write_byte(&mut self, mut data: u8) -> Result<(), Error<E>> {
    let patterns = [0b1000_1000, 0b1000_1110, 0b11101000, 0b11101110];

    if self.index > self.data.len() - 4 {
        return Err(Error::OutOfBounds);
    }
    for _ in 0..4 {
        let bits = (data & 0b1100_0000) >> 6;  // extract top 2 bits → index 0..3
        self.data[self.index] = patterns[bits as usize];
        self.index += 1;
        data <<= 2;                              // shift next 2 bits into position
    }
    Ok(())
}
```

Each color byte produces **4 output bytes** (one per two-bit pair, MSB-first).
One 24-bit LED color (3 bytes × 4 output bytes each) = **12 output bytes per LED**.

---

## Q3 — Buffer size formula

### Baseline (no feature flags)

```
min_buffer_bytes = 12 * num_leds
```

By default, the reset pulse is sent as a separate SPI transaction by `send_reset()` after
the pixel data transaction completes.
`send_reset` does **not** use the caller-provided buffer — it calls `spi.write(&[0])` 140 times
in a separate loop.
Therefore, the baseline buffer only needs to hold the encoded pixel data.

### With feature flags

The buffer must be enlarged when features cause the reset bytes to be written into the buffer:

| Feature flags active       | Extra bytes | Total formula         |
|:---------------------------|:------------|:----------------------|
| Neither                    | 0           | `12 * num_leds`       |
| `reset_single_transaction` | +140        | `12 * num_leds + 140` |
| `mosi_idle_high`           | +140        | `12 * num_leds + 140` |
| Both                       | +280        | `12 * num_leds + 280` |

`RESET_DATA_LEN = 140` is a constant in the source.
The 140-byte calculation is documented in `lib.rs` as:
> "Should be > 300 µs, so for an SPI Freq. of 3.8 MHz, we have to send at least 1140 low bits or 140 low bytes."

### Why 12 bytes per LED

```
24 WS2812 data bits per LED
× 4 SPI bits per WS2812 data bit
= 96 SPI bits per LED
÷ 8 bits per byte
= 12 bytes per LED
```

### Historical correction to `docs/research-avr-ws2812.md`

An earlier revision of that document stated `12 * num_leds + 20`, which was wrong in two ways:
the trailing constant is 0 (baseline), 140, or 280 — never 20.
The 20-byte figure does not appear anywhere in the `ws2812-spi` v0.5.1 source.
The research document has since been corrected and now matches the values here.

---

## Q4 — SPI clock frequency

**2 MHz to 3.8 MHz for WS2812** (2.3 MHz to 3.8 MHz for SK6812W).

The SPI mode is `CPOL=IdleLow`, `CPHA=CaptureOnFirstTransition` (MODE 0).

On a 16 MHz ATmega328P, hardware SPI divisors are 2 (8 MHz), 4 (4 MHz), 8 (2 MHz), and 16 (1 MHz).
Only **÷8 = 2 MHz** falls within the acceptable range.
This is the correct divisor for AVR WS2812 SPI use.

---

## Q5 — GRB byte ordering

**Yes, GRB order is applied before bit encoding.**

The `write` implementation for WS2812 emits color channels in this sequence:

```rust
self.write_byte(item.g)?;  // Green first
self.write_byte(item.r)?;  // Red second
self.write_byte(item.b)?;  // Blue third
```

The caller passes `RGB8` values (as used by `smart-leds-trait`).
`ws2812-spi` internally swaps to GRB before encoding.
A `prerender_spi` function in `bunting` must therefore either:

- Accept pre-ordered GRB bytes directly and document that expectation, or
- Accept `Rgb` values and apply the GRB swap internally (as `ws2812-spi` does) before encoding.

The second approach is consistent with how `bunting` already handles GRB conversion
(see `rgb_to_grb` in `bunting`).

---

## Encoding Summary — Implementation Reference

For a `prerender_spi` function in `bunting`:

```
Input:  &[Rgb]  — RGB pixel values, one per LED
Output: &mut [u8]  — caller-provided buffer, must be ≥ 12 * num_leds bytes

Per LED:
  1. Convert RGB → GRB  (g, r, b byte order)
  2. For each of the 3 bytes [g, r, b]:
       For each of 4 two-bit pairs (MSB-first):
         bits = (byte & 0b1100_0000) >> 6
         output[i] = patterns[bits]   where patterns = [0x88, 0x8E, 0xE8, 0xEE]
         byte <<= 2
         i += 1

SPI encoding constants:
  WS2812 bit 0  →  SPI nibble 0b1000  (high 1, low 3)
  WS2812 bit 1  →  SPI nibble 0b1110  (high 3, low 1)
  Two-bit lookup table: [0b10001000, 0b10001110, 0b11101000, 0b11101110]

Reset pulse (separate, not in pixel buffer):
  Send 140 zero bytes via SPI after the pixel buffer
  At 3.8 MHz: 140 × 8 bits / 3.8 MHz ≈ 295 µs ≥ 280 µs WS2812 reset threshold
```

---

## Sources

- [smart-leds-rs/ws2812-spi-rs source — prerendered.rs](https://raw.githubusercontent.com/smart-leds-rs/ws2812-spi-rs/master/src/prerendered.rs)
- [smart-leds-rs/ws2812-spi-rs source — lib.rs](https://raw.githubusercontent.com/smart-leds-rs/ws2812-spi-rs/master/src/lib.rs)
- [ws2812-spi 0.5.1 prerendered struct docs — docs.rs](https://docs.rs/ws2812-spi/0.5.1/ws2812_spi/prerendered/struct.Ws2812.html)
- [ws2812-spi on crates.io](https://crates.io/crates/ws2812-spi)

*Research date: 2026-03-13*

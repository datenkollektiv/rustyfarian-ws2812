# Bench — Arduino Nano (ATmega328P) with a 12-LED WS2812B ring

```sh
just bench avr
```

The Nano is found by its USB identity (FTDI FT232R, `0403:6001`), not by espflash.

Wiring: D11 through 330 Ω to DIN, GND to GND, 5V to VCC.

| Check     | Tier         | Examples           | Verdict                       |
|:----------|:-------------|:-------------------|:------------------------------|
| `pulse`   | AVR bit-bang | `bitbang_demo`     | blue breathing on all 12 LEDs |
| `rainbow` | AVR bit-bang | `avr-nano-rainbow` | rotating rainbow              |

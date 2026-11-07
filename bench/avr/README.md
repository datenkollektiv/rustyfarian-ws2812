# Bench — Arduino Nano (ATmega328P) with a 12-LED WS2812B ring

```sh
just bench avr --port /dev/cu.usbserial-XXXX
```

The Nano is not detected, so the port is required; `ls /dev/cu.usb*` before and after plugging it in shows which one.

Wiring: D11 through 330 Ω to DIN, GND to GND, 5V to VCC.

| Check     | Tier         | Examples           | Verdict                       |
|:----------|:-------------|:-------------------|:------------------------------|
| `pulse`   | AVR bit-bang | `bitbang_demo`     | blue breathing on all 12 LEDs |
| `rainbow` | AVR bit-bang | `avr-nano-rainbow` | rotating rainbow              |

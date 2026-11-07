# Bench — ESP32-C6 with a 12-LED WS2812B ring

```sh
just bench c6
```

Wiring: GPIO18 through 330 Ω to DIN, GND to GND, 3V3 to VCC.

| Check     | Tier                   | Examples                             | Verdict                                 |
|:----------|:-----------------------|:-------------------------------------|:----------------------------------------|
| `pulse`   | esp-hal, esp-hal async | `hal_c6_pulse`, `hal_c6_pulse_async` | blue breathing on all 12 LEDs, no crash |
| `rainbow` | ESP-IDF                | `idf_c6_rainbow`                     | rotating rainbow, app started           |

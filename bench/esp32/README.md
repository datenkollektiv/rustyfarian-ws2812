# Bench — ESP32-WROOM-32 DevKit with a 12-LED WS2812B ring

```sh
just bench esp32
```

Wiring: GPIO4 through 330 Ω to DIN, GND to GND, 3V3 to VCC.

| Check     | Tier                   | Examples                                   | Verdict                                 |
|:----------|:-----------------------|:-------------------------------------------|:----------------------------------------|
| `pulse`   | esp-hal, esp-hal async | `hal_esp32_pulse`, `hal_esp32_pulse_async` | blue breathing on all 12 LEDs, no crash |
| `rainbow` | ESP-IDF                | `idf_esp32_rainbow`                        | rotating rainbow, app started           |

The RGB adapter examples (`idf_esp32_rgb_cycle`, `idf_esp32_rgb_pulse`) are not benched: they need a different wiring.

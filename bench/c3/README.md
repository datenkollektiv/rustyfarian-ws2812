# Bench — ESP32-C3 with a 12-LED WS2812B ring

```sh
just bench c3
```

Wiring: GPIO4 through 330 Ω to DIN, GND to GND, 3V3 to VCC; connect the board over its native USB port so the log arrives.

| Check         | Tier                            | Examples                                             | Verdict                                      |
|:--------------|:--------------------------------|:-----------------------------------------------------|:---------------------------------------------|
| `pulse`       | esp-hal, esp-hal async, ESP-IDF | `hal_c3_pulse`, `hal_c3_pulse_async`, `idf_c3_pulse` | blue breathing on all 12 LEDs, no crash      |
| `rainbow`     | ESP-IDF                         | `idf_c3_rainbow`                                     | rotating rainbow, app started                |
| `colors`      | esp-hal, ESP-IDF                | `hal_c3_colors`, `idf_c3_colors`                     | solid red, green, blue, white, off, 3 s each |
| `comet`       | esp-hal async                   | `hal_c3_rainbow_comet_async`                         | three reports with `0 errors`, smooth comet  |
| `frame-sweep` | esp-hal async                   | `hal_c3_frame_sweep_async`                           | two cycles `PASS (0 failures)`, log only     |

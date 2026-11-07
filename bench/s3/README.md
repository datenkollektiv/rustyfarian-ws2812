# Bench — CrowPanel 1.28" HMI (ESP32-S3)

```sh
just bench s3
```

No wiring: the 5-LED WS2812 ring is on board, data on GPIO48.

| Check     | Tier                   | Examples                             | Verdict                                |
|:----------|:-----------------------|:-------------------------------------|:---------------------------------------|
| `pulse`   | esp-hal, esp-hal async | `hal_s3_pulse`, `hal_s3_pulse_async` | blue breathing on all 5 LEDs, no crash |
| `rainbow` | ESP-IDF                | `idf_s3_rainbow`                     | rotating rainbow, app started          |

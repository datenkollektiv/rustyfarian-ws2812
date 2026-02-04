# ADR 002: Rainbow Effect LED Limit

## Status

Accepted

## Context

The `RainbowEffect` in `led-effects` distributes hue values (0–255) across LEDs in a ring to create a rainbow gradient.
The original implementation used this calculation:

```rust
let hue_per_led = 256u16 / self.num_leds as u16;
let led_hue = (i as u16 * hue_per_led) as u8;
```

This approach has a bug: when `num_leds > 256`, integer division yields `hue_per_led == 0`, causing all LEDs to display the same color.

Two solutions were considered:

1. **Fix the formula for arbitrary sizes** using multiplication-first arithmetic:
   `let led_hue = ((i as u32 * 256) / num_leds as u32) as u8`
   This works for any ring size but provides diminishing hue resolution as `num_leds` increases.

2. **Limit the ring size** to a reasonable maximum (256 LEDs) and validate at construction time.

## Decision

**Limit `RainbowEffect` to a maximum of 256 LEDs.**

The implementation:

- Defines `MAX_LEDS = 256` as a public constant
- Returns `EffectError::TooManyLeds` if the limit is exceeded
- Uses the improved multiplication-first formula for correct hue distribution within the limit

## Rationale

- **Practical use cases**: Common LED ring sizes are 8, 12, 16, 24, 60, or occasionally 144 LEDs.
  Rings with more than 256 individually addressable LEDs are rare.
- **Hue resolution**: With 256 hue values in the HSV model, having more than 256 LEDs means some LEDs would share the same hue anyway.
  The rainbow effect provides no visual benefit beyond this limit.
- **Simplicity**: Explicit limits make the API predictable and avoid surprising behavior at edge cases.
- **Fail-fast**: Returning an error at construction time is better than silently producing poor results at runtime.

## Consequences

**Positive:**

- Clear, documented API contract
- Fail-fast error handling prevents silent misbehavior
- Simple integer math without complex scaling logic
- Correct hue distribution for all supported ring sizes

**Negative:**

- Users with LED installations > 256 LEDs would need to segment them into multiple rings or use a different effect implementation

**Migration:**

None required.
This is a new constraint on a new API.

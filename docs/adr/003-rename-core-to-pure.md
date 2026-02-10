# ADR 003: Rename `ws2812-core` to `ws2812-pure`

## Status

Accepted

## Context

The workspace contains a crate named `ws2812-core` that provides pure Rust WS2812 color utilities (RGB-to-GRB conversion, bit encoding) with no hardware dependencies.
This crate embodies the project's core philosophy: **extract testable pure logic from hardware-specific code**.

The naming question arose in the context of the **sans-io pattern**—a design approach that separates protocol/logic from I/O operations.
Pure functions take inputs and produce outputs without side effects, making them:

- Testable without hardware or mocking
- Reusable across different I/O backends
- Easier to reason about and verify

The current `-core` suffix is a common Rust convention (e.g., `tokio-core`, `hyper-core`) but has an overloaded meaning.
It typically implies "foundational functionality" but does not communicate the **purity** or **testability** aspects that distinguish this crate.

The question: should we rename `ws2812-core` to `ws2812-pure` to better express its sans-io nature?

## Decision

Rename `ws2812-core` to `ws2812-pure`.

## Rationale

### Arguments for `ws2812-pure`

| Factor                   | Analysis                                                                                                  |
|:-------------------------|:----------------------------------------------------------------------------------------------------------|
| **Explicit intent**      | "pure" directly communicates the sans-io/no-side-effects nature                                           |
| **Philosophy alignment** | Project docs state "Extract testable pure logic from hardware-specific code"                              |
| **Consistency**          | README already describes it as "Pure Rust WS2812 utilities"                                               |
| **Searchability**        | Developers familiar with functional programming or sans-io patterns may search for "pure" implementations |
| **Teaching value**       | Every `use ws2812_pure::...` reinforces the architectural boundary to users                               |
| **Differentiation**      | Clearly distinguishes this crate from the hardware-specific `rustyfarian-esp-idf-ws2812`                            |

### Arguments against (keeping `ws2812-core`)

| Factor              | Analysis                                                                  |
|:--------------------|:--------------------------------------------------------------------------|
| **Rust convention** | `-core` is common, but its meaning is overloaded                          |
| **Breaking change** | Requires downstream updates, but project is young with minimal dependents |
| **Familiarity**     | Developers expect `-core`, but `-pure` is equally intuitive               |

### Sans-IO Context

The sans-io pattern originated in the Python community but applies universally.
Key principles:

- Separate "what to do" (pure logic) from "how to do it" (I/O)
- Pure components are infinitely testable
- I/O components become thin wrappers

The name `ws2812-pure` directly signals membership in this architectural category.

## Consequences

### Positive

- **Clearer communication**: The name immediately signals the crate's testability and lack of hardware dependencies
- **Philosophy reinforcement**: Aligns with documented project principles
- **Import clarity**: `use ws2812_pure::rgb_to_grb` reads as self-documenting code
- **Ecosystem differentiation**: Stands out among WS2812 crates that mix pure and impure code

### Negative

- **Breaking change**: Downstream projects must update their `Cargo.toml` and imports
- **Unfamiliar suffix**: `-pure` is less common than `-core` in the Rust ecosystem (though equally valid)

### Migration

Downstream projects update:

```toml
# Before
ws2812-core = { git = "..." }

# After
ws2812-pure = { git = "..." }
```

```rust
// Before
use ws2812_core::rgb_to_grb;

// After
use ws2812_pure::rgb_to_grb;
```

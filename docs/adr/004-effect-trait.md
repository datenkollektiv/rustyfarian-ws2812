# ADR 004: Effect Trait for Polymorphic LED Animations

## Status

Accepted

## Context

The `ferriswheel` crate provides multiple LED ring effects (`RainbowEffect`, `PulseEffect`, `SpinnerEffect`, `ProgressEffect`).
Each effect has the same three core methods: `update()`, `current()`, and `reset()`.
Without a shared trait, users cannot store or switch between effects dynamically at runtime.

Two approaches were considered:

1. **Enum-based dispatch** — a single enum wrapping all effect variants.
   Simple but requires modifying the enum whenever a new effect is added.

2. **Trait object dispatch** — a common `Effect` trait that all effects implement.
   Extensible and allows users to implement their own effects.

The crate is `no_std`, so `Box<dyn Effect>` (which requires `alloc`) cannot be assumed.
However, `&dyn Effect` and `&mut dyn Effect` work without an allocator.

## Decision

**Define an `Effect` trait with `update`, `current`, and `reset` methods.**

```rust
pub trait Effect {
    fn update(&mut self, buffer: &mut [RGB8]) -> Result<(), EffectError>;
    fn current(&self, buffer: &mut [RGB8]) -> Result<(), EffectError>;
    fn reset(&mut self);
}
```

Each effect struct retains its inherent methods with the same signatures.
The `impl Effect for T` block delegates to the inherent methods.
Inherent methods take precedence over trait methods in method resolution,
so `self.update()` inside the trait impl calls the inherent method without recursion.

## Consequences

**Positive:**

- Effects can be used polymorphically via `&dyn Effect` or `&mut dyn Effect`
- Users can implement custom effects against the same trait
- No `alloc` dependency required for trait object usage
- Existing code calling inherent methods continues to work unchanged

**Negative:**

- Trait objects cannot access effect-specific methods (e.g., `set_progress()` on `ProgressEffect`)
- Slight indirection cost for dynamic dispatch (negligible on embedded targets with small vtables)

#![cfg_attr(not(test), no_std)]
//! RGB LED ring effects and animations.
//!
//! This crate provides reusable animation effects for circular LED rings.
//! It is `no_std` compatible for embedded use.
//!
//! All effects implement the [`Effect`] trait, which provides a uniform
//! interface for rendering animations into an `RGB8` buffer.
//!
//! # Available Effects
//!
//! - [`RainbowEffect`] — smooth rainbow gradient rotation
//! - [`PulseEffect`] — heartbeat animation: sine-wave brightness with a pause at the floor
//! - [`BreatheEffect`] — breathing animation: symmetric full-sine brightness with no pause
//! - [`SpinnerEffect`] — rotating dot with fading tail (linear decay, brightness floor)
//! - [`MeteorEffect`] — meteor / comet: bright head with exponentially-decaying tail
//! - [`TwinkleEffect`] — ambient sparkle: random LEDs flash to peak brightness then decay
//! - [`FireEffect`] — fire simulation: heat diffuses upward from a sparking base
//! - [`CylonEffect`] — Cylon / bouncing scanner: bright head sweeps back and forth with fading tail
//! - [`KnightRiderEffect`] — Knight Rider / dual-headed scanner: two heads sweep in opposite directions
//! - [`ChaseEffect`] — moving a solid segment around the ring
//! - [`FlashEffect`] — rapid on/off toggle with configurable duty cycle
//! - [`ProgressEffect`] — proportional ring fill
//! - [`SectionEffect`] — weighted color sections on a ring
//! - [`RainbowCometEffect`] — Rainbow comet: orbiting head with a hue-cycling fading tail
//!
//! # Utilities
//!
//! - [`ColorPalette`] — three-color theme for effects
//! - [`fill_solid`] — fill a buffer with a single color
//! - [`sine_wave`] — half-wave rectified sine lookup (0→255→0 then plateau at 0)
//! - [`sine_full`] — full symmetric sine lookup (0→255→0, no plateau)
//! - [`scale_brightness`] — scale an RGB color's brightness
//! - [`lerp_color`] — linearly interpolate between two colors
//!
//! # Color type
//!
//! All effects operate on [`RGB8`], re-exported here from the [`rgb`] crate.
//! Downstream crates do not need a direct `rgb` dependency — `use ferriswheel::RGB8`
//! is sufficient.
//!
//! # Example
//!
//! ```
//! use ferriswheel::{Effect, RainbowEffect, Direction, RGB8};
//!
//! let mut rainbow = RainbowEffect::new(12).unwrap();
//! let mut buffer = [RGB8::default(); 12];
//!
//! // Fill the buffer with rainbow colors and advance animation
//! rainbow.update(&mut buffer).unwrap();
//!
//! // Use as a trait object
//! let effect: &dyn Effect = &rainbow;
//! effect.current(&mut buffer).unwrap();
//! ```

mod breathe;
mod chase;
mod cylon;
mod effect;
mod fire;
mod flash;
mod hsv;
mod knight_rider;
mod meteor;
mod palette;
mod progress;
mod pulse;
mod rainbow;
mod rainbow_comet;
mod section;
mod spinner;
mod twinkle;
mod util;

pub use rgb::RGB8;

pub use breathe::BreatheEffect;
pub use chase::ChaseEffect;
pub use cylon::CylonEffect;
pub use effect::{Direction, Effect, EffectError, MAX_LEDS};
pub use fire::FireEffect;
pub use flash::FlashEffect;
pub use hsv::hsv_to_rgb;
pub use knight_rider::KnightRiderEffect;
pub use meteor::MeteorEffect;
pub use palette::ColorPalette;
pub use progress::ProgressEffect;
pub use pulse::PulseEffect;
pub use rainbow::RainbowEffect;
pub use rainbow_comet::RainbowCometEffect;
pub use section::{SectionEffect, MAX_SECTIONS};
pub use spinner::SpinnerEffect;
pub use twinkle::TwinkleEffect;
pub use util::{fill_solid, lerp_color, scale_brightness, sine_full, sine_wave};

// When `smart-leds-compat` is enabled, this forces `smart-leds-trait` into the
// dependency graph and makes compilation fail if its `rgb::RGB8` differs from
// ours — i.e. if the two crates resolve to incompatible `rgb` versions.
#[cfg(feature = "smart-leds-compat")]
fn _assert_rgb8_same(c: rgb::RGB8) -> smart_leds_trait::RGB8 {
    c
}

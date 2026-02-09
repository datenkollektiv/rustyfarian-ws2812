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
//! - [`PulseEffect`] — sine-wave breathing animation
//! - [`SpinnerEffect`] — rotating dot with fading tail
//! - [`ProgressEffect`] — proportional ring fill
//!
//! # Utilities
//!
//! - [`fill_solid`] — fill a buffer with a single color
//! - [`sine_wave`] — sine lookup for smooth animations
//! - [`scale_brightness`] — scale an RGB color's brightness
//! - [`lerp_color`] — linearly interpolate between two colors
//!
//! # Example
//!
//! ```
//! use ferriswheel::{Effect, RainbowEffect, Direction};
//! use rgb::RGB8;
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

mod effect;
mod hsv;
mod progress;
mod pulse;
mod rainbow;
mod spinner;
mod util;

pub use effect::{Direction, Effect, EffectError, MAX_LEDS};
pub use hsv::hsv_to_rgb;
pub use progress::ProgressEffect;
pub use pulse::PulseEffect;
pub use rainbow::RainbowEffect;
pub use spinner::SpinnerEffect;
pub use util::{fill_solid, lerp_color, scale_brightness, sine_wave};

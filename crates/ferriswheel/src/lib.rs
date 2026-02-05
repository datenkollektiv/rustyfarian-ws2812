#![cfg_attr(not(test), no_std)]
//! RGB LED ring effects and animations.
//!
//! This crate provides reusable animation effects for circular LED rings.
//! It is `no_std` compatible for embedded use.
//!
//! # RainbowEffect
//!
//! The [`RainbowEffect`] creates smooth rainbow animations for LED rings of any size.
//! It uses pure integer math for HSV-to-RGB conversion, making it suitable for
//! embedded systems without floating-point support.
//!
//! # Example
//!
//! ```
//! use ferriswheel::{RainbowEffect, Direction};
//! use rgb::RGB8;
//!
//! let mut rainbow = RainbowEffect::new(12).unwrap();
//! let mut buffer = [RGB8::default(); 12];
//!
//! // Fill the buffer with rainbow colors and advance animation
//! rainbow.update(&mut buffer).unwrap();
//! ```

mod hsv;
mod rainbow;

pub use hsv::hsv_to_rgb;
pub use rainbow::{Direction, EffectError, RainbowEffect, MAX_LEDS};

use core::convert::Infallible;

use rgb::RGB8;

use crate::StatusLed;

/// A no-op LED stub that satisfies the [`StatusLed`] trait without any hardware dependency.
///
/// Use `NoLed` when a type parameter requires a [`StatusLed`] implementation but no
/// physical LED is present — for example in unit tests, CI environments, or board
/// configurations that omit the status LED.
///
/// All operations succeed silently; no color data is stored or transmitted.
///
/// # Example
///
/// ```
/// use led_effects::{NoLed, StatusLed};
/// use rgb::RGB8;
///
/// let mut led = NoLed::default();
/// led.set_color(RGB8::new(255, 0, 0)).unwrap(); // always Ok
/// ```
#[derive(Copy, Clone, Default, Debug)]
pub struct NoLed;

impl StatusLed for NoLed {
    type Error = Infallible;

    fn set_color(&mut self, _color: RGB8) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_color_always_returns_ok() {
        let mut led = NoLed;
        assert!(led.set_color(RGB8::new(255, 0, 0)).is_ok());
        assert!(led.set_color(RGB8::new(0, 255, 0)).is_ok());
        assert!(led.set_color(RGB8::new(0, 0, 255)).is_ok());
        assert!(led.set_color(RGB8::new(0, 0, 0)).is_ok());
        assert!(led.set_color(RGB8::new(255, 255, 255)).is_ok());
    }

    #[test]
    fn error_type_is_infallible() {
        let mut led = NoLed;
        // Unwrap is safe because the error type is Infallible
        let _: () = led.set_color(RGB8::new(128, 64, 32)).unwrap();
    }
}

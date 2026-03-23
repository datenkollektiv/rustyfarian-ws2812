use core::convert::Infallible;

use rgb::RGB8;

use crate::AsyncStatusLed;
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

impl AsyncStatusLed for NoLed {
    type Error = Infallible;

    async fn set_color(&mut self, _color: RGB8) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_color_always_returns_ok() {
        let mut led = NoLed;
        assert!(StatusLed::set_color(&mut led, RGB8::new(255, 0, 0)).is_ok());
        assert!(StatusLed::set_color(&mut led, RGB8::new(0, 255, 0)).is_ok());
        assert!(StatusLed::set_color(&mut led, RGB8::new(0, 0, 255)).is_ok());
        assert!(StatusLed::set_color(&mut led, RGB8::new(0, 0, 0)).is_ok());
        assert!(StatusLed::set_color(&mut led, RGB8::new(255, 255, 255)).is_ok());
    }

    #[test]
    fn error_type_is_infallible() {
        let mut led = NoLed;
        // Unwrap is safe because the error type is Infallible
        let _: () = StatusLed::set_color(&mut led, RGB8::new(128, 64, 32)).unwrap();
    }

    fn block_on<F: core::future::Future>(f: F) -> F::Output {
        use core::pin::pin;
        use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(core::ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        // SAFETY: The waker is a no-op; it is only valid for futures that
        // complete in a single poll (i.e. never return Poll::Pending), which
        // is guaranteed for NoLed's async impls.
        let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut f = pin!(f);
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("future returned Pending"),
        }
    }

    #[test]
    fn async_set_color_always_returns_ok() {
        let mut led = NoLed;
        block_on(async {
            assert!(AsyncStatusLed::set_color(&mut led, RGB8::new(255, 0, 0))
                .await
                .is_ok());
            assert!(AsyncStatusLed::set_color(&mut led, RGB8::new(0, 0, 0))
                .await
                .is_ok());
        });
    }

    #[test]
    fn async_error_type_is_infallible() {
        let mut led = NoLed;
        block_on(async {
            let _: () = AsyncStatusLed::set_color(&mut led, RGB8::new(128, 64, 32))
                .await
                .unwrap();
        });
    }
}

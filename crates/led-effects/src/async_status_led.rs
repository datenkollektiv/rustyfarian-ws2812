use rgb::RGB8;

/// Async counterpart of [`StatusLed`](crate::StatusLed) for drivers with async I/O.
///
/// This trait mirrors `StatusLed` but uses `async fn`, allowing implementations
/// to yield to an executor during hardware operations (e.g., RMT transmission).
///
/// # Object safety
///
/// This trait is **not** object-safe — `dyn AsyncStatusLed` will not compile.
/// Each implementation produces a distinct future type that cannot be erased
/// through a vtable. Use generics or enum dispatch instead.
///
/// # Example
///
/// ```ignore
/// use led_effects::AsyncStatusLed;
/// use rgb::RGB8;
///
/// async fn set_status(led: &mut impl AsyncStatusLed) {
///     led.set_color(RGB8::new(0, 255, 0)).await.unwrap();
/// }
/// ```
pub trait AsyncStatusLed {
    /// The error type returned by async LED operations.
    type Error;

    /// Sets the LED to the specified color, yielding during hardware I/O.
    async fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error>;
}

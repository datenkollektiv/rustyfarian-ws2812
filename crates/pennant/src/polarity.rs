//! Wiring polarity shared by the `hal` adapters.
//!
//! Holds the [`Polarity`] enum, used by `SimpleLed`, `RgbGpioLed` and `RgbPwmLed`,
//! and the on/off channel helper used by the first two.

use embedded_hal::digital::OutputPin;

/// Wiring polarity of a discrete LED channel.
///
/// Selects which pin level corresponds to the **lit / maximum-brightness** state.
/// Shared by [`RgbGpioLed`](crate::RgbGpioLed) (on/off) and [`RgbPwmLed`](crate::RgbPwmLed) (PWM); for
/// the PWM adapter, [`ActiveLow`](Polarity::ActiveLow) means the channel is
/// brightest at minimum duty (the pin spends the least time high), so the duty is
/// the inverse of brightness.
/// [`SimpleLed`](crate::SimpleLed) also honours it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Polarity {
    /// The channel is brightest when the pin is driven **high** (common-cathode).
    #[default]
    ActiveHigh,
    /// The channel is brightest when the pin is driven **low** (common-anode).
    ActiveLow,
}

/// Drives a single channel pin to reflect the desired on/off state, honouring
/// polarity.
///
/// Shared by the on/off adapters.
#[inline]
pub(crate) fn set_channel<P: OutputPin>(
    pin: &mut P,
    on: bool,
    polarity: Polarity,
) -> Result<(), P::Error> {
    let drive_high = match polarity {
        Polarity::ActiveHigh => on,
        Polarity::ActiveLow => !on,
    };
    if drive_high {
        pin.set_high()
    } else {
        pin.set_low()
    }
}

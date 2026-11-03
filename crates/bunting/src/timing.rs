//! WS2812 pulse timings shared by the driver crates.

/// Time the data line stays high for a `0` bit, in nanoseconds.
///
/// 300 ns sits inside every datasheet window found on 2026-11-03: WS2812 350 ±150 ns, legacy WS2812B 400 ±150 ns, WS2812B-V5 220–380 ns.
/// It is also a whole 100 ns tick, whereas 350 ns is not.
pub const WS2812_T0H_NS: u32 = 300;

/// Time the data line stays low for a `0` bit, in nanoseconds.
pub const WS2812_T0L_NS: u32 = 800;

/// Time the data line stays high for a `1` bit, in nanoseconds.
pub const WS2812_T1H_NS: u32 = 700;

/// Time the data line stays low for a `1` bit, in nanoseconds.
pub const WS2812_T1L_NS: u32 = 600;

/// Minimum low time that latches a frame, in microseconds.
///
/// The WS2812B datasheet asks for at least 280 µs; older parts latch after 50 µs.
/// The longer value is safe for both.
pub const WS2812_RESET_US: u32 = 280;

/// Converts a duration in nanoseconds to whole ticks of `tick_ns` nanoseconds.
///
/// Returns `None` when `tick_ns` is zero, when `ns` is not a whole multiple of `tick_ns`, or when the tick count does not fit in a `u16`.
/// It can be evaluated at compile time, so a timing that would silently round fails the build.
///
/// # Example
///
/// ```
/// use bunting::{ns_to_ticks, WS2812_T0H_NS};
///
/// assert_eq!(ns_to_ticks(WS2812_T0H_NS, 100), Some(3));
/// assert_eq!(ns_to_ticks(350, 100), None);
/// ```
// `u32::is_multiple_of` needs a newer rustc than the workspace MSRV.
#[allow(unknown_lints, clippy::manual_is_multiple_of)]
pub const fn ns_to_ticks(ns: u32, tick_ns: u32) -> Option<u16> {
    if tick_ns == 0 || ns % tick_ns != 0 {
        return None;
    }
    let ticks = ns / tick_ns;
    if ticks > u16::MAX as u32 {
        return None;
    }
    Some(ticks as u16)
}

const _: () = {
    match ns_to_ticks(WS2812_T0H_NS, 100) {
        Some(_) => {}
        None => panic!("T0H is not a whole 100 ns tick"),
    }
    match ns_to_ticks(WS2812_T0L_NS, 100) {
        Some(8) => {}
        _ => panic!("T0L must be 8 ticks of 100 ns"),
    }
    match ns_to_ticks(WS2812_T1H_NS, 100) {
        Some(_) => {}
        None => panic!("T1H is not a whole 100 ns tick"),
    }
    match ns_to_ticks(WS2812_T1L_NS, 100) {
        Some(_) => {}
        None => panic!("T1L is not a whole 100 ns tick"),
    }
    assert!(
        ns_to_ticks(350, 100).is_none(),
        "350 ns must be rejected as a partial tick"
    );
    assert!(
        WS2812_T0H_NS < WS2812_T1H_NS,
        "T0H must be shorter than T1H"
    );
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_timings_are_whole_ticks_at_100_ns() {
        assert_eq!(ns_to_ticks(WS2812_T0H_NS, 100), Some(3));
        assert_eq!(ns_to_ticks(WS2812_T0L_NS, 100), Some(8));
        assert_eq!(ns_to_ticks(WS2812_T1H_NS, 100), Some(7));
        assert_eq!(ns_to_ticks(WS2812_T1L_NS, 100), Some(6));
    }

    #[test]
    fn zero_tick_period_is_none() {
        assert_eq!(ns_to_ticks(300, 0), None);
    }

    #[test]
    fn partial_tick_is_none() {
        assert_eq!(ns_to_ticks(350, 100), None);
    }

    #[test]
    fn quotient_overflow_is_none() {
        assert_eq!(ns_to_ticks(u32::MAX, 1), None);
        assert_eq!(ns_to_ticks(u16::MAX as u32, 1), Some(u16::MAX));
        assert_eq!(ns_to_ticks(u16::MAX as u32 + 1, 1), None);
    }

    #[test]
    fn zero_ns_is_zero_ticks() {
        assert_eq!(ns_to_ticks(0, 100), Some(0));
    }
}

//! Clock-free helpers for the WS2812 latch interval.
//!
//! Every function takes the current time as a `u64` argument in microseconds and never reads a clock.
//! The tier that uses them keeps the clock read and the busy-wait.

use crate::WS2812_RESET_US;

/// Consecutive unchanged timer reads after which a wait treats the timer as stopped.
///
/// A running timer advances by 1 µs every few reads (one read costs roughly 0.2–1 µs),
/// so 1000 identical reads only happen when the time source is not running.
/// The wait is then skipped after well under a millisecond instead of hanging.
pub const LATCH_WAIT_MAX_STALLED_READS: u32 = 1000;

/// The time [`WS2812_RESET_US`] after `now_us`, in microseconds.
///
/// Uses a plain addition, like adding a duration to an instant.
#[inline(always)]
pub const fn latch_deadline_us(now_us: u64) -> u64 {
    now_us + WS2812_RESET_US as u64
}

/// Microseconds left until `target_us`, given the current time `now_us`.
///
/// Zero when the target has passed.
/// The result is not capped.
#[inline(always)]
pub const fn remaining_us(now_us: u64, target_us: u64) -> u64 {
    target_us.saturating_sub(now_us)
}

/// Converts the remaining latch time in nanoseconds into whole microseconds to wait.
///
/// Any sub-microsecond remainder rounds up, and the result is capped at [`WS2812_RESET_US`].
/// This differs from [`remaining_us`], which neither rounds nor caps.
pub const fn latch_wait_us_from_ns(remaining_ns: u64) -> u32 {
    let us = remaining_ns.div_ceil(1000);
    if us > WS2812_RESET_US as u64 {
        WS2812_RESET_US
    } else {
        us as u32
    }
}

/// Detects a stopped clock by counting consecutive unchanged reads.
#[derive(Clone, Copy, Debug)]
pub struct StallGuard {
    previous_us: u64,
    stalled_reads: u32,
}

impl StallGuard {
    /// Creates a guard whose first comparison is against `previous_us`.
    pub const fn new(previous_us: u64) -> Self {
        Self {
            previous_us,
            stalled_reads: 0,
        }
    }

    /// Records a clock read and returns `true` once [`LATCH_WAIT_MAX_STALLED_READS`] consecutive reads were unchanged.
    #[inline(always)]
    pub fn observe(&mut self, now_us: u64) -> bool {
        if now_us == self.previous_us {
            self.stalled_reads += 1;
            self.stalled_reads >= LATCH_WAIT_MAX_STALLED_READS
        } else {
            self.stalled_reads = 0;
            self.previous_us = now_us;
            false
        }
    }
}

/// State of a busy-wait for a latch target, fed with clock reads by the tier.
#[derive(Clone, Copy, Debug)]
pub struct LatchWait {
    target_us: u64,
    guard: StallGuard,
}

impl LatchWait {
    /// Starts a wait for `target_us`, seeded with the clock read taken before the loop.
    pub const fn new(target_us: u64, first_now_us: u64) -> Self {
        Self {
            target_us,
            guard: StallGuard::new(first_now_us),
        }
    }

    /// Returns `true` when the wait is over: the target has passed, or the clock looks stopped.
    ///
    /// The target is checked before the stall guard.
    #[inline]
    pub fn done(&mut self, now_us: u64) -> bool {
        if remaining_us(now_us, self.target_us) == 0 {
            return true;
        }
        self.guard.observe(now_us)
    }
}

const _: () = {
    assert!(latch_deadline_us(0) == 280);
    assert!(remaining_us(1000, 1280) == 280);
    assert!(remaining_us(1279, 1280) == 1);
    assert!(remaining_us(1280, 1280) == 0);
    assert!(remaining_us(1500, 1280) == 0);
    assert!(latch_wait_us_from_ns(100_001) == 101);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_us_from_ns_rounds_up_and_caps() {
        assert_eq!(latch_wait_us_from_ns(0), 0);
        assert_eq!(latch_wait_us_from_ns(1), 1);
        assert_eq!(latch_wait_us_from_ns(100_000), 100);
        assert_eq!(latch_wait_us_from_ns(100_001), 101);
        assert_eq!(latch_wait_us_from_ns(280_000), 280);
        assert_eq!(latch_wait_us_from_ns(280_001), 280);
        assert_eq!(latch_wait_us_from_ns(5_000_000_000), 280);
        assert_eq!(latch_wait_us_from_ns(u64::MAX), 280);
    }

    #[test]
    fn remaining_us_cases() {
        assert_eq!(remaining_us(1000, 1280), 280);
        assert_eq!(remaining_us(1279, 1280), 1);
        assert_eq!(remaining_us(1280, 1280), 0);
        assert_eq!(remaining_us(1500, 1280), 0);
        assert_eq!(remaining_us(0, u64::MAX), u64::MAX);
        assert_eq!(remaining_us(u64::MAX, 0), 0);
        assert_eq!(remaining_us(u64::MAX, u64::MAX), 0);
        assert_eq!(remaining_us(0, 0), 0);
    }

    #[test]
    fn deadline_adds_reset() {
        assert_eq!(latch_deadline_us(0), 280);
        assert_eq!(latch_deadline_us(1234), 1234 + WS2812_RESET_US as u64);
    }

    #[test]
    fn stall_guard_trips_on_nth_unchanged_read() {
        let mut g = StallGuard::new(5);
        for _ in 0..LATCH_WAIT_MAX_STALLED_READS - 1 {
            assert!(!g.observe(5));
        }
        assert!(g.observe(5));
    }

    #[test]
    fn stall_guard_change_resets() {
        let mut g = StallGuard::new(5);
        for _ in 0..LATCH_WAIT_MAX_STALLED_READS - 1 {
            assert!(!g.observe(5));
        }
        assert!(!g.observe(6));
        for _ in 0..LATCH_WAIT_MAX_STALLED_READS - 1 {
            assert!(!g.observe(6));
        }
        assert!(g.observe(6));
    }

    #[test]
    fn stall_guard_first_read_equal_to_seed_counts() {
        let mut g = StallGuard::new(0);
        for _ in 0..LATCH_WAIT_MAX_STALLED_READS - 1 {
            assert!(!g.observe(0));
        }
        assert!(g.observe(0));
    }

    #[test]
    fn latch_wait_done_when_target_passed_even_if_stalled() {
        let mut w = LatchWait::new(100, 200);
        assert!(w.done(200));
        let mut w = LatchWait::new(200, 200);
        assert!(w.done(200));
    }

    #[test]
    fn latch_wait_stopped_clock_ends_after_max() {
        let mut w = LatchWait::new(1000, 10);
        for _ in 0..LATCH_WAIT_MAX_STALLED_READS - 1 {
            assert!(!w.done(10));
        }
        assert!(w.done(10));
    }

    #[test]
    fn latch_wait_advancing_clock_ends_at_target() {
        let mut w = LatchWait::new(1280, 1000);
        for now in 1001..1280 {
            assert!(!w.done(now));
        }
        assert!(w.done(1280));
    }
}

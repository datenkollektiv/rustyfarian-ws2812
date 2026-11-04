//! Clock-free policy and budget helpers for frames longer than one RMT channel RAM.
//!
//! Every function takes the current time as a `u64` argument in microseconds and never reads a clock.
//! Tick constants stay with the tier and are passed in as [`CodeTiming`].

use crate::latch::StallGuard;

/// How a driver sends a frame that does not fit in one RMT channel's RAM.
///
/// Frames that fit are never refilled and are not affected by the policy.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LongFramePolicy {
    /// Busy-poll the transmit inside a critical section, bounded by a timeout.
    ///
    /// All interrupts are blocked for the whole frame.
    /// A frame whose estimated nominal masked time exceeds the ceiling ([`DEFAULT_MAX_NOMINAL_MASKED_US`] by default) is refused instead.
    /// Masking removes interrupt latency from the refill; it does not guarantee the waveform.
    #[default]
    CriticalSection,
    /// Busy-poll the transmit with interrupts live, bounded by a timeout.
    ///
    /// Interrupts, such as radio traffic, can delay the refill past its deadline and garble the frame.
    /// Choose it only when the firmware cannot tolerate interrupts-off windows,
    /// and accept that long frames can be corrupted under radio load.
    Poll,
    /// Refuse the frame without transmitting.
    ///
    /// Choose this when the firmware must never mask interrupts for a frame,
    /// for example because of radio traffic.
    Reject,
}

/// Default ceiling in microseconds on the estimated nominal masked time of one `LongFramePolicy::CriticalSection` frame.
///
/// The estimate takes 1.3 µs per pulse code.
/// This limits the nominal estimate, not a hard interrupts-off bound.
/// 1 ms is about 32 LEDs.
pub const DEFAULT_MAX_NOMINAL_MASKED_US: u32 = 1000;

/// What a driver does with a frame of a given length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongFrameAction {
    /// The frame fits in channel RAM: no refill is needed, so the transmit is sent directly.
    Direct,
    /// Long frame: busy-poll the refill with interrupts live.
    Poll,
    /// Long frame: busy-poll the refill inside a critical section.
    CriticalSection,
    /// Long frame: refuse it.
    Reject,
}

/// Tick facts of the slowest pulse code, as defined by the tier.
///
/// They are passed in rather than derived from the nanosecond constants,
/// because tick rounding makes the slowest code 13 ticks (1300 ns), not 1250 ns.
#[derive(Debug, Clone, Copy)]
pub struct CodeTiming {
    /// Ticks of the slowest pulse code (`T1H + T1L`).
    pub ticks_per_one_code: u32,
    /// Timer ticks per microsecond.
    pub ticks_per_us: u32,
}

/// Nominal wire time in microseconds of a frame of `len` pulse codes, rounded up.
///
/// This is the estimate that the masked-time ceiling of [`LongFramePolicy::CriticalSection`] admits frames by, not a bound on the interrupts-off time.
/// A stalled refill keeps polling, with interrupts masked, until the larger transmit timeout ([`tx_timeout_us`]), and cleanup can take longer still.
/// Takes the slowest code for every code.
/// The multiplication saturates instead of overflowing.
pub const fn masked_us(len: usize, t: CodeTiming) -> u64 {
    (len as u64)
        .saturating_mul(t.ticks_per_one_code as u64)
        .div_ceil(t.ticks_per_us as u64)
}

/// Returns `true` if `len` pulse codes fit in `ram` codes of one channel's RAM, so no refill is needed.
pub const fn fits_channel_ram(len: usize, ram: usize) -> bool {
    len <= ram
}

/// Decides how to send `len` pulse codes given `ram` codes of channel RAM, the policy and the masked-time ceiling.
///
/// A `CriticalSection` frame whose [`masked_us`] exceeds `max_nominal_masked_us` is rejected.
/// `Poll`, `Reject` and frames that fit in channel RAM ignore the ceiling.
pub const fn long_frame_action(
    len: usize,
    ram: usize,
    policy: LongFramePolicy,
    max_nominal_masked_us: Option<u32>,
    t: CodeTiming,
) -> LongFrameAction {
    if fits_channel_ram(len, ram) {
        return LongFrameAction::Direct;
    }
    match policy {
        LongFramePolicy::Reject => LongFrameAction::Reject,
        LongFramePolicy::Poll => LongFrameAction::Poll,
        LongFramePolicy::CriticalSection => match max_nominal_masked_us {
            Some(ceiling) if masked_us(len, t) > ceiling as u64 => LongFrameAction::Reject,
            _ => LongFrameAction::CriticalSection,
        },
    }
}

/// Timeout budget per pulse code, in microseconds.
///
/// About 1.5 times the worst-case code time of 1.3 µs.
/// Assumes a 10 MHz timer tick.
pub const TIMEOUT_US_PER_CODE: u64 = 2;

/// Fixed timeout margin in microseconds, covering transmit start-up and polling overhead.
pub const TIMEOUT_MARGIN_US: u64 = 1000;

/// Upper bound in microseconds for a refill-driven transmit of `len` pulse codes.
#[inline(always)]
pub const fn tx_timeout_us(len: usize) -> u64 {
    len as u64 * TIMEOUT_US_PER_CODE + TIMEOUT_MARGIN_US
}

/// The divider that turns `source_hz` into exactly `timing_clock_hz`, if one exists in `1..=255`.
pub const fn clk_divider_for(source_hz: u32, timing_clock_hz: u32) -> Option<u8> {
    if timing_clock_hz == 0 || !source_hz.is_multiple_of(timing_clock_hz) {
        return None;
    }
    let divider = source_hz / timing_clock_hz;
    if divider == 0 || divider > u8::MAX as u32 {
        None
    } else {
        Some(divider as u8)
    }
}

/// Polls between two clock reads of a [`PollBudget`].
pub const POLLS_PER_CLOCK_READ: u32 = 16;

/// Bounds a polling loop by a deadline and a stopped-clock guard.
///
/// The clock is read lazily, once per [`POLLS_PER_CLOCK_READ`] polls.
#[derive(Clone, Copy, Debug)]
pub struct PollBudget {
    deadline_us: u64,
    polls: u32,
    guard: StallGuard,
}

impl PollBudget {
    /// Starts a budget that expires [`tx_timeout_us`] for `len` pulse codes after `now_us`.
    #[inline(always)]
    pub fn start(now_us: u64, len: usize) -> Self {
        Self {
            deadline_us: now_us + tx_timeout_us(len),
            polls: 0,
            guard: StallGuard::new(0),
        }
    }

    /// Counts one poll and returns `true` once the budget is spent.
    ///
    /// `read_now` is only called every [`POLLS_PER_CLOCK_READ`] polls.
    /// The budget is spent at the deadline, or after [`LATCH_WAIT_MAX_STALLED_READS`](crate::LATCH_WAIT_MAX_STALLED_READS) unchanged clock reads.
    /// The deadline is checked before the stall guard.
    #[inline(always)]
    pub fn spent(&mut self, read_now: impl FnOnce() -> u64) -> bool {
        self.polls = self.polls.wrapping_add(1);
        if !self.polls.is_multiple_of(POLLS_PER_CLOCK_READ) {
            return false;
        }
        let now_us = read_now();
        if now_us >= self.deadline_us {
            return true;
        }
        self.guard.observe(now_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latch::LATCH_WAIT_MAX_STALLED_READS;
    use LongFrameAction as A;
    use LongFramePolicy as P;

    const T: CodeTiming = CodeTiming {
        ticks_per_one_code: 13,
        ticks_per_us: 10,
    };

    const _: () = {
        assert!(masked_us(0, T) == 0);
        assert!(masked_us(1, T) == 2);
        assert!(masked_us(769, T) == 1000);
        assert!(masked_us(770, T) == 1001);
        assert!(tx_timeout_us(289) == 1578);
        assert!(matches!(clk_divider_for(80_000_000, 10_000_000), Some(8)));
    };

    #[test]
    fn default_policy_is_critical_section() {
        assert_eq!(LongFramePolicy::default(), P::CriticalSection);
    }

    #[test]
    fn masked_us_cases() {
        assert_eq!(masked_us(0, T), 0);
        assert_eq!(masked_us(1, T), 2);
        assert_eq!(masked_us(2, T), 3);
        assert_eq!(masked_us(10, T), 13);
        assert_eq!(masked_us(13, T), 17);
        assert_eq!(masked_us(289, T), 376);
        assert_eq!(masked_us(769, T), 1000);
        assert_eq!(masked_us(770, T), 1001);
    }

    #[test]
    fn fits_ram_boundary() {
        assert!(fits_channel_ram(25, 48));
        assert!(fits_channel_ram(48, 48));
        assert!(!fits_channel_ram(49, 48));
        assert!(fits_channel_ram(49, 64));
        assert!(!fits_channel_ram(289, 48));
    }

    #[test]
    fn short_frames_are_direct_for_every_policy() {
        for policy in [P::CriticalSection, P::Poll, P::Reject] {
            for (len, ram) in [(25, 48), (25, 64), (48, 48), (64, 64), (0, 48)] {
                assert_eq!(long_frame_action(len, ram, policy, None, T), A::Direct);
                assert_eq!(long_frame_action(len, ram, policy, Some(0), T), A::Direct);
            }
            assert_ne!(long_frame_action(49, 48, policy, None, T), A::Direct);
            assert_ne!(long_frame_action(65, 64, policy, None, T), A::Direct);
        }
    }

    #[test]
    fn long_frames_follow_policy() {
        for (len, ram) in [(49, 48), (65, 64)] {
            assert_eq!(
                long_frame_action(len, ram, P::CriticalSection, None, T),
                A::CriticalSection
            );
            assert_eq!(long_frame_action(len, ram, P::Poll, None, T), A::Poll);
            assert_eq!(long_frame_action(len, ram, P::Reject, None, T), A::Reject);
        }
        assert_eq!(long_frame_action(289, 48, P::Reject, None, T), A::Reject);
    }

    #[test]
    fn ceiling_routing() {
        let cs = |len, c| long_frame_action(len, 48, P::CriticalSection, c, T);
        assert_eq!(cs(769, Some(1000)), A::CriticalSection);
        assert_eq!(cs(770, Some(1000)), A::Reject);
        assert_eq!(cs(289, Some(1000)), A::CriticalSection);
        assert_eq!(cs(1537, None), A::CriticalSection);
        assert_eq!(cs(25, Some(0)), A::Direct);
        assert_eq!(cs(769, Some(1001)), A::CriticalSection);
        assert_eq!(cs(769, Some(999)), A::Reject);
    }

    #[test]
    fn poll_never_masks_and_only_reject_policy_rejects_without_ceiling() {
        assert_eq!(long_frame_action(770, 48, P::Poll, Some(1000), T), A::Poll);
        assert_eq!(
            long_frame_action(770, 48, P::Reject, Some(1000), T),
            A::Reject
        );
        for len in [49, 289, 770, 100_000] {
            assert_ne!(
                long_frame_action(len, 48, P::Poll, Some(0), T),
                A::CriticalSection
            );
            assert_ne!(long_frame_action(len, 48, P::Poll, None, T), A::Reject);
            assert_ne!(
                long_frame_action(len, 48, P::CriticalSection, None, T),
                A::Reject
            );
        }
    }

    #[test]
    fn extreme_lengths_do_not_panic() {
        for policy in [P::CriticalSection, P::Poll, P::Reject] {
            for ceiling in [None, Some(0), Some(1000), Some(u32::MAX)] {
                let _ = long_frame_action(0, 48, policy, ceiling, T);
                let _ = long_frame_action(usize::MAX, 48, policy, ceiling, T);
            }
        }
        assert_eq!(
            long_frame_action(usize::MAX, 48, P::CriticalSection, Some(1000), T),
            A::Reject
        );
    }

    #[test]
    fn timeout_cases() {
        assert_eq!(tx_timeout_us(0), 1000);
        assert_eq!(tx_timeout_us(16), 1032);
        assert_eq!(tx_timeout_us(289), 1578);
        assert_eq!(TIMEOUT_US_PER_CODE, 2);
        assert_eq!(TIMEOUT_MARGIN_US, 1000);
    }

    #[test]
    fn divider_cases() {
        assert_eq!(clk_divider_for(80_000_000, 10_000_000), Some(8));
        assert_eq!(clk_divider_for(40_000_000, 10_000_000), Some(4));
        assert_eq!(clk_divider_for(10_000_000, 10_000_000), Some(1));
        assert_eq!(clk_divider_for(96_000_000, 10_000_000), None);
        assert_eq!(clk_divider_for(0, 10_000_000), None);
        assert_eq!(clk_divider_for(2_560_000_000, 10_000_000), None);
        assert_eq!(clk_divider_for(5_000_000, 10_000_000), None);
        assert_eq!(clk_divider_for(2_550_000_000, 10_000_000), Some(255));
        assert_eq!(clk_divider_for(80_000_000, 0), None);
    }

    #[test]
    fn poll_budget_reads_once_per_sixteen_polls() {
        let mut b = PollBudget::start(0, 1000);
        let mut reads = 0u32;
        let mut clock = 0u64;
        for _ in 0..160 {
            let spent = b.spent(|| {
                reads += 1;
                clock += 1;
                clock
            });
            assert!(!spent);
        }
        assert_eq!(reads, 10);
    }

    #[test]
    fn poll_budget_trips_at_deadline() {
        let mut b = PollBudget::start(100, 0);
        let deadline = 100 + tx_timeout_us(0);
        for _ in 0..15 {
            assert!(!b.spent(|| unreachable!()));
        }
        assert!(!b.spent(|| deadline - 1));
        for _ in 0..15 {
            assert!(!b.spent(|| unreachable!()));
        }
        assert!(b.spent(|| deadline));
    }

    #[test]
    fn poll_budget_deadline_precedes_stall() {
        let mut b = PollBudget::start(0, 0);
        for _ in 0..15 {
            b.spent(|| unreachable!());
        }
        assert!(b.spent(|| tx_timeout_us(0)));
    }

    #[test]
    fn poll_budget_stopped_clock_ends() {
        let mut b = PollBudget::start(0, 1000);
        let mut polls = 0u32;
        loop {
            polls += 1;
            if b.spent(|| 7) {
                break;
            }
            assert!(polls < 1_000_000);
        }
        // The first read differs from the seed 0 and resets the counter.
        assert_eq!(
            polls,
            POLLS_PER_CLOCK_READ * (LATCH_WAIT_MAX_STALLED_READS + 1)
        );
    }

    #[test]
    fn poll_budget_clock_at_seed_counts_as_stalled() {
        let mut b = PollBudget::start(0, 1000);
        let mut polls = 0u32;
        while !b.spent(|| 0) {
            polls += 1;
        }
        assert_eq!(
            polls + 1,
            POLLS_PER_CLOCK_READ * LATCH_WAIT_MAX_STALLED_READS
        );
    }

    #[test]
    fn poll_budget_wrapping_counter_does_not_panic() {
        let mut b = PollBudget::start(0, 1000);
        b.polls = u32::MAX - 3;
        for _ in 0..40 {
            let _ = b.spent(|| 1);
        }
    }
}

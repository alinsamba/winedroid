const NANOS_PER_SECOND: i64 = 1_000_000_000;

pub(crate) fn frame_interval_nanos_for_fps(frames_per_second: u32) -> i64 {
    let frames_per_second = i64::from(frames_per_second.max(1));
    NANOS_PER_SECOND / frames_per_second
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FramePace {
    pub(crate) elapsed_intervals: i64,
}

impl FramePace {
    pub(crate) fn skipped_intervals(self) -> i64 {
        self.elapsed_intervals - 1
    }
}

pub(crate) struct FrameClock {
    interval_nanos: i64,
    last_frame_nanos: Option<i64>,
}

impl FrameClock {
    pub(crate) fn new(interval_nanos: i64) -> Self {
        assert!(
            interval_nanos > 0,
            "frame interval must be positive, got {interval_nanos}"
        );
        Self {
            interval_nanos,
            last_frame_nanos: None,
        }
    }

    pub(crate) fn poll(&mut self, host_now_nanos: i64) -> Option<FramePace> {
        let Some(last) = self.last_frame_nanos else {
            self.last_frame_nanos = Some(host_now_nanos);
            return Some(FramePace {
                elapsed_intervals: 1,
            });
        };

        let elapsed = host_now_nanos.saturating_sub(last);
        if elapsed < self.interval_nanos {
            return None;
        }

        let elapsed_intervals = elapsed / self.interval_nanos;
        let advanced = elapsed_intervals.saturating_mul(self.interval_nanos);
        self.last_frame_nanos = Some(last.saturating_add(advanced));
        Some(FramePace { elapsed_intervals })
    }

    pub(crate) fn reset(&mut self) {
        self.last_frame_nanos = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_interval_nanos_match_common_refresh_rates() {
        assert_eq!(frame_interval_nanos_for_fps(60), 16_666_666);
        assert_eq!(frame_interval_nanos_for_fps(120), 8_333_333);
        assert_eq!(frame_interval_nanos_for_fps(90), 11_111_111);
        assert_eq!(frame_interval_nanos_for_fps(1), NANOS_PER_SECOND);
    }

    #[test]
    fn first_poll_dispatches_one_frame_and_sets_phase() {
        let mut clock = FrameClock::new(frame_interval_nanos_for_fps(60));

        let pace = clock
            .poll(5_000)
            .expect("first poll should dispatch a frame");
        assert_eq!(pace.elapsed_intervals, 1);
        assert_eq!(pace.skipped_intervals(), 0);
    }

    #[test]
    fn poll_before_interval_elapses_returns_none() {
        let interval = frame_interval_nanos_for_fps(60);
        let mut clock = FrameClock::new(interval);
        clock.poll(0).expect("first poll dispatches");

        assert_eq!(clock.poll(interval - 1), None);
    }

    #[test]
    fn steady_polls_advance_one_interval_each() {
        let interval = frame_interval_nanos_for_fps(60);
        let mut clock = FrameClock::new(interval);
        clock.poll(0).expect("first poll dispatches");

        for tick in 1..=4 {
            let pace = clock
                .poll(interval * tick)
                .unwrap_or_else(|| panic!("frame {tick} should be due"));
            assert_eq!(pace.elapsed_intervals, 1, "tick {tick} should not coalesce");
        }
    }

    #[test]
    fn overrun_coalesces_missed_intervals_into_one_frame() {
        let interval = frame_interval_nanos_for_fps(60);
        let mut clock = FrameClock::new(interval);
        clock.poll(0).expect("first poll dispatches");

        let pace = clock
            .poll(interval * 5 + interval / 2)
            .expect("a frame should be due after a long gap");
        assert_eq!(pace.elapsed_intervals, 5);
        assert_eq!(pace.skipped_intervals(), 4);

        assert_eq!(clock.poll(interval * 5 + interval / 2 + 1), None);
        let pace = clock
            .poll(interval * 6)
            .expect("the next grid-aligned frame should be due");
        assert_eq!(pace.elapsed_intervals, 1);
    }

    #[test]
    fn reset_restarts_phase_without_coalescing_the_idle_gap() {
        let interval = frame_interval_nanos_for_fps(60);
        let mut clock = FrameClock::new(interval);
        clock.poll(0).expect("first poll dispatches");

        clock.reset();

        let pace = clock
            .poll(interval * 1000)
            .expect("first poll after reset dispatches");
        assert_eq!(pace.elapsed_intervals, 1);
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static ENABLED: AtomicBool = AtomicBool::new(false);
static METRICS: OnceLock<Mutex<RenderMetrics>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RenderMetrics {
    pub(crate) frame_count: u64,
    pub(crate) work_frame_count: u64,
    pub(crate) dropped_frame_count: u64,
    pub(crate) jank_frame_count: u64,
    pub(crate) frame_total_us: u64,
    pub(crate) frame_max_us: u64,
    pub(crate) traversal_count: u64,
    pub(crate) traversal_us: u64,
    pub(crate) measure_layout_count: u64,
    pub(crate) measure_layout_us: u64,
    pub(crate) draw_count: u64,
    pub(crate) draw_us: u64,
    pub(crate) retained_replay_count: u64,
    pub(crate) retained_replay_us: u64,
    pub(crate) surface_read_count: u64,
    pub(crate) surface_read_us: u64,
    pub(crate) surface_present_count: u64,
    pub(crate) surface_present_us: u64,
    pub(crate) host_redraw_requests: u64,
}

pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        reset();
    }
}

pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn reset() {
    if !enabled() {
        return;
    }
    with_metrics(|metrics| {
        *metrics = RenderMetrics::default();
    });
}

pub(crate) fn drain() -> RenderMetrics {
    if !enabled() {
        return RenderMetrics::default();
    }
    let mut drained = RenderMetrics::default();
    with_metrics(|metrics| {
        drained = *metrics;
        *metrics = RenderMetrics::default();
    });
    drained
}

pub(crate) fn start_timing() -> Option<Instant> {
    enabled().then(Instant::now)
}

pub(crate) fn record_frame(
    started_at: Option<Instant>,
    skipped_intervals: i64,
    view_changed: bool,
    janked: bool,
) {
    let Some(started_at) = started_at else {
        return;
    };
    let elapsed_us = duration_micros(started_at.elapsed());
    with_metrics(|metrics| {
        metrics.frame_count = metrics.frame_count.saturating_add(1);
        if view_changed {
            metrics.work_frame_count = metrics.work_frame_count.saturating_add(1);
        }
        if skipped_intervals > 0 {
            metrics.dropped_frame_count = metrics
                .dropped_frame_count
                .saturating_add(skipped_intervals as u64);
        }
        if janked {
            metrics.jank_frame_count = metrics.jank_frame_count.saturating_add(1);
        }
        metrics.frame_total_us = metrics.frame_total_us.saturating_add(elapsed_us);
        metrics.frame_max_us = metrics.frame_max_us.max(elapsed_us);
    });
}

pub(crate) fn record_traversal_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.traversal_count = metrics.traversal_count.saturating_add(1);
        metrics.traversal_us = metrics.traversal_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_measure_layout_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.measure_layout_count = metrics.measure_layout_count.saturating_add(1);
        metrics.measure_layout_us = metrics.measure_layout_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_draw_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.draw_count = metrics.draw_count.saturating_add(1);
        metrics.draw_us = metrics.draw_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_retained_replay_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.retained_replay_count = metrics.retained_replay_count.saturating_add(1);
        metrics.retained_replay_us = metrics.retained_replay_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_surface_read_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.surface_read_count = metrics.surface_read_count.saturating_add(1);
        metrics.surface_read_us = metrics.surface_read_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_surface_present_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.surface_present_count = metrics.surface_present_count.saturating_add(1);
        metrics.surface_present_us = metrics.surface_present_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_host_redraw_request() {
    if !enabled() {
        return;
    }
    with_metrics(|metrics| {
        metrics.host_redraw_requests = metrics.host_redraw_requests.saturating_add(1);
    });
}

fn record_elapsed(started_at: Option<Instant>, record: impl FnOnce(&mut RenderMetrics, u64)) {
    let Some(started_at) = started_at else {
        return;
    };
    let elapsed_us = duration_micros(started_at.elapsed());
    with_metrics(|metrics| record(metrics, elapsed_us));
}

fn duration_micros(duration: Duration) -> u64 {
    (duration.as_micros()).min(u128::from(u64::MAX)) as u64
}

fn with_metrics(update: impl FnOnce(&mut RenderMetrics)) {
    let metrics = METRICS.get_or_init(|| Mutex::new(RenderMetrics::default()));
    let Ok(mut metrics) = metrics.lock() else {
        return;
    };
    update(&mut metrics);
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static ENABLED: AtomicBool = AtomicBool::new(false);
static METRICS: OnceLock<Mutex<MethodMetrics>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MethodMetrics {
    pub(crate) call_count: u64,
    pub(crate) call_us: u64,
    pub(crate) field_op_count: u64,
    pub(crate) field_op_us: u64,
    pub(crate) array_op_count: u64,
    pub(crate) array_op_us: u64,
    pub(crate) branch_count: u64,
    pub(crate) branch_us: u64,
    pub(crate) object_alloc_count: u64,
    pub(crate) object_alloc_us: u64,
    pub(crate) builtin_call_count: u64,
    pub(crate) builtin_call_us: u64,
    pub(crate) render_callback_count: u64,
    pub(crate) render_callback_us: u64,
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
        *metrics = MethodMetrics::default();
    });
}

pub(crate) fn drain() -> MethodMetrics {
    if !enabled() {
        return MethodMetrics::default();
    }
    let mut drained = MethodMetrics::default();
    with_metrics(|metrics| {
        drained = *metrics;
        *metrics = MethodMetrics::default();
    });
    drained
}

pub(crate) fn start_timing() -> Option<Instant> {
    enabled().then(Instant::now)
}

pub(crate) fn record_call_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.call_count = metrics.call_count.saturating_add(1);
        metrics.call_us = metrics.call_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_field_op_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.field_op_count = metrics.field_op_count.saturating_add(1);
        metrics.field_op_us = metrics.field_op_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_array_op_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.array_op_count = metrics.array_op_count.saturating_add(1);
        metrics.array_op_us = metrics.array_op_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_branch_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.branch_count = metrics.branch_count.saturating_add(1);
        metrics.branch_us = metrics.branch_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_object_alloc_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.object_alloc_count = metrics.object_alloc_count.saturating_add(1);
        metrics.object_alloc_us = metrics.object_alloc_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_builtin_call_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.builtin_call_count = metrics.builtin_call_count.saturating_add(1);
        metrics.builtin_call_us = metrics.builtin_call_us.saturating_add(elapsed_us);
    });
}

pub(crate) fn record_render_callback_elapsed(started_at: Option<Instant>) {
    record_elapsed(started_at, |metrics, elapsed_us| {
        metrics.render_callback_count = metrics.render_callback_count.saturating_add(1);
        metrics.render_callback_us = metrics.render_callback_us.saturating_add(elapsed_us);
    });
}

fn record_elapsed(started_at: Option<Instant>, record: impl FnOnce(&mut MethodMetrics, u64)) {
    let Some(started_at) = started_at else {
        return;
    };
    let elapsed_us = duration_micros(started_at.elapsed());
    with_metrics(|metrics| record(metrics, elapsed_us));
}

fn duration_micros(duration: Duration) -> u64 {
    (duration.as_micros()).min(u128::from(u64::MAX)) as u64
}

fn with_metrics(update: impl FnOnce(&mut MethodMetrics)) {
    let metrics = METRICS.get_or_init(|| Mutex::new(MethodMetrics::default()));
    let Ok(mut metrics) = metrics.lock() else {
        return;
    };
    update(&mut metrics);
}

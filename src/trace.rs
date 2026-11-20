use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static TRACE_CONFIG: OnceLock<TraceConfig> = OnceLock::new();
static TRACE_STATS: OnceLock<Mutex<BTreeMap<TraceStatKey, TraceStat>>> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct TraceConfig {
    verbose: bool,
    categories: BTreeSet<String>,
    all: bool,
    any: bool,
}

impl TraceConfig {
    pub fn from_options(verbose: bool, trace_categories: Option<&str>) -> Self {
        let mut config = Self {
            verbose,
            categories: BTreeSet::new(),
            all: false,
            any: false,
        };

        if let Ok(value) = env::var("WINEDROID_TRACE") {
            config.add_categories(&value);
        }

        if let Some(value) = trace_categories {
            config.add_categories(value);
        }

        config.all = config.categories.contains("all");
        config.any = !config.categories.is_empty();
        config
    }

    fn add_categories(&mut self, value: &str) {
        let value = value.trim();
        if value == "1" || value.eq_ignore_ascii_case("true") {
            self.categories.insert("all".to_owned());
            return;
        }

        for category in value.split(',') {
            let category = category.trim();
            if category.is_empty() {
                continue;
            }

            self.categories.insert(category.to_ascii_lowercase());
        }
    }

    #[inline]
    fn enabled(&self, category: &str) -> bool {
        if self.verbose || self.all {
            return true;
        }
        self.any && self.categories.contains(category)
    }
}

pub fn configure(config: TraceConfig) {
    let _ = TRACE_CONFIG.set(config);
}

pub fn enabled(category: &str) -> bool {
    config().enabled(category)
}

pub fn event(category: &str, message: impl AsRef<str>) {
    if !enabled(category) {
        return;
    }

    eprintln!("[winedroid:{category}] {}", message.as_ref());
}

pub fn span(category: &'static str, name: &'static str) -> TraceSpan {
    TraceSpan {
        category,
        name,
        start: enabled(category).then(Instant::now),
    }
}

pub fn stat_span(category: &'static str, name: &'static str) -> TraceStatSpan {
    TraceStatSpan {
        category,
        name,
        start: enabled(category).then(Instant::now),
    }
}

pub fn count(category: &'static str, name: &'static str, amount: u64) {
    if !enabled(category) {
        return;
    }

    record_stat(category, name, Duration::ZERO, amount);
}

pub fn report_stats() {
    if !enabled("perf") {
        return;
    }

    let Some(stats) = TRACE_STATS.get() else {
        return;
    };
    let Ok(stats) = stats.lock() else {
        return;
    };
    if stats.is_empty() {
        return;
    }

    let mut rows = stats
        .iter()
        .map(|(key, stat)| (key, stat))
        .collect::<Vec<_>>();
    rows.sort_by(|(_, left), (_, right)| {
        right
            .total
            .cmp(&left.total)
            .then_with(|| right.count.cmp(&left.count))
    });

    eprintln!("[winedroid:perf] timing summary");
    for (key, stat) in rows.into_iter().take(40) {
        let average = if stat.count == 0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(stat.total.as_secs_f64() / stat.count as f64)
        };
        eprintln!(
            "[winedroid:perf] {} count={} total={} avg={} max={}",
            key.name,
            stat.count,
            format_duration(stat.total),
            format_duration(average),
            format_duration(stat.max)
        );
    }
}

fn config() -> &'static TraceConfig {
    TRACE_CONFIG.get_or_init(|| TraceConfig::from_options(false, None))
}

pub struct TraceSpan {
    category: &'static str,
    name: &'static str,
    start: Option<Instant>,
}

impl Drop for TraceSpan {
    fn drop(&mut self) {
        let Some(start) = self.start else {
            return;
        };

        eprintln!(
            "[winedroid:{}] {} took {}",
            self.category,
            self.name,
            format_duration(start.elapsed())
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct TraceStatKey {
    category: &'static str,
    name: &'static str,
}

#[derive(Clone, Copy, Debug, Default)]
struct TraceStat {
    count: u64,
    total: Duration,
    max: Duration,
}

pub struct TraceStatSpan {
    category: &'static str,
    name: &'static str,
    start: Option<Instant>,
}

impl Drop for TraceStatSpan {
    fn drop(&mut self) {
        let Some(start) = self.start else {
            return;
        };

        record_stat(self.category, self.name, start.elapsed(), 1);
    }
}

fn record_stat(category: &'static str, name: &'static str, duration: Duration, count: u64) {
    let stats = TRACE_STATS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let Ok(mut stats) = stats.lock() else {
        return;
    };
    let stat = stats
        .entry(TraceStatKey { category, name })
        .or_insert_with(TraceStat::default);

    stat.count = stat.count.saturating_add(count);
    stat.total = stat.total.saturating_add(duration);
    if duration > stat.max {
        stat.max = duration;
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        return format!("{:.3}s", duration.as_secs_f64());
    }

    let millis = duration.as_secs_f64() * 1000.0;
    format!("{millis:.2}ms")
}

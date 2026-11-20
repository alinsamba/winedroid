use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const MISSING_PRIMITIVE_MARKER: &str = "next missing primitive: ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingPrimitiveEntry {
    pub primitive: String,
    pub count: u64,
    pub first_detail: String,
    pub abandoned_calls: BTreeMap<String, AbandonedCallEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbandonedCallEntry {
    pub count: u64,
    pub first_detail: String,
}

fn registry() -> &'static Mutex<BTreeMap<String, MissingPrimitiveEntry>> {
    static REGISTRY: OnceLock<Mutex<BTreeMap<String, MissingPrimitiveEntry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub(crate) fn strict_enabled() -> bool {
    env::var("WINEDROID_STRICT").is_ok_and(|value| value == "1")
}

pub(crate) fn primitive_from_message(message: &str) -> Option<&str> {
    let marker_start = message.find(MISSING_PRIMITIVE_MARKER)?;
    let primitive = message[marker_start + MISSING_PRIMITIVE_MARKER.len()..]
        .lines()
        .next()?
        .trim();
    if primitive.is_empty() {
        return None;
    }
    Some(primitive)
}

pub fn record_from_error_message(message: &str) {
    let Some(primitive) = primitive_from_message(message) else {
        return;
    };
    record(primitive, message);
}

pub(crate) fn record(primitive: &str, message: &str) {
    let Ok(mut registry) = registry().lock() else {
        return;
    };
    let first = !registry.contains_key(primitive);
    registry
        .entry(primitive.to_owned())
        .and_modify(|entry| entry.count += 1)
        .or_insert_with(|| MissingPrimitiveEntry {
            primitive: primitive.to_owned(),
            count: 1,
            first_detail: message.to_owned(),
            abandoned_calls: BTreeMap::new(),
        });
    write_log_if_requested(&registry.values().cloned().collect::<Vec<_>>());
    drop(registry);
    if first {
        eprintln!("FIXME: not implemented: {primitive}");
        crate::trace::event("missing", message);
    }
}

pub(crate) fn annotate(primitive: &str, detail: &str) {
    let Ok(mut entries) = registry().lock() else {
        return;
    };
    if let Some(entry) = entries.get_mut(primitive) {
        if !entry.first_detail.contains("policy:") {
            entry.first_detail = detail.to_owned();
            crate::trace::event("missing", detail);
            write_log_if_requested(&entries.values().cloned().collect::<Vec<_>>());
        }
    }
}

pub(crate) fn record_abandoned_call(primitive: &str, signature: &str, detail: &str) {
    let Ok(mut entries) = registry().lock() else {
        return;
    };
    let Some(entry) = entries.get_mut(primitive) else {
        return;
    };
    let first = !entry.abandoned_calls.contains_key(signature);
    entry
        .abandoned_calls
        .entry(signature.to_owned())
        .and_modify(|call| call.count += 1)
        .or_insert_with(|| AbandonedCallEntry {
            count: 1,
            first_detail: detail.to_owned(),
        });
    write_log_if_requested(&entries.values().cloned().collect::<Vec<_>>());
    drop(entries);
    if first {
        eprintln!("CONTEXT: abandoned {signature}; caused by: {primitive}");
        crate::trace::event("missing", detail);
    }
}

pub fn recorded_entries() -> Vec<MissingPrimitiveEntry> {
    let Ok(registry) = registry().lock() else {
        return Vec::new();
    };

    let mut entries: Vec<MissingPrimitiveEntry> = registry.values().cloned().collect();
    entries.sort_by(|a, b| b.count.cmp(&a.count).then(a.primitive.cmp(&b.primitive)));
    entries
}

pub fn report() {
    let entries = recorded_entries();
    write_log_if_requested(&entries);

    if entries.is_empty() {
        return;
    }

    println!("Missing primitives (incomplete execution; not compatibility evidence)");
    for entry in &entries {
        println!("  {} (hits: {})", entry.primitive, entry.count);
        for (signature, call) in &entry.abandoned_calls {
            println!("    CONTEXT: abandoned {signature} (hits: {})", call.count);
        }
    }
    println!();
}

fn write_log_if_requested(entries: &[MissingPrimitiveEntry]) {
    let Some(path) = env::var_os("WINEDROID_MISSING_PRIMITIVE_LOG") else {
        return;
    };

    if let Err(error) = write_report(Path::new(&path), entries) {
        eprintln!(
            "failed to write WINEDROID_MISSING_PRIMITIVE_LOG to `{}`: {error}",
            path.to_string_lossy()
        );
    }
}

fn write_report(path: &Path, entries: &[MissingPrimitiveEntry]) -> std::io::Result<()> {
    let mut body = String::from("Incomplete execution: missing support was encountered.\n\n");
    for entry in entries {
        body.push_str(&format!(
            "primitive: {}\nhits: {}\nfirst detail:\n{}\n\n",
            entry.primitive,
            entry.count,
            indent_detail(&entry.first_detail)
        ));
        for (signature, call) in &entry.abandoned_calls {
            body.push_str(&format!(
                "context: abandoned {signature}\ncaused by: {}\nhits: {}\nfirst detail:\n{}\n\n",
                entry.primitive,
                call.count,
                indent_detail(&call.first_detail)
            ));
        }
    }
    if entries.is_empty() {
        body = String::from("No missing primitives recorded.\n");
    }

    static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary_name = path.as_os_str().to_os_string();
    temporary_name.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    let temporary_path = Path::new(&temporary_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary_path)?;
    if let Err(error) = file.write_all(body.as_bytes()) {
        drop(file);
        let _ = fs::remove_file(temporary_path);
        return Err(error);
    }
    drop(file);
    if let Err(error) = fs::rename(temporary_path, path) {
        let _ = fs::remove_file(temporary_path);
        return Err(error);
    }
    Ok(())
}

fn indent_detail(detail: &str) -> String {
    detail
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_report_replaces_previous_snapshot_with_counts_and_context() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "winedroid-missing-{}-{stamp}.txt",
            std::process::id()
        ));
        let entry = MissingPrimitiveEntry {
            primitive: "method Ltest/Probe;.missing()V".to_owned(),
            count: 2,
            first_detail: "caller class: Ltest/Caller;".to_owned(),
            abandoned_calls: BTreeMap::from([(
                "Ltest/Caller;.run()V".to_owned(),
                AbandonedCallEntry {
                    count: 3,
                    first_detail: "nested failure".to_owned(),
                },
            )]),
        };
        write_report(&path, &[entry]).unwrap();
        let report = fs::read_to_string(&path).unwrap();
        assert!(report.contains("method Ltest/Probe;.missing()V"));
        assert!(report.contains("hits: 2"));
        assert!(report.contains("caller class: Ltest/Caller;"));
        assert!(report.contains("context: abandoned Ltest/Caller;.run()V"));
        assert!(report.contains("caused by: method Ltest/Probe;.missing()V\nhits: 3"));
        write_report(&path, &[]).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "No missing primitives recorded.\n"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn records_and_counts_primitives_from_error_messages() {
        record_from_error_message(
            "DEX invocation not implemented\nmethod: Landroid/graphics/Path;.cubicTo(FFFFFF)V\nnext missing primitive: test dispatch primitive alpha",
        );
        record_from_error_message(
            "another failure\nnext missing primitive: test dispatch primitive alpha",
        );
        record_from_error_message("unrelated error without the marker");

        let entries = recorded_entries();
        let entry = entries
            .iter()
            .find(|entry| entry.primitive == "test dispatch primitive alpha")
            .expect("primitive should be recorded");

        assert_eq!(entry.count, 2);
        assert!(entry.first_detail.contains("cubicTo"));
    }

    #[test]
    fn primitive_name_stops_at_the_end_of_the_line() {
        record_from_error_message(
            "failure\nnext missing primitive: test line scoped primitive\ntrailing context line",
        );

        let entries = recorded_entries();
        assert!(entries
            .iter()
            .any(|entry| entry.primitive == "test line scoped primitive"));
    }
}

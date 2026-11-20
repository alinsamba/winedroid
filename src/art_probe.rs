use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, WinedroidError};

const ART_RUNNER_ENV: &str = "WINEDROID_ART_RUNNER";
const ART_BOOTCLASSPATH_ENV: &str = "WINEDROID_ART_BOOTCLASSPATH";
const D8_ENV: &str = "WINEDROID_D8";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtProbeReport {
    java_compiler: ToolProbe,
    dex_compiler: ToolProbe,
    art_runner: ToolProbe,
    boot_classpath: SettingProbe,
}

impl ArtProbeReport {
    pub fn detect() -> Self {
        Self {
            java_compiler: ToolProbe::new("Java compiler", find_on_path("javac")),
            dex_compiler: ToolProbe::new("DEX compiler", find_d8()),
            art_runner: ToolProbe::new("ART runner", find_art_runner()),
            boot_classpath: SettingProbe::new(
                "ART boot classpath",
                env::var_os(ART_BOOTCLASSPATH_ENV).map(PathBuf::from),
            ),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.java_compiler.path.is_some()
            && self.dex_compiler.path.is_some()
            && self.art_runner.path.is_some()
            && self.boot_classpath.value.is_some()
    }

    pub fn next_missing_primitive(&self) -> Option<&'static str> {
        if self.java_compiler.path.is_none() {
            return Some("Java compiler for probe payload");
        }

        if self.dex_compiler.path.is_none() {
            return Some("DEX compiler for probe payload");
        }

        if self.art_runner.path.is_none() {
            return Some("host-built ART runner");
        }

        if self.boot_classpath.value.is_none() {
            return Some("Android boot classpath for host ART");
        }

        None
    }

    fn missing_hint(&self) -> Option<&'static str> {
        if self.java_compiler.path.is_none() {
            return Some("install OpenJDK and ensure `javac` is on PATH");
        }

        if self.dex_compiler.path.is_none() {
            return Some(
                "set WINEDROID_D8 or ANDROID_HOME so winedroid can find Android build-tools/d8",
            );
        }

        if self.art_runner.path.is_none() {
            return Some("set WINEDROID_ART_RUNNER to a host-built ART runner such as dalvikvm");
        }

        if self.boot_classpath.value.is_none() {
            return Some(
                "set WINEDROID_ART_BOOTCLASSPATH to extracted Android runtime/framework jars",
            );
        }

        None
    }

    #[cfg(test)]
    fn for_test(
        java_compiler: Option<PathBuf>,
        dex_compiler: Option<PathBuf>,
        art_runner: Option<PathBuf>,
        boot_classpath: Option<PathBuf>,
    ) -> Self {
        Self {
            java_compiler: ToolProbe::new("Java compiler", java_compiler),
            dex_compiler: ToolProbe::new("DEX compiler", dex_compiler),
            art_runner: ToolProbe::new("ART runner", art_runner),
            boot_classpath: SettingProbe::new("ART boot classpath", boot_classpath),
        }
    }
}

pub fn ensure_probe_can_run(report: &ArtProbeReport) -> Result<()> {
    if report.is_ready() {
        return Ok(());
    }

    let missing = report
        .next_missing_primitive()
        .unwrap_or("unknown ART probe primitive");
    let hint = report
        .missing_hint()
        .unwrap_or("inspect the ART probe report");

    Err(WinedroidError::new(format!(
        "ART probe cannot run\nnext missing primitive: {missing}\nhint: {hint}"
    )))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolProbe {
    label: &'static str,
    path: Option<PathBuf>,
}

impl ToolProbe {
    fn new(label: &'static str, path: Option<PathBuf>) -> Self {
        Self { label, path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SettingProbe {
    label: &'static str,
    value: Option<PathBuf>,
}

impl SettingProbe {
    fn new(label: &'static str, value: Option<PathBuf>) -> Self {
        Self { label, value }
    }
}

impl fmt::Display for ArtProbeReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "ART probe")?;
        writeln!(formatter, "  mode: translation-layer")?;
        writeln!(formatter, "  hardware/system VM: forbidden")?;
        write_tool(formatter, &self.java_compiler)?;
        write_tool(formatter, &self.dex_compiler)?;
        write_tool(formatter, &self.art_runner)?;
        write_setting(formatter, &self.boot_classpath)?;
        writeln!(formatter, "  ready: {}", yes_no(self.is_ready()))?;

        if let Some(primitive) = self.next_missing_primitive() {
            writeln!(formatter, "  next missing primitive: {primitive}")?;
        }

        Ok(())
    }
}

fn write_tool(formatter: &mut fmt::Formatter<'_>, tool: &ToolProbe) -> fmt::Result {
    match &tool.path {
        Some(path) => writeln!(formatter, "  {}: yes ({})", tool.label, path.display()),
        None => writeln!(formatter, "  {}: no", tool.label),
    }
}

fn write_setting(formatter: &mut fmt::Formatter<'_>, setting: &SettingProbe) -> fmt::Result {
    match &setting.value {
        Some(path) => writeln!(formatter, "  {}: yes ({})", setting.label, path.display()),
        None => writeln!(formatter, "  {}: no", setting.label),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        return "yes";
    }

    "no"
}

fn find_art_runner() -> Option<PathBuf> {
    if let Some(path) = path_from_env(ART_RUNNER_ENV) {
        return Some(path);
    }

    for name in ["dalvikvm", "dalvikvm64", "art"] {
        if let Some(path) = find_on_path(name) {
            return Some(path);
        }
    }

    None
}

fn find_d8() -> Option<PathBuf> {
    if let Some(path) = path_from_env(D8_ENV) {
        return Some(path);
    }

    if let Some(path) = find_on_path("d8") {
        return Some(path);
    }

    let Some(sdk_root) = android_sdk_root() else {
        return None;
    };

    find_latest_build_tool(&sdk_root, "d8")
}

fn android_sdk_root() -> Option<PathBuf> {
    env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
}

fn path_from_env(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    let path = PathBuf::from(value);

    if path.is_file() {
        return Some(path);
    }

    None
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;

    for directory in env::split_paths(&paths) {
        let candidate = directory.join(name);

        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn find_latest_build_tool(sdk_root: &Path, tool_name: &str) -> Option<PathBuf> {
    let build_tools_dir = sdk_root.join("build-tools");
    let entries = fs::read_dir(build_tools_dir).ok()?;
    let mut versions = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };

        let path = entry.path();

        if path.is_dir() {
            versions.push(path);
        }
    }

    versions.sort();

    for version in versions.iter().rev() {
        let candidate = version.join(tool_name);

        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_runner_blocks_probe() {
        let report = ArtProbeReport::for_test(
            Some(PathBuf::from("/bin/javac")),
            Some(PathBuf::from("/android/d8")),
            None,
            Some(PathBuf::from("/android/bootclasspath")),
        );
        let error = ensure_probe_can_run(&report).expect_err("missing runner should fail");

        assert_eq!(
            report.next_missing_primitive(),
            Some("host-built ART runner")
        );
        assert!(
            error.to_string().contains("host-built ART runner"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn boot_classpath_blocks_probe_after_runner() {
        let report = ArtProbeReport::for_test(
            Some(PathBuf::from("/bin/javac")),
            Some(PathBuf::from("/android/d8")),
            Some(PathBuf::from("/android/dalvikvm")),
            None,
        );

        assert_eq!(
            report.next_missing_primitive(),
            Some("Android boot classpath for host ART")
        );
    }

    #[test]
    fn complete_probe_is_ready() {
        let report = ArtProbeReport::for_test(
            Some(PathBuf::from("/bin/javac")),
            Some(PathBuf::from("/android/d8")),
            Some(PathBuf::from("/android/dalvikvm")),
            Some(PathBuf::from("/android/bootclasspath")),
        );

        assert!(report.is_ready());
        assert!(ensure_probe_can_run(&report).is_ok());
    }
}

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuestAbi {
    Arm64V8a,
    ArmeabiV7a,
    X86,
    X86_64,
}

impl GuestAbi {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "arm64-v8a" => Some(Self::Arm64V8a),
            "armeabi-v7a" => Some(Self::ArmeabiV7a),
            "x86" => Some(Self::X86),
            "x86_64" => Some(Self::X86_64),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Arm64V8a => "arm64-v8a",
            Self::ArmeabiV7a => "armeabi-v7a",
            Self::X86 => "x86",
            Self::X86_64 => "x86_64",
        }
    }

    pub fn cpu_arch(self) -> GuestCpuArch {
        match self {
            Self::Arm64V8a => GuestCpuArch::Arm64,
            Self::ArmeabiV7a => GuestCpuArch::Arm32,
            Self::X86 => GuestCpuArch::X86,
            Self::X86_64 => GuestCpuArch::X86_64,
        }
    }
}

impl fmt::Display for GuestAbi {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestCpuArch {
    Arm64,
    Arm32,
    X86,
    X86_64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiInventory {
    pub known: Vec<GuestAbi>,
    pub unknown: Vec<String>,
}

impl AbiInventory {
    pub fn from_names(names: &[String]) -> Self {
        let mut known = Vec::new();
        let mut unknown = Vec::new();

        for name in names {
            match GuestAbi::parse(name) {
                Some(abi) => known.push(abi),
                None => unknown.push(name.clone()),
            }
        }

        known.sort();
        known.dedup();
        unknown.sort();
        unknown.dedup();

        Self { known, unknown }
    }

    pub fn has_native_code(&self) -> bool {
        !self.known.is_empty() || !self.unknown.is_empty()
    }

    pub fn known_display(&self) -> String {
        if self.known.is_empty() {
            return "none".to_owned();
        }

        self.known
            .iter()
            .map(|abi| abi.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn unknown_display(&self) -> String {
        if self.unknown.is_empty() {
            return "none".to_owned();
        }

        self.unknown.join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPlatform {
    pub os: HostOs,
    pub arch: HostArch,
}

impl HostPlatform {
    pub fn detect() -> Self {
        Self {
            os: HostOs::detect(std::env::consts::OS),
            arch: HostArch::detect(std::env::consts::ARCH),
        }
    }

    pub fn preferred_abis(&self) -> Vec<GuestAbi> {
        match self.arch {
            HostArch::Arm64 => vec![
                GuestAbi::Arm64V8a,
                GuestAbi::ArmeabiV7a,
                GuestAbi::X86_64,
                GuestAbi::X86,
            ],
            HostArch::Arm32 => vec![
                GuestAbi::ArmeabiV7a,
                GuestAbi::Arm64V8a,
                GuestAbi::X86,
                GuestAbi::X86_64,
            ],
            HostArch::X86_64 => vec![
                GuestAbi::X86_64,
                GuestAbi::X86,
                GuestAbi::Arm64V8a,
                GuestAbi::ArmeabiV7a,
            ],
            HostArch::X86 => vec![
                GuestAbi::X86,
                GuestAbi::X86_64,
                GuestAbi::ArmeabiV7a,
                GuestAbi::Arm64V8a,
            ],
            HostArch::Unsupported(_) => vec![
                GuestAbi::Arm64V8a,
                GuestAbi::ArmeabiV7a,
                GuestAbi::X86_64,
                GuestAbi::X86,
            ],
        }
    }

    pub fn matches_guest_cpu(&self, guest_abi: GuestAbi) -> bool {
        matches!(
            (self.arch.clone(), guest_abi.cpu_arch()),
            (HostArch::Arm64, GuestCpuArch::Arm64)
                | (HostArch::Arm32, GuestCpuArch::Arm32)
                | (HostArch::X86, GuestCpuArch::X86)
                | (HostArch::X86_64, GuestCpuArch::X86_64)
        )
    }
}

impl fmt::Display for HostPlatform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.os, self.arch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostOs {
    Linux,
    Macos,
    Windows,
    Browser,
    Unsupported(String),
}

impl HostOs {
    fn detect(name: &str) -> Self {
        match name {
            "linux" => Self::Linux,
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            "emscripten" => Self::Browser,
            other => Self::Unsupported(other.to_owned()),
        }
    }
}

impl fmt::Display for HostOs {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linux => write!(formatter, "linux"),
            Self::Macos => write!(formatter, "macos"),
            Self::Windows => write!(formatter, "windows"),
            Self::Browser => write!(formatter, "browser"),
            Self::Unsupported(name) => write!(formatter, "{name}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostArch {
    Arm64,
    Arm32,
    X86,
    X86_64,
    Unsupported(String),
}

impl HostArch {
    fn detect(name: &str) -> Self {
        match name {
            "aarch64" | "arm64" => Self::Arm64,
            "arm" | "armv7" => Self::Arm32,
            "x86" | "i386" | "i586" | "i686" => Self::X86,
            "x86_64" | "amd64" => Self::X86_64,
            other => Self::Unsupported(other.to_owned()),
        }
    }
}

impl fmt::Display for HostArch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arm64 => write!(formatter, "arm64"),
            Self::Arm32 => write!(formatter, "arm"),
            Self::X86 => write!(formatter, "x86"),
            Self::X86_64 => write!(formatter, "x86_64"),
            Self::Unsupported(name) => write!(formatter, "{name}"),
        }
    }
}

pub fn select_guest_abi(host: &HostPlatform, inventory: &AbiInventory) -> Option<GuestAbi> {
    for preferred_abi in host.preferred_abis() {
        if inventory.known.contains(&preferred_abi) {
            return Some(preferred_abi);
        }
    }

    inventory.known.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_and_unknown_abis() {
        let names = vec![
            "arm64-v8a".to_owned(),
            "x86_64".to_owned(),
            "mips".to_owned(),
            "arm64-v8a".to_owned(),
        ];

        let inventory = AbiInventory::from_names(&names);

        assert_eq!(inventory.known, vec![GuestAbi::Arm64V8a, GuestAbi::X86_64]);
        assert_eq!(inventory.unknown, vec!["mips".to_owned()]);
    }

    #[test]
    fn prefers_native_x86_64_before_arm_on_x86_64_host() {
        let host = HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        };
        let names = vec!["arm64-v8a".to_owned(), "x86_64".to_owned()];
        let inventory = AbiInventory::from_names(&names);

        assert_eq!(select_guest_abi(&host, &inventory), Some(GuestAbi::X86_64));
    }

    #[test]
    fn falls_back_to_arm64_on_x86_64_when_no_x86_abi_exists() {
        let host = HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        };
        let names = vec!["arm64-v8a".to_owned(), "armeabi-v7a".to_owned()];
        let inventory = AbiInventory::from_names(&names);

        assert_eq!(
            select_guest_abi(&host, &inventory),
            Some(GuestAbi::Arm64V8a)
        );
    }
}

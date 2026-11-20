use std::fmt;

use crate::abi::{self, AbiInventory, GuestAbi, HostOs, HostPlatform};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRuntimePlan {
    pub host: HostPlatform,
    pub inventory: AbiInventory,
    pub selected_guest_abi: Option<GuestAbi>,
    pub branch: NativeBackendBranch,
}

impl NativeRuntimePlan {
    pub fn detect(native_abi_names: &[String]) -> Self {
        Self::new(HostPlatform::detect(), native_abi_names)
    }

    pub fn new(host: HostPlatform, native_abi_names: &[String]) -> Self {
        let inventory = AbiInventory::from_names(native_abi_names);
        let selected_guest_abi = abi::select_guest_abi(&host, &inventory);
        let branch = choose_backend_branch(&host, &inventory, selected_guest_abi);

        Self {
            host,
            inventory,
            selected_guest_abi,
            branch,
        }
    }

    pub fn selected_guest_abi_display(&self) -> String {
        match self.selected_guest_abi {
            Some(abi) => abi.to_string(),
            None => "none".to_owned(),
        }
    }

    pub fn execution_strategy(&self) -> NativeExecutionStrategy {
        match self.branch {
            NativeBackendBranch::NotRequired => NativeExecutionStrategy::NotRequired,
            branch => NativeExecutionStrategy::UnsupportedUntilNativeCode {
                branch,
                missing_primitive: branch.missing_primitive(),
            },
        }
    }
}

fn choose_backend_branch(
    host: &HostPlatform,
    inventory: &AbiInventory,
    selected_guest_abi: Option<GuestAbi>,
) -> NativeBackendBranch {
    if !inventory.has_native_code() {
        return NativeBackendBranch::NotRequired;
    }

    let Some(selected_guest_abi) = selected_guest_abi else {
        return NativeBackendBranch::UnknownGuestAbi;
    };

    if host.os == HostOs::Browser {
        return NativeBackendBranch::BrowserWasm;
    }

    if !host.matches_guest_cpu(selected_guest_abi) {
        return NativeBackendBranch::CpuTranslationBionic;
    }

    match host.os {
        HostOs::Linux => NativeBackendBranch::DirectLinuxBionic,
        HostOs::Macos => NativeBackendBranch::MacosAndroidElfBionic,
        HostOs::Windows | HostOs::Unsupported(_) => NativeBackendBranch::UnsupportedHostNative,
        HostOs::Browser => NativeBackendBranch::BrowserWasm,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBackendBranch {
    NotRequired,
    DirectLinuxBionic,
    MacosAndroidElfBionic,
    CpuTranslationBionic,
    BrowserWasm,
    UnknownGuestAbi,
    UnsupportedHostNative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExecutionStrategy {
    NotRequired,
    UnsupportedUntilNativeCode {
        branch: NativeBackendBranch,
        missing_primitive: &'static str,
    },
}

impl NativeExecutionStrategy {
    pub fn status(self) -> String {
        match self {
            Self::NotRequired => "not required".to_owned(),
            Self::UnsupportedUntilNativeCode {
                missing_primitive, ..
            } => format!(
                "unsupported until native code is reached; missing primitive: {missing_primitive}"
            ),
        }
    }
}

impl NativeBackendBranch {
    pub fn missing_primitive(self) -> &'static str {
        match self {
            Self::NotRequired => "none",
            Self::DirectLinuxBionic => "Android ELF loader with Bionic runtime on Linux",
            Self::MacosAndroidElfBionic => "Android ELF loader with Bionic runtime on macOS",
            Self::CpuTranslationBionic => "CPU translation plus Android ELF/Bionic runtime",
            Self::BrowserWasm => "Android ELF/Bionic execution inside browser WebAssembly",
            Self::UnknownGuestAbi => "guest ABI classification",
            Self::UnsupportedHostNative => "native backend for this host platform",
        }
    }
}

impl fmt::Display for NativeBackendBranch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRequired => write!(formatter, "not-required"),
            Self::DirectLinuxBionic => write!(formatter, "direct-linux-bionic"),
            Self::MacosAndroidElfBionic => write!(formatter, "macos-android-elf-bionic"),
            Self::CpuTranslationBionic => write!(formatter, "cpu-translation-bionic"),
            Self::BrowserWasm => write!(formatter, "browser-wasm"),
            Self::UnknownGuestAbi => write!(formatter, "unknown-guest-abi"),
            Self::UnsupportedHostNative => write!(formatter, "unsupported-host-native"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{HostArch, HostOs};

    #[test]
    fn dex_only_apps_do_not_need_native_backend() {
        let host = HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        };
        let plan = NativeRuntimePlan::new(host, &[]);

        assert_eq!(plan.branch, NativeBackendBranch::NotRequired);
    }

    #[test]
    fn linux_x86_64_same_arch_uses_direct_bionic_branch() {
        let host = HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        };
        let names = vec!["x86_64".to_owned()];
        let plan = NativeRuntimePlan::new(host, &names);

        assert_eq!(plan.selected_guest_abi, Some(GuestAbi::X86_64));
        assert_eq!(plan.branch, NativeBackendBranch::DirectLinuxBionic);
    }

    #[test]
    fn linux_x86_64_arm_apk_uses_translation_branch() {
        let host = HostPlatform {
            os: HostOs::Linux,
            arch: HostArch::X86_64,
        };
        let names = vec!["arm64-v8a".to_owned()];
        let plan = NativeRuntimePlan::new(host, &names);

        assert_eq!(plan.selected_guest_abi, Some(GuestAbi::Arm64V8a));
        assert_eq!(plan.branch, NativeBackendBranch::CpuTranslationBionic);
    }

    #[test]
    fn macos_arm64_same_arch_uses_macos_loader_branch() {
        let host = HostPlatform {
            os: HostOs::Macos,
            arch: HostArch::Arm64,
        };
        let names = vec!["arm64-v8a".to_owned()];
        let plan = NativeRuntimePlan::new(host, &names);

        assert_eq!(plan.selected_guest_abi, Some(GuestAbi::Arm64V8a));
        assert_eq!(plan.branch, NativeBackendBranch::MacosAndroidElfBionic);
    }

    #[test]
    fn browser_native_inventory_stays_explicitly_unsupported() {
        let host = HostPlatform {
            os: HostOs::Browser,
            arch: HostArch::Unsupported("wasm32".to_owned()),
        };
        let names = vec!["arm64-v8a".to_owned()];
        let plan = NativeRuntimePlan::new(host, &names);

        assert_eq!(plan.selected_guest_abi, Some(GuestAbi::Arm64V8a));
        assert_eq!(plan.branch, NativeBackendBranch::BrowserWasm);
        assert!(matches!(
            plan.execution_strategy(),
            NativeExecutionStrategy::UnsupportedUntilNativeCode {
                branch: NativeBackendBranch::BrowserWasm,
                ..
            }
        ));
    }
}

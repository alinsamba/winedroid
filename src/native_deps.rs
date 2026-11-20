use crate::abi::GuestAbi;
use crate::inspect::NativeLibrary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDependencyPlan {
    pub selected_guest_abi: Option<GuestAbi>,
    pub app_libraries: Vec<String>,
    pub android_system_libraries: Vec<String>,
    pub missing_libraries: Vec<String>,
}

impl NativeDependencyPlan {
    pub fn build(
        selected_guest_abi: Option<GuestAbi>,
        selected_libraries: &[NativeLibrary],
    ) -> Self {
        let app_libraries = app_library_names(selected_libraries);
        let mut android_system_libraries = Vec::new();
        let mut missing_libraries = Vec::new();

        for library in selected_libraries {
            for dependency in library.needed_libraries() {
                if app_libraries.contains(dependency) {
                    continue;
                }

                if is_known_android_system_library(dependency) {
                    android_system_libraries.push(dependency.clone());
                } else {
                    missing_libraries.push(dependency.clone());
                }
            }
        }

        android_system_libraries.sort();
        android_system_libraries.dedup();
        missing_libraries.sort();
        missing_libraries.dedup();

        Self {
            selected_guest_abi,
            app_libraries,
            android_system_libraries,
            missing_libraries,
        }
    }

    pub fn selected_guest_abi_display(&self) -> String {
        match self.selected_guest_abi {
            Some(abi) => abi.to_string(),
            None => "none".to_owned(),
        }
    }

    pub fn app_libraries_display(&self) -> String {
        display_list(&self.app_libraries)
    }

    pub fn android_system_libraries_display(&self) -> String {
        display_list(&self.android_system_libraries)
    }

    pub fn missing_libraries_display(&self) -> String {
        display_list(&self.missing_libraries)
    }
}

fn app_library_names(libraries: &[NativeLibrary]) -> Vec<String> {
    let mut names = libraries
        .iter()
        .map(|library| library.name().to_owned())
        .collect::<Vec<_>>();

    names.sort();
    names.dedup();
    names
}

fn is_known_android_system_library(name: &str) -> bool {
    matches!(
        name,
        "libc.so" | "libdl.so" | "liblog.so" | "libm.so" | "libz.so"
    )
}

fn display_list(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::{ElfClass, ElfEndian, ElfMachine, ElfSummary};
    use crate::inspect::NativeLibrary;

    #[test]
    fn separates_app_system_and_missing_dependencies() {
        let libraries = vec![
            library(
                "libmain.so",
                vec![
                    "libhelper.so".to_owned(),
                    "libc.so".to_owned(),
                    "libcustom.so".to_owned(),
                ],
            ),
            library("libhelper.so", vec!["liblog.so".to_owned()]),
        ];

        let plan = NativeDependencyPlan::build(Some(GuestAbi::Arm64V8a), &libraries);

        assert_eq!(
            plan.app_libraries,
            vec!["libhelper.so".to_owned(), "libmain.so".to_owned()]
        );
        assert_eq!(
            plan.android_system_libraries,
            vec!["libc.so".to_owned(), "liblog.so".to_owned()]
        );
        assert_eq!(plan.missing_libraries, vec!["libcustom.so".to_owned()]);
    }

    fn library(name: &str, needed_libraries: Vec<String>) -> NativeLibrary {
        NativeLibrary::for_test(
            "arm64-v8a",
            name,
            ElfSummary {
                class: ElfClass::Elf64,
                endian: ElfEndian::Little,
                machine: ElfMachine::Aarch64,
                needed_libraries,
            },
        )
    }
}

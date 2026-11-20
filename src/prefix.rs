use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{Result, WinedroidError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinedroidPrefix {
    root: PathBuf,
}

impl WinedroidPrefix {
    pub fn default() -> Result<Self> {
        Self::resolve(env::var_os("WINEDROID_PREFIX"), env::var_os("HOME"))
    }

    fn resolve(prefix_env: Option<OsString>, home_env: Option<OsString>) -> Result<Self> {
        if let Some(prefix) = prefix_env {
            if prefix.is_empty() {
                return Err(WinedroidError::new("WINEDROID_PREFIX is set but empty"));
            }

            return Ok(Self {
                root: PathBuf::from(prefix),
            });
        }

        let Some(home) = home_env else {
            return Err(WinedroidError::new(
                "neither WINEDROID_PREFIX nor HOME is set, so the winedroid prefix cannot be resolved",
            ));
        };

        Ok(Self {
            root: PathBuf::from(home).join(".winedroid").join("default"),
        })
    }

    #[cfg(test)]
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn apps_root(&self) -> PathBuf {
        self.root.join("apps")
    }

    pub fn storage_root(&self) -> PathBuf {
        self.root.join("storage")
    }

    pub fn app_dir(&self, package_name: &str) -> Result<PathBuf> {
        validate_package_name(package_name)?;
        reject_case_folded_package_collision(&self.apps_root(), package_name)?;
        Ok(self.apps_root().join(package_name))
    }
}

fn reject_case_folded_package_collision(apps_root: &Path, package_name: &str) -> Result<()> {
    let entries = match fs::read_dir(apps_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(WinedroidError::with_source(
                format!(
                    "failed to inspect installed apps directory `{}`",
                    apps_root.display()
                ),
                error,
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            WinedroidError::with_source(
                format!(
                    "failed to inspect installed apps directory `{}`",
                    apps_root.display()
                ),
                error,
            )
        })?;
        let Some(existing) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if existing != package_name && existing.eq_ignore_ascii_case(package_name) {
            return Err(WinedroidError::new(format!(
                "package `{package_name}` conflicts with installed package directory `{existing}`"
            )));
        }
    }

    Ok(())
}

fn validate_package_name(package_name: &str) -> Result<()> {
    if package_name.is_empty() {
        return Err(WinedroidError::new("package name is empty"));
    }

    if package_name.starts_with('.') || package_name.ends_with('.') {
        return Err(WinedroidError::new(format!(
            "package name `{package_name}` cannot start or end with a dot"
        )));
    }

    for part in package_name.split('.') {
        validate_package_part(package_name, part)?;
    }

    Ok(())
}

fn validate_package_part(package_name: &str, part: &str) -> Result<()> {
    if part.is_empty() {
        return Err(WinedroidError::new(format!(
            "package name `{package_name}` contains an empty segment"
        )));
    }

    for character in part.chars() {
        let is_valid = character.is_ascii_alphanumeric() || character == '_';

        if !is_valid {
            return Err(WinedroidError::new(format!(
                "package name `{package_name}` contains unsupported character `{character}`"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_app_dir_for_valid_package_name() {
        let prefix = WinedroidPrefix::from_root(PathBuf::from("/tmp/winedroid-test"));
        let app_dir = prefix
            .app_dir("com.example.app")
            .expect("package should be accepted");

        assert_eq!(
            app_dir,
            PathBuf::from("/tmp/winedroid-test/apps/com.example.app")
        );
    }

    #[test]
    fn rejects_package_name_with_path_separator() {
        let prefix = WinedroidPrefix::from_root(PathBuf::from("/tmp/winedroid-test"));
        let error = prefix
            .app_dir("com.example/evil")
            .expect_err("package should be rejected");

        assert!(
            error.to_string().contains("unsupported character"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_a_case_folded_installed_package_collision() {
        let root = std::env::temp_dir().join(format!(
            "winedroid-prefix-case-collision-{}",
            std::process::id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(root.join("apps/com.example"))
            .expect("installed package directory should exist");
        let prefix = WinedroidPrefix::from_root(root.clone());

        let error = prefix
            .app_dir("com.Example")
            .expect_err("case-folded package should be rejected");

        assert!(error.to_string().contains("conflicts"));
        assert_eq!(
            prefix
                .app_dir("com.example")
                .expect("exact package should remain usable"),
            root.join("apps/com.example")
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn config_path_lives_at_prefix_root() {
        let prefix = WinedroidPrefix::from_root(PathBuf::from("/tmp/winedroid-test"));

        assert_eq!(
            prefix.config_path(),
            PathBuf::from("/tmp/winedroid-test/config.json")
        );
    }

    #[test]
    fn winedroid_prefix_env_overrides_home() {
        let prefix = WinedroidPrefix::resolve(
            Some(OsString::from("/tmp/winedroid-prefix")),
            Some(OsString::from("/Users/someone")),
        )
        .expect("prefix should resolve");

        assert_eq!(prefix.root(), &PathBuf::from("/tmp/winedroid-prefix"));
    }

    #[test]
    fn home_provides_the_default_prefix_when_env_is_unset() {
        let prefix = WinedroidPrefix::resolve(None, Some(OsString::from("/Users/someone")))
            .expect("prefix should resolve");

        assert_eq!(
            prefix.root(),
            &PathBuf::from("/Users/someone/.winedroid/default")
        );
    }

    #[test]
    fn empty_winedroid_prefix_fails_loudly() {
        let error = WinedroidPrefix::resolve(
            Some(OsString::new()),
            Some(OsString::from("/Users/someone")),
        )
        .expect_err("empty prefix should be rejected");

        assert!(
            error.to_string().contains("WINEDROID_PREFIX"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn missing_prefix_and_home_fails_loudly() {
        let error =
            WinedroidPrefix::resolve(None, None).expect_err("no prefix source should error");

        assert!(
            error.to_string().contains("WINEDROID_PREFIX"),
            "unexpected error: {error}"
        );
    }
}

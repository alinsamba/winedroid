use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::capability_fs::Directory;
use crate::error::{Result, WinedroidError};
use crate::prefix::WinedroidPrefix;

pub const GUEST_PRIMARY_EXTERNAL_ROOT: &str = "/storage/emulated/0";

pub const GUEST_PRIMARY_EXTERNAL_ALIAS: &str = "/sdcard";

pub const PUBLIC_DIRECTORIES: &[&str] = &[
    "Download",
    "Documents",
    "DCIM",
    "Pictures",
    "Music",
    "Movies",
    "Alarms",
    "Notifications",
    "Podcasts",
    "Ringtones",
];

pub const SCREENSHOTS_GUEST_PATH: &str = "/storage/emulated/0/Pictures/Screenshots";

#[derive(Debug, Clone)]
pub struct ScreenshotDirectory {
    root: Directory,
    components: Arc<[CString]>,
    display_path: PathBuf,
}

impl PartialEq for ScreenshotDirectory {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components && self.display_path == other.display_path
    }
}

impl ScreenshotDirectory {
    pub fn save(&self, name: &str, bytes: &[u8]) -> Result<PathBuf> {
        self.root.mkdir_all(&self.components).map_err(|error| {
            WinedroidError::with_source(
                format!(
                    "failed to create screenshot directory `{}`",
                    self.display_path.display()
                ),
                error,
            )
        })?;
        let mut components = self.components.to_vec();
        components.push(CString::new(name).map_err(|error| {
            WinedroidError::with_source("screenshot filename contains NUL", error)
        })?);
        let mut file = self.root.open_truncate(&components).map_err(|error| {
            WinedroidError::with_source(
                format!(
                    "failed to create screenshot `{}`",
                    self.display_path.join(name).display()
                ),
                error,
            )
        })?;
        file.write_all(bytes).map_err(|error| {
            WinedroidError::with_source(
                format!(
                    "failed to write screenshot `{}`",
                    self.display_path.join(name).display()
                ),
                error,
            )
        })?;
        Ok(self.display_path.join(name))
    }

    #[cfg(test)]
    pub(crate) fn from_host_root(path: &Path) -> Result<Self> {
        let root = Directory::open_ambient(path).map_err(|error| {
            WinedroidError::with_source(
                format!("failed to open screenshot test root `{}`", path.display()),
                error,
            )
        })?;
        Ok(Self {
            root,
            components: Arc::from([]),
            display_path: path.to_path_buf(),
        })
    }
}

pub const READ_EXTERNAL_STORAGE: &str = "android.permission.READ_EXTERNAL_STORAGE";
pub const WRITE_EXTERNAL_STORAGE: &str = "android.permission.WRITE_EXTERNAL_STORAGE";
pub const READ_MEDIA_AUDIO: &str = "android.permission.READ_MEDIA_AUDIO";
pub const READ_MEDIA_VIDEO: &str = "android.permission.READ_MEDIA_VIDEO";
pub const READ_MEDIA_IMAGES: &str = "android.permission.READ_MEDIA_IMAGES";
pub const READ_MEDIA_VISUAL_USER_SELECTED: &str =
    "android.permission.READ_MEDIA_VISUAL_USER_SELECTED";
pub const ACCESS_MEDIA_LOCATION: &str = "android.permission.ACCESS_MEDIA_LOCATION";
pub const MANAGE_EXTERNAL_STORAGE: &str = "android.permission.MANAGE_EXTERNAL_STORAGE";
pub const MANAGE_MEDIA: &str = "android.permission.MANAGE_MEDIA";
pub const WRITE_MEDIA_STORAGE: &str = "android.permission.WRITE_MEDIA_STORAGE";
pub const WRITE_OBB: &str = "android.permission.WRITE_OBB";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalStorageAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalStorageRuntimeAccess {
    granted_permissions: HashSet<String>,
    legacy_storage: bool,
    legacy_read_access: bool,
    legacy_write_access: bool,
    all_files_access: bool,
}

impl ExternalStorageRuntimeAccess {
    pub fn new(
        granted_permissions: Vec<String>,
        legacy_storage: bool,
        all_files_access: bool,
    ) -> Self {
        Self {
            granted_permissions: granted_permissions.into_iter().collect(),
            legacy_storage,
            legacy_read_access: false,
            legacy_write_access: false,
            all_files_access,
        }
    }

    pub fn with_legacy_file_access(
        granted_permissions: Vec<String>,
        legacy_storage: bool,
        legacy_read_access: bool,
        legacy_write_access: bool,
    ) -> Self {
        let mut access = Self::new(granted_permissions, legacy_storage, false);
        access.legacy_read_access = legacy_read_access;
        access.legacy_write_access = legacy_write_access;
        access
    }

    fn fresh_install(
        target_sdk_version: i32,
        request_legacy_external_storage: bool,
        requested_permissions: &[String],
    ) -> Self {
        // cite: frameworks-base PackageManager.java#FLAG_PERMISSION_REVOKE_ON_UPGRADE, FLAG_PERMISSION_REVIEW_REQUIRED
        let granted_permissions = if target_sdk_version < 23 {
            requested_permissions
                .iter()
                .filter(|permission| is_runtime_storage_permission(permission.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // cite: frameworks-base Environment.java#DEFAULT_SCOPED_STORAGE, FORCE_ENABLE_SCOPED_STORAGE, isExternalStorageLegacy
        let legacy_storage = target_sdk_version < 29
            || (target_sdk_version == 29 && request_legacy_external_storage);

        Self::with_legacy_file_access(granted_permissions, legacy_storage, false, false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalStoragePolicy {
    requested_permissions: HashSet<String>,
    runtime_access: ExternalStorageRuntimeAccess,
}

impl ExternalStoragePolicy {
    pub fn for_fresh_install(
        min_sdk_version: i32,
        target_sdk_version: i32,
        request_legacy_external_storage: bool,
        requested_permissions: Vec<String>,
    ) -> Self {
        let requested_permissions = normalize_requested_permissions(
            min_sdk_version,
            target_sdk_version,
            requested_permissions,
        );
        let runtime_access = ExternalStorageRuntimeAccess::fresh_install(
            target_sdk_version,
            request_legacy_external_storage,
            &requested_permissions,
        );
        Self::new(
            min_sdk_version,
            target_sdk_version,
            requested_permissions,
            runtime_access,
        )
    }

    pub fn new(
        min_sdk_version: i32,
        target_sdk_version: i32,
        requested_permissions: Vec<String>,
        runtime_access: ExternalStorageRuntimeAccess,
    ) -> Self {
        let requested_permissions = normalize_requested_permissions(
            min_sdk_version,
            target_sdk_version,
            requested_permissions,
        );
        Self {
            requested_permissions: requested_permissions.into_iter().collect(),
            runtime_access,
        }
    }

    pub fn restricted(target_sdk_version: i32) -> Self {
        Self::for_fresh_install(target_sdk_version, target_sdk_version, false, Vec::new())
    }

    pub fn permission_granted(&self, permission: &str) -> Option<bool> {
        if !is_external_storage_permission(permission) {
            return None;
        }

        // cite: frameworks-base Activity.java#requestPermissions, onRequestPermissionsResult
        // cite: frameworks-base ContextImpl.java#checkPermission, checkSelfPermission
        let granted = self.requested_permissions.contains(permission)
            && self.runtime_access.granted_permissions.contains(permission);
        Some(granted)
    }

    pub fn declared_permissions(&self) -> impl Iterator<Item = &str> {
        self.requested_permissions.iter().map(String::as_str)
    }

    pub fn grant_permission(&mut self, permission: &str) -> bool {
        if !self.requested_permissions.contains(permission) {
            return false;
        }

        // cite: frameworks-base Activity.java#requestPermissions, onRequestPermissionsResult
        self.runtime_access
            .granted_permissions
            .insert(permission.to_owned());
        // cite: frameworks-base StorageManager.java#checkExternalStoragePermissionAndAppOp, noteAppOpAllowingLegacy
        if permission == READ_EXTERNAL_STORAGE && self.runtime_access.legacy_storage {
            self.runtime_access.legacy_read_access = true;
        }
        if permission == WRITE_EXTERNAL_STORAGE && self.runtime_access.legacy_storage {
            self.runtime_access.legacy_write_access = true;
        }
        // cite: frameworks-base Environment.java#isExternalStorageManager
        if permission == MANAGE_EXTERNAL_STORAGE {
            self.runtime_access.all_files_access = true;
        }
        true
    }

    pub fn manages_external_storage(&self) -> bool {
        // cite: frameworks-base Environment.java#isExternalStorageManager
        self.runtime_access.all_files_access
    }

    pub fn allows_public_path(&self, access: ExternalStorageAccess) -> bool {
        // cite: frameworks-base Environment.java#isExternalStorageManager
        if self.runtime_access.all_files_access {
            return true;
        }
        if !self.has_effective_legacy_storage() {
            return false;
        }

        let permission = match access {
            ExternalStorageAccess::Read => READ_EXTERNAL_STORAGE,
            ExternalStorageAccess::Write => WRITE_EXTERNAL_STORAGE,
        };
        // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE
        // cite: frameworks-base StorageManager.java#checkExternalStoragePermissionAndAppOp, noteAppOpAllowingLegacy
        self.permission_granted(permission) == Some(true)
            && match access {
                ExternalStorageAccess::Read => self.runtime_access.legacy_read_access,
                ExternalStorageAccess::Write => self.runtime_access.legacy_write_access,
            }
    }

    fn has_effective_legacy_storage(&self) -> bool {
        // cite: frameworks-base Environment.java#DEFAULT_SCOPED_STORAGE, FORCE_ENABLE_SCOPED_STORAGE, isExternalStorageLegacy
        self.runtime_access.legacy_storage
    }
}

pub fn is_external_storage_permission(permission: &str) -> bool {
    // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE, READ_MEDIA_AUDIO, READ_MEDIA_VIDEO, READ_MEDIA_IMAGES, READ_MEDIA_VISUAL_USER_SELECTED, ACCESS_MEDIA_LOCATION, MANAGE_EXTERNAL_STORAGE, MANAGE_MEDIA, WRITE_MEDIA_STORAGE, WRITE_OBB
    matches!(
        permission,
        READ_EXTERNAL_STORAGE
            | WRITE_EXTERNAL_STORAGE
            | READ_MEDIA_AUDIO
            | READ_MEDIA_VIDEO
            | READ_MEDIA_IMAGES
            | READ_MEDIA_VISUAL_USER_SELECTED
            | ACCESS_MEDIA_LOCATION
            | MANAGE_EXTERNAL_STORAGE
            | MANAGE_MEDIA
            | WRITE_MEDIA_STORAGE
            | WRITE_OBB
    )
}

fn is_runtime_storage_permission(permission: &str) -> bool {
    // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE, READ_MEDIA_AUDIO, READ_MEDIA_VIDEO, READ_MEDIA_IMAGES, READ_MEDIA_VISUAL_USER_SELECTED, ACCESS_MEDIA_LOCATION, MANAGE_EXTERNAL_STORAGE, MANAGE_MEDIA, WRITE_MEDIA_STORAGE, WRITE_OBB
    matches!(
        permission,
        READ_EXTERNAL_STORAGE
            | WRITE_EXTERNAL_STORAGE
            | READ_MEDIA_AUDIO
            | READ_MEDIA_VIDEO
            | READ_MEDIA_IMAGES
            | READ_MEDIA_VISUAL_USER_SELECTED
            | ACCESS_MEDIA_LOCATION
    )
}

fn normalize_requested_permissions(
    min_sdk_version: i32,
    target_sdk_version: i32,
    mut requested_permissions: Vec<String>,
) -> Vec<String> {
    // cite: frameworks-base PackageParser.java#NEW_PERMISSIONS
    if target_sdk_version < 4
        && !requested_permissions
            .iter()
            .any(|permission| permission == WRITE_EXTERNAL_STORAGE)
    {
        requested_permissions.push(WRITE_EXTERNAL_STORAGE.to_owned());
    }

    // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE
    if min_sdk_version <= 3
        && target_sdk_version <= 3
        && !requested_permissions
            .iter()
            .any(|permission| permission == READ_EXTERNAL_STORAGE)
    {
        requested_permissions.push(READ_EXTERNAL_STORAGE.to_owned());
    }

    // cite: frameworks-base AndroidManifest.xml#READ_MEDIA_VISUAL_USER_SELECTED
    let adds_visual_selection = requested_permissions.iter().any(|permission| {
        matches!(
            permission.as_str(),
            READ_MEDIA_IMAGES | READ_MEDIA_VIDEO | ACCESS_MEDIA_LOCATION
        )
    });
    if adds_visual_selection
        && !requested_permissions
            .iter()
            .any(|permission| permission == READ_MEDIA_VISUAL_USER_SELECTED)
    {
        requested_permissions.push(READ_MEDIA_VISUAL_USER_SELECTED.to_owned());
    }

    requested_permissions
}

pub fn screenshots_dir(prefix: &WinedroidPrefix) -> Result<ScreenshotDirectory> {
    let display_path = resolve_guest_path(prefix, SCREENSHOTS_GUEST_PATH)?;
    let root = Directory::open_ambient(&prefix.storage_root()).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to acquire shared storage capability `{}`",
                prefix.storage_root().display()
            ),
            error,
        )
    })?;
    Ok(ScreenshotDirectory {
        root,
        components: Arc::from([
            CString::new("Pictures").expect("literal has no NUL"),
            CString::new("Screenshots").expect("literal has no NUL"),
        ]),
        display_path,
    })
}

pub fn provision_shared_storage(prefix: &WinedroidPrefix) -> Result<()> {
    fs::create_dir_all(prefix.root()).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to create prefix directory `{}`",
                prefix.root().display()
            ),
            error,
        )
    })?;
    let storage_root = prefix.storage_root();
    create_real_directory(&storage_root)?;
    let storage = Directory::open_ambient(&storage_root).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to acquire shared storage capability `{}`",
                storage_root.display()
            ),
            error,
        )
    })?;

    for directory in PUBLIC_DIRECTORIES {
        let guest_path = format!("{GUEST_PRIMARY_EXTERNAL_ROOT}/{directory}");
        storage
            .mkdir_all(&guest_path_components(&guest_path)?)
            .map_err(|error| {
                WinedroidError::with_source(
                    format!("failed to provision shared path `{guest_path}`"),
                    error,
                )
            })?;
    }

    Ok(())
}

pub fn provision_app_external_storage(prefix: &WinedroidPrefix, package_name: &str) -> Result<()> {
    let storage_root = prefix.storage_root();
    create_real_directory(&storage_root)?;
    let storage = Directory::open_ambient(&storage_root).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to acquire shared storage capability `{}`",
                storage_root.display()
            ),
            error,
        )
    })?;
    for guest_path in app_external_guest_paths(package_name) {
        storage
            .mkdir_all(&guest_path_components(&guest_path)?)
            .map_err(|error| {
                WinedroidError::with_source(
                    format!("failed to provision app external path `{guest_path}`"),
                    error,
                )
            })?;
    }

    Ok(())
}

fn app_external_guest_paths(package_name: &str) -> [String; 4] {
    // cite: frameworks-base Environment.java#buildExternalStorageAppFilesDirs, buildExternalStorageAppCacheDirs, buildExternalStorageAppObbDirs, buildExternalStorageAppMediaDirs
    [
        format!("{GUEST_PRIMARY_EXTERNAL_ROOT}/Android/data/{package_name}/files"),
        format!("{GUEST_PRIMARY_EXTERNAL_ROOT}/Android/data/{package_name}/cache"),
        format!("{GUEST_PRIMARY_EXTERNAL_ROOT}/Android/obb/{package_name}"),
        format!("{GUEST_PRIMARY_EXTERNAL_ROOT}/Android/media/{package_name}"),
    ]
}

pub fn resolve_guest_path(prefix: &WinedroidPrefix, guest_path: &str) -> Result<PathBuf> {
    let relative = strip_mount_prefix(guest_path).ok_or_else(|| {
        WinedroidError::new(format!(
            "guest path `{guest_path}` is not on the primary external volume `{GUEST_PRIMARY_EXTERNAL_ROOT}`"
        ))
    })?;

    let mut host_path = prefix.storage_root();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => host_path.push(part),
            Component::CurDir => {}
            Component::RootDir => {}
            Component::ParentDir | Component::Prefix(_) => {
                return Err(WinedroidError::new(format!(
                    "guest path `{guest_path}` escapes the external storage volume"
                )));
            }
        }
    }

    Ok(host_path)
}

fn strip_mount_prefix(guest_path: &str) -> Option<&str> {
    for mount in [GUEST_PRIMARY_EXTERNAL_ROOT, GUEST_PRIMARY_EXTERNAL_ALIAS] {
        if guest_path == mount {
            return Some("");
        }
        if let Some(rest) = guest_path.strip_prefix(mount) {
            if let Some(rest) = rest.strip_prefix('/') {
                return Some(rest);
            }
        }
    }

    None
}

fn guest_path_components(guest_path: &str) -> Result<Vec<CString>> {
    let relative = strip_mount_prefix(guest_path).ok_or_else(|| {
        WinedroidError::new(format!(
            "guest path `{guest_path}` is not on primary external storage"
        ))
    })?;
    relative
        .split('/')
        .filter(|component| !component.is_empty())
        .map(|component| {
            CString::new(component).map_err(|error| {
                WinedroidError::with_source(
                    format!("guest path `{guest_path}` contains NUL"),
                    error,
                )
            })
        })
        .collect()
}

fn create_real_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(WinedroidError::new(format!(
                "external storage path `{}` is not a real directory",
                path.display()
            )));
        }
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(WinedroidError::with_source(
                format!(
                    "failed to inspect external storage path `{}`",
                    path.display()
                ),
                error,
            ));
        }
    }

    fs::create_dir(path).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to create external storage directory `{}`",
                path.display()
            ),
            error,
        )
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    fn test_prefix() -> WinedroidPrefix {
        WinedroidPrefix::from_root(PathBuf::from("/prefix"))
    }

    #[test]
    fn context_permission_state_requires_declaration_and_effective_runtime_grant() {
        // cite: frameworks-base ContextImpl.java#checkPermission, checkSelfPermission
        let undeclared = ExternalStoragePolicy::new(
            28,
            28,
            Vec::new(),
            ExternalStorageRuntimeAccess::new(vec![READ_EXTERNAL_STORAGE.to_owned()], true, false),
        );
        let declared_ungranted = ExternalStoragePolicy::for_fresh_install(
            34,
            34,
            false,
            vec![READ_MEDIA_IMAGES.to_owned()],
        );
        let declared_granted = ExternalStoragePolicy::new(
            34,
            34,
            vec![READ_MEDIA_IMAGES.to_owned()],
            ExternalStorageRuntimeAccess::new(vec![READ_MEDIA_IMAGES.to_owned()], false, false),
        );

        assert_eq!(
            undeclared.permission_granted(READ_EXTERNAL_STORAGE),
            Some(false)
        );
        assert_eq!(
            declared_ungranted.permission_granted(READ_MEDIA_IMAGES),
            Some(false)
        );
        assert_eq!(
            declared_granted.permission_granted(READ_MEDIA_IMAGES),
            Some(true)
        );
        assert_eq!(
            declared_granted.permission_granted("android.permission.INTERNET"),
            None
        );
    }

    #[test]
    fn package_manager_pre_m_grant_does_not_fabricate_effective_file_access() {
        // cite: frameworks-base PackageManager.java#FLAG_PERMISSION_REVOKE_ON_UPGRADE, FLAG_PERMISSION_REVIEW_REQUIRED
        // cite: frameworks-base StorageManager.java#checkExternalStoragePermissionAndAppOp
        let pre_m = ExternalStoragePolicy::for_fresh_install(
            22,
            22,
            true,
            vec![READ_EXTERNAL_STORAGE.to_owned()],
        );
        let runtime = ExternalStoragePolicy::for_fresh_install(
            23,
            23,
            true,
            vec![READ_EXTERNAL_STORAGE.to_owned()],
        );

        assert_eq!(pre_m.permission_granted(READ_EXTERNAL_STORAGE), Some(true));
        assert!(!pre_m.allows_public_path(ExternalStorageAccess::Read));
        assert_eq!(
            runtime.permission_granted(READ_EXTERNAL_STORAGE),
            Some(false)
        );
    }

    #[test]
    fn package_parser_adds_pre_donut_write_and_selected_media_permissions() {
        // cite: frameworks-base PackageParser.java#NEW_PERMISSIONS
        // cite: frameworks-base AndroidManifest.xml#READ_MEDIA_VISUAL_USER_SELECTED
        let pre_donut = ExternalStoragePolicy::for_fresh_install(3, 3, true, Vec::new());
        let higher_minimum = ExternalStoragePolicy::for_fresh_install(4, 3, true, Vec::new());
        let donut = ExternalStoragePolicy::for_fresh_install(4, 4, true, Vec::new());
        let selected_media = ExternalStoragePolicy::new(
            34,
            34,
            vec![READ_MEDIA_IMAGES.to_owned()],
            ExternalStorageRuntimeAccess::new(
                vec![READ_MEDIA_VISUAL_USER_SELECTED.to_owned()],
                false,
                false,
            ),
        );

        assert_eq!(
            pre_donut.permission_granted(WRITE_EXTERNAL_STORAGE),
            Some(true)
        );
        assert_eq!(
            pre_donut.permission_granted(READ_EXTERNAL_STORAGE),
            Some(true)
        );
        assert!(!pre_donut.allows_public_path(ExternalStorageAccess::Write));
        assert!(!pre_donut.allows_public_path(ExternalStorageAccess::Read));
        assert_eq!(
            higher_minimum.permission_granted(READ_EXTERNAL_STORAGE),
            Some(false)
        );
        assert_eq!(
            donut.permission_granted(WRITE_EXTERNAL_STORAGE),
            Some(false)
        );
        assert_eq!(
            selected_media.permission_granted(READ_MEDIA_VISUAL_USER_SELECTED),
            Some(true)
        );
    }

    #[test]
    fn environment_legacy_access_requires_effective_state_and_independent_permissions() {
        // cite: frameworks-base Environment.java#isExternalStorageLegacy, DEFAULT_SCOPED_STORAGE, FORCE_ENABLE_SCOPED_STORAGE
        // cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE
        let read_only = legacy_policy(28, vec![READ_EXTERNAL_STORAGE]);
        let write_only = legacy_policy(28, vec![WRITE_EXTERNAL_STORAGE]);
        let no_legacy_state = ExternalStoragePolicy::new(
            28,
            28,
            vec![READ_EXTERNAL_STORAGE.to_owned()],
            ExternalStorageRuntimeAccess::new(vec![READ_EXTERNAL_STORAGE.to_owned()], false, false),
        );

        assert!(read_only.allows_public_path(ExternalStorageAccess::Read));
        assert!(!read_only.allows_public_path(ExternalStorageAccess::Write));
        assert!(!write_only.allows_public_path(ExternalStorageAccess::Read));
        assert!(write_only.allows_public_path(ExternalStorageAccess::Write));
        assert!(!no_legacy_state.allows_public_path(ExternalStorageAccess::Read));
    }

    #[test]
    fn environment_fresh_install_and_preserved_runtime_state_control_legacy_storage() {
        // cite: frameworks-base PackageParser.java#parseBaseApplication, AndroidManifestApplication_requestLegacyExternalStorage
        // cite: frameworks-base Environment.java#isExternalStorageLegacy, DEFAULT_SCOPED_STORAGE, FORCE_ENABLE_SCOPED_STORAGE
        let target_28 = ExternalStoragePolicy::for_fresh_install(28, 28, false, Vec::new());
        let target_29_opted_in = ExternalStoragePolicy::for_fresh_install(29, 29, true, Vec::new());
        let target_29_opted_out =
            ExternalStoragePolicy::for_fresh_install(29, 29, false, Vec::new());
        let target_30 = ExternalStoragePolicy::for_fresh_install(30, 30, true, Vec::new());
        let preserved_target_30 = legacy_policy(30, vec![READ_EXTERNAL_STORAGE]);

        assert!(target_28.runtime_access.legacy_storage);
        assert!(target_29_opted_in.runtime_access.legacy_storage);
        assert!(!target_29_opted_out.runtime_access.legacy_storage);
        assert!(!target_30.runtime_access.legacy_storage);
        assert!(preserved_target_30.allows_public_path(ExternalStorageAccess::Read));
    }

    #[test]
    fn media_grants_do_not_fabricate_raw_scoped_file_access() {
        // cite: frameworks-base AndroidManifest.xml#READ_MEDIA_AUDIO, READ_MEDIA_VIDEO, READ_MEDIA_IMAGES, READ_MEDIA_VISUAL_USER_SELECTED
        // cite: frameworks-base Environment.java#isExternalStorageLegacy
        let media = ExternalStoragePolicy::new(
            34,
            34,
            vec![
                READ_MEDIA_AUDIO.to_owned(),
                READ_MEDIA_VIDEO.to_owned(),
                READ_MEDIA_IMAGES.to_owned(),
            ],
            ExternalStorageRuntimeAccess::new(
                vec![
                    READ_MEDIA_AUDIO.to_owned(),
                    READ_MEDIA_VIDEO.to_owned(),
                    READ_MEDIA_IMAGES.to_owned(),
                    READ_MEDIA_VISUAL_USER_SELECTED.to_owned(),
                ],
                false,
                false,
            ),
        );

        assert!(!media.allows_public_path(ExternalStorageAccess::Read));
        assert!(!media.allows_public_path(ExternalStorageAccess::Write));
    }

    #[test]
    fn environment_all_files_access_is_effective_state_not_manifest_declaration() {
        // cite: frameworks-base Environment.java#isExternalStorageManager
        let declaration_only = ExternalStoragePolicy::new(
            34,
            34,
            vec![MANAGE_EXTERNAL_STORAGE.to_owned()],
            ExternalStorageRuntimeAccess::default(),
        );
        let effective_state = ExternalStoragePolicy::new(
            34,
            34,
            Vec::new(),
            ExternalStorageRuntimeAccess::new(Vec::new(), false, true),
        );

        assert!(!declaration_only.allows_public_path(ExternalStorageAccess::Read));
        assert!(!declaration_only.allows_public_path(ExternalStorageAccess::Write));
        assert!(effective_state.allows_public_path(ExternalStorageAccess::Read));
        assert!(effective_state.allows_public_path(ExternalStorageAccess::Write));
    }

    #[test]
    fn resolves_public_directory_under_storage_root() {
        let prefix = test_prefix();
        let resolved =
            resolve_guest_path(&prefix, "/storage/emulated/0/Download").expect("download resolves");

        assert_eq!(resolved, PathBuf::from("/prefix/storage/Download"));
    }

    #[test]
    fn resolves_legacy_sdcard_alias_to_same_root() {
        let prefix = test_prefix();
        let resolved =
            resolve_guest_path(&prefix, "/sdcard/Documents/notes.txt").expect("sdcard resolves");

        assert_eq!(
            resolved,
            PathBuf::from("/prefix/storage/Documents/notes.txt")
        );
    }

    #[test]
    fn resolves_bare_volume_root() {
        let prefix = test_prefix();
        let resolved =
            resolve_guest_path(&prefix, "/storage/emulated/0").expect("volume root resolves");

        assert_eq!(resolved, PathBuf::from("/prefix/storage"));
    }

    #[test]
    fn rejects_paths_outside_the_volume() {
        let prefix = test_prefix();
        let error =
            resolve_guest_path(&prefix, "/etc/passwd").expect_err("host paths must not resolve");

        assert!(
            error
                .to_string()
                .contains("not on the primary external volume"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_parent_directory_escape() {
        let prefix = test_prefix();
        let error = resolve_guest_path(&prefix, "/storage/emulated/0/../../etc/passwd")
            .expect_err("traversal must be rejected");

        assert!(
            error
                .to_string()
                .contains("escapes the external storage volume"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn app_external_paths_include_data_obb_and_media() {
        // cite: frameworks-base Environment.java#buildExternalStorageAppFilesDirs, buildExternalStorageAppCacheDirs, buildExternalStorageAppObbDirs, buildExternalStorageAppMediaDirs
        let paths = app_external_guest_paths("com.example.app");

        assert_eq!(
            paths,
            [
                "/storage/emulated/0/Android/data/com.example.app/files".to_owned(),
                "/storage/emulated/0/Android/data/com.example.app/cache".to_owned(),
                "/storage/emulated/0/Android/obb/com.example.app".to_owned(),
                "/storage/emulated/0/Android/media/com.example.app".to_owned(),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn provisioning_rejects_a_symlinked_storage_root() {
        let root = test_root("storage-symlink");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).expect("outside directory should exist");
        std::os::unix::fs::symlink(&outside, root.join("storage"))
            .expect("test storage symlink should exist");
        let prefix = WinedroidPrefix::from_root(root.clone());

        assert!(provision_shared_storage(&prefix).is_err());
        assert!(!outside.join("Download").exists());

        fs::remove_dir_all(root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn app_provisioning_rejects_a_symlinked_android_ancestor() {
        let root = test_root("android-symlink");
        let storage = root.join("storage");
        let outside = root.join("outside");
        fs::create_dir_all(&storage).expect("storage directory should exist");
        fs::create_dir_all(&outside).expect("outside directory should exist");
        std::os::unix::fs::symlink(&outside, storage.join("Android"))
            .expect("test Android symlink should exist");
        let prefix = WinedroidPrefix::from_root(root.clone());

        assert!(provision_app_external_storage(&prefix, "com.example").is_err());
        assert!(!outside.join("data").exists());

        fs::remove_dir_all(root).ok();
    }

    fn test_root(name: &str) -> PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "winedroid-external-storage-{name}-{}-{id}",
            std::process::id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("test prefix should exist");
        root
    }

    fn legacy_policy(target_sdk_version: i32, permissions: Vec<&str>) -> ExternalStoragePolicy {
        let permissions = permissions
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let read_access = permissions
            .iter()
            .any(|permission| permission == READ_EXTERNAL_STORAGE);
        let write_access = permissions
            .iter()
            .any(|permission| permission == WRITE_EXTERNAL_STORAGE);
        ExternalStoragePolicy::new(
            target_sdk_version,
            target_sdk_version,
            permissions.clone(),
            ExternalStorageRuntimeAccess::with_legacy_file_access(
                permissions,
                true,
                read_access,
                write_access,
            ),
        )
    }
}

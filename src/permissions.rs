use crate::external_storage::ExternalStoragePolicy;

pub fn grant_declared_permissions(policy: &mut ExternalStoragePolicy) {
    let declared: Vec<String> = policy.declared_permissions().map(str::to_owned).collect();
    for permission in declared {
        policy.grant_permission(&permission);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_storage::{
        MANAGE_EXTERNAL_STORAGE, READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE,
    };

    #[test]
    fn grant_declared_permissions_grants_every_manifest_declared_permission() {
        let mut policy = ExternalStoragePolicy::for_fresh_install(
            29,
            29,
            true,
            vec![
                READ_EXTERNAL_STORAGE.to_owned(),
                WRITE_EXTERNAL_STORAGE.to_owned(),
                MANAGE_EXTERNAL_STORAGE.to_owned(),
            ],
        );

        assert_eq!(
            policy.permission_granted(READ_EXTERNAL_STORAGE),
            Some(false)
        );
        assert!(!policy.manages_external_storage());

        grant_declared_permissions(&mut policy);

        assert_eq!(policy.permission_granted(READ_EXTERNAL_STORAGE), Some(true));
        assert_eq!(
            policy.permission_granted(WRITE_EXTERNAL_STORAGE),
            Some(true)
        );
        assert!(policy.manages_external_storage());
        assert!(policy.allows_public_path(crate::external_storage::ExternalStorageAccess::Read));
        assert!(policy.allows_public_path(crate::external_storage::ExternalStorageAccess::Write));
    }

    #[test]
    fn grant_declared_permissions_does_not_grant_undeclared_permissions() {
        let mut policy = ExternalStoragePolicy::for_fresh_install(29, 29, true, Vec::new());

        grant_declared_permissions(&mut policy);

        assert_eq!(
            policy.permission_granted(READ_EXTERNAL_STORAGE),
            Some(false)
        );
    }
}

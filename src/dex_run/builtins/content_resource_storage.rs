use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_content_resource_storage_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if let Some(result) = self.invoke_android_content_resources_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_android_content_storage_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_android_platform_misc_builtin(method, invocation) {
            return Some(result);
        }

        None
    }
}

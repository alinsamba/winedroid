use super::super::*;

impl DexProgram {
    pub(super) fn invoke_concurrency_json_androidx_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if let Some(result) = self.invoke_java_json_collection_extra_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_concurrency_atomic_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_androidx_runtime_widget_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_third_party_runtime_builtin(method, invocation) {
            return Some(result);
        }

        None
    }
}

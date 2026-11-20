use super::super::*;

impl DexProgram {
    pub(super) fn invoke_java_core_text_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if let Some(result) = self.invoke_java_core_runtime_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_box_locale_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_java_string_text_builtin(method, invocation) {
            return Some(result);
        }

        None
    }
}

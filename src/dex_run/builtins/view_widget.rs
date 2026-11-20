use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_view_widget_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if let Some(result) = self.invoke_android_widget_adapters_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_android_view_core_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_android_layout_menu_builtin(method, invocation) {
            return Some(result);
        }

        if let Some(result) = self.invoke_android_text_image_builtin(method, invocation) {
            return Some(result);
        }

        None
    }
}

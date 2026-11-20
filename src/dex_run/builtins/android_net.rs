use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_net_builtin(
        &self,
        method: &DexMethodId,
        _invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.class_descriptor == "Landroid/net/ConnectivityManager;"
            && method.name == "getActiveNetworkInfo"
            && method.proto.descriptor_is("()Landroid/net/NetworkInfo;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        if method.class_descriptor == "Landroid/net/ConnectivityManager;"
            && method.name == "getActiveNetwork"
            && method.proto.descriptor_is("()Landroid/net/Network;")
        {
            return Some(DexExecutionResult::Value(DexValue::Null));
        }

        None
    }
}

use super::super::*;

impl DexProgram {
    pub(super) fn invoke_third_party_runtime_builtin(
        &self,
        method: &DexMethodId,
        _invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if method.matches_builtin_signature(
            "Lcom/google/firebase/analytics/FirebaseAnalytics;",
            "getInstance",
            "(Landroid/content/Context;)Lcom/google/firebase/analytics/FirebaseAnalytics;",
        ) {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Lcom/google/firebase/analytics/FirebaseAnalytics;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature(
            "Lcom/google/android/gms/internal/measurement/w6;",
            "<init>",
            "(Lcom/google/android/gms/internal/measurement/j7;Ljava/lang/Boolean;)V",
        ) {
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature(
            "Lcom/google/android/gms/internal/measurement/j7;",
            "a",
            "(Lcom/google/android/gms/internal/measurement/f7;)V",
        ) {
            return Some(DexExecutionResult::Void);
        }

        if method.matches_builtin_signature("Lw8/b0;", "getCoroutineContext", "()La8/i;") {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("La8/i;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature("Lw8/d0;", "z", "(Lw8/b0;La8/i;Lw8/c0;Lk8/e;I)Lw8/v1;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Lw8/v1;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature("Lw8/d0;", "y", "(Lw8/b0;La8/i;Lw8/c0;Lk8/e;)Lw8/v1;") {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Lw8/v1;.bootstrap"),
            )));
        }

        if method.matches_builtin_signature(
            "Lm2/b;",
            "f",
            "(Landroid/content/pm/PackageManager;Ljava/lang/String;I)Landroid/content/pm/PackageInfo;",
        ) {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/content/pm/PackageInfo;.bootstrap"),
            )));
        }

        None
    }
}

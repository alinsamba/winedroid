use super::super::*;

#[test]
fn builtin_firebase_analytics_get_instance_returns_bootstrap_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lcom/google/firebase/analytics/FirebaseAnalytics;".to_owned(),
        name: "getInstance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Lcom/google/firebase/analytics/FirebaseAnalytics;".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lcom/google/firebase/analytics/FirebaseAnalytics;.bootstrap")
        )))
    );
}

#[test]
fn builtin_measurement_collection_worker_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lcom/google/android/gms/internal/measurement/w6;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Lcom/google/android/gms/internal/measurement/j7;".to_owned(),
                "Ljava/lang/Boolean;".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_measurement_scheduler_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lcom/google/android/gms/internal/measurement/j7;".to_owned(),
        name: "a".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Lcom/google/android/gms/internal/measurement/f7;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_coroutine_scope_context_returns_bootstrap_context() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lw8/b0;".to_owned(),
        name: "getCoroutineContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "La8/i;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("La8/i;.bootstrap")
        )))
    );
}

#[test]
fn builtin_coroutine_launch_helpers_return_bootstrap_job() {
    let program = DexProgram::new(Vec::new());
    let launch_default = DexMethodId {
        class_descriptor: "Lw8/d0;".to_owned(),
        name: "z".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLILI".to_owned(),
            return_type: "Lw8/v1;".to_owned(),
            parameters: vec![
                "Lw8/b0;".to_owned(),
                "La8/i;".to_owned(),
                "Lw8/c0;".to_owned(),
                "Lk8/e;".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let launch = DexMethodId {
        class_descriptor: "Lw8/d0;".to_owned(),
        name: "y".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLLL".to_owned(),
            return_type: "Lw8/v1;".to_owned(),
            parameters: vec![
                "Lw8/b0;".to_owned(),
                "La8/i;".to_owned(),
                "Lw8/c0;".to_owned(),
                "Lk8/e;".to_owned(),
            ],
        },
    };
    let expected = Some(DexExecutionResult::Value(DexValue::Reference(
        std::rc::Rc::from("Lw8/v1;.bootstrap"),
    )));

    assert_eq!(
        program.invoke_builtin_method(&launch_default, None),
        expected
    );
    assert_eq!(
        program.invoke_builtin_method(&launch, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lw8/v1;.bootstrap")
        )))
    );
}

#[test]
fn builtin_package_info_wrapper_returns_bootstrap_package_info() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lm2/b;".to_owned(),
        name: "f".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLLI".to_owned(),
            return_type: "Landroid/content/pm/PackageInfo;".to_owned(),
            parameters: vec![
                "Landroid/content/pm/PackageManager;".to_owned(),
                "Ljava/lang/String;".to_owned(),
                "I".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/content/pm/PackageInfo;.bootstrap")
        )))
    );
}

use super::super::*;
use super::helpers::*;

#[test]
fn builtin_fragment_controller_dispatch_create_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/fragment/app/FragmentController;".to_owned(),
        name: "dispatchCreate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_obfuscated_auto_night_constructor_stores_delegate() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ld0/g;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Lg/F;".to_owned()],
        },
    };
    let receiver = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Lg/B;"),
    });
    let delegate = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Lg/F;"),
    });
    let field = DexFieldId {
        class_descriptor: "Ld0/g;".to_owned(),
        type_descriptor: "Ljava/lang/Object;".to_owned(),
        name: "a".to_owned(),
    };
    let invocation = test_invocation(vec![receiver.clone(), delegate.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&receiver, &field),
        Some(delegate)
    );
}

#[test]
fn builtin_appcompat_delegate_set_content_view_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/appcompat/app/AppCompatDelegate;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("delegate")),
        DexValue::Int(2131624000),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(program.view_snapshot().content_layouts, vec![2131624000]);
}

#[test]
// cite: androidx AppCompatDelegateImpl.java#findViewById
// cite: frameworks-base View.java#findViewById, findViewTraversal
fn builtin_appcompat_delegate_find_view_by_id_returns_null_when_id_is_absent() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/appcompat/app/AppCompatDelegate;".to_owned(),
        name: "findViewById".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("delegate")),
        DexValue::Int(2131362000),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(program.view_snapshot().view_lookup_ids, vec![2131362000]);
}

#[test]
fn builtin_linear_layout_manager_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/LinearLayoutManager;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "I".to_owned(),
                "Z".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_recycler_view_set_layout_manager_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "setLayoutManager".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroidx/recyclerview/widget/RecyclerView$LayoutManager;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_recycler_view_add_item_decoration_returns_void() {
    let program = DexProgram::new(Vec::new());
    let one_argument_method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "addItemDecoration".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroidx/recyclerview/widget/RecyclerView$ItemDecoration;".to_owned()
            ],
        },
    };
    let two_argument_method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "addItemDecoration".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroidx/recyclerview/widget/RecyclerView$ItemDecoration;".to_owned(),
                "I".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&one_argument_method, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&two_argument_method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_recycler_view_set_adapter_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "setAdapter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroidx/recyclerview/widget/RecyclerView$Adapter;".to_owned()],
        },
    };
    let recycler_view = DexValue::Reference(std::rc::Rc::from(
        "Landroidx/recyclerview/widget/RecyclerView;.bootstrap",
    ));
    let adapter = DexValue::Reference(std::rc::Rc::from("adapter"));
    let field = DexFieldId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        type_descriptor: "Landroidx/recyclerview/widget/RecyclerView$Adapter;".to_owned(),
        name: "mAdapter".to_owned(),
    };
    let invocation = test_invocation(vec![recycler_view.clone(), adapter.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&recycler_view, &field),
        Some(adapter)
    );
}

#[test]
fn builtin_recycler_view_scroll_methods_return_void() {
    let program = DexProgram::new(Vec::new());
    let scroll_to_position = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "scrollToPosition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let stop_scroll = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/RecyclerView;".to_owned(),
        name: "stopScroll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&scroll_to_position, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&stop_scroll, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_item_touch_helper_attach_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroidx/recyclerview/widget/ItemTouchHelper;".to_owned(),
        name: "attachToRecyclerView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroidx/recyclerview/widget/RecyclerView;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_tab_layout_remove_all_tabs_returns_void() {
    let program = DexProgram::new(Vec::new());
    let remove_all_tabs = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "removeAllTabs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let new_tab = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "newTab".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add_tab = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "addTab".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let get_tab_count = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "getTabCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_selected_tab_position = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "getSelectedTabPosition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_scroll_position = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout;".to_owned(),
        name: "setScrollPosition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIFZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "F".to_owned(), "Z".to_owned()],
        },
    };
    let set_content_description = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
        name: "setContentDescription".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let set_icon = DexMethodId {
        class_descriptor: "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
        name: "setIcon".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Lcom/google/android/material/tabs/TabLayout$Tab;".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let tab = DexValue::Reference(std::rc::Rc::from(
        "Lcom/google/android/material/tabs/TabLayout$Tab;:1",
    ));
    let setter_invocation = test_invocation(vec![
        tab.clone(),
        DexValue::String(std::rc::Rc::from("Home")),
    ]);
    let icon_invocation = test_invocation(vec![
        tab.clone(),
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/graphics/drawable/Drawable;:resource:1",
        )),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&remove_all_tabs, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&new_tab, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lcom/google/android/material/tabs/TabLayout$Tab;:1")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&add_tab, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_tab_count, None),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_selected_tab_position, None),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_scroll_position, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_content_description, Some(&setter_invocation)),
        Some(DexExecutionResult::Value(tab.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_icon, Some(&icon_invocation)),
        Some(DexExecutionResult::Value(tab))
    );
}

#[test]
fn builtin_target_app_singleton_accessor_returns_reference() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lt2/g;".to_owned(),
        name: "c".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Lt2/g;".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Lt2/g;.default")
        )))
    );
}

#[test]
fn builtin_target_app_returns_bootstrap_crashlytics_component() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Lt2/g;".to_owned(),
        name: "b".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/lang/Object;".to_owned(),
            parameters: vec!["Ljava/lang/Class;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Lt2/g;.default")),
        DexValue::Reference(std::rc::Rc::from("Ljava/lang/Class;:La3/e;")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("La3/e;.bootstrap")
        )))
    );
}

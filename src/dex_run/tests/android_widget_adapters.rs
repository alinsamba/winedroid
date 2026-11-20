use super::super::*;
use super::helpers::*;

#[test]
fn builtin_base_adapter_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/widget/BaseAdapter;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let item_view_type = DexMethodId {
        class_descriptor: "Landroid/widget/Adapter;".to_owned(),
        name: "getItemViewType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view_type_count = DexMethodId {
        class_descriptor: "Landroid/widget/BaseAdapter;".to_owned(),
        name: "getViewTypeCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &item_view_type,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("adapter")),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &view_type_count,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("adapter")
            )]))
        ),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_data_set_observer_constructor_and_callbacks_return_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/database/DataSetObserver;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_changed = DexMethodId {
        class_descriptor: "Landroid/database/DataSetObserver;".to_owned(),
        name: "onChanged".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let on_invalidated = DexMethodId {
        class_descriptor: "Landroid/database/DataSetObserver;".to_owned(),
        name: "onInvalidated".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let register_observer = DexMethodId {
        class_descriptor: "Landroid/widget/Adapter;".to_owned(),
        name: "registerDataSetObserver".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/database/DataSetObserver;".to_owned()],
        },
    };
    let unregister_observer = DexMethodId {
        class_descriptor: "Landroid/widget/Adapter;".to_owned(),
        name: "unregisterDataSetObserver".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/database/DataSetObserver;".to_owned()],
        },
    };
    let adapter = DexValue::Reference(std::rc::Rc::from("Landroid/widget/Adapter;:popup"));
    let observer = DexValue::Reference(std::rc::Rc::from(
        "Landroid/database/DataSetObserver;:popup",
    ));

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_changed, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&on_invalidated, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &register_observer,
            Some(&test_invocation(vec![adapter.clone(), observer.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&adapter, &adapter_data_set_observer_field()),
        Some(observer.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &unregister_observer,
            Some(&test_invocation(vec![adapter.clone(), observer]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&adapter, &adapter_data_set_observer_field()),
        Some(DexValue::Null)
    );
}

#[test]
fn builtin_abs_list_view_cache_color_hint_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_cache_color_hint = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "setCacheColorHint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_cache_color_hint = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "getCacheColorHint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_scroll_listener = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "setOnScrollListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/widget/AbsListView$OnScrollListener;".to_owned()],
        },
    };
    let get_list_padding_top = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "getListPaddingTop".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_divider_height = DexMethodId {
        class_descriptor: "Landroid/widget/ListView;".to_owned(),
        name: "setDividerHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_divider_height = DexMethodId {
        class_descriptor: "Landroid/widget/ListView;".to_owned(),
        name: "getDividerHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_divider = DexMethodId {
        class_descriptor: "Landroid/widget/ListView;".to_owned(),
        name: "setDivider".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let get_divider = DexMethodId {
        class_descriptor: "Landroid/widget/ListView;".to_owned(),
        name: "getDivider".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/drawable/Drawable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_in_touch_mode = DexMethodId {
        class_descriptor: "Landroid/widget/ListView;".to_owned(),
        name: "isInTouchMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    let scroll_listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/AbsListView$OnScrollListener;:popup",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &get_cache_color_hint,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_cache_color_hint,
            Some(&test_invocation(vec![list_view.clone(), DexValue::Int(16)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_cache_color_hint,
            Some(&test_invocation(vec![list_view]))
        ),
        Some(DexExecutionResult::Int(16))
    );
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    assert_eq!(
        program.invoke_builtin_method(
            &set_scroll_listener,
            Some(&test_invocation(vec![
                list_view.clone(),
                scroll_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&list_view, &abs_list_view_scroll_listener_field()),
        Some(scroll_listener)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_list_padding_top,
            Some(&test_invocation(vec![list_view]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    assert_eq!(
        program.invoke_builtin_method(
            &get_divider_height,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_divider_height,
            Some(&test_invocation(vec![list_view.clone(), DexValue::Int(3)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_divider_height, Some(&test_invocation(vec![list_view]))),
        Some(DexExecutionResult::Int(3))
    );
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    let divider = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:divider",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &get_divider,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_divider,
            Some(&test_invocation(vec![list_view.clone(), divider.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_divider,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(divider))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_in_touch_mode, Some(&test_invocation(vec![list_view]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_horizontal_scroll_view_full_scroll_records_requested_edge() {
    let program = DexProgram::new(Vec::new());
    let full_scroll = DexMethodId {
        class_descriptor: "Landroid/widget/HorizontalScrollView;".to_owned(),
        name: "fullScroll".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ZI".to_owned(),
            return_type: "Z".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let scroll_view = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/HorizontalScrollView;:calculator-input",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &full_scroll,
            Some(&test_invocation(vec![
                scroll_view.clone(),
                DexValue::Int(66)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&scroll_view, &horizontal_scroll_view_scroll_x_field()),
        Some(DexValue::Int(i32::MAX))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &full_scroll,
            Some(&test_invocation(vec![
                scroll_view.clone(),
                DexValue::Int(17)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&scroll_view, &horizontal_scroll_view_scroll_x_field()),
        Some(DexValue::Int(0))
    );
}

#[test]
fn builtin_adapter_view_adapter_round_trips() {
    let program = DexProgram::new(Vec::new());
    let set_adapter = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "setAdapter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/widget/ListAdapter;".to_owned()],
        },
    };
    let get_adapter = DexMethodId {
        class_descriptor: "Landroid/widget/AbsListView;".to_owned(),
        name: "getAdapter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/widget/ListAdapter;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    let adapter = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListAdapter;:popup"));

    assert_eq!(
        program.invoke_builtin_method(
            &get_adapter,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_adapter,
            Some(&test_invocation(vec![list_view.clone(), adapter.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_adapter, Some(&test_invocation(vec![list_view]))),
        Some(DexExecutionResult::Value(adapter))
    );
}

#[test]
fn builtin_adapter_view_item_listeners_round_trip() {
    let program = DexProgram::new(Vec::new());
    let set_click_listener = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "setOnItemClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/widget/AdapterView$OnItemClickListener;".to_owned()],
        },
    };
    let get_click_listener = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "getOnItemClickListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/widget/AdapterView$OnItemClickListener;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_selected_listener = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "setOnItemSelectedListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/widget/AdapterView$OnItemSelectedListener;".to_owned()],
        },
    };
    let get_selected_listener = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "getOnItemSelectedListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/widget/AdapterView$OnItemSelectedListener;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_selection = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "setSelection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_selected_item_position = DexMethodId {
        class_descriptor: "Landroid/widget/AdapterView;".to_owned(),
        name: "getSelectedItemPosition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let list_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ListView;:popup"));
    let click_listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/AdapterView$OnItemClickListener;:popup",
    ));
    let selected_listener = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/AdapterView$OnItemSelectedListener;:popup",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &get_click_listener,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_click_listener,
            Some(&test_invocation(vec![
                list_view.clone(),
                click_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_click_listener,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(click_listener))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selected_listener,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_selected_listener,
            Some(&test_invocation(vec![
                list_view.clone(),
                selected_listener.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selected_listener,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Value(selected_listener))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selected_item_position,
            Some(&test_invocation(vec![list_view.clone()]))
        ),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_selection,
            Some(&test_invocation(vec![list_view.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selected_item_position,
            Some(&test_invocation(vec![list_view]))
        ),
        Some(DexExecutionResult::Int(2))
    );
}

#[test]
fn builtin_view_widget_context_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/FrameLayout;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("frame")),
                DexValue::Reference(std::rc::Rc::from("context")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_popup_window_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "Landroid/util/AttributeSet;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let set_overlap_anchor = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setOverlapAnchor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_background = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setBackgroundDrawable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let get_background = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getBackground".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/drawable/Drawable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_input_method_mode = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setInputMethodMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_input_method_mode = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getInputMethodMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_max_available_height = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getMaxAvailableHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILIZ".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec![
                "Landroid/view/View;".to_owned(),
                "I".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let set_window_layout_type = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setWindowLayoutType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_window_layout_type = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getWindowLayoutType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_is_clipped_to_screen = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setIsClippedToScreen".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let set_epicenter_bounds = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setEpicenterBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Rect;".to_owned()],
        },
    };
    let get_epicenter_bounds = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getEpicenterBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Rect;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_content_view = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "setContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let get_content_view = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "getContentView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_showing = DexMethodId {
        class_descriptor: "Landroid/widget/PopupWindow;".to_owned(),
        name: "isShowing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let popup = DexValue::Reference(std::rc::Rc::from("popup"));
    let drawable = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:popup",
    ));

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_background, Some(&test_invocation(vec![popup.clone()]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_overlap_anchor,
            Some(&test_invocation(vec![popup.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&popup, &popup_window_overlap_anchor_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background,
            Some(&test_invocation(vec![popup.clone(), drawable.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_background, Some(&test_invocation(vec![popup.clone()]))),
        Some(DexExecutionResult::Value(drawable))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_input_method_mode,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_input_method_mode,
            Some(&test_invocation(vec![popup.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_input_method_mode,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    // cite: frameworks-base PopupWindow.java#getMaxAvailableHeight
    assert_eq!(
        program.invoke_builtin_method(
            &get_max_available_height,
            Some(&test_invocation(vec![
                popup.clone(),
                DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:anchor")),
                DexValue::Int(12),
                DexValue::Int(1)
            ]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_background,
            Some(&test_invocation(vec![popup.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_max_available_height,
            Some(&test_invocation(vec![
                popup.clone(),
                DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:anchor")),
                DexValue::Int(12),
                DexValue::Int(1)
            ]))
        ),
        Some(DexExecutionResult::Int(748))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_window_layout_type,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_window_layout_type,
            Some(&test_invocation(vec![popup.clone(), DexValue::Int(1002)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_window_layout_type,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Int(1002))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_is_clipped_to_screen,
            Some(&test_invocation(vec![popup.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&popup, &popup_window_is_clipped_to_screen_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_epicenter_bounds,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let epicenter = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Rect;:popup"));
    assert_eq!(
        program.invoke_builtin_method(
            &set_epicenter_bounds,
            Some(&test_invocation(vec![popup.clone(), epicenter.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_epicenter_bounds,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Value(epicenter))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_content_view,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let content_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:popup-content"));
    assert_eq!(
        program.invoke_builtin_method(
            &set_content_view,
            Some(&test_invocation(vec![popup.clone(), content_view.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_content_view,
            Some(&test_invocation(vec![popup.clone()]))
        ),
        Some(DexExecutionResult::Value(content_view))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_showing, Some(&test_invocation(vec![popup]))),
        Some(DexExecutionResult::Int(0))
    );
}

// cite: frameworks-base PopupWindow.java#getMaxAvailableHeight, setOverlapAnchor
// cite: frameworks-base View.java#getLocationOnScreen, getLocationInWindow, transformFromViewToWindowSpace
#[test]
fn popup_window_available_height_uses_custom_display_visible_frame_and_anchor_geometry() {
    let program = DexProgram::with_config(Vec::new(), WinedroidConfig::with_display_size(360, 800));
    let popup = DexValue::Reference(std::rc::Rc::from("Landroid/widget/PopupWindow;:geometry"));
    let parent = DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;:geometry"));
    let anchor = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:geometry"));

    for (view, bounds) in [
        (
            &parent,
            [("left", 0), ("top", 50), ("right", 360), ("bottom", 750)],
        ),
        (
            &anchor,
            [("left", 20), ("top", 100), ("right", 60), ("bottom", 140)],
        ),
    ] {
        for (name, value) in bounds {
            assert!(program.write_stored_instance_field(
                view,
                &view_bounds_int_field(name),
                DexValue::Int(value),
            ));
        }
    }
    program
        .runtime
        .borrow_mut()
        .view_parents
        .insert(map_identity_key(&anchor), parent.clone());
    {
        let mut runtime = program.runtime.borrow_mut();
        runtime.runtime_view_roots.push(DexRuntimeViewRoot {
            view_key: map_identity_key(&parent),
            class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        });
        runtime.window_visible_frame = Some((360, 700));
    }

    let invocation = |ignore_bottom_decorations| {
        test_invocation(vec![
            popup.clone(),
            anchor.clone(),
            DexValue::Int(10),
            DexValue::Int(ignore_bottom_decorations),
        ])
    };
    assert_eq!(
        program.popup_window_max_available_height(&invocation(0)),
        Some(500)
    );
    assert!(program.write_stored_instance_field(
        &popup,
        &popup_window_overlap_anchor_field(),
        DexValue::Int(1),
    ));
    assert_eq!(
        program.popup_window_max_available_height(&invocation(0)),
        Some(540)
    );
    assert_eq!(
        program.popup_window_max_available_height(&invocation(1)),
        Some(640)
    );
}

#[test]
fn popup_menu_constructor_records_anchor_and_get_menu_returns_a_menu_reference() {
    let program = DexProgram::new(Vec::new());
    let popup = DexValue::Reference(std::rc::Rc::from("Landroid/widget/PopupMenu;:1"));
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;:1"));
    let anchor = view_reference(2131034112);

    let constructor = DexMethodId {
        class_descriptor: "Landroid/widget/PopupMenu;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "Landroid/view/View;".to_owned(),
            ],
        },
    };
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                popup.clone(),
                context,
                anchor.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );

    let get_menu = DexMethodId {
        class_descriptor: "Landroid/widget/PopupMenu;".to_owned(),
        name: "getMenu".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/Menu;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let menu = program.invoke_builtin_method(&get_menu, Some(&test_invocation(vec![popup])));
    let Some(DexExecutionResult::Value(DexValue::Reference(menu_ref))) = menu else {
        panic!("getMenu should return a Menu reference");
    };
    assert!(
        menu_ref.starts_with("Landroid/view/Menu;:popup/"),
        "menu ref should be popup-scoped: {menu_ref}"
    );
}

use super::super::*;
use super::helpers::*;

#[test]
fn builtin_image_view_set_adjust_view_bounds_stores_flag() {
    // cite: frameworks-base ImageView.java#setAdjustViewBounds
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setAdjustViewBounds".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ImageView;:icon"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &image_view_adjust_view_bounds_field()),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_text_view_set_width_stores_value_and_requests_layout() {
    // cite: frameworks-base TextView.java#setWidth
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:label"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(240)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &text_view_width_field()),
        Some(DexValue::Int(240))
    );
}

#[test]
fn builtin_image_view_set_max_width_and_height_store_values() {
    // cite: frameworks-base ImageView.java#setMaxWidth, setMaxHeight
    let program = DexProgram::new(Vec::new());
    let set_max_width = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setMaxWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_max_height = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setMaxHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ImageView;:icon"));

    assert_eq!(
        program.invoke_builtin_method(
            &set_max_width,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(64)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_max_height,
            Some(&test_invocation(vec![view.clone(), DexValue::Int(48)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &image_view_max_width_field()),
        Some(DexValue::Int(64))
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &image_view_max_height_field()),
        Some(DexValue::Int(48))
    );
}

#[test]
fn builtin_image_view_set_scale_type_stores_value_and_requests_layout() {
    // cite: frameworks-base ImageView.java#setScaleType
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setScaleType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/widget/ImageView$ScaleType;".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ImageView;:icon"));
    let fit_center = DexValue::Reference(std::rc::Rc::from(
        "Landroid/widget/ImageView$ScaleType;:FIT_CENTER",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), fit_center.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &image_view_scale_type_field()),
        Some(fit_center)
    );
}

#[test]
fn builtin_text_view_set_text_size_no_unit_defaults_to_sp() {
    // cite: frameworks-base TextView.java#setTextSize
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTextSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/TextView;:label"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![view.clone(), DexValue::Float(18.0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&view, &text_view_text_size_field()),
        Some(DexValue::Float(18.0))
    );
}

#[test]
fn builtin_text_view_set_typeface_returns_void() {
    let program = DexProgram::new(Vec::new());
    let typeface = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTypeface".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Typeface;".to_owned()],
        },
    };
    let create_typeface = DexMethodId {
        class_descriptor: "Landroid/graphics/Typeface;".to_owned(),
        name: "create".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLI".to_owned(),
            return_type: "Landroid/graphics/Typeface;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned(), "I".to_owned()],
        },
    };
    let create_typeface_weight = DexMethodId {
        class_descriptor: "Landroid/graphics/Typeface;".to_owned(),
        name: "create".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLIZ".to_owned(),
            return_type: "Landroid/graphics/Typeface;".to_owned(),
            parameters: vec![
                "Landroid/graphics/Typeface;".to_owned(),
                "I".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let enabled = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let min_width = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setMinWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_text_size = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTextSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VIF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned(), "F".to_owned()],
        },
    };
    let set_text_color = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTextColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_text_appearance = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTextAppearance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned(), "I".to_owned()],
        },
    };
    let set_text_appearance_legacy = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setTextAppearance".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_paint = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "getPaint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/text/TextPaint;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let show_soft_input_on_focus = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setShowSoftInputOnFocus".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let add_text_watcher = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "addTextChangedListener".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/text/TextWatcher;".to_owned()],
        },
    };
    let watched_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296539"));
    let watcher = DexValue::Object(DexObjectRef {
        id: 7,
        class_descriptor: std::rc::Rc::from("Lexample/Watcher;"),
    });
    let add_text_watcher_invocation = test_invocation(vec![watched_view.clone(), watcher.clone()]);
    let set_text_size_invocation = test_invocation(vec![
        watched_view.clone(),
        DexValue::Int(2),
        DexValue::Float(16.0),
    ]);
    let set_text_color_invocation = test_invocation(vec![
        watched_view.clone(),
        DexValue::Int(0xff112233u32 as i32),
    ]);
    let set_text_appearance_invocation = test_invocation(vec![
        watched_view.clone(),
        DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
        DexValue::Int(2131886334),
    ]);
    let set_text_appearance_legacy_invocation =
        test_invocation(vec![watched_view.clone(), DexValue::Int(2131886335)]);

    assert_eq!(
        program.invoke_builtin_method(&typeface, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &create_typeface,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("sans-serif")),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/graphics/Typeface;:family/sans-serif:style/0")
        )))
    );
    let base_typeface = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Typeface;:family/sans-serif:style/0",
    ));
    assert_eq!(
        program.invoke_builtin_method(
            &create_typeface_weight,
            Some(&test_invocation(vec![
                base_typeface.clone(),
                DexValue::Int(400),
                DexValue::Int(0),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!(
                "Landroid/graphics/Typeface;:base/{}:weight/400:italic/0",
                map_identity_key(&base_typeface)
            ))
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&enabled, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&min_width, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_text_size, Some(&set_text_size_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&watched_view, &text_view_text_size_field()),
        Some(DexValue::Float(16.0))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_text_color, Some(&set_text_color_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&watched_view, &text_view_text_color_field()),
        Some(DexValue::Int(0xff112233u32 as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(&set_text_appearance, Some(&set_text_appearance_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&watched_view, &text_view_text_appearance_field()),
        Some(DexValue::Int(2131886334))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_text_appearance_legacy,
            Some(&set_text_appearance_legacy_invocation)
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&watched_view, &text_view_text_appearance_field()),
        Some(DexValue::Int(2131886335))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_paint,
            Some(&test_invocation(vec![watched_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from(format!(
                "Landroid/text/TextPaint;:{}",
                map_identity_key(&watched_view)
            ))
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&show_soft_input_on_focus, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&add_text_watcher, Some(&add_text_watcher_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .view_text_watchers
            .get(&map_identity_key(&watched_view)),
        Some(&vec![watcher])
    );
}

#[test]
// cite: frameworks-base TextPaint.java#TextPaint
// cite: frameworks-base Paint.java#Paint, HIDDEN_DEFAULT_PAINT_FLAGS
fn builtin_text_paint_constructor_stores_flags_and_default_text_size() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/TextPaint;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_flags = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getFlags".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_text_size = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getTextSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_text_scale_x = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getTextScaleX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_letter_spacing = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getLetterSpacing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "F".to_owned(),
            return_type: "F".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_color_long = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getColorLong".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_alpha = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_filter_bitmap = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "isFilterBitmap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_flags = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setFlags".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_hinting = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setHinting".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_color = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_alpha = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setAlpha".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_color_long = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VJ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["J".to_owned()],
        },
    };
    let set_text_size = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setTextSize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_text_scale_x = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setTextScaleX".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_letter_spacing = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setLetterSpacing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VF".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["F".to_owned()],
        },
    };
    let set_shader = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setShader".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/graphics/Shader;".to_owned(),
            parameters: vec!["Landroid/graphics/Shader;".to_owned()],
        },
    };
    let set_style = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setStyle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Paint$Style;".to_owned()],
        },
    };
    let set_blend_mode = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setBlendMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/BlendMode;".to_owned()],
        },
    };
    let set_color_filter = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setColorFilter".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/graphics/ColorFilter;".to_owned(),
            parameters: vec!["Landroid/graphics/ColorFilter;".to_owned()],
        },
    };
    let set_typeface = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "setTypeface".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/graphics/Typeface;".to_owned(),
            parameters: vec!["Landroid/graphics/Typeface;".to_owned()],
        },
    };
    let text_paint = DexValue::Object(DexObjectRef {
        id: 8,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let typeface = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Typeface;:family/serif:style/0",
    ));
    let replacement_typeface = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/Typeface;:family/sans:style/1",
    ));
    let shader = DexValue::Object(DexObjectRef {
        id: 9,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Shader;"),
    });
    let replacement_shader =
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Shader;:replacement"));
    let style = DexValue::Reference(std::rc::Rc::from("Landroid/graphics/Paint$Style;.STROKE"));
    let blend_mode =
        DexValue::Reference(std::rc::Rc::from("Landroid/graphics/BlendMode;.SRC_OVER"));
    let color_filter = DexValue::Object(DexObjectRef {
        id: 10,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/ColorFilter;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(3)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_flags_field()),
        Some(DexValue::Int(0x503))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_flags, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(0x503))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(255))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_filter_bitmap,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_flags,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(5)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_flags, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_filter_bitmap,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_hinting,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_hinting_field()),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_color,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Int(0xff445566u32 as i32)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_field()),
        Some(DexValue::Int(0xff445566u32 as i32))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_alpha_field()),
        Some(DexValue::Int(0xff))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(0xff))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_color_long,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            ((0xff445566u32 as u64) << 32) as i64
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_alpha,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Int(0x80)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_alpha_field()),
        Some(DexValue::Int(0x80))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(0x80))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_field()),
        Some(DexValue::Int(0x80445566u32 as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_color_long,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(
            ((0x80445566u32 as u64) << 32) as i64
        )))
    );
    let color_long = ((0xaa102030u32 as u64) << 32) as i64;
    assert_eq!(
        program.invoke_builtin_method(
            &set_color_long,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Long(color_long),
                DexValue::Long(color_long)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_long_field()),
        Some(DexValue::Long(color_long))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_field()),
        Some(DexValue::Int(0xaa102030u32 as i32))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_alpha_field()),
        Some(DexValue::Int(0xaa))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_alpha, Some(&test_invocation(vec![text_paint.clone()]))),
        Some(DexExecutionResult::Int(0xaa))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_color_long,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Long(color_long)))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_text_size_field()),
        Some(DexValue::Float(16.0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_text_size,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(16.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_text_size,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Float(18.0)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_text_size_field()),
        Some(DexValue::Float(18.0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_text_size,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(18.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_text_scale_x,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(1.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_text_scale_x,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Float(1.25)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_text_scale_x_field()),
        Some(DexValue::Float(1.25))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_text_scale_x,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(1.25)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_letter_spacing,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_letter_spacing,
            Some(&test_invocation(vec![
                text_paint.clone(),
                DexValue::Float(0.05)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_letter_spacing_field()),
        Some(DexValue::Float(0.05))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_letter_spacing,
            Some(&test_invocation(vec![text_paint.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.05)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_shader,
            Some(&test_invocation(vec![text_paint.clone(), shader.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_shader_field()),
        Some(shader.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_shader,
            Some(&test_invocation(vec![
                text_paint.clone(),
                replacement_shader.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(shader))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_shader_field()),
        Some(replacement_shader)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_shader,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/graphics/Shader;:replacement")
        )))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_shader_field()),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_style,
            Some(&test_invocation(vec![text_paint.clone(), style.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_style_field()),
        Some(style)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_blend_mode,
            Some(&test_invocation(vec![
                text_paint.clone(),
                blend_mode.clone()
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_blend_mode_field()),
        Some(blend_mode)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_blend_mode,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_blend_mode_field()),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_color_filter,
            Some(&test_invocation(vec![
                text_paint.clone(),
                color_filter.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(color_filter.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_filter_field()),
        Some(color_filter)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_color_filter,
            Some(&test_invocation(vec![text_paint.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_filter_field()),
        Some(DexValue::Null)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_typeface,
            Some(&test_invocation(vec![text_paint.clone(), typeface.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_typeface_field()),
        Some(typeface.clone())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_typeface,
            Some(&test_invocation(vec![
                text_paint.clone(),
                replacement_typeface.clone()
            ]))
        ),
        Some(DexExecutionResult::Value(typeface))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_typeface_field()),
        Some(replacement_typeface)
    );
}

#[test]
fn builtin_character_style_constructor_is_noop() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/style/CharacterStyle;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_foreground_color_span_stores_color_and_updates_text_paint() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/style/ForegroundColorSpan;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_color = DexMethodId {
        class_descriptor: "Landroid/text/style/ForegroundColorSpan;".to_owned(),
        name: "getForegroundColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let update_draw_state = DexMethodId {
        class_descriptor: "Landroid/text/style/ForegroundColorSpan;".to_owned(),
        name: "updateDrawState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/text/TextPaint;".to_owned()],
        },
    };
    let span = DexValue::Object(DexObjectRef {
        id: 21,
        class_descriptor: std::rc::Rc::from("Landroid/text/style/ForegroundColorSpan;"),
    });
    let text_paint = DexValue::Object(DexObjectRef {
        id: 22,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let color = 0xff12_3456u32 as i32;

    assert!(program.value_matches_type(&span, "Landroid/text/style/CharacterStyle;"));
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![span.clone(), DexValue::Int(color)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_color, Some(&test_invocation(vec![span.clone()]))),
        Some(DexExecutionResult::Int(color))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &update_draw_state,
            Some(&test_invocation(vec![span, text_paint.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_color_field()),
        Some(DexValue::Int(color))
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &paint_alpha_field()),
        Some(DexValue::Int(255))
    );
}

#[test]
fn builtin_background_color_span_stores_color_and_updates_text_paint() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/style/BackgroundColorSpan;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_color = DexMethodId {
        class_descriptor: "Landroid/text/style/BackgroundColorSpan;".to_owned(),
        name: "getBackgroundColor".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let update_draw_state = DexMethodId {
        class_descriptor: "Landroid/text/style/BackgroundColorSpan;".to_owned(),
        name: "updateDrawState".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/text/TextPaint;".to_owned()],
        },
    };
    let span = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Landroid/text/style/BackgroundColorSpan;"),
    });
    let text_paint = DexValue::Object(DexObjectRef {
        id: 24,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let color = 0x7f65_4321i32;

    assert!(program.value_matches_type(&span, "Landroid/text/style/CharacterStyle;"));
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![span.clone(), DexValue::Int(color)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_color, Some(&test_invocation(vec![span.clone()]))),
        Some(DexExecutionResult::Int(color))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &update_draw_state,
            Some(&test_invocation(vec![span, text_paint.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&text_paint, &text_paint_background_color_field()),
        Some(DexValue::Int(color))
    );
}

#[test]
fn builtin_spannable_string_constructor_stores_char_sequence_text() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/SpannableString;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let set_span = DexMethodId {
        class_descriptor: "Landroid/text/Spannable;".to_owned(),
        name: "setSpan".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLIII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/Object;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let spannable = DexValue::Object(DexObjectRef {
        id: 11,
        class_descriptor: std::rc::Rc::from("Landroid/text/SpannableString;"),
    });
    let span = DexValue::Object(DexObjectRef {
        id: 12,
        class_descriptor: std::rc::Rc::from("Landroid/text/style/CharacterStyle;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                spannable.clone(),
                DexValue::String(std::rc::Rc::from("timer"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .string_values
            .get(&map_identity_key(&spannable)),
        Some(&"timer".to_owned())
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_span,
            Some(&test_invocation(vec![
                spannable.clone(),
                span.clone(),
                DexValue::Int(0),
                DexValue::Int(5),
                DexValue::Int(33),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .spannable_spans
            .get(&map_identity_key(&spannable))
            .cloned(),
        Some(vec![SpannableSpanState {
            value: span,
            start: 0,
            end: 5,
            flags: 33,
        }])
    );
}

#[test]
// cite: frameworks-base Paint.java#Paint, HIDDEN_DEFAULT_PAINT_FLAGS
fn builtin_paint_constructors_match_frameworks_base_paint_defaults() {
    let program = DexProgram::new(Vec::new());
    let flags_constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let default_constructor = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 81,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });
    let default_paint = DexValue::Object(DexObjectRef {
        id: 82,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Paint;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &flags_constructor,
            Some(&test_invocation(vec![paint.clone(), DexValue::Int(7)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_flags_field()),
        Some(DexValue::Int(0x507))
    );
    assert_eq!(
        program.read_stored_instance_field(&paint, &paint_text_size_field()),
        Some(DexValue::Float(16.0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &default_constructor,
            Some(&test_invocation(vec![default_paint.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&default_paint, &paint_flags_field()),
        Some(DexValue::Int(0x503))
    );
    assert_eq!(
        program.read_stored_instance_field(&default_paint, &paint_text_size_field()),
        Some(DexValue::Float(16.0))
    );
    let get_color_long = DexMethodId {
        class_descriptor: "Landroid/graphics/Paint;".to_owned(),
        name: "getColorLong".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "J".to_owned(),
            return_type: "J".to_owned(),
            parameters: Vec::new(),
        },
    };
    assert_eq!(
        program.invoke_builtin_method(&get_color_long, Some(&test_invocation(vec![paint]))),
        Some(DexExecutionResult::Value(DexValue::Long(
            ((0xff000000u32 as u64) << 32) as i64
        )))
    );
}

#[test]
fn builtin_boring_layout_is_boring_writes_metrics_for_simple_text() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        name: "isBoring".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLLZL".to_owned(),
            return_type: "Landroid/text/BoringLayout$Metrics;".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "Landroid/text/TextPaint;".to_owned(),
                "Landroid/text/TextDirectionHeuristic;".to_owned(),
                "Z".to_owned(),
                "Landroid/text/BoringLayout$Metrics;".to_owned(),
            ],
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 13,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let metrics = DexValue::Object(DexObjectRef {
        id: 14,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout$Metrics;"),
    });
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(20.0),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("timer")),
                paint,
                DexValue::Null,
                DexValue::Int(1),
                metrics.clone(),
            ]))
        ),
        Some(DexExecutionResult::Value(metrics.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(&metrics, &boring_layout_metrics_width_field()),
        Some(DexValue::Int(50))
    );
    assert_eq!(
        program.read_stored_instance_field(&metrics, &paint_font_metrics_int_field("ascent")),
        Some(DexValue::Int(-16))
    );
    assert_eq!(
        program.read_stored_instance_field(&metrics, &paint_font_metrics_int_field("descent")),
        Some(DexValue::Int(4))
    );
}

#[test]
fn builtin_layout_get_desired_width_measures_utf16_range_with_text_paint() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getDesiredWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FLIIL".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Landroid/text/TextPaint;".to_owned(),
            ],
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 15,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(10.0),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_scale_x_field(),
        DexValue::Float(2.0),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}bc\ndef")),
                DexValue::Int(1),
                DexValue::Int(5),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(40.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("a\u{1f600}bc\ndef")),
                DexValue::Int(0),
                DexValue::Int(9),
                paint.clone(),
            ]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(50.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::String(std::rc::Rc::from("timer")),
                DexValue::Int(4),
                DexValue::Int(3),
                paint.clone(),
            ]))
        ),
        None
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                DexValue::Null,
                DexValue::Int(0),
                DexValue::Int(0),
                paint,
            ]))
        ),
        None
    );
}

#[test]
fn builtin_layout_get_line_for_offset_uses_utf16_line_starts() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineForOffset".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 16,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout;"),
    });
    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_char_sequence_field("text"),
        DexValue::String(std::rc::Rc::from("a\nb\u{1f600}")),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(5)]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout, DexValue::Int(6)]))
        ),
        None
    );
}

#[test]
fn builtin_layout_get_primary_horizontal_measures_from_line_start() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getPrimaryHorizontal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 18,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(10.0),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_scale_x_field(),
        DexValue::Float(2.0),
    ));
    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_char_sequence_field("text"),
        DexValue::String(std::rc::Rc::from("a\nb\u{1f600}")),
    ));
    assert!(program.write_stored_instance_field(&layout, &text_layout_paint_field(), paint,));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(10.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(5)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(30.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![layout, DexValue::Int(6)]))
        ),
        None
    );
}

#[test]
fn builtin_layout_alignment_values_returns_android_enum_order() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/Layout$Alignment;".to_owned(),
        name: "values".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[Landroid/text/Layout$Alignment;".to_owned(),
            parameters: Vec::new(),
        },
    };

    let Some(DexExecutionResult::Value(array)) = program.invoke_builtin_method(&method, None)
    else {
        panic!("Layout.Alignment.values should return an array");
    };
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&array)),
        Some(&vec![
            DexValue::Reference(std::rc::Rc::from(
                "Landroid/text/Layout$Alignment;.ALIGN_NORMAL"
            )),
            DexValue::Reference(std::rc::Rc::from(
                "Landroid/text/Layout$Alignment;.ALIGN_OPPOSITE"
            )),
            DexValue::Reference(std::rc::Rc::from(
                "Landroid/text/Layout$Alignment;.ALIGN_CENTER"
            )),
        ])
    );
}

#[test]
fn builtin_static_layout_builder_obtain_stores_text_layout_defaults() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "obtain".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLIILI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
                "Landroid/text/TextPaint;".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let paint = DexValue::Object(DexObjectRef {
        id: 20,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });

    let Some(DexExecutionResult::Value(builder)) = program.invoke_builtin_method(
        &method,
        Some(&test_invocation(vec![
            DexValue::String(std::rc::Rc::from("xTimerz")),
            DexValue::Int(1),
            DexValue::Int(6),
            paint.clone(),
            DexValue::Int(120),
        ])),
    ) else {
        panic!("StaticLayout.Builder.obtain should return a builder");
    };

    assert!(matches!(
        &builder,
        DexValue::Object(DexObjectRef {
            class_descriptor,
            ..
        }) if class_descriptor.as_ref() == "Landroid/text/StaticLayout$Builder;"
    ));
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_char_sequence_field("text")
        ),
        Some(DexValue::String(std::rc::Rc::from("Timer")))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("paint", "Landroid/text/TextPaint;")
        ),
        Some(paint)
    );
    assert_eq!(
        program.read_stored_instance_field(&builder, &static_layout_builder_int_field("width")),
        Some(DexValue::Int(120))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("alignment", "Landroid/text/Layout$Alignment;")
        ),
        Some(DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/Layout$Alignment;.ALIGN_NORMAL"
        )))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&builder, &static_layout_builder_int_field("includePad")),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_float_field("spacingMult")
        ),
        Some(DexValue::Float(1.0))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&builder, &static_layout_builder_float_field("spacingAdd")),
        Some(DexValue::Float(0.0))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("ellipsizedWidth")
        ),
        Some(DexValue::Int(120))
    );
    assert_eq!(
        program.read_stored_instance_field(&builder, &static_layout_builder_int_field("maxLines")),
        Some(DexValue::Int(i32::MAX))
    );
}

#[test]
fn builtin_static_layout_builder_set_text_direction_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setTextDirection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Landroid/text/TextDirectionHeuristic;".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 21,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let heuristic = DexValue::Reference(std::rc::Rc::from(
        "Landroid/text/TextDirectionHeuristics;.LTR",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), heuristic.clone()]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field(
                "textDirection",
                "Landroid/text/TextDirectionHeuristic;"
            )
        ),
        Some(heuristic)
    );
}

#[test]
fn builtin_static_layout_builder_set_alignment_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setAlignment".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Landroid/text/Layout$Alignment;".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 22,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let alignment = DexValue::Reference(std::rc::Rc::from(
        "Landroid/text/Layout$Alignment;.ALIGN_CENTER",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), alignment.clone()]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("alignment", "Landroid/text/Layout$Alignment;")
        ),
        Some(alignment)
    );
}

#[test]
fn builtin_static_layout_builder_set_max_lines_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setMaxLines".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 23,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(3)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(&builder, &static_layout_builder_int_field("maxLines")),
        Some(DexValue::Int(3))
    );
}

#[test]
fn builtin_static_layout_builder_set_ellipsize_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setEllipsize".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Landroid/text/TextUtils$TruncateAt;".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 24,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let ellipsize =
        DexValue::Reference(std::rc::Rc::from("Landroid/text/TextUtils$TruncateAt;.END"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), ellipsize.clone()]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("ellipsize", "Landroid/text/TextUtils$TruncateAt;")
        ),
        Some(ellipsize)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("ellipsize", "Landroid/text/TextUtils$TruncateAt;")
        ),
        Some(DexValue::Null)
    );
}

#[test]
fn builtin_static_layout_builder_set_ellipsized_width_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setEllipsizedWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 25,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(96)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("ellipsizedWidth")
        ),
        Some(DexValue::Int(96))
    );
}

#[test]
fn builtin_static_layout_builder_set_line_spacing_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setLineSpacing".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LFF".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["F".to_owned(), "F".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 26,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                builder.clone(),
                DexValue::Float(2.5),
                DexValue::Float(1.25),
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&builder, &static_layout_builder_float_field("spacingAdd")),
        Some(DexValue::Float(2.5))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_float_field("spacingMult")
        ),
        Some(DexValue::Float(1.25))
    );
}

#[test]
fn builtin_static_layout_builder_set_include_pad_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setIncludePad".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 27,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program
            .read_stored_instance_field(&builder, &static_layout_builder_int_field("includePad")),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder, DexValue::Int(2)]))
        ),
        None
    );
}

#[test]
fn builtin_static_layout_builder_set_break_strategy_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setBreakStrategy".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 28,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("breakStrategy")
        ),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_static_layout_builder_set_hyphenation_frequency_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setHyphenationFrequency".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 29,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("hyphenationFrequency")
        ),
        Some(DexValue::Int(2))
    );
}

#[test]
fn builtin_static_layout_builder_set_indents_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setIndents".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLL".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["[I".to_owned(), "[I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 30,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let left = DexValue::Reference(std::rc::Rc::from("[I:left"));

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![
                builder.clone(),
                left.clone(),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("leftIndents", "[I")
        ),
        Some(left)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field("rightIndents", "[I")
        ),
        Some(DexValue::Null)
    );
}

#[test]
fn builtin_static_layout_builder_set_justification_mode_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setJustificationMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 31,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("justificationMode")
        ),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_static_layout_builder_set_use_line_spacing_from_fallbacks_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setUseLineSpacingFromFallbacks".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 32,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("useLineSpacingFromFallbacks")
        ),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_static_layout_builder_set_line_break_config_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setLineBreakConfig".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Landroid/graphics/text/LineBreakConfig;".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 33,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let config = DexValue::Object(DexObjectRef {
        id: 34,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), config.clone()]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_object_field(
                "lineBreakConfig",
                "Landroid/graphics/text/LineBreakConfig;"
            )
        ),
        Some(config)
    );
}

#[test]
fn builtin_static_layout_builder_set_use_bounds_for_width_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "setUseBoundsForWidth".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LZ".to_owned(),
            return_type: "Landroid/text/StaticLayout$Builder;".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 35,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &static_layout_builder_int_field("useBoundsForWidth")
        ),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_static_layout_builder_build_creates_static_layout_state() {
    let program = DexProgram::new(Vec::new());
    let build = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout$Builder;".to_owned(),
        name: "build".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/text/StaticLayout;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_line_count = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_height = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 36,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout$Builder;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 37,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let line_break_config = DexValue::Object(DexObjectRef {
        id: 38,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig;"),
    });
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(20.0),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_char_sequence_field("text"),
        DexValue::String(std::rc::Rc::from("hi\nthere")),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_object_field("paint", "Landroid/text/TextPaint;"),
        paint.clone(),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_int_field("width"),
        DexValue::Int(120),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_object_field("alignment", "Landroid/text/Layout$Alignment;"),
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/text/Layout$Alignment;.ALIGN_NORMAL"
        )),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_float_field("spacingMult"),
        DexValue::Float(1.0),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_float_field("spacingAdd"),
        DexValue::Float(0.0),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_int_field("includePad"),
        DexValue::Int(1),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_int_field("ellipsizedWidth"),
        DexValue::Int(96),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_int_field("maxLines"),
        DexValue::Int(2),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &static_layout_builder_object_field(
            "lineBreakConfig",
            "Landroid/graphics/text/LineBreakConfig;"
        ),
        line_break_config.clone(),
    ));

    let Some(DexExecutionResult::Value(layout)) =
        program.invoke_builtin_method(&build, Some(&test_invocation(vec![builder])))
    else {
        panic!("StaticLayout.Builder.build should return a layout");
    };

    assert!(matches!(
        &layout,
        DexValue::Object(DexObjectRef {
            class_descriptor,
            ..
        }) if class_descriptor.as_ref() == "Landroid/text/StaticLayout;"
    ));
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_char_sequence_field("text")),
        Some(DexValue::String(std::rc::Rc::from("hi\nthere")))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_paint_field()),
        Some(paint)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_int_field("width")),
        Some(DexValue::Int(120))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &boring_layout_int_field("ellipsizedWidth")),
        Some(DexValue::Int(96))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &layout,
            &text_layout_object_field("lineBreakConfig", "Landroid/graphics/text/LineBreakConfig;")
        ),
        Some(line_break_config)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_count,
            Some(&test_invocation(vec![layout.clone()]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_height, Some(&test_invocation(vec![layout]))),
        Some(DexExecutionResult::Int(48))
    );
}

#[test]
fn builtin_static_layout_is_fallback_line_spacing_enabled_reads_layout_state() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/text/StaticLayout;".to_owned(),
        name: "isFallbackLineSpacingEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 39,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![layout.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_int_field("useLineSpacingFromFallbacks"),
        DexValue::Int(1),
    ));
    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![layout]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn builtin_line_break_config_builder_constructor_sets_defaults() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 33,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![builder.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &line_break_config_builder_int_field("lineBreakStyle")
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &line_break_config_builder_int_field("lineBreakWordStyle")
        ),
        Some(DexValue::Int(0))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &line_break_config_builder_int_field("hyphenation")
        ),
        Some(DexValue::Int(0))
    );
}

#[test]
fn builtin_line_break_config_builder_set_line_break_style_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
        name: "setLineBreakStyle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 34,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(2)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &line_break_config_builder_int_field("lineBreakStyle")
        ),
        Some(DexValue::Int(2))
    );
}

#[test]
fn builtin_line_break_config_builder_set_line_break_word_style_returns_builder() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
        name: "setLineBreakWordStyle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LI".to_owned(),
            return_type: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 35,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig$Builder;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &method,
            Some(&test_invocation(vec![builder.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Value(builder.clone()))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &builder,
            &line_break_config_builder_int_field("lineBreakWordStyle")
        ),
        Some(DexValue::Int(1))
    );
}

#[test]
fn builtin_line_break_config_builder_build_copies_builder_state() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/graphics/text/LineBreakConfig$Builder;".to_owned(),
        name: "build".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/text/LineBreakConfig;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let builder = DexValue::Object(DexObjectRef {
        id: 36,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/text/LineBreakConfig$Builder;"),
    });
    assert!(program.write_stored_instance_field(
        &builder,
        &line_break_config_builder_int_field("lineBreakStyle"),
        DexValue::Int(2),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &line_break_config_builder_int_field("lineBreakWordStyle"),
        DexValue::Int(1),
    ));
    assert!(program.write_stored_instance_field(
        &builder,
        &line_break_config_builder_int_field("hyphenation"),
        DexValue::Int(3),
    ));

    let Some(DexExecutionResult::Value(config)) =
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![builder])))
    else {
        panic!("LineBreakConfig.Builder.build should return a config");
    };

    assert!(matches!(
        &config,
        DexValue::Object(DexObjectRef {
            class_descriptor,
            ..
        }) if class_descriptor.as_ref() == "Landroid/graphics/text/LineBreakConfig;"
    ));
    assert_eq!(
        program.read_stored_instance_field(&config, &line_break_config_int_field("lineBreakStyle")),
        Some(DexValue::Int(2))
    );
    assert_eq!(
        program.read_stored_instance_field(
            &config,
            &line_break_config_int_field("lineBreakWordStyle")
        ),
        Some(DexValue::Int(1))
    );
    assert_eq!(
        program.read_stored_instance_field(&config, &line_break_config_int_field("hyphenation")),
        Some(DexValue::Int(3))
    );
}

#[test]
fn builtin_boring_layout_constructor_stores_layout_state() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLLILFFLZLIZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "Landroid/text/TextPaint;".to_owned(),
                "I".to_owned(),
                "Landroid/text/Layout$Alignment;".to_owned(),
                "F".to_owned(),
                "F".to_owned(),
                "Landroid/text/BoringLayout$Metrics;".to_owned(),
                "Z".to_owned(),
                "Landroid/text/TextUtils$TruncateAt;".to_owned(),
                "I".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let get_line_count = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_height = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getHeight".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_line_start = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineStart".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_end = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineEnd".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_top = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineTop".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_ascent = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineAscent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_descent = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineDescent".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_baseline = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineBaseline".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_bottom = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineBottom".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_line_left = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineLeft".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "FI".to_owned(),
            return_type: "F".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_ellipsis_count = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getEllipsisCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let get_top_padding = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getTopPadding".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_bottom_padding = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getBottomPadding".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_text = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_paint = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getPaint".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/text/TextPaint;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let draw = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "draw".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/Canvas;".to_owned()],
        },
    };
    let fallback_line_spacing = DexMethodId {
        class_descriptor: "Landroid/text/BoringLayout;".to_owned(),
        name: "isFallbackLineSpacingEnabled".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 15,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 16,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });
    let metrics = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Landroid/text/BoringLayout$Metrics;"),
    });
    let canvas = DexValue::Object(DexObjectRef {
        id: 18,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Canvas;"),
    });
    let text_canvas = DexValue::Object(DexObjectRef {
        id: 19,
        class_descriptor: std::rc::Rc::from("Ltp2;"),
    });
    let alignment = DexValue::Reference(std::rc::Rc::from(
        "Landroid/text/Layout$Alignment;.ALIGN_NORMAL",
    ));
    assert!(program.write_stored_instance_field(
        &metrics,
        &paint_font_metrics_int_field("top"),
        DexValue::Int(-15),
    ));
    assert!(program.write_stored_instance_field(
        &metrics,
        &paint_font_metrics_int_field("ascent"),
        DexValue::Int(-12),
    ));
    assert!(program.write_stored_instance_field(
        &metrics,
        &paint_font_metrics_int_field("descent"),
        DexValue::Int(4),
    ));
    assert!(program.write_stored_instance_field(
        &metrics,
        &paint_font_metrics_int_field("bottom"),
        DexValue::Int(5),
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                layout.clone(),
                DexValue::String(std::rc::Rc::from("timer")),
                paint.clone(),
                DexValue::Int(80),
                alignment.clone(),
                DexValue::Float(1.0),
                DexValue::Float(0.0),
                metrics.clone(),
                DexValue::Int(1),
                DexValue::Int(0),
                DexValue::Int(80),
                DexValue::Int(1),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_char_sequence_field("text")),
        Some(DexValue::String(std::rc::Rc::from("timer")))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_paint_field()),
        Some(paint.clone())
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_int_field("width")),
        Some(DexValue::Int(80))
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &text_layout_alignment_field()),
        Some(alignment)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &boring_layout_metrics_field()),
        Some(metrics)
    );
    assert_eq!(
        program.read_stored_instance_field(&layout, &boring_layout_int_field("height")),
        Some(DexValue::Int(20))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_count,
            Some(&test_invocation(vec![layout.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_height, Some(&test_invocation(vec![layout.clone()]))),
        Some(DexExecutionResult::Int(20))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_start,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_start,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_end,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(5))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_top,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_top,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(1)]))
        ),
        Some(DexExecutionResult::Int(20))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_ascent,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(-12))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_descent,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(4))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_baseline,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(15))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_bottom,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(20))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_line_left,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Value(DexValue::Float(0.0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_ellipsis_count,
            Some(&test_invocation(vec![layout.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_top_padding,
            Some(&test_invocation(vec![layout.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_bottom_padding,
            Some(&test_invocation(vec![layout.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_text, Some(&test_invocation(vec![layout.clone()]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("timer")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_paint, Some(&test_invocation(vec![layout.clone()]))),
        Some(DexExecutionResult::Value(paint.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &draw,
            Some(&test_invocation(vec![layout.clone(), canvas.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&map_identity_key(&canvas))
            .cloned(),
        Some(vec![CanvasDrawOp::TextLayout {
            text: "timer".to_owned(),
            paint: paint.clone(),
            layout: layout.clone(),
        }])
    );
    program.runtime.borrow_mut().active_canvas_key = Some(map_identity_key(&canvas));
    assert_eq!(
        program.invoke_builtin_method(
            &draw,
            Some(&test_invocation(vec![layout.clone(), text_canvas.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .canvas_draw_ops
            .get(&map_identity_key(&canvas))
            .cloned(),
        Some(vec![
            CanvasDrawOp::TextLayout {
                text: "timer".to_owned(),
                paint: paint.clone(),
                layout: layout.clone(),
            },
            CanvasDrawOp::TextLayout {
                text: "timer".to_owned(),
                paint,
                layout: layout.clone(),
            },
        ])
    );
    assert!(!program
        .runtime
        .borrow()
        .canvas_draw_ops
        .contains_key(&map_identity_key(&text_canvas)));
    assert_eq!(
        program.invoke_builtin_method(&fallback_line_spacing, Some(&test_invocation(vec![layout]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_layout_get_line_for_vertical_uses_line_tops() {
    let program = DexProgram::new(Vec::new());
    let get_line_for_vertical = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getLineForVertical".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "II".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 91,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout;"),
    });

    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_char_sequence_field("text"),
        DexValue::String(std::rc::Rc::from("one\ntwo\nthree")),
    ));
    assert!(program.write_stored_instance_field(
        &layout,
        &boring_layout_int_field("height"),
        DexValue::Int(60),
    ));

    for (vertical, expected_line) in [(-1, 0), (0, 0), (19, 0), (20, 1), (59, 2), (60, 2)] {
        assert_eq!(
            program.invoke_builtin_method(
                &get_line_for_vertical,
                Some(&test_invocation(vec![
                    layout.clone(),
                    DexValue::Int(vertical)
                ]))
            ),
            Some(DexExecutionResult::Int(expected_line))
        );
    }
}

#[test]
fn builtin_layout_get_offset_for_horizontal_returns_nearest_line_offset() {
    let program = DexProgram::new(Vec::new());
    let get_offset_for_horizontal = DexMethodId {
        class_descriptor: "Landroid/text/Layout;".to_owned(),
        name: "getOffsetForHorizontal".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "IIF".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["I".to_owned(), "F".to_owned()],
        },
    };
    let layout = DexValue::Object(DexObjectRef {
        id: 92,
        class_descriptor: std::rc::Rc::from("Landroid/text/StaticLayout;"),
    });
    let paint = DexValue::Object(DexObjectRef {
        id: 93,
        class_descriptor: std::rc::Rc::from("Landroid/text/TextPaint;"),
    });

    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_char_sequence_field("text"),
        DexValue::String(std::rc::Rc::from("abcd\nef")),
    ));
    assert!(program.write_stored_instance_field(
        &layout,
        &text_layout_paint_field(),
        paint.clone(),
    ));
    assert!(program.write_stored_instance_field(
        &paint,
        &paint_text_size_field(),
        DexValue::Float(20.0),
    ));

    for (line, horizontal, expected_offset) in [
        (0, 0.0, 0),
        (0, 6.0, 1),
        (0, 26.0, 3),
        (0, 100.0, 4),
        (1, 12.0, 6),
    ] {
        assert_eq!(
            program.invoke_builtin_method(
                &get_offset_for_horizontal,
                Some(&test_invocation(vec![
                    layout.clone(),
                    DexValue::Int(line),
                    DexValue::Float(horizontal),
                ]))
            ),
            Some(DexExecutionResult::Int(expected_offset))
        );
    }
}

#[test]
fn builtin_text_view_stores_and_reads_text() {
    let program = DexProgram::new(Vec::new());
    let set_text = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };
    let get_text = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "getText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/CharSequence;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let edit_text_get_text = DexMethodId {
        class_descriptor: "Landroid/widget/EditText;".to_owned(),
        name: "getText".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/text/Editable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let length = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "length".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_selection_start = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "getSelectionStart".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let get_selection_end = DexMethodId {
        class_descriptor: "Landroid/widget/EditText;".to_owned(),
        name: "getSelectionEnd".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_selection = DexMethodId {
        class_descriptor: "Landroid/widget/EditText;".to_owned(),
        name: "setSelection".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let text_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296745"));

    assert_eq!(
        program.invoke_builtin_method(&get_text, Some(&test_invocation(vec![text_view.clone()]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from(String::new())
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_text,
            Some(&test_invocation(vec![
                text_view.clone(),
                DexValue::String(std::rc::Rc::from("7"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &edit_text_get_text,
            Some(&test_invocation(vec![text_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_text, Some(&test_invocation(vec![text_view]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("7")
        )))
    );
    let text_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296745"));
    assert_eq!(
        program.invoke_builtin_method(&length, Some(&test_invocation(vec![text_view.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selection_start,
            Some(&test_invocation(vec![text_view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_selection,
            Some(&test_invocation(vec![text_view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_selection_end,
            Some(&test_invocation(vec![text_view.clone()]))
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.view_snapshot().view_texts,
        vec![DexViewText {
            view_id: 2131296745,
            text: "7".to_owned(),
        }]
    );
    program.set_focused_view(Some(text_view));
    assert_eq!(
        program.view_snapshot().view_text_cursors,
        vec![DexViewTextCursor {
            view_id: 2131296745,
            selection_start: 0,
            selection_end: 0,
            cursor_visible: true,
        }]
    );
}

#[test]
fn text_change_range_uses_java_char_indices() {
    assert_eq!(
        text_change_range("a\u{1f600}bc", "a\u{1f600}bXYZ"),
        TextChangeRange {
            start: 4,
            removed: 1,
            added: 3,
        }
    );
}

#[test]
fn activity_runtime_registers_layout_view_text() {
    let program = DexProgram::new(Vec::new());

    program
        .write_view_text(2131296745, "7")
        .expect("view text should store");

    assert_eq!(program.view_text(2131296745).unwrap(), Some("7".to_owned()));
    assert_eq!(
        program.view_snapshot().view_texts,
        vec![DexViewText {
            view_id: 2131296745,
            text: "7".to_owned(),
        }]
    );
}

#[test]
fn view_snapshot_prefers_typed_reference_for_duplicate_id_in_any_write_order() {
    let view_id = 2131296745;
    let generic = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/view/View;:id/{view_id}"
    )));
    let typed = DexValue::Reference(std::rc::Rc::from(format!(
        "Landroid/widget/TextView;:id/{view_id}"
    )));
    let field = text_view_text_field();

    for typed_first in [false, true] {
        let program = DexProgram::new(Vec::new());
        let writes = if typed_first {
            [(&typed, "typed"), (&generic, "generic")]
        } else {
            [(&generic, "generic"), (&typed, "typed")]
        };
        for (view, text) in writes {
            assert!(program.write_stored_instance_field(
                view,
                &field,
                DexValue::String(std::rc::Rc::from(text)),
            ));
        }

        assert_eq!(
            program.view_snapshot().view_texts,
            vec![DexViewText {
                view_id,
                text: "typed".to_owned(),
            }]
        );
    }
}

#[test]
fn builtin_text_view_stores_and_reads_cursor_visible() {
    let program = DexProgram::new(Vec::new());
    let set_cursor_visible = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "setCursorVisible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let is_cursor_visible = DexMethodId {
        class_descriptor: "Landroid/widget/TextView;".to_owned(),
        name: "isCursorVisible".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let text_view = DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296745"));

    assert_eq!(
        program.invoke_builtin_method(
            &is_cursor_visible,
            Some(&test_invocation(vec![text_view.clone()]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_cursor_visible,
            Some(&test_invocation(vec![text_view.clone(), DexValue::Int(0)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&is_cursor_visible, Some(&test_invocation(vec![text_view]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_image_view_set_visibility_returns_void() {
    let program = DexProgram::new(Vec::new());
    let set_visibility = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setVisibility".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_image_resource = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setImageResource".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let set_image_drawable = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setImageDrawable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/drawable/Drawable;".to_owned()],
        },
    };
    let get_drawable = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "getDrawable".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/drawable/Drawable;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_image_tint_list = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setImageTintList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/res/ColorStateList;".to_owned()],
        },
    };
    let get_image_tint_list = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "getImageTintList".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/content/res/ColorStateList;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_image_tint_mode = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "setImageTintMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/graphics/PorterDuff$Mode;".to_owned()],
        },
    };
    let get_image_tint_mode = DexMethodId {
        class_descriptor: "Landroid/widget/ImageView;".to_owned(),
        name: "getImageTintMode".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/PorterDuff$Mode;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let image_view = DexValue::Reference(std::rc::Rc::from("Landroid/widget/ImageView;:layout/1"));
    let image_button =
        DexValue::Reference(std::rc::Rc::from("Landroid/widget/ImageButton;:layout/2"));
    let id_backed_image =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/View;:id/2131296685"));
    let drawable = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/drawable/Drawable;:menu",
    ));
    let tint = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/res/ColorStateList;:image",
    ));
    let mode = DexValue::Reference(std::rc::Rc::from(
        "Landroid/graphics/PorterDuff$Mode;:SRC_IN",
    ));

    assert_eq!(
        program.invoke_builtin_method(
            &set_visibility,
            Some(&test_invocation(vec![image_view.clone(), DexValue::Int(4)]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_resource,
            Some(&test_invocation(vec![
                image_view.clone(),
                DexValue::Int(2131230999)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_drawable,
            Some(&test_invocation(vec![image_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/graphics/drawable/Drawable;:resource:2131230999")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_resource,
            Some(&test_invocation(vec![
                id_backed_image,
                DexValue::Int(2131230872)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.view_snapshot().view_images,
        vec![DexViewImage {
            view_id: 2131296685,
            drawable_resource_id: 2131230872,
        }]
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_drawable,
            Some(&test_invocation(vec![image_view.clone(), drawable.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_drawable,
            Some(&test_invocation(vec![image_view.clone()]))
        ),
        Some(DexExecutionResult::Value(drawable))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_drawable,
            Some(&test_invocation(vec![image_button.clone(), DexValue::Null]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_drawable, Some(&test_invocation(vec![image_button]))),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_image_tint_list,
            Some(&test_invocation(vec![image_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_tint_list,
            Some(&test_invocation(vec![image_view.clone(), tint.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_image_tint_list,
            Some(&test_invocation(vec![image_view.clone()]))
        ),
        Some(DexExecutionResult::Value(tint))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_image_tint_mode,
            Some(&test_invocation(vec![image_view.clone()]))
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &set_image_tint_mode,
            Some(&test_invocation(vec![image_view.clone(), mode.clone()]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_image_tint_mode,
            Some(&test_invocation(vec![image_view]))
        ),
        Some(DexExecutionResult::Value(mode))
    );
}

#[test]
fn builtin_bitmapdrawable_getbitmap_returns_decoded_bitmap_field() {
    let program = DexProgram::new(Vec::new());
    let get_bitmap = DexMethodId {
        class_descriptor: "Landroid/graphics/drawable/BitmapDrawable;".to_owned(),
        name: "getBitmap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/graphics/Bitmap;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let drawable = DexValue::Object(DexObjectRef {
        id: 980,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/drawable/BitmapDrawable;"),
    });
    let bitmap = DexValue::Object(DexObjectRef {
        id: 981,
        class_descriptor: std::rc::Rc::from("Landroid/graphics/Bitmap;"),
    });
    assert!(program.write_stored_instance_field(
        &drawable,
        &bitmap_drawable_bitmap_field(),
        bitmap.clone(),
    ));

    // cite: frameworks-base BitmapDrawable.java#getBitmap
    assert_eq!(
        program.invoke_builtin_method(&get_bitmap, Some(&test_invocation(vec![drawable]))),
        Some(DexExecutionResult::Value(bitmap))
    );
}

use super::helpers::*;
use super::*;

impl DexProgram {
    pub(in crate::dex_run) fn read_text_view_text(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        Some(
            self.read_stored_instance_field(&invocation.arguments[0], &text_view_text_field())
                .unwrap_or(DexValue::String(std::rc::Rc::from(String::new()))),
        )
    }

    pub(in crate::dex_run) fn write_text_view_text(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = text_view_text_value(&invocation.arguments[1])?;
        let selection = match &value {
            DexValue::String(value) => i32::try_from(value.encode_utf16().count()).ok(),
            DexValue::Null
            | DexValue::Object(_)
            | DexValue::Reference(_)
            | DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_) => None,
        };
        let receiver = &invocation.arguments[0];

        if !self.write_stored_instance_field(receiver, &text_view_text_field(), value) {
            return None;
        }

        if let Some(selection) = selection {
            self.write_text_view_selection(receiver, selection, selection)?;
        }

        Some(())
    }

    pub(in crate::dex_run) fn set_text_view_text_and_dispatch(
        &self,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<()> {
        if invocation.arguments.len() != 2 {
            return Err(WinedroidError::new(format!(
                "TextView.setText expected 2 arguments, got {}",
                invocation.arguments.len()
            )));
        }

        let receiver = invocation.arguments[0].clone();
        match &receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => {
                return Err(WinedroidError::new(format!(
                    "TextView.setText requires an object-backed TextView receiver\nreceiver: {receiver}"
                )));
            }
        }

        let old_text = self.text_view_text_string(&receiver)?;
        let new_text = text_view_text_argument_string(&invocation.arguments[1])?;
        let change = text_change_range(&old_text, &new_text);
        let watchers = self.text_view_text_watchers(&receiver);

        for watcher in &watchers {
            self.dispatch_text_watcher_before_text_changed(watcher, &old_text, change, depth)?;
        }

        if !self.write_stored_instance_field(
            &receiver,
            &text_view_text_field(),
            DexValue::String(std::rc::Rc::from(new_text.clone())),
        ) {
            return Err(WinedroidError::new(format!(
                "TextView.setText failed to store text\nreceiver: {receiver}"
            )));
        }

        self.write_text_view_selection(
            &receiver,
            change.start + change.added,
            change.start + change.added,
        )
        .ok_or_else(|| {
            WinedroidError::new(format!(
                "TextView.setText failed to update selection\nreceiver: {receiver}"
            ))
        })?;

        for watcher in &watchers {
            self.dispatch_text_watcher_on_text_changed(watcher, &new_text, change, depth)?;
        }

        for watcher in &watchers {
            self.dispatch_text_watcher_after_text_changed(watcher, &new_text, depth)?;
        }

        Ok(())
    }

    pub(in crate::dex_run) fn set_text_view_text_resource_and_dispatch(
        &self,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<()> {
        let (receiver, text) = self.text_view_resource_text(invocation)?;
        let text_invocation = DexInvocation {
            kind: invocation.kind,
            range: invocation.range,
            method_index: invocation.method_index,
            argument_registers: invocation.argument_registers.clone(),
            arguments: smallvec::smallvec![receiver, DexValue::String(std::rc::Rc::from(text))],
        };

        self.set_text_view_text_and_dispatch(&text_invocation, depth)
    }

    pub(in crate::dex_run::view) fn text_view_resource_text(
        &self,
        invocation: &DexInvocation,
    ) -> Result<(DexValue, String)> {
        if invocation.arguments.len() != 2 {
            return Err(WinedroidError::new(format!(
                "TextView.setText resource overload expected 2 arguments, got {}",
                invocation.arguments.len()
            )));
        }

        let receiver = invocation.arguments[0].clone();
        let DexValue::Int(resource_id) = invocation.arguments[1] else {
            return Err(WinedroidError::new(format!(
                "TextView.setText resource overload requires an int resource ID\nvalue: {}",
                invocation.arguments[1]
            )));
        };
        let resource_id = u32::try_from(resource_id).map_err(|error| {
            WinedroidError::with_source(
                format!("TextView.setText resource ID is negative\nresource id: {resource_id}"),
                error,
            )
        })?;
        let Some(text) = self.resolve_string_resource_text(resource_id)? else {
            return Err(WinedroidError::new(format!(
                "TextView.setText could not resolve string resource 0x{resource_id:08x}\nnext missing primitive: Android string resource resolution"
            )));
        };

        Ok((receiver, text))
    }

    pub(in crate::dex_run) fn text_view_text_string(&self, receiver: &DexValue) -> Result<String> {
        let value = self
            .read_stored_instance_field(receiver, &text_view_text_field())
            .unwrap_or(DexValue::String(std::rc::Rc::from(String::new())));

        match value {
            DexValue::String(value) => Ok(value.to_string()),
            other => Err(WinedroidError::new(format!(
                "TextView text is not represented as a string\nreceiver: {receiver}\nvalue: {other}\nnext missing primitive: CharSequence text storage"
            ))),
        }
    }

    pub(in crate::dex_run) fn write_text_view_text_watcher(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        match &invocation.arguments[1] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .view_text_watchers
            .entry(view_key)
            .or_default()
            .push(invocation.arguments[1].clone());

        Some(())
    }

    pub(in crate::dex_run) fn text_view_text_watchers(&self, receiver: &DexValue) -> Vec<DexValue> {
        let view_key = map_identity_key(receiver);

        self.runtime
            .borrow()
            .view_text_watchers
            .get(&view_key)
            .cloned()
            .unwrap_or_default()
    }

    pub(in crate::dex_run) fn dispatch_text_watcher_before_text_changed(
        &self,
        watcher: &DexValue,
        old_text: &str,
        change: TextChangeRange,
        depth: usize,
    ) -> Result<()> {
        self.dispatch_text_watcher_void(
            watcher,
            "beforeTextChanged",
            crate::dex::DexProto {
                shorty: "VLIII".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![
                    "Ljava/lang/CharSequence;".to_owned(),
                    "I".to_owned(),
                    "I".to_owned(),
                    "I".to_owned(),
                ],
            },
            vec![
                DexValue::String(std::rc::Rc::from(old_text.to_owned())),
                DexValue::Int(change.start),
                DexValue::Int(change.removed),
                DexValue::Int(change.added),
            ],
            depth,
        )
    }

    pub(in crate::dex_run) fn dispatch_text_watcher_on_text_changed(
        &self,
        watcher: &DexValue,
        new_text: &str,
        change: TextChangeRange,
        depth: usize,
    ) -> Result<()> {
        self.dispatch_text_watcher_void(
            watcher,
            "onTextChanged",
            crate::dex::DexProto {
                shorty: "VLIII".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![
                    "Ljava/lang/CharSequence;".to_owned(),
                    "I".to_owned(),
                    "I".to_owned(),
                    "I".to_owned(),
                ],
            },
            vec![
                DexValue::String(std::rc::Rc::from(new_text.to_owned())),
                DexValue::Int(change.start),
                DexValue::Int(change.removed),
                DexValue::Int(change.added),
            ],
            depth,
        )
    }

    pub(in crate::dex_run) fn dispatch_text_watcher_after_text_changed(
        &self,
        watcher: &DexValue,
        new_text: &str,
        depth: usize,
    ) -> Result<()> {
        self.dispatch_text_watcher_void(
            watcher,
            "afterTextChanged",
            crate::dex::DexProto {
                shorty: "VL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec!["Landroid/text/Editable;".to_owned()],
            },
            vec![DexValue::String(std::rc::Rc::from(new_text.to_owned()))],
            depth,
        )
    }

    pub(in crate::dex_run) fn dispatch_text_watcher_void(
        &self,
        watcher: &DexValue,
        method_name: &str,
        proto: crate::dex::DexProto,
        mut arguments: Vec<DexValue>,
        depth: usize,
    ) -> Result<()> {
        let Some(watcher_descriptor) = value_class_descriptor(watcher) else {
            return Err(WinedroidError::new(format!(
                "TextWatcher callback requires an object-backed watcher\nwatcher: {watcher}"
            )));
        };

        let method = DexMethodId {
            class_descriptor: watcher_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto,
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(watcher_descriptor, &method)?
        else {
            return Err(WinedroidError::new(format!(
                "TextWatcher callback method was not found\nwatcher class: {watcher_descriptor}\nmethod: {method_name}{}\nnext missing primitive: Android TextWatcher dispatch",
                method.proto.descriptor()
            )));
        };

        let mut initial_arguments = Vec::with_capacity(arguments.len() + 1);
        initial_arguments.push(watcher.clone());
        initial_arguments.append(&mut arguments);

        let result = self.execute_method(None, &resolved, Some(&initial_arguments), depth + 1)?;
        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(format!(
                "TextWatcher callback `{method_name}` did not complete normally"
            ))),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "TextWatcher callback `{method_name}` did not complete normally"
                )))
            }
        }
    }

    pub(in crate::dex_run) fn read_text_view_selection_start(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.read_text_view_selection(&invocation.arguments[0], &text_view_selection_start_field())
    }

    pub(in crate::dex_run) fn read_text_view_selection_end(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.read_text_view_selection(&invocation.arguments[0], &text_view_selection_end_field())
    }

    pub(in crate::dex_run) fn write_text_view_selection_from_invocation(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let (receiver, start, end) = match invocation.arguments.as_slice() {
            [receiver, DexValue::Int(selection)] => (receiver, *selection, *selection),
            [receiver, DexValue::Int(start), DexValue::Int(end)] => (receiver, *start, *end),
            _ => return None,
        };

        self.write_text_view_selection(receiver, start, end)
    }

    pub(in crate::dex_run) fn read_text_view_selection(
        &self,
        receiver: &DexValue,
        field: &DexFieldId,
    ) -> Option<i32> {
        match receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        if let Some(DexValue::Int(value)) = self.read_stored_instance_field(receiver, field) {
            return Some(value);
        }

        self.text_view_text_length(receiver)
    }

    pub(in crate::dex_run) fn write_text_view_selection(
        &self,
        receiver: &DexValue,
        start: i32,
        end: i32,
    ) -> Option<()> {
        if start < 0 || end < 0 {
            return None;
        }

        match receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let text_length = self.text_view_text_length(receiver)?;
        if start > text_length || end > text_length {
            return None;
        }

        self.write_stored_instance_field(
            receiver,
            &text_view_selection_start_field(),
            DexValue::Int(start),
        );
        self.write_stored_instance_field(
            receiver,
            &text_view_selection_end_field(),
            DexValue::Int(end),
        );

        Some(())
    }

    pub(in crate::dex_run) fn text_view_text_length(&self, receiver: &DexValue) -> Option<i32> {
        let value = self
            .read_stored_instance_field(receiver, &text_view_text_field())
            .unwrap_or(DexValue::String(std::rc::Rc::from(String::new())));

        match value {
            DexValue::String(value) => i32::try_from(value.encode_utf16().count()).ok(),
            DexValue::Null
            | DexValue::Object(_)
            | DexValue::Reference(_)
            | DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_) => None,
        }
    }

    pub(in crate::dex_run) fn android_graphics_insets_of(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 4 {
            return None;
        }

        let values = match invocation.arguments.as_slice() {
            [DexValue::Int(left), DexValue::Int(top), DexValue::Int(right), DexValue::Int(bottom)] => {
                [*left, *top, *right, *bottom]
            }
            _ => return None,
        };
        let insets = self.allocate_object("Landroid/graphics/Insets;".to_owned());

        for (name, value) in ["left", "top", "right", "bottom"].into_iter().zip(values) {
            self.write_stored_instance_field(
                &insets,
                &insets_int_field(name),
                DexValue::Int(value),
            );
        }

        Some(insets)
    }

    pub(in crate::dex_run) fn bitmap_create_bitmap(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let [display_metrics, DexValue::Int(width), DexValue::Int(height), config, DexValue::Int(mutable), color_space] =
            invocation.arguments.as_slice()
        else {
            return None;
        };

        if *width <= 0 || *height <= 0 {
            return None;
        }

        if !dex_value_is_nullable_object_like(display_metrics)
            || !dex_value_is_object_like(config)
            || !dex_value_is_object_like(color_space)
        {
            return None;
        }

        let bitmap = self.allocate_object("Landroid/graphics/Bitmap;".to_owned());
        for (field, value) in [
            (bitmap_int_field("width"), DexValue::Int(*width)),
            (bitmap_int_field("height"), DexValue::Int(*height)),
            (
                bitmap_object_field("displayMetrics", "Landroid/util/DisplayMetrics;"),
                display_metrics.clone(),
            ),
            (
                bitmap_object_field("config", "Landroid/graphics/Bitmap$Config;"),
                config.clone(),
            ),
            (
                bitmap_bool_field("mutable"),
                DexValue::Int(i32::from(*mutable != 0)),
            ),
            (
                bitmap_object_field("colorSpace", "Landroid/graphics/ColorSpace;"),
                color_space.clone(),
            ),
        ] {
            if !self.write_stored_instance_field(&bitmap, &field, value) {
                return None;
            }
        }

        Some(bitmap)
    }

    pub(in crate::dex_run) fn bitmap_prepare_to_draw(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        dex_value_is_object_like(&invocation.arguments[0]).then_some(())
    }

    pub(in crate::dex_run) fn bitmap_get_dimension(
        &self,
        invocation: &DexInvocation,
        method_name: &str,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let field_name = match method_name {
            "getWidth" => "width",
            "getHeight" => "height",
            _ => return None,
        };
        let value = self
            .read_stored_instance_field(&invocation.arguments[0], &bitmap_int_field(field_name))
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value.max(0))
    }

    pub(in crate::dex_run) fn bitmap_get_config(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(
            self.read_stored_instance_field(
                &invocation.arguments[0],
                &bitmap_object_field("config", "Landroid/graphics/Bitmap$Config;"),
            )
            .unwrap_or_else(|| {
                DexValue::Reference(std::rc::Rc::from(
                    "Landroid/graphics/Bitmap$Config;.ARGB_8888",
                ))
            }),
        )
    }

    pub(in crate::dex_run) fn window_insets_builder_set_insets(
        &self,
        invocation: &DexInvocation,
        field_name: &str,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match (&invocation.arguments[0], &invocation.arguments[1]) {
            (
                DexValue::Object(_) | DexValue::Reference(_),
                DexValue::Object(_) | DexValue::Reference(_),
            ) => {}
            _ => return None,
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &window_insets_builder_field(field_name),
            invocation.arguments[1].clone(),
        );

        Some(invocation.arguments[0].clone())
    }

    pub(in crate::dex_run) fn read_text_view_cursor_visible(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self
            .read_stored_instance_field(&invocation.arguments[0], &text_view_cursor_visible_field())
            .unwrap_or(DexValue::Int(1));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_text_view_cursor_visible(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        if value != 0 && value != 1 {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_view_cursor_visible_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_text_view_text_size(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let DexValue::Int(_unit) = invocation.arguments[1] else {
            return None;
        };
        let size = match invocation.arguments[2] {
            DexValue::Float(value) => value,
            DexValue::Int(value) => f32::from_bits(value as u32),
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Long(_)
            | DexValue::String(_)
            | DexValue::Object(_)
            | DexValue::Reference(_) => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_view_text_size_field(),
            DexValue::Float(size),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_text_view_text_color(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &text_view_text_color_field(),
            DexValue::Int(value),
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn write_text_view_text_appearance(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let (receiver, resource_id) = match invocation.arguments.as_slice() {
            [receiver, DexValue::Int(resource_id)] => (receiver, *resource_id),
            [receiver, DexValue::Object(_) | DexValue::Reference(_), DexValue::Int(resource_id)] => {
                (receiver, *resource_id)
            }
            _ => return None,
        };

        match receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        self.write_stored_instance_field(
            receiver,
            &text_view_text_appearance_field(),
            DexValue::Int(resource_id),
        )
        .then_some(())
    }
}

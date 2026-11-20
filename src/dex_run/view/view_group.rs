use super::helpers::*;
use super::*;

impl DexProgram {
    pub(in crate::dex_run) fn view_manager_add_view(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        self.view_group_add_view_with_layout_params(invocation)?;

        let view = invocation.arguments[1].clone();
        if layout_id_from_view_value(&view).is_some() {
            return Some(());
        }

        let view_key = map_identity_key(&view);
        let Some(class_descriptor) = value_class_descriptor(&view) else {
            return Some(());
        };

        trace::event("ui", format!("view manager addView view={view_key}"));
        let mut runtime = self.runtime.borrow_mut();
        if !runtime
            .runtime_view_roots
            .iter()
            .any(|root| root.view_key == view_key)
        {
            runtime.runtime_view_roots.push(DexRuntimeViewRoot {
                view_key: view_key.clone(),
                class_descriptor: class_descriptor.to_owned(),
            });
        }
        runtime.view_render_node_dirty.insert(view_key);
        runtime.request_layout_traversal();

        Some(())
    }

    // cite: frameworks-base WindowManagerGlobal.java#removeView
    pub(in crate::dex_run) fn view_manager_remove_view(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let manager_key = map_identity_key(&invocation.arguments[0]);
        let view = &invocation.arguments[1];
        let view_key = map_identity_key(view);
        trace::event("ui", format!("view manager removeView view={view_key}"));
        let mut runtime = self.runtime.borrow_mut();
        runtime
            .runtime_view_roots
            .retain(|root| root.view_key != view_key);
        runtime
            .runtime_canvas_frames
            .retain(|frame| frame.root_view_key != view_key);
        if let Some(children) = runtime.view_group_children.get_mut(&manager_key) {
            children.retain(|child| map_identity_key(child) != view_key);
        }
        runtime.view_parents.remove(&view_key);
        runtime.request_layout_traversal();
        drop(runtime);
        self.discard_view_render_node_subtree(view);

        Some(())
    }

    pub(in crate::dex_run) fn view_manager_update_view_layout(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
            || !dex_value_is_object_like(&invocation.arguments[2])
        {
            return None;
        }

        if !self.write_stored_instance_field(
            &invocation.arguments[1],
            &view_layout_params_field(),
            invocation.arguments[2].clone(),
        ) {
            return None;
        }

        self.mark_view_render_node_dirty(&invocation.arguments[1]);
        self.runtime.borrow_mut().request_layout_traversal();
        Some(())
    }

    pub(in crate::dex_run) fn view_group_add_view_with_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        if dex_value_is_object_like(&invocation.arguments[2]) {
            self.write_stored_instance_field(
                &invocation.arguments[1],
                &view_layout_params_field(),
                invocation.arguments[2].clone(),
            );
        }

        self.view_group_add_view(&DexInvocation {
            kind: invocation.kind,
            range: invocation.range,
            method_index: invocation.method_index,
            argument_registers: invocation.argument_registers.clone(),
            arguments: smallvec::smallvec![
                invocation.arguments[0].clone(),
                invocation.arguments[1].clone(),
            ],
        })
    }

    pub(in crate::dex_run) fn view_group_add_view_at_index(
        &self,
        invocation: &DexInvocation,
        has_layout_params: bool,
    ) -> Option<()> {
        let expected_len = if has_layout_params { 4 } else { 3 };
        if invocation.arguments.len() != expected_len {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let DexValue::Int(index) = invocation.arguments[2] else {
            return None;
        };
        if has_layout_params && dex_value_is_object_like(&invocation.arguments[3]) {
            self.write_stored_instance_field(
                &invocation.arguments[1],
                &view_layout_params_field(),
                invocation.arguments[3].clone(),
            );
        }

        let group_key = map_identity_key(&invocation.arguments[0]);
        let child_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(previous_parent) = runtime.view_parents.get(&child_key).cloned() {
            let previous_parent_key = map_identity_key(&previous_parent);
            if let Some(previous_siblings) =
                runtime.view_group_children.get_mut(&previous_parent_key)
            {
                previous_siblings.retain(|child| map_identity_key(child) != child_key);
            }
        }
        runtime
            .view_parents
            .insert(child_key, invocation.arguments[0].clone());
        let siblings = runtime.view_group_children.entry(group_key).or_default();
        let insert_index = if index < 0 {
            siblings.len()
        } else {
            usize::try_from(index).ok()?.min(siblings.len())
        };
        siblings.insert(insert_index, invocation.arguments[1].clone());
        drop(runtime);
        self.mark_view_render_node_dirty(&invocation.arguments[0]);
        self.mark_view_render_node_dirty(&invocation.arguments[1]);

        self.notify_view_hierarchy_change(
            &invocation.arguments[0],
            &invocation.arguments[1],
            "onChildViewAdded",
        )?;

        Some(())
    }

    pub(in crate::dex_run) fn view_group_add_view_in_layout(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        let DexValue::Int(index) = invocation.arguments[2] else {
            return None;
        };
        let add_invocation = DexInvocation {
            kind: invocation.kind,
            range: invocation.range,
            method_index: invocation.method_index,
            argument_registers: invocation.argument_registers.clone(),
            arguments: smallvec::smallvec![
                invocation.arguments[0].clone(),
                invocation.arguments[1].clone(),
                DexValue::Int(index),
                invocation.arguments[3].clone(),
            ],
        };
        self.view_group_add_view_at_index(&add_invocation, true)?;

        Some(1)
    }

    // cite: frameworks-base ViewGroup.java#setOnHierarchyChangeListener
    pub(in crate::dex_run) fn write_view_hierarchy_change_listener(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_hierarchy_change_listener_field(),
            invocation.arguments[1].clone(),
        );

        Some(())
    }

    pub(in crate::dex_run) fn notify_view_hierarchy_change(
        &self,
        parent: &DexValue,
        child: &DexValue,
        method_name: &str,
    ) -> Option<()> {
        match self.dispatch_view_hierarchy_change(parent, child, method_name) {
            Ok(()) => Some(()),
            Err(error) => {
                eprintln!("ViewGroup.OnHierarchyChangeListener dispatch failed\n  error: {error}");
                None
            }
        }
    }

    // cite: frameworks-base ViewGroup.java#dispatchViewAdded, dispatchViewRemoved
    fn dispatch_view_hierarchy_change(
        &self,
        parent: &DexValue,
        child: &DexValue,
        method_name: &str,
    ) -> Result<()> {
        let Some(listener) =
            self.read_stored_instance_field(parent, &view_hierarchy_change_listener_field())
        else {
            return Ok(());
        };
        if !dex_value_is_object_like(&listener) {
            return Ok(());
        }

        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Err(WinedroidError::new(format!(
                "ViewGroup hierarchy change dispatch requires an object-backed listener\nlistener: {listener}"
            )));
        };

        let listener_method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "VLL".to_owned(),
                return_type: "V".to_owned(),
                parameters: vec![
                    "Landroid/view/View;".to_owned(),
                    "Landroid/view/View;".to_owned(),
                ],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(listener_descriptor, &listener_method)?
        else {
            return Err(WinedroidError::new(format!(
                "ViewGroup hierarchy change listener method was not found\nlistener class: {listener_descriptor}\nmethod: {method_name}\nnext missing primitive: ViewGroup.OnHierarchyChangeListener dispatch"
            )));
        };

        let result = self.execute_method(
            None,
            &resolved,
            Some(&[listener, parent.clone(), child.clone()]),
            0,
        )?;

        match result {
            DexExecutionResult::Void => Ok(()),
            DexExecutionResult::Int(_)
            | DexExecutionResult::Value(_)
            | DexExecutionResult::Throw(_) => Err(WinedroidError::new(format!(
                "ViewGroup hierarchy change listener {method_name} did not complete normally"
            ))),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "ViewGroup hierarchy change listener {method_name} did not complete normally"
                )))
            }
        }
    }

    pub(in crate::dex_run) fn clear_view_group_children(&self, parent: &DexValue) {
        let parent_key = map_identity_key(parent);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(children) = runtime.view_group_children.remove(&parent_key) {
            for child in children {
                runtime.view_parents.remove(&map_identity_key(&child));
            }
        }
    }

    pub(in crate::dex_run) fn append_view_child(&self, parent: &DexValue, child: &DexValue) {
        let parent_key = map_identity_key(parent);
        let child_key = map_identity_key(child);
        let mut runtime = self.runtime.borrow_mut();
        if let Some(previous_parent) = runtime.view_parents.get(&child_key).cloned() {
            let previous_parent_key = map_identity_key(&previous_parent);
            if let Some(previous_siblings) =
                runtime.view_group_children.get_mut(&previous_parent_key)
            {
                previous_siblings.retain(|sibling| map_identity_key(sibling) != child_key);
            }
        }
        runtime.view_parents.insert(child_key, parent.clone());
        runtime
            .view_group_children
            .entry(parent_key)
            .or_default()
            .push(child.clone());
    }

    pub(in crate::dex_run) fn view_group_child_count(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let group_key = map_identity_key(&invocation.arguments[0]);
        let count = self
            .runtime
            .borrow()
            .view_group_children
            .get(&group_key)
            .map(|children| children.len())
            .unwrap_or(0);

        i32::try_from(count).ok()
    }

    pub(in crate::dex_run) fn view_group_child_at(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(index) = invocation.arguments[1] else {
            return None;
        };
        if index < 0 {
            return Some(DexValue::Null);
        }

        let group_key = map_identity_key(&invocation.arguments[0]);
        Some(
            self.runtime
                .borrow()
                .view_group_children
                .get(&group_key)
                .and_then(|children| children.get(usize::try_from(index).ok()?))
                .cloned()
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn generate_default_layout_params(&self) -> DexValue {
        let layout_params =
            self.allocate_object("Landroid/view/ViewGroup$LayoutParams;".to_owned());
        self.write_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("width"),
            DexValue::Int(-2),
        );
        self.write_stored_instance_field(
            &layout_params,
            &view_group_layout_params_int_field("height"),
            DexValue::Int(-2),
        );

        layout_params
    }

    pub(in crate::dex_run) fn view_group_check_layout_params(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        Some(i32::from(dex_value_is_object_like(
            &invocation.arguments[1],
        )))
    }

    pub(in crate::dex_run) fn write_view_group_layout_params_size(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(width) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(height) = invocation.arguments[2] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_group_layout_params_int_field("width"),
            DexValue::Int(width),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_group_layout_params_int_field("height"),
            DexValue::Int(height),
        );

        Some(())
    }

    pub(in crate::dex_run) fn write_linear_layout_params_size_and_weight(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        // cite: frameworks-base LinearLayout.java#LayoutParams
        if invocation.arguments.len() != 4 {
            return None;
        }

        let weight = read_float_argument(&invocation.arguments[3])?;

        let sized_invocation = DexInvocation {
            arguments: smallvec::smallvec![
                invocation.arguments[0].clone(),
                invocation.arguments[1].clone(),
                invocation.arguments[2].clone(),
            ],
            ..invocation.clone()
        };
        self.write_view_group_layout_params_size(&sized_invocation)?;
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &linear_layout_params_weight_field(),
            DexValue::Float(weight),
        );

        Some(())
    }

    pub(in crate::dex_run) fn write_view_group_margin_layout_params_margins(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let margin_names = ["leftMargin", "topMargin", "rightMargin", "bottomMargin"];
        for (index, margin_name) in margin_names.iter().enumerate() {
            let DexValue::Int(value) = invocation.arguments[index + 1] else {
                return None;
            };
            self.write_stored_instance_field(
                &invocation.arguments[0],
                &view_group_margin_layout_params_int_field(margin_name),
                DexValue::Int(value),
            );
        }

        Some(())
    }

    pub(in crate::dex_run) fn relative_layout_params_add_rule(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        // cite: frameworks-base RelativeLayout.java#addRule
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(verb) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(subject) = invocation.arguments[2] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &relative_layout_params_rule_field(verb),
            DexValue::Int(subject),
        );

        Some(())
    }

    pub(in crate::dex_run) fn write_view_padding(&self, invocation: &DexInvocation) -> Option<()> {
        // cite: frameworks-base View.java#setPadding
        if invocation.arguments.len() != 5 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let padding_names = ["paddingLeft", "paddingTop", "paddingRight", "paddingBottom"];
        for (index, padding_name) in padding_names.iter().enumerate() {
            let DexValue::Int(value) = invocation.arguments[index + 1] else {
                return None;
            };
            self.write_stored_instance_field(
                &invocation.arguments[0],
                &view_padding_field(padding_name),
                DexValue::Int(value),
            );
        }
        self.runtime.borrow_mut().request_layout_traversal();

        Some(())
    }

    pub(in crate::dex_run) fn write_view_int_field_and_request_layout(
        &self,
        invocation: &DexInvocation,
        field: &DexFieldId,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        let changed = self.read_stored_instance_field(&invocation.arguments[0], field)
            != Some(DexValue::Int(value));
        self.write_stored_instance_field(&invocation.arguments[0], field, DexValue::Int(value));
        if changed {
            self.runtime.borrow_mut().request_layout_traversal();
        }

        Some(())
    }

    pub(in crate::dex_run) fn write_view_measure(&self, invocation: &DexInvocation) -> Option<()> {
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
        let DexValue::Int(width_spec) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(height_spec) = invocation.arguments[2] else {
            return None;
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_measured_width_field(),
            DexValue::Int(measure_spec_size(width_spec)),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_measured_height_field(),
            DexValue::Int(measure_spec_size(height_spec)),
        );

        Some(())
    }

    pub(in crate::dex_run) fn write_view_layout(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 5 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        for (index, name) in ["left", "top", "right", "bottom"].iter().enumerate() {
            let DexValue::Int(value) = invocation.arguments[index + 1] else {
                return None;
            };
            self.write_stored_instance_field(
                &invocation.arguments[0],
                &view_bounds_int_field(name),
                DexValue::Int(value),
            );
        }
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_layout_requested_field(),
            DexValue::Int(0),
        );

        Some(())
    }

    pub(in crate::dex_run) fn read_view_measured_dimension(
        &self,
        invocation: &DexInvocation,
        method_name: &str,
    ) -> Option<i32> {
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
        let field = match method_name {
            "getMeasuredWidth" => view_measured_width_field(),
            "getMeasuredHeight" => view_measured_height_field(),
            _ => return None,
        };
        let value = self
            .read_stored_instance_field(&invocation.arguments[0], &field)
            .unwrap_or(DexValue::Int(0));
        let DexValue::Int(value) = value else {
            return None;
        };

        Some(value)
    }

    pub(in crate::dex_run) fn write_view_set_measured_dimension(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(width) = invocation.arguments[1] else {
            return None;
        };
        let DexValue::Int(height) = invocation.arguments[2] else {
            return None;
        };

        self.write_view_measured_dimensions(&invocation.arguments[0], width, height)
            .then_some(())
    }

    pub(in crate::dex_run) fn write_view_on_layout(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 6 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(left) = invocation.arguments[2] else {
            return None;
        };
        let DexValue::Int(top) = invocation.arguments[3] else {
            return None;
        };
        let DexValue::Int(right) = invocation.arguments[4] else {
            return None;
        };
        let DexValue::Int(bottom) = invocation.arguments[5] else {
            return None;
        };

        self.write_view_bounds(&invocation.arguments[0], left, top, right, bottom)
            .then_some(())
    }

    pub(in crate::dex_run::view) fn write_view_measured_dimensions(
        &self,
        view: &DexValue,
        width: i32,
        height: i32,
    ) -> bool {
        let wrote_width = self.write_stored_instance_field(
            view,
            &view_measured_width_field(),
            DexValue::Int(width.max(0)),
        );
        let wrote_height = self.write_stored_instance_field(
            view,
            &view_measured_height_field(),
            DexValue::Int(height.max(0)),
        );

        wrote_width && wrote_height
    }

    pub(in crate::dex_run::view) fn write_view_bounds(
        &self,
        view: &DexValue,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> bool {
        let mut wrote_all = true;
        for (name, value) in [
            ("left", left),
            ("top", top),
            ("right", right),
            ("bottom", bottom),
        ] {
            wrote_all &= self.write_stored_instance_field(
                view,
                &view_bounds_int_field(name),
                DexValue::Int(value),
            );
        }
        wrote_all &= self.write_stored_instance_field(
            view,
            &view_layout_requested_field(),
            DexValue::Int(0),
        );
        if wrote_all {
            self.mark_view_render_node_dirty(view);
        }

        wrote_all
    }

    pub(in crate::dex_run) fn write_view_bound(
        &self,
        invocation: &DexInvocation,
        method_name: &str,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };
        let field_name = match method_name {
            "setLeft" => "left",
            "setTop" => "top",
            "setRight" => "right",
            "setBottom" => "bottom",
            _ => return None,
        };

        let wrote = self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_bounds_int_field(field_name),
            DexValue::Int(value),
        );
        if wrote {
            self.mark_view_render_node_dirty(&invocation.arguments[0]);
            self.runtime.borrow_mut().request_draw_traversal();
        }
        wrote.then_some(())
    }

    pub(in crate::dex_run::view) fn view_group_child_snapshot(
        &self,
        view: &DexValue,
    ) -> Vec<DexValue> {
        let view_key = map_identity_key(view);

        self.runtime
            .borrow()
            .view_group_children
            .get(&view_key)
            .cloned()
            .unwrap_or_default()
    }

    pub(in crate::dex_run) fn read_view_dimension(
        &self,
        invocation: &DexInvocation,
        method_name: &str,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let left = self.read_view_bound(&invocation.arguments[0], "left");
        let top = self.read_view_bound(&invocation.arguments[0], "top");
        let right = self.read_view_bound(&invocation.arguments[0], "right");
        let bottom = self.read_view_bound(&invocation.arguments[0], "bottom");

        match method_name {
            "getWidth" => Some(right.saturating_sub(left).max(0)),
            "getHeight" => Some(bottom.saturating_sub(top).max(0)),
            "getLeft" => Some(left),
            "getTop" => Some(top),
            "getRight" => Some(right),
            "getBottom" => Some(bottom),
            _ => None,
        }
    }

    pub(in crate::dex_run) fn read_view_is_laid_out(
        &self,
        invocation: &DexInvocation,
    ) -> Option<i32> {
        if invocation.arguments.len() != 1 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let width = self.read_view_dimension(invocation, "getWidth")?;
        let height = self.read_view_dimension(invocation, "getHeight")?;

        Some(if width > 0 && height > 0 { 1 } else { 0 })
    }

    pub(in crate::dex_run) fn write_view_location_into_array(
        &self,
        invocation: &DexInvocation,
        on_screen: bool,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let array_key = map_identity_key(&invocation.arguments[1]);
        let (left, top) = if on_screen {
            self.view_location_on_screen(&invocation.arguments[0])?
        } else {
            self.view_location_in_window(&invocation.arguments[0])?
        };
        let mut runtime = self.runtime.borrow_mut();
        let values = runtime.array_entries.get_mut(&array_key)?;

        if values.len() < 2 {
            return None;
        }

        values[0] = DexValue::Int(left);
        values[1] = DexValue::Int(top);

        Some(())
    }

    pub(in crate::dex_run) fn view_location_on_screen(
        &self,
        view: &DexValue,
    ) -> Option<(i32, i32)> {
        // cite: frameworks-base View.java#getLocationOnScreen, getLocationInWindow, transformFromViewToWindowSpace
        self.view_location_in_window(view)
    }

    fn view_location_in_window(&self, view: &DexValue) -> Option<(i32, i32)> {
        if !dex_value_is_object_like(view) {
            return None;
        }

        // cite: frameworks-base View.java#getLocationOnScreen, getLocationInWindow, transformFromViewToWindowSpace
        if !self.view_value_is_attached_to_window(view) {
            return Some((0, 0));
        }

        let mut current = Some(view.clone());
        let mut visited = Vec::new();
        let mut x = 0_i32;
        let mut y = 0_i32;

        while let Some(value) = current {
            let key = map_identity_key(&value);
            if visited.contains(&key) {
                return None;
            }
            visited.push(key.clone());
            if !self.view_location_transform_is_identity(&key)? {
                return None;
            }
            let left = self.read_view_bound(&value, "left");
            let top = self.read_view_bound(&value, "top");
            x = x.wrapping_add(left);
            y = y.wrapping_add(top);
            let parent = self.runtime.borrow().view_parents.get(&key).cloned();
            if let Some(parent) = &parent {
                let parent_scroll_x = match self
                    .read_stored_instance_field(parent, &horizontal_scroll_view_scroll_x_field())
                {
                    Some(DexValue::Int(value)) => value,
                    Some(_) => return None,
                    None => 0,
                };
                let parent_scroll_y =
                    match self.read_stored_instance_field(parent, &view_scroll_y_field()) {
                        Some(DexValue::Int(value)) => value,
                        Some(_) => return None,
                        None => 0,
                    };
                x = x.wrapping_sub(parent_scroll_x);
                y = y.wrapping_sub(parent_scroll_y);
            }
            current = parent;
        }

        Some((x, y))
    }

    fn view_location_transform_is_identity(&self, view_key: &str) -> Option<bool> {
        let render_node = self
            .runtime
            .borrow()
            .view_render_nodes
            .get(view_key)
            .cloned();
        let Some(render_node) = render_node else {
            return Some(true);
        };

        for (field, identity) in [
            (render_node_scale_x_field(), 1.0),
            (render_node_scale_y_field(), 1.0),
            (render_node_float_field("translationX"), 0.0),
            (render_node_float_field("translationY"), 0.0),
            (render_node_float_field("rotationZ"), 0.0),
            (render_node_float_field("rotationX"), 0.0),
            (render_node_float_field("rotationY"), 0.0),
        ] {
            match self.read_stored_instance_field(&render_node, &field) {
                Some(DexValue::Float(value)) if value != identity => return Some(false),
                Some(DexValue::Float(_)) | None => {}
                Some(_) => return None,
            }
        }

        Some(true)
    }

    pub(in crate::dex_run) fn write_view_window_visible_display_frame(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2
            || !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[1])
        {
            return None;
        }

        let (visible_width, visible_height) = match self.runtime.borrow().window_visible_frame {
            Some((width, height)) => (width.max(0), height.max(0)),
            None => {
                let root = self.read_view_root_value(&invocation.arguments[0])?;
                let root_width = self
                    .read_view_bound(&root, "right")
                    .saturating_sub(self.read_view_bound(&root, "left"))
                    .max(0);
                let root_height = self
                    .read_view_bound(&root, "bottom")
                    .saturating_sub(self.read_view_bound(&root, "top"))
                    .max(0);
                (root_width, root_height)
            }
        };

        for (name, value) in [
            ("left", 0),
            ("top", 0),
            ("right", visible_width),
            ("bottom", visible_height),
        ] {
            if !self.write_stored_instance_field(
                &invocation.arguments[1],
                &rect_int_field(name),
                DexValue::Int(value),
            ) {
                return None;
            }
        }

        Some(())
    }

    pub(in crate::dex_run) fn read_view_bound(&self, view: &DexValue, name: &str) -> i32 {
        let Some(DexValue::Int(value)) =
            self.read_stored_instance_field(view, &view_bounds_int_field(name))
        else {
            return 0;
        };

        value
    }

    pub(in crate::dex_run) fn read_view_background(
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
            self.read_stored_instance_field(&invocation.arguments[0], &view_background_field())
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_background(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.write_stored_instance_field(&invocation.arguments[0], &view_background_field(), value)
            .then_some(())
    }

    // cite: frameworks-base View.java#setBackgroundColor
    pub(in crate::dex_run) fn write_view_background_color(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }
        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        let DexValue::Int(color) = invocation.arguments[1] else {
            return None;
        };

        let drawable = self.allocate_object("Landroid/graphics/drawable/ColorDrawable;".to_owned());
        self.write_stored_instance_field(
            &drawable,
            &color_drawable_color_field(),
            DexValue::Int(color),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_background_field(),
            drawable,
        );

        Some(())
    }

    pub(in crate::dex_run) fn read_view_background_tint_list(
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
                &view_background_tint_list_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_background_tint_list(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_background_tint_list_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_background_tint_mode(
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
                &view_background_tint_mode_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_background_tint_mode(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = match &invocation.arguments[1] {
            DexValue::Null | DexValue::Int(0) => DexValue::Null,
            DexValue::Object(_) | DexValue::Reference(_) => invocation.arguments[1].clone(),
            DexValue::Uninitialized
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_background_tint_mode_field(),
            value,
        )
        .then_some(())
    }

    pub(in crate::dex_run) fn read_view_outline_provider(
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
                &view_outline_provider_field(),
            )
            .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_view_outline_provider(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }

        let value = nullable_object_argument(&invocation.arguments[1])?;

        self.write_stored_instance_field(
            &invocation.arguments[0],
            &view_outline_provider_field(),
            value,
        )
        .then_some(())
    }
}

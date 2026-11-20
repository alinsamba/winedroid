use super::helpers::*;
use super::*;
use crate::input::InputKey;

const MOTION_KEY_ACTION_DOWN: i32 = 0;
const MOTION_KEY_ACTION_UP: i32 = 1;

const IME_MASK_ACTION: i32 = 0x0000_00ff;
const IME_ACTION_UNSPECIFIED: i32 = 0;
const IME_ACTION_DONE: i32 = 6;

const TYPE_CLASS_TEXT: i32 = 0x0000_0001;
const TYPE_TEXT_FLAG_MULTI_LINE: i32 = 0x0002_0000;

enum EnterBehavior {
    InsertNewline,
    Action(i32),
}

impl DexProgram {
    pub(in crate::dex_run) fn focused_view(&self) -> Option<DexValue> {
        self.runtime.borrow().focused_view.clone()
    }

    pub(in crate::dex_run) fn set_focused_view(&self, view: Option<DexValue>) -> bool {
        let previous = self.runtime.borrow().focused_view.clone();
        let unchanged = match (&previous, &view) {
            (Some(old), Some(new)) => map_identity_key(old) == map_identity_key(new),
            (None, None) => true,
            _ => false,
        };
        if unchanged {
            return false;
        }

        if let Some(previous) = previous {
            self.write_stored_instance_field(&previous, &view_focused_field(), DexValue::Int(0));
            self.write_stored_instance_field(
                &previous,
                &text_view_cursor_visible_field(),
                DexValue::Int(0),
            );
        }

        if let Some(view) = &view {
            self.write_stored_instance_field(view, &view_focused_field(), DexValue::Int(1));
            self.write_stored_instance_field(
                view,
                &text_view_cursor_visible_field(),
                DexValue::Int(1),
            );
            if self
                .read_stored_instance_field(view, &text_view_selection_start_field())
                .is_none()
            {
                if let Some(length) = self.text_view_text_length(view) {
                    self.write_text_view_selection(view, length, length);
                }
            }
        }

        self.runtime.borrow_mut().focused_view = view;
        true
    }

    pub(in crate::dex_run) fn auto_focus_default_view(&self) -> Result<()> {
        if self.runtime.borrow().focused_view.is_some() {
            return Ok(());
        }

        let content_layouts = {
            let runtime = self.runtime.borrow();
            let start = runtime
                .activity_stack
                .last()
                .map(|activity| activity.content_layout_start)
                .unwrap_or(0);
            runtime.content_layouts[start..].to_vec()
        };

        for layout_id in content_layouts {
            if let Some(view_id) = self.first_focusable_editable_in_layout(layout_id)? {
                let view = self.canonical_view_for_id(view_id);
                self.set_focused_view(Some(view));
                return Ok(());
            }
        }

        Ok(())
    }

    fn first_focusable_editable_in_layout(&self, layout_id: i32) -> Result<Option<i32>> {
        let Some(events) = self.layout_xml_events(layout_id)? else {
            return Ok(None);
        };

        for event in events {
            let XmlEvent::Start(element) = event else {
                continue;
            };
            if !element.name.ends_with("EditText") {
                continue;
            }
            if let Some(view_id) = xml_element_view_id(&element) {
                return Ok(Some(view_id));
            }
        }

        Ok(None)
    }

    pub(in crate::dex_run) fn focus_view_by_id(&self, view_id: i32) -> Result<i32> {
        if view_id <= 0 {
            return Ok(0);
        }

        let view = self
            .active_registered_view_by_id(view_id)
            .unwrap_or_else(|| view_reference(view_id));

        Ok(i32::from(self.set_focused_view(Some(view))))
    }

    pub(in crate::dex_run) fn handle_key_input(
        &self,
        characters: &str,
        key: InputKey,
        depth: usize,
    ) -> Result<i32> {
        if self.edit_active_dialog_input(characters, key) == 1 {
            return Ok(1);
        }

        let Some(view) = self.focused_view() else {
            return Ok(0);
        };

        // cite: frameworks-base View.java#onCreateInputConnection
        // cite: androidx AndroidComposeView.android.kt#onCreateInputConnection
        if let Some(connection) = self.editor_input_connection(&view, depth)? {
            return self.dispatch_input_connection_key(&connection, characters, key, depth);
        }

        match key {
            InputKey::Enter => match self.enter_key_behavior(&view) {
                EnterBehavior::InsertNewline => self.insert_newline_at_caret(&view, depth),
                EnterBehavior::Action(action_id) => {
                    self.dispatch_editor_action(&view, action_id, depth)
                }
            },
            InputKey::Left | InputKey::Right | InputKey::Home | InputKey::End => {
                self.move_text_caret(&view, key)
            }
            InputKey::DeleteBackward | InputKey::DeleteForward => {
                self.delete_text_at_caret(&view, key, depth)
            }
            InputKey::Text => self.insert_text_at_caret(&view, characters, depth),
        }
    }

    // cite: androidx RecordingInputConnection.android.kt#commitText, deleteSurroundingText, sendKeyEvent
    fn dispatch_input_connection_key(
        &self,
        connection: &DexValue,
        characters: &str,
        key: InputKey,
        depth: usize,
    ) -> Result<i32> {
        match key {
            InputKey::DeleteBackward => {
                self.input_connection_delete_surrounding_text(connection, 1, 0, depth)
            }
            InputKey::DeleteForward => {
                self.input_connection_delete_surrounding_text(connection, 0, 1, depth)
            }
            InputKey::Enter | InputKey::Left | InputKey::Right | InputKey::Home | InputKey::End => {
                self.input_connection_send_key_event(connection, key, depth)
            }
            InputKey::Text => {
                let text: String = characters.chars().filter(|c| !c.is_control()).collect();
                if text.is_empty() {
                    return Ok(0);
                }
                self.input_connection_commit_text(connection, &text, depth)
            }
        }
    }

    // cite: frameworks-base View.java#onCreateInputConnection, onCheckIsTextEditor
    fn editor_input_connection(&self, view: &DexValue, depth: usize) -> Result<Option<DexValue>> {
        if !matches!(
            self.call_view_bool_method(view, "onCheckIsTextEditor", depth)?,
            Some(value) if value != 0
        ) {
            return Ok(None);
        }

        // cite: frameworks-base InputMethodManager.java#startInputInner
        let editor_info = self.allocate_object("Landroid/view/inputmethod/EditorInfo;".to_owned());
        let Some(connection) = self.call_view_input_connection(view, &editor_info, depth)? else {
            return Ok(None);
        };
        if !dex_value_is_object_like(&connection) {
            return Ok(None);
        }

        Ok(Some(connection))
    }

    fn input_connection_commit_text(
        &self,
        connection: &DexValue,
        text: &str,
        depth: usize,
    ) -> Result<i32> {
        let handled = self.call_input_connection_bool(
            connection,
            "commitText",
            crate::dex::DexProto {
                shorty: "ZLI".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec!["Ljava/lang/CharSequence;".to_owned(), "I".to_owned()],
            },
            vec![
                DexValue::String(std::rc::Rc::from(text.to_owned())),
                DexValue::Int(1),
            ],
            depth,
        )?;

        Ok(i32::from(handled))
    }

    fn input_connection_delete_surrounding_text(
        &self,
        connection: &DexValue,
        before: i32,
        after: i32,
        depth: usize,
    ) -> Result<i32> {
        let handled = self.call_input_connection_bool(
            connection,
            "deleteSurroundingText",
            crate::dex::DexProto {
                shorty: "ZII".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec!["I".to_owned(), "I".to_owned()],
            },
            vec![DexValue::Int(before), DexValue::Int(after)],
            depth,
        )?;

        Ok(i32::from(handled))
    }

    fn input_connection_send_key_event(
        &self,
        connection: &DexValue,
        key: InputKey,
        depth: usize,
    ) -> Result<i32> {
        let android_key_code = android_key_code(key);
        let mut handled = false;
        for action in [MOTION_KEY_ACTION_DOWN, MOTION_KEY_ACTION_UP] {
            let event = self.allocate_key_event(action, android_key_code)?;
            handled |= self.call_input_connection_bool(
                connection,
                "sendKeyEvent",
                crate::dex::DexProto {
                    shorty: "ZL".to_owned(),
                    return_type: "Z".to_owned(),
                    parameters: vec!["Landroid/view/KeyEvent;".to_owned()],
                },
                vec![event],
                depth,
            )?;
        }

        Ok(i32::from(handled))
    }

    fn call_input_connection_bool(
        &self,
        connection: &DexValue,
        method_name: &str,
        proto: crate::dex::DexProto,
        mut arguments: Vec<DexValue>,
        depth: usize,
    ) -> Result<bool> {
        let Some(class_descriptor) = value_class_descriptor(connection) else {
            return Ok(false);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto,
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &method)?
        else {
            return Ok(false);
        };
        let mut call_arguments = Vec::with_capacity(arguments.len() + 1);
        call_arguments.push(connection.clone());
        call_arguments.append(&mut arguments);
        match self.execute_method(None, &resolved, Some(&call_arguments), depth + 1)? {
            DexExecutionResult::Int(value) => Ok(value != 0),
            DexExecutionResult::Void => Ok(true),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "InputConnection.{method_name} did not complete normally"
                )))
            }
            _ => Ok(false),
        }
    }

    fn allocate_key_event(&self, action: i32, key_code: i32) -> Result<DexValue> {
        let event = self.allocate_object("Landroid/view/KeyEvent;".to_owned());
        self.write_stored_instance_field(&event, &key_event_action_field(), DexValue::Int(action));
        self.write_stored_instance_field(&event, &key_event_code_field(), DexValue::Int(key_code));

        Ok(event)
    }

    // cite: frameworks-base InputConnectionWrapper.java#InputConnectionWrapper, setTarget
    pub(in crate::dex_run) fn input_connection_wrapper_init(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &input_connection_wrapper_target_field(),
            invocation.arguments[1].clone(),
        );
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &input_connection_wrapper_mutable_field(),
            invocation.arguments[2].clone(),
        );

        Some(())
    }

    pub(in crate::dex_run) fn input_connection_wrapper_set_target(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 || !dex_value_is_object_like(&invocation.arguments[0]) {
            return None;
        }
        self.write_stored_instance_field(
            &invocation.arguments[0],
            &input_connection_wrapper_target_field(),
            invocation.arguments[1].clone(),
        );

        Some(())
    }

    // cite: frameworks-base InputConnectionWrapper.java#mTarget
    pub(in crate::dex_run) fn input_connection_wrapper_delegate(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let Some(target) =
            self.read_stored_instance_field(receiver, &input_connection_wrapper_target_field())
        else {
            return Ok(None);
        };
        if !dex_value_is_object_like(&target) {
            return Ok(None);
        }
        let Some(target_class) = value_class_descriptor(&target) else {
            return Ok(None);
        };
        let delegated = DexMethodId {
            class_descriptor: target_class.to_owned(),
            name: method.name.clone(),
            proto: method.proto.clone(),
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(target_class, &delegated)?
        else {
            return Ok(None);
        };
        let mut arguments = Vec::with_capacity(invocation.arguments.len());
        arguments.push(target);
        arguments.extend_from_slice(&invocation.arguments[1..]);
        let result = self.execute_method(None, &resolved, Some(&arguments), depth + 1)?;

        Ok(Some(result))
    }

    fn call_view_bool_method(
        &self,
        view: &DexValue,
        method_name: &str,
        depth: usize,
    ) -> Result<Option<i32>> {
        let Some(class_descriptor) = value_class_descriptor(view) else {
            return Ok(None);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: method_name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "Z".to_owned(),
                return_type: "Z".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &method)?
        else {
            return Ok(None);
        };
        match self.execute_method(None, &resolved, Some(&[view.clone()]), depth + 1)? {
            DexExecutionResult::Int(value) => Ok(Some(value)),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(format!(
                    "View.{method_name} did not complete normally"
                )))
            }
            _ => Ok(None),
        }
    }

    fn call_view_input_connection(
        &self,
        view: &DexValue,
        editor_info: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        let Some(class_descriptor) = value_class_descriptor(view) else {
            return Ok(None);
        };
        let method = DexMethodId {
            class_descriptor: class_descriptor.to_owned(),
            name: "onCreateInputConnection".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "LL".to_owned(),
                return_type: "Landroid/view/inputmethod/InputConnection;".to_owned(),
                parameters: vec!["Landroid/view/inputmethod/EditorInfo;".to_owned()],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(class_descriptor, &method)?
        else {
            return Ok(None);
        };
        match self.execute_method(
            None,
            &resolved,
            Some(&[view.clone(), editor_info.clone()]),
            depth + 1,
        )? {
            DexExecutionResult::Value(value) => Ok(Some(value)),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "View.onCreateInputConnection did not complete normally",
                ))
            }
            _ => Ok(None),
        }
    }

    fn enter_key_behavior(&self, view: &DexValue) -> EnterBehavior {
        let element = self
            .view_id_from_value(view)
            .and_then(|view_id| self.focused_layout_element(view_id));

        let input_type = element
            .as_ref()
            .and_then(|element| element.attribute("inputType"))
            .and_then(|attribute| int_attribute_value(&attribute.value))
            .unwrap_or(0);
        if input_type & TYPE_MASK_CLASS == TYPE_CLASS_TEXT
            && input_type & TYPE_TEXT_FLAG_MULTI_LINE != 0
        {
            return EnterBehavior::InsertNewline;
        }

        let ime_options = element
            .as_ref()
            .and_then(|element| element.attribute("imeOptions"))
            .and_then(|attribute| int_attribute_value(&attribute.value))
            .unwrap_or(0);
        let action = ime_options & IME_MASK_ACTION;
        let action_id = if action == IME_ACTION_UNSPECIFIED {
            IME_ACTION_DONE
        } else {
            action
        };

        EnterBehavior::Action(action_id)
    }

    fn insert_newline_at_caret(&self, view: &DexValue, depth: usize) -> Result<i32> {
        let (units, start, end) = self.focused_text_units(view);
        let newline: Vec<u16> = "\n".encode_utf16().collect();
        let mut next = Vec::with_capacity(units.len() - (end - start) + newline.len());
        next.extend_from_slice(&units[..start]);
        next.extend_from_slice(&newline);
        next.extend_from_slice(&units[end..]);

        let change = TextChangeRange {
            start: start as i32,
            removed: (end - start) as i32,
            added: newline.len() as i32,
        };
        self.apply_text_edit(view, &next, start + newline.len(), change, depth)
    }

    fn focused_text_units(&self, view: &DexValue) -> (Vec<u16>, usize, usize) {
        let text = self.text_view_text_string(view).unwrap_or_default();
        let units: Vec<u16> = text.encode_utf16().collect();
        let length = units.len();

        let read = |field: DexFieldId| {
            self.read_stored_instance_field(view, &field)
                .and_then(|value| dex_int(&value))
                .map(|value| (value.max(0) as usize).min(length))
                .unwrap_or(length)
        };
        let start = read(text_view_selection_start_field());
        let end = read(text_view_selection_end_field());

        (units, start.min(end), start.max(end))
    }

    fn insert_text_at_caret(&self, view: &DexValue, characters: &str, depth: usize) -> Result<i32> {
        let constraints = self.input_constraints(view)?;

        let accepted: String = characters
            .chars()
            .filter(|character| !character.is_control() && constraints.permits(*character))
            .collect();
        let mut insertion: Vec<u16> = accepted.encode_utf16().collect();
        if insertion.is_empty() {
            return Ok(0);
        }

        let (units, start, end) = self.focused_text_units(view);

        if let Some(max_length) = constraints.max_length {
            let kept = units.len() - (end - start);
            let room = max_length.saturating_sub(kept);
            insertion.truncate(room);
            if insertion.is_empty() {
                return Ok(0);
            }
        }

        let mut next = Vec::with_capacity(units.len() - (end - start) + insertion.len());
        next.extend_from_slice(&units[..start]);
        next.extend_from_slice(&insertion);
        next.extend_from_slice(&units[end..]);

        let change = TextChangeRange {
            start: start as i32,
            removed: (end - start) as i32,
            added: insertion.len() as i32,
        };
        self.apply_text_edit(view, &next, start + insertion.len(), change, depth)
    }

    fn input_constraints(&self, view: &DexValue) -> Result<InputConstraints> {
        let Some(view_id) = self.view_id_from_value(view) else {
            return Ok(InputConstraints::unconstrained());
        };
        let Some(element) = self.focused_layout_element(view_id) else {
            return Ok(InputConstraints::unconstrained());
        };

        let allowed = self.allowed_input_characters(&element)?;
        let max_length = element
            .attribute("maxLength")
            .and_then(|attribute| int_attribute_value(&attribute.value))
            .and_then(|value| usize::try_from(value).ok());

        Ok(InputConstraints {
            allowed,
            max_length,
        })
    }

    fn allowed_input_characters(
        &self,
        element: &crate::binary_xml::XmlStartElement,
    ) -> Result<Option<std::collections::HashSet<char>>> {
        if let Some(attribute) = element.attribute("digits") {
            if let Some(text) = self.digits_attribute_text(&attribute.value)? {
                return Ok(Some(text.chars().collect()));
            }
        }

        let Some(input_type) = element
            .attribute("inputType")
            .and_then(|attribute| int_attribute_value(&attribute.value))
        else {
            return Ok(None);
        };
        Ok(allowed_characters_for_input_type(input_type))
    }

    fn digits_attribute_text(&self, value: &crate::binary_xml::XmlValue) -> Result<Option<String>> {
        match value {
            crate::binary_xml::XmlValue::String(text) => Ok(Some(text.clone())),
            crate::binary_xml::XmlValue::Reference(resource_id) => {
                self.resolve_string_resource_text(*resource_id)
            }
            _ => Ok(None),
        }
    }

    fn focused_layout_element(&self, view_id: i32) -> Option<crate::binary_xml::XmlStartElement> {
        let layouts = {
            let runtime = self.runtime.borrow();
            let activity = runtime.activity_stack.last();
            let content_start = activity.map(|a| a.content_layout_start).unwrap_or(0);
            let inflated_start = activity.map(|a| a.inflated_layout_start).unwrap_or(0);
            runtime.content_layouts[content_start..]
                .iter()
                .chain(runtime.inflated_layouts[inflated_start..].iter())
                .copied()
                .collect::<Vec<i32>>()
        };

        for layout_id in layouts {
            let Some(events) = self.layout_xml_events(layout_id).ok().flatten() else {
                continue;
            };
            for event in events {
                if let XmlEvent::Start(element) = event {
                    if xml_element_view_id(&element) == Some(view_id) {
                        return Some(element);
                    }
                }
            }
        }

        None
    }

    fn delete_text_at_caret(&self, view: &DexValue, key: InputKey, depth: usize) -> Result<i32> {
        let (units, start, end) = self.focused_text_units(view);
        let (remove_start, remove_end, caret) = if start != end {
            (start, end, start)
        } else if key == InputKey::DeleteBackward {
            if start == 0 {
                return Ok(1);
            }
            (start - 1, start, start - 1)
        } else {
            if end >= units.len() {
                return Ok(1);
            }
            (start, start + 1, start)
        };

        let mut next = Vec::with_capacity(units.len() - (remove_end - remove_start));
        next.extend_from_slice(&units[..remove_start]);
        next.extend_from_slice(&units[remove_end..]);

        let change = TextChangeRange {
            start: remove_start as i32,
            removed: (remove_end - remove_start) as i32,
            added: 0,
        };
        self.apply_text_edit(view, &next, caret, change, depth)
    }

    fn move_text_caret(&self, view: &DexValue, key: InputKey) -> Result<i32> {
        let (units, start, end) = self.focused_text_units(view);
        let length = units.len();
        let caret = match key {
            InputKey::Left if start != end => start,
            InputKey::Left => start.saturating_sub(1),
            InputKey::Right if start != end => end,
            InputKey::Right => (end + 1).min(length),
            InputKey::Home => 0,
            InputKey::End => length,
            _ => return Ok(0),
        };

        self.write_text_view_selection(view, caret as i32, caret as i32);
        Ok(1)
    }

    fn apply_text_edit(
        &self,
        view: &DexValue,
        units: &[u16],
        caret: usize,
        change: TextChangeRange,
        depth: usize,
    ) -> Result<i32> {
        let new_text = String::from_utf16_lossy(units);
        let old_text = self.text_view_text_string(view).unwrap_or_default();
        let watchers = self.text_view_text_watchers(view);

        for watcher in &watchers {
            self.dispatch_text_watcher_before_text_changed(watcher, &old_text, change, depth)?;
        }

        if !self.write_stored_instance_field(
            view,
            &text_view_text_field(),
            DexValue::String(std::rc::Rc::from(new_text.clone())),
        ) {
            return Err(WinedroidError::new(
                "focused text edit failed to store the edited text",
            ));
        }
        self.write_text_view_selection(view, caret as i32, caret as i32);
        self.write_stored_instance_field(view, &text_view_cursor_visible_field(), DexValue::Int(1));

        for watcher in &watchers {
            self.dispatch_text_watcher_on_text_changed(watcher, &new_text, change, depth)?;
        }
        for watcher in &watchers {
            self.dispatch_text_watcher_after_text_changed(watcher, &new_text, depth)?;
        }

        Ok(1)
    }

    fn dispatch_editor_action(&self, view: &DexValue, action_id: i32, depth: usize) -> Result<i32> {
        let listener = self
            .read_stored_instance_field(view, &text_view_editor_action_listener_field())
            .unwrap_or(DexValue::Null);
        let Some(listener_descriptor) = value_class_descriptor(&listener) else {
            return Ok(1);
        };

        let method = DexMethodId {
            class_descriptor: listener_descriptor.to_owned(),
            name: "onEditorAction".to_owned(),
            proto: crate::dex::DexProto {
                shorty: "ZLIL".to_owned(),
                return_type: "Z".to_owned(),
                parameters: vec![
                    "Landroid/widget/TextView;".to_owned(),
                    "I".to_owned(),
                    "Landroid/view/KeyEvent;".to_owned(),
                ],
            },
        };
        let Some(resolved) =
            self.resolve_exact_method_in_class_hierarchy(listener_descriptor, &method)?
        else {
            return Err(WinedroidError::new(format!(
                "OnEditorActionListener.onEditorAction was not found\nlistener class: {listener_descriptor}\nnext missing primitive: TextView.OnEditorActionListener dispatch"
            )));
        };

        let result = self.execute_method(
            None,
            &resolved,
            Some(&[
                listener,
                view.clone(),
                DexValue::Int(action_id),
                DexValue::Null,
            ]),
            depth + 1,
        )?;

        match result {
            DexExecutionResult::Void | DexExecutionResult::Int(_) => Ok(1),
            DexExecutionResult::Value(_) | DexExecutionResult::Throw(_) => {
                Err(WinedroidError::new(
                    "OnEditorActionListener.onEditorAction did not complete normally",
                ))
            }
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                Err(WinedroidError::new(
                    "OnEditorActionListener.onEditorAction did not complete normally",
                ))
            }
        }
    }
}

fn dex_int(value: &DexValue) -> Option<i32> {
    match value {
        DexValue::Int(value) => Some(*value),
        _ => None,
    }
}

fn input_connection_wrapper_target_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/inputmethod/InputConnectionWrapper;".to_owned(),
        type_descriptor: "Landroid/view/inputmethod/InputConnection;".to_owned(),
        name: "mTarget".to_owned(),
    }
}

fn input_connection_wrapper_mutable_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/inputmethod/InputConnectionWrapper;".to_owned(),
        type_descriptor: "Z".to_owned(),
        name: "mMutable".to_owned(),
    }
}

fn key_event_action_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/KeyEvent;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "action".to_owned(),
    }
}

fn key_event_code_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroid/view/KeyEvent;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "keyCode".to_owned(),
    }
}

// cite: frameworks-base KeyEvent.java#KEYCODE_DEL, KEYCODE_FORWARD_DEL, KEYCODE_ENTER, KEYCODE_DPAD_LEFT, KEYCODE_DPAD_RIGHT, KEYCODE_MOVE_HOME, KEYCODE_MOVE_END
fn android_key_code(key: InputKey) -> i32 {
    match key {
        InputKey::DeleteBackward => 67,
        InputKey::DeleteForward => 112,
        InputKey::Enter => 66,
        InputKey::Left => 21,
        InputKey::Right => 22,
        InputKey::Home => 122,
        InputKey::End => 123,
        InputKey::Text => 0,
    }
}

const TYPE_MASK_CLASS: i32 = 0x0000_000f;
const TYPE_CLASS_NUMBER: i32 = 0x0000_0002;
const TYPE_CLASS_PHONE: i32 = 0x0000_0003;
const TYPE_NUMBER_FLAG_SIGNED: i32 = 0x0000_1000;
const TYPE_NUMBER_FLAG_DECIMAL: i32 = 0x0000_2000;

struct InputConstraints {
    allowed: Option<std::collections::HashSet<char>>,
    max_length: Option<usize>,
}

impl InputConstraints {
    fn unconstrained() -> Self {
        Self {
            allowed: None,
            max_length: None,
        }
    }

    fn permits(&self, character: char) -> bool {
        self.allowed
            .as_ref()
            .is_none_or(|set| set.contains(&character))
    }
}

fn int_attribute_value(value: &crate::binary_xml::XmlValue) -> Option<i32> {
    match value {
        crate::binary_xml::XmlValue::Integer(value) => Some(*value),
        crate::binary_xml::XmlValue::Hex(value) => Some(*value as i32),
        _ => None,
    }
}

fn allowed_characters_for_input_type(input_type: i32) -> Option<std::collections::HashSet<char>> {
    match input_type & TYPE_MASK_CLASS {
        TYPE_CLASS_NUMBER => {
            let mut set: std::collections::HashSet<char> = ('0'..='9').collect();
            if input_type & TYPE_NUMBER_FLAG_DECIMAL != 0 {
                set.insert('.');
            }
            if input_type & TYPE_NUMBER_FLAG_SIGNED != 0 {
                set.insert('-');
            }
            Some(set)
        }
        TYPE_CLASS_PHONE => {
            let mut set: std::collections::HashSet<char> = ('0'..='9').collect();
            for character in ['+', '-', '(', ')', '#', '*', ' ', '.', '/', ',', ';', 'N'] {
                set.insert(character);
            }
            Some(set)
        }
        _ => None,
    }
}

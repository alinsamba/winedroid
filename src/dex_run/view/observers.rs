use super::helpers::*;
use super::*;

impl DexProgram {
    pub(in crate::dex_run) fn read_view_tag(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(tag_id) = invocation.arguments[1] else {
            return None;
        };

        if let Some(value) = self
            .runtime
            .borrow()
            .view_tags
            .get(&(view_key, tag_id))
            .cloned()
        {
            return Some(value);
        }

        self.androidx_view_tree_owner_tag(tag_id)
            .or(Some(DexValue::Null))
    }

    pub(in crate::dex_run) fn write_view_tag(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        let DexValue::Int(tag_id) = invocation.arguments[1] else {
            return None;
        };
        let value = invocation.arguments[2].clone();

        self.runtime
            .borrow_mut()
            .view_tags
            .insert((view_key, tag_id), value);

        Some(())
    }

    pub(in crate::dex_run::view) fn androidx_view_tree_owner_tag(
        &self,
        tag_id: i32,
    ) -> Option<DexValue> {
        let resource_id = u32::try_from(tag_id).ok()?;
        let (type_name, key_name) = self.resource_type_and_key_name(resource_id)?;
        let fallback = androidx_view_tree_owner_tag_value(&type_name, &key_name)?;
        let decor_key = map_identity_key(&decor_view_reference());

        self.runtime
            .borrow()
            .view_tags
            .get(&(decor_key, tag_id))
            .cloned()
            .or(Some(fallback))
    }

    pub(in crate::dex_run) fn read_default_view_tag(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let view_key = map_identity_key(&invocation.arguments[0]);

        Some(
            self.runtime
                .borrow()
                .view_tags
                .get(&(view_key, DEFAULT_VIEW_TAG_ID))
                .cloned()
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn write_default_view_tag(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let view_key = map_identity_key(&invocation.arguments[0]);
        let value = invocation.arguments[1].clone();

        self.runtime
            .borrow_mut()
            .view_tags
            .insert((view_key, DEFAULT_VIEW_TAG_ID), value);

        Some(())
    }

    pub(in crate::dex_run) fn add_view_attach_state_listener(
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

        let view_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_attach_state_listeners
            .entry(view_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_attach_state_listener(
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

        let view_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime.view_attach_state_listeners.get_mut(&view_key) else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime.view_attach_state_listeners.remove(&view_key);
        }

        Some(())
    }

    // cite: frameworks-base ViewTreeObserver.java#addOnPreDrawListener
    pub(in crate::dex_run) fn add_view_tree_observer_pre_draw_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_pre_draw_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    // cite: frameworks-base ViewTreeObserver.java#removeOnPreDrawListener
    pub(in crate::dex_run) fn remove_view_tree_observer_pre_draw_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime.view_tree_pre_draw_listeners.get_mut(&observer_key) else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime.view_tree_pre_draw_listeners.remove(&observer_key);
        }

        Some(())
    }

    pub(in crate::dex_run) fn add_view_tree_observer_draw_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_draw_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_tree_observer_draw_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime.view_tree_draw_listeners.get_mut(&observer_key) else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime.view_tree_draw_listeners.remove(&observer_key);
        }

        Some(())
    }

    pub(in crate::dex_run) fn add_view_tree_observer_global_layout_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_global_layout_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_tree_observer_global_layout_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime
            .view_tree_global_layout_listeners
            .get_mut(&observer_key)
        else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime
                .view_tree_global_layout_listeners
                .remove(&observer_key);
        }

        Some(())
    }

    pub(in crate::dex_run) fn add_view_tree_observer_scroll_changed_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_scroll_changed_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_tree_observer_scroll_changed_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime
            .view_tree_scroll_changed_listeners
            .get_mut(&observer_key)
        else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime
                .view_tree_scroll_changed_listeners
                .remove(&observer_key);
        }

        Some(())
    }

    pub(in crate::dex_run) fn add_view_tree_observer_window_focus_change_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_window_focus_change_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_tree_observer_window_focus_change_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime
            .view_tree_window_focus_change_listeners
            .get_mut(&observer_key)
        else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime
                .view_tree_window_focus_change_listeners
                .remove(&observer_key);
        }

        Some(())
    }

    pub(in crate::dex_run) fn add_view_tree_observer_touch_mode_change_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener = invocation.arguments[1].clone();
        self.runtime
            .borrow_mut()
            .view_tree_touch_mode_change_listeners
            .entry(observer_key)
            .or_default()
            .push(listener);

        Some(())
    }

    pub(in crate::dex_run) fn remove_view_tree_observer_touch_mode_change_listener(
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

        let observer_key = map_identity_key(&invocation.arguments[0]);
        let listener_key = map_identity_key(&invocation.arguments[1]);
        let mut runtime = self.runtime.borrow_mut();
        let Some(listeners) = runtime
            .view_tree_touch_mode_change_listeners
            .get_mut(&observer_key)
        else {
            return Some(());
        };

        if let Some(index) = listeners
            .iter()
            .position(|listener| map_identity_key(listener) == listener_key)
        {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            runtime
                .view_tree_touch_mode_change_listeners
                .remove(&observer_key);
        }

        Some(())
    }
}

impl DexProgram {
    // cite: frameworks-base WindowOnBackInvokedDispatcher.java#registerOnBackInvokedCallbackUnchecked
    pub(in crate::dex_run) fn register_on_back_invoked_callback(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        if !dex_value_is_object_like(&invocation.arguments[0])
            || !dex_value_is_object_like(&invocation.arguments[2])
        {
            return None;
        }
        let DexValue::Int(priority) = invocation.arguments[1] else {
            return None;
        };

        let dispatcher_key = map_identity_key(&invocation.arguments[0]);
        let callback = invocation.arguments[2].clone();
        let mut runtime = self.runtime.borrow_mut();
        let callbacks = runtime
            .back_invoked_callbacks
            .entry(dispatcher_key)
            .or_default();
        callbacks.retain(|(_, registered)| registered != &callback);
        callbacks.push((priority, callback));

        Some(())
    }

    // cite: frameworks-base WindowOnBackInvokedDispatcher.java#unregisterOnBackInvokedCallback
    pub(in crate::dex_run) fn unregister_on_back_invoked_callback(
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

        let dispatcher_key = map_identity_key(&invocation.arguments[0]);
        let callback = &invocation.arguments[1];
        let mut runtime = self.runtime.borrow_mut();
        if let Some(callbacks) = runtime.back_invoked_callbacks.get_mut(&dispatcher_key) {
            callbacks.retain(|(_, registered)| registered != callback);
        }

        Some(())
    }
}

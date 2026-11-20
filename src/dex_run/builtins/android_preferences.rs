use super::super::*;

impl DexProgram {
    pub(super) fn invoke_android_preferences_builtin(
        &self,
        method: &DexMethodId,
        invocation: Option<&DexInvocation>,
    ) -> Option<DexExecutionResult> {
        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/PreferenceFragment;" | "Landroid/app/PreferenceFragment;"
        ) && method.name == "<init>"
            && method.proto.descriptor_is("()V")
        {
            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/preference/PreferenceFragment;"
            && matches!(method.name.as_str(), "onCreate" | "onResume" | "onPause")
            && matches!(
                method.proto.descriptor().as_str(),
                "(Landroid/os/Bundle;)V" | "()V"
            )
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/app/FragmentTransaction;" | "Landroidx/fragment/app/FragmentTransaction;"
        ) && method.name == "replace"
        {
            let invocation = invocation?;
            if let Some(fragment) = invocation.arguments.get(2) {
                if matches!(fragment, DexValue::Object(_) | DexValue::Reference(_)) {
                    self.runtime.borrow_mut().pending_fragment = Some(fragment.clone());
                }
            }
            return Some(DexExecutionResult::Value(invocation.arguments[0].clone()));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/PreferenceFragment;"
                | "Landroid/app/Fragment;"
                | "Landroidx/fragment/app/Fragment;"
        ) && matches!(method.name.as_str(), "getActivity" | "getContext")
        {
            let activity = self.active_activity_receiver()?;
            return Some(DexExecutionResult::Value(activity));
        }

        if method.class_descriptor == "Landroid/preference/PreferenceFragment;"
            && method.name == "addPreferencesFromResource"
            && method.proto.descriptor_is("(I)V")
        {
            let invocation = invocation?;
            // cite: frameworks-base PreferenceFragment.java#addPreferencesFromResource
            match self.inflate_preferences_from_resource(invocation) {
                Ok(Some(())) => {}
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "preference XML"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }

            return Some(DexExecutionResult::Void);
        }

        if method.class_descriptor == "Landroid/preference/PreferenceFragment;"
            && method.name == "findPreference"
            && method.proto.descriptor()
                == "(Ljava/lang/CharSequence;)Landroid/preference/Preference;"
        {
            let invocation = invocation?;
            let value = self.preference_find(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/preference/PreferenceFragment;"
            && method.name == "getPreferenceScreen"
            && method
                .proto
                .descriptor_is("()Landroid/preference/PreferenceScreen;")
        {
            return Some(DexExecutionResult::Value(DexValue::Reference(
                std::rc::Rc::from("Landroid/preference/PreferenceScreen;.instance"),
            )));
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/PreferenceScreen;" | "Landroid/preference/Preference;"
        ) && method.name == "getSharedPreferences"
            && method
                .proto
                .descriptor_is("()Landroid/content/SharedPreferences;")
        {
            let value = self
                .default_shared_preferences_reference()
                .unwrap_or(DexValue::Null);
            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/content/SharedPreferences;"
            && matches!(
                method.name.as_str(),
                "registerOnSharedPreferenceChangeListener"
                    | "unregisterOnSharedPreferenceChangeListener"
            )
        {
            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/Preference;" | "Landroid/preference/ListPreference;"
        ) && method.name == "setSummary"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/lang/CharSequence;)V" | "(I)V"
            )
        {
            let invocation = invocation?;
            // cite: frameworks-base Preference.java#setSummary
            match self.preference_set_summary(invocation) {
                Ok(Some(())) => {}
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "preference summary"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/Preference;" | "Landroid/preference/ListPreference;"
        ) && method.name == "setTitle"
            && matches!(
                method.proto.descriptor().as_str(),
                "(Ljava/lang/CharSequence;)V" | "(I)V"
            )
        {
            let invocation = invocation?;
            // cite: frameworks-base Preference.java#setTitle
            match self.preference_set_title(invocation) {
                Ok(Some(())) => {}
                Ok(None) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_for_request(invocation, "preference title"),
                    ));
                }
                Err(error) => {
                    return Some(DexExecutionResult::Throw(
                        self.resource_not_found_exception(error),
                    ));
                }
            }

            return Some(DexExecutionResult::Void);
        }

        if matches!(
            method.class_descriptor.as_str(),
            "Landroid/preference/Preference;" | "Landroid/preference/ListPreference;"
        ) && method.name == "getSummary"
            && method.proto.descriptor_is("()Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            let value = self.preference_get_summary(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/preference/ListPreference;"
            && method.name == "getEntry"
            && method.proto.descriptor_is("()Ljava/lang/CharSequence;")
        {
            let invocation = invocation?;
            let value = self.list_preference_get_entry(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        if method.class_descriptor == "Landroid/preference/ListPreference;"
            && method.name == "getValue"
            && method.proto.descriptor_is("()Ljava/lang/String;")
        {
            let invocation = invocation?;
            let value = self.list_preference_get_value(invocation)?;

            return Some(DexExecutionResult::Value(value));
        }

        None
    }
}

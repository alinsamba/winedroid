use super::*;
use crate::binary_xml::{self, XmlEvent, XmlStartElement, XmlValue};
use crate::dex_run::runtime::InflatedPreference;
use crate::dex_run::{DexPreferenceKind, DexPreferenceSnapshot};

// cite: frameworks-base PreferenceGroup.java#findPreference
const PREFERENCE_KEY_MARKER: &str = ":key/";

impl DexProgram {
    pub(in crate::dex_run) fn run_pending_fragment(&self, depth: usize) -> Result<()> {
        let Some(fragment) = self.runtime.borrow_mut().pending_fragment.take() else {
            return Ok(());
        };
        let Some(class_descriptor) = value_class_descriptor(&fragment).map(str::to_owned) else {
            return Ok(());
        };

        self.attach_pending_fragment_to_active_host(&fragment);

        for (name, proto) in [
            (
                "onCreate",
                crate::dex::DexProto {
                    shorty: "VL".to_owned(),
                    return_type: "V".to_owned(),
                    parameters: vec!["Landroid/os/Bundle;".to_owned()],
                },
            ),
            (
                "onResume",
                crate::dex::DexProto {
                    shorty: "V".to_owned(),
                    return_type: "V".to_owned(),
                    parameters: Vec::new(),
                },
            ),
        ] {
            let method = DexMethodId {
                class_descriptor: class_descriptor.clone(),
                name: name.to_owned(),
                proto,
            };
            let Some(resolved) =
                self.resolve_exact_method_in_class_hierarchy(&class_descriptor, &method)?
            else {
                continue;
            };
            let arguments = if name == "onCreate" {
                vec![fragment.clone(), DexValue::Null]
            } else {
                vec![fragment.clone()]
            };
            let result = self.execute_method(None, &resolved, Some(&arguments), depth + 1)?;
            match result {
                DexExecutionResult::Void => {}
                DexExecutionResult::Parked(execution) => {
                    self.discard_parked_execution(&execution);
                    return Err(WinedroidError::new(format!(
                        "Fragment lifecycle method parked without a guest-thread scheduler\nclass: {class_descriptor}\nmethod: {name}"
                    )));
                }
                result => {
                    return Err(WinedroidError::new(format!(
                        "Fragment lifecycle method did not complete normally\nclass: {class_descriptor}\nmethod: {name}\nresult: {result}"
                    )));
                }
            }
        }

        Ok(())
    }

    pub(in crate::dex_run) fn inflate_preferences_from_resource(
        &self,
        invocation: &DexInvocation,
    ) -> Result<Option<()>> {
        let Some(DexValue::Int(resource_id)) = invocation.arguments.get(1) else {
            return Ok(None);
        };
        let Some(preferences) = self.preferences_from_xml_resource(*resource_id)? else {
            return Ok(None);
        };
        self.runtime.borrow_mut().inflated_preferences = preferences;

        Ok(Some(()))
    }

    fn preferences_from_xml_resource(
        &self,
        resource_id: i32,
    ) -> Result<Option<Vec<InflatedPreference>>> {
        if resource_id <= 0 {
            return Err(WinedroidError::new(format!(
                "preference XML resource ID must be positive\nresource id: {resource_id}"
            )));
        }
        let Some(apk_path) = self.apk_path.as_ref() else {
            return Ok(None);
        };
        let resource = self
            .resolve_resource_file(resource_id as u32)?
            .ok_or_else(|| preference_resource_error("XML", resource_id as u32))?;
        let mut apk = ApkArchive::open(apk_path)?;
        let data = apk.read_required_file(&resource.path)?;
        let events = binary_xml::parse_binary_xml(&data)?;

        let mut preferences = Vec::new();
        let mut depth = 0usize;
        let mut first = true;

        for event in events {
            match event {
                XmlEvent::Start(element) => {
                    if first {
                        first = false;
                        depth += 1;
                        continue;
                    }
                    if let Some(preference) = self.inflate_preference_element(&element, depth)? {
                        preferences.push(preference);
                    }
                    depth += 1;
                }
                XmlEvent::End(_) => depth = depth.saturating_sub(1),
            }
        }

        Ok(Some(preferences))
    }

    fn inflate_preference_element(
        &self,
        element: &XmlStartElement,
        depth: usize,
    ) -> Result<Option<InflatedPreference>> {
        let class_descriptor = preference_class_descriptor(&element.name);
        let key = self.preference_attribute_string(element, "key")?;
        let title = self
            .preference_attribute_string(element, "title")?
            .unwrap_or_default();
        let summary = self.preference_attribute_string(element, "summary")?;
        let icon_resource_id = match element.attribute("icon").map(|attribute| &attribute.value) {
            Some(XmlValue::Reference(id)) => Some(*id),
            Some(XmlValue::AttributeReference(attr_id)) => {
                self.resolve_preference_icon_attribute(*attr_id)
            }
            _ => None,
        };

        let kind = if element.name.ends_with("PreferenceCategory") {
            DexPreferenceKind::Category
        } else if element.name.ends_with("CheckBoxPreference")
            || element.name.ends_with("SwitchPreference")
        {
            let checked = self
                .preference_stored_value(key.as_deref())
                .and_then(|value| value.parse::<bool>().ok())
                .unwrap_or_else(|| {
                    element
                        .attribute("defaultValue")
                        .and_then(|attribute| attribute.value.as_bool())
                        .unwrap_or(false)
                });
            DexPreferenceKind::Switch { checked }
        } else if element.name.ends_with("ListPreference") {
            let entries = self
                .preference_attribute_string_array(element, "entries")?
                .unwrap_or_default();
            let entry_values = self
                .preference_attribute_string_array(element, "entryValues")?
                .unwrap_or_default();
            let value = match self.preference_stored_value(key.as_deref()) {
                Some(value) => Some(value),
                None => self.preference_attribute_string(element, "defaultValue")?,
            };
            DexPreferenceKind::List {
                value,
                entries,
                entry_values,
            }
        } else {
            DexPreferenceKind::Plain
        };

        Ok(Some(InflatedPreference {
            key,
            title,
            summary,
            icon_resource_id,
            class_descriptor,
            kind,
            depth,
        }))
    }

    fn preference_attribute_string(
        &self,
        element: &XmlStartElement,
        name: &str,
    ) -> Result<Option<String>> {
        let Some(attribute) = element.attribute(name) else {
            return Ok(None);
        };
        match &attribute.value {
            XmlValue::String(value) => Ok(Some(value.clone())),
            XmlValue::Reference(id) => {
                let value = self
                    .resolve_string_resource_text(*id)?
                    .ok_or_else(|| preference_resource_error("string", *id))?;
                Ok(Some(value))
            }
            XmlValue::Integer(value) => Ok(Some(value.to_string())),
            XmlValue::Boolean(value) => Ok(Some(value.to_string())),
            _ => Ok(None),
        }
    }

    fn preference_attribute_string_array(
        &self,
        element: &XmlStartElement,
        name: &str,
    ) -> Result<Option<Vec<String>>> {
        let Some(attribute) = element.attribute(name) else {
            return Ok(None);
        };
        match &attribute.value {
            XmlValue::Reference(id) => {
                let values = self
                    .resolve_text_array_resource(*id)?
                    .ok_or_else(|| preference_resource_error("text array", *id))?;
                let mut strings = Vec::with_capacity(values.len());
                for value in values {
                    let value = value.ok_or_else(|| {
                        WinedroidError::new(format!(
                            "preference text array 0x{id:08x} contains a null entry"
                        ))
                    })?;
                    strings.push(value);
                }
                Ok(Some(strings))
            }
            _ => Ok(None),
        }
    }

    fn resolve_preference_icon_attribute(&self, attribute_id: u32) -> Option<u32> {
        let activity = self.active_activity_receiver()?;
        let theme = self.ensure_context_theme(&activity)?;
        match self.resolve_theme_attribute(&theme, attribute_id)? {
            crate::resource_table::ResourcePrimitive::Reference(resource_id) => Some(resource_id),
            _ => None,
        }
    }

    fn preference_stored_value(&self, key: Option<&str>) -> Option<String> {
        let key = key?;
        let preferences = self.default_shared_preferences_reference()?;
        let entry_key = (map_identity_key(&preferences), key.to_owned());
        match self
            .runtime
            .borrow()
            .shared_preferences_entries
            .get(&entry_key)
        {
            Some(DexValue::String(value)) => Some(value.clone().to_string()),
            Some(DexValue::Int(value)) => Some(value.to_string()),
            _ => None,
        }
    }

    pub(in crate::dex_run) fn preference_find(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let DexValue::String(key) = invocation.arguments.get(1)? else {
            return None;
        };
        let class_descriptor = self
            .runtime
            .borrow()
            .inflated_preferences
            .iter()
            .find(|preference| preference.key.as_deref() == Some(key.as_ref()))
            .map(|preference| preference.class_descriptor.clone());
        let Some(class_descriptor) = class_descriptor else {
            return Some(DexValue::Null);
        };

        Some(DexValue::Reference(std::rc::Rc::from(format!(
            "{class_descriptor}{PREFERENCE_KEY_MARKER}{key}"
        ))))
    }

    pub(in crate::dex_run) fn preference_set_summary(
        &self,
        invocation: &DexInvocation,
    ) -> Result<Option<()>> {
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let Some(key) = preference_reference_key(receiver) else {
            return Ok(None);
        };
        let Some(value) = invocation.arguments.get(1) else {
            return Ok(None);
        };
        let summary = match value {
            DexValue::String(value) => Some(value.to_string()),
            DexValue::Null => None,
            DexValue::Int(resource_id) => {
                let resource_id = u32::try_from(*resource_id).map_err(|error| {
                    WinedroidError::with_source(
                        format!("Preference summary resource ID is negative: {resource_id}"),
                        error,
                    )
                })?;
                Some(
                    self.resolve_string_resource_text(resource_id)?
                        .ok_or_else(|| preference_resource_error("string", resource_id))?,
                )
            }
            other => self.char_sequence_text(other),
        };
        Ok(self.with_inflated_preference(&key, |preference| preference.summary = summary))
    }

    pub(in crate::dex_run) fn preference_set_title(
        &self,
        invocation: &DexInvocation,
    ) -> Result<Option<()>> {
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let Some(key) = preference_reference_key(receiver) else {
            return Ok(None);
        };
        let Some(value) = invocation.arguments.get(1) else {
            return Ok(None);
        };
        let title = match value {
            DexValue::String(value) => value.clone(),
            DexValue::Int(resource_id) => {
                let resource_id = u32::try_from(*resource_id).map_err(|error| {
                    WinedroidError::with_source(
                        format!("Preference title resource ID is negative: {resource_id}"),
                        error,
                    )
                })?;
                let title = self
                    .resolve_string_resource_text(resource_id)?
                    .ok_or_else(|| preference_resource_error("string", resource_id))?;
                std::rc::Rc::from(title)
            }
            other => {
                let Some(title) = self.char_sequence_text(other) else {
                    return Ok(None);
                };
                std::rc::Rc::from(title)
            }
        };
        Ok(self.with_inflated_preference(&key, |preference| preference.title = title.to_string()))
    }

    pub(in crate::dex_run) fn preference_get_summary(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let key = preference_reference_key(invocation.arguments.first()?)?;
        let summary = self
            .runtime
            .borrow()
            .inflated_preferences
            .iter()
            .find(|preference| preference.key.as_deref() == Some(key.as_str()))?
            .summary
            .clone();

        Some(
            summary
                .map(|value| DexValue::String(value.into()))
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn list_preference_get_entry(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let key = preference_reference_key(invocation.arguments.first()?)?;
        let runtime = self.runtime.borrow();
        let preference = runtime
            .inflated_preferences
            .iter()
            .find(|preference| preference.key.as_deref() == Some(key.as_str()))?;
        let DexPreferenceKind::List {
            value,
            entries,
            entry_values,
        } = &preference.kind
        else {
            return Some(DexValue::Null);
        };

        let entry = value.as_ref().and_then(|value| {
            entry_values
                .iter()
                .position(|candidate| candidate == value)
                .and_then(|index| entries.get(index))
                .cloned()
        });

        Some(
            entry
                .map(|value| DexValue::String(value.into()))
                .unwrap_or(DexValue::Null),
        )
    }

    pub(in crate::dex_run) fn list_preference_get_value(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let key = preference_reference_key(invocation.arguments.first()?)?;
        let runtime = self.runtime.borrow();
        let preference = runtime
            .inflated_preferences
            .iter()
            .find(|preference| preference.key.as_deref() == Some(key.as_str()))?;
        let DexPreferenceKind::List { value, .. } = &preference.kind else {
            return Some(DexValue::Null);
        };

        Some(
            value
                .clone()
                .map(|value| DexValue::String(value.into()))
                .unwrap_or(DexValue::Null),
        )
    }

    fn with_inflated_preference(
        &self,
        key: &str,
        update: impl FnOnce(&mut InflatedPreference),
    ) -> Option<()> {
        let mut runtime = self.runtime.borrow_mut();
        let preference = runtime
            .inflated_preferences
            .iter_mut()
            .find(|preference| preference.key.as_deref() == Some(key))?;
        update(preference);

        Some(())
    }

    pub(in crate::dex_run) fn inflated_preference_snapshot(&self) -> Vec<DexPreferenceSnapshot> {
        self.runtime
            .borrow()
            .inflated_preferences
            .iter()
            .enumerate()
            .map(|(index, preference)| DexPreferenceSnapshot {
                runtime_key: format!("inflated-preference/{index}"),
                key: preference.key.clone(),
                title: preference.title.clone(),
                summary: list_preference_substituted_summary(preference),
                icon_resource_id: preference.icon_resource_id,
                class_descriptor: preference.class_descriptor.clone(),
                kind: preference.kind.clone(),
                depth: preference.depth,
            })
            .collect()
    }
}

impl DexProgram {
    fn attach_pending_fragment_to_active_host(&self, fragment: &DexValue) {
        // cite: androidx FragmentStateManager.java#attach
        // cite: androidx Fragment.java#performAttach
        let Some(host) = self.active_activity_fragment_host() else {
            return;
        };

        if self
            .read_stored_instance_field(fragment, &fragment_host_field())
            .is_none()
        {
            self.write_stored_instance_field(fragment, &fragment_host_field(), host.clone());
        }

        if self
            .read_stored_instance_field(fragment, &fragment_manager_field())
            .is_none()
        {
            if let Some(manager) =
                self.read_stored_instance_field(&host, &host_callback_fragment_manager_field())
            {
                self.write_stored_instance_field(fragment, &fragment_manager_field(), manager);
            }
        }

        // cite: androidx FragmentManager.java#attachController
        if let Some(child_manager) =
            self.read_stored_instance_field(fragment, &fragment_child_manager_field())
        {
            if dex_value_is_object_like(&child_manager)
                && self
                    .read_stored_instance_field(&child_manager, &fragment_manager_host_field())
                    .is_none()
            {
                self.write_stored_instance_field(
                    &child_manager,
                    &fragment_manager_host_field(),
                    host,
                );
            }
        }
    }

    fn active_activity_fragment_host(&self) -> Option<DexValue> {
        let activity = self.active_activity_receiver()?;
        let controller =
            self.read_stored_instance_field(&activity, &activity_fragment_controller_field())?;
        let host = self.read_stored_instance_field(&controller, &controller_host_field())?;
        if !dex_value_is_object_like(&host) {
            return None;
        }

        Some(host)
    }
}

fn fragment_host_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/Fragment;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentHostCallback;".to_owned(),
        name: "mHost".to_owned(),
    }
}

fn fragment_manager_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/Fragment;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentManager;".to_owned(),
        name: "mFragmentManager".to_owned(),
    }
}

fn activity_fragment_controller_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentActivity;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentController;".to_owned(),
        name: "mFragments".to_owned(),
    }
}

fn controller_host_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentController;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentHostCallback;".to_owned(),
        name: "mHost".to_owned(),
    }
}

fn host_callback_fragment_manager_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentHostCallback;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentManager;".to_owned(),
        name: "mFragmentManager".to_owned(),
    }
}

fn fragment_child_manager_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/Fragment;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentManager;".to_owned(),
        name: "mChildFragmentManager".to_owned(),
    }
}

fn fragment_manager_host_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Landroidx/fragment/app/FragmentManager;".to_owned(),
        type_descriptor: "Landroidx/fragment/app/FragmentHostCallback;".to_owned(),
        name: "mHost".to_owned(),
    }
}

fn list_preference_substituted_summary(preference: &InflatedPreference) -> Option<String> {
    let summary = preference.summary.as_ref()?;
    let DexPreferenceKind::List {
        value,
        entries,
        entry_values,
    } = &preference.kind
    else {
        return Some(summary.clone());
    };

    if !summary.contains("%s") {
        return Some(summary.clone());
    }

    let entry = value
        .as_ref()
        .and_then(|value| entry_values.iter().position(|candidate| candidate == value))
        .and_then(|index| entries.get(index))
        .cloned()
        .unwrap_or_default();

    Some(summary.replacen("%s", &entry, 1))
}

fn preference_resource_error(resource_kind: &str, resource_id: u32) -> WinedroidError {
    WinedroidError::new(format!(
        "preference {resource_kind} resource 0x{resource_id:08x} was not found"
    ))
}

fn preference_reference_key(value: &DexValue) -> Option<String> {
    let DexValue::Reference(reference) = value else {
        return None;
    };
    reference
        .split_once(PREFERENCE_KEY_MARKER)
        .map(|(_, key)| key.to_owned())
}

fn preference_class_descriptor(name: &str) -> String {
    if name.contains('.') {
        return format!("L{};", name.replace('.', "/"));
    }

    format!("Landroid/preference/{name};")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_preference(value: &str, summary: &str) -> InflatedPreference {
        InflatedPreference {
            key: Some("pref_theme".to_owned()),
            title: "Theme".to_owned(),
            summary: Some(summary.to_owned()),
            icon_resource_id: None,
            class_descriptor: "Landroid/preference/ListPreference;".to_owned(),
            kind: DexPreferenceKind::List {
                value: Some(value.to_owned()),
                entries: vec!["Light".to_owned(), "Dark".to_owned(), "System".to_owned()],
                entry_values: vec!["0".to_owned(), "1".to_owned(), "2".to_owned()],
            },
            depth: 1,
        }
    }

    #[test]
    fn list_preference_summary_substitutes_current_entry_for_percent_s() {
        let preference = list_preference("2", "Current theme: %s");

        assert_eq!(
            list_preference_substituted_summary(&preference),
            Some("Current theme: System".to_owned())
        );
    }

    #[test]
    fn list_preference_summary_without_placeholder_is_unchanged() {
        let preference = list_preference("0", "Choose app theme.");

        assert_eq!(
            list_preference_substituted_summary(&preference),
            Some("Choose app theme.".to_owned())
        );
    }

    #[test]
    fn preference_class_descriptors_map_short_and_custom_names() {
        assert_eq!(
            preference_class_descriptor("CheckBoxPreference"),
            "Landroid/preference/CheckBoxPreference;"
        );
        assert_eq!(
            preference_class_descriptor("org.billthefarmer.currency.AboutPreference"),
            "Lorg/billthefarmer/currency/AboutPreference;"
        );
    }
}

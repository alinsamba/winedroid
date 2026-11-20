use super::helpers::*;
use super::*;
use crate::dex_run::android::app::drawable_resource_id_from_value;
use crate::dex_run::android::content_res::bitmap_drawable_resource_id_field;

const MAX_DYNAMIC_VIEW_DEPTH: usize = 24;

const DYNAMIC_VIEW_CLICK_ID_BASE: i32 = 0x6E00_0000;
const DYNAMIC_VIEW_CLICK_ID_MASK: i32 = 0x00FF_FFFF;

fn dynamic_view_click_id(object_id: u64) -> i32 {
    DYNAMIC_VIEW_CLICK_ID_BASE | (object_id as i32 & DYNAMIC_VIEW_CLICK_ID_MASK)
}

const WIDGET_TAG_NAMES: &[(&str, &str)] = &[
    ("Landroid/widget/TextView;", "TextView"),
    ("Landroid/widget/EditText;", "EditText"),
    ("Landroid/widget/Button;", "Button"),
    ("Landroid/widget/ImageButton;", "ImageButton"),
    ("Landroid/widget/ImageView;", "ImageView"),
    ("Landroid/widget/LinearLayout;", "LinearLayout"),
    ("Landroid/widget/FrameLayout;", "FrameLayout"),
    ("Landroid/widget/RelativeLayout;", "RelativeLayout"),
    ("Landroid/widget/TableLayout;", "TableLayout"),
    ("Landroid/widget/TableRow;", "TableRow"),
    ("Landroid/widget/ListView;", "ListView"),
    ("Landroid/widget/ScrollView;", "ScrollView"),
    ("Landroid/widget/Space;", "Space"),
    (
        "Landroid/widget/HorizontalScrollView;",
        "HorizontalScrollView",
    ),
    ("Landroid/view/ViewGroup;", "ViewGroup"),
    ("Landroid/view/View;", "View"),
];

impl DexProgram {
    // cite: frameworks-base ViewGroup.java#addView
    pub(in crate::dex_run) fn dynamic_container_children_snapshot(
        &self,
    ) -> Vec<DexDynamicViewGroup> {
        let entries: Vec<(String, Vec<DexValue>)> = self
            .runtime
            .borrow()
            .view_group_children
            .iter()
            .map(|(key, children)| (key.clone(), children.clone()))
            .collect();

        let mut result = Vec::new();
        for (key, children) in entries {
            if children.is_empty() {
                continue;
            }
            let Some(view_id) = view_id_from_layout_child_reference_key(&key) else {
                continue;
            };
            let materialized: Vec<DexDynamicView> = children
                .iter()
                .filter_map(|child| self.materialize_dynamic_view(child, 0))
                .collect();
            if materialized.is_empty() {
                continue;
            }
            result.push(DexDynamicViewGroup {
                container_view_id: view_id,
                children: materialized,
            });
        }

        result
    }

    fn materialize_dynamic_view(&self, view: &DexValue, depth: usize) -> Option<DexDynamicView> {
        // cite: frameworks-base LayoutInflater.java#inflate
        if depth > MAX_DYNAMIC_VIEW_DEPTH || !matches!(view, DexValue::Object(_)) {
            return None;
        }
        let class_descriptor = value_class_descriptor(view)?;
        let tag_name = self.widget_tag_name_for_class_descriptor(class_descriptor);

        let text = match self.read_stored_instance_field(view, &text_view_text_field()) {
            Some(DexValue::String(text)) => Some(text.to_string()),
            _ => None,
        };
        let background_color = self
            .read_stored_instance_field(view, &view_background_field())
            .and_then(|drawable| {
                self.read_stored_instance_field(&drawable, &color_drawable_color_field())
            })
            .and_then(|value| match value {
                DexValue::Int(color) => Some(color as u32),
                _ => None,
            });
        let drawable_resource_id = self
            .read_stored_instance_field(view, &image_view_drawable_field())
            .and_then(|value| match &value {
                DexValue::Reference(_) => drawable_resource_id_from_value(&value),
                DexValue::Object(_) => {
                    match self
                        .read_stored_instance_field(&value, &bitmap_drawable_resource_id_field())
                    {
                        Some(DexValue::Int(resource_id)) => u32::try_from(resource_id).ok(),
                        _ => None,
                    }
                }
                _ => None,
            });
        let visibility = match self.read_stored_instance_field(view, &view_visibility_field()) {
            Some(DexValue::Int(visibility)) => visibility,
            _ => 0,
        };
        let explicit_width =
            match self.read_stored_instance_field(view, &view_minimum_width_field()) {
                Some(DexValue::Int(width)) if width > 0 => Some(width as u32),
                _ => None,
            };
        let explicit_height =
            match self.read_stored_instance_field(view, &view_minimum_height_field()) {
                Some(DexValue::Int(height)) if height > 0 => Some(height as u32),
                _ => None,
            };
        let click_id = match view {
            DexValue::Object(object) if self.view_has_click_binding(view) => {
                let click_id = dynamic_view_click_id(object.id);
                self.runtime
                    .borrow_mut()
                    .dynamic_view_click_targets
                    .insert(click_id, view.clone());
                Some(click_id)
            }
            _ => None,
        };

        let child_refs = self
            .runtime
            .borrow()
            .view_group_children
            .get(&map_identity_key(view))
            .cloned()
            .unwrap_or_default();
        let children = child_refs
            .iter()
            .filter_map(|child| self.materialize_dynamic_view(child, depth + 1))
            .collect();

        Some(DexDynamicView {
            tag_name,
            text,
            background_color,
            drawable_resource_id,
            visibility,
            explicit_width,
            explicit_height,
            click_id,
            children,
        })
    }

    // cite: frameworks-base ViewGroup.java#addView
    pub(in crate::dex_run) fn dynamic_view_click_target(&self, click_id: i32) -> Option<DexValue> {
        self.runtime
            .borrow()
            .dynamic_view_click_targets
            .get(&click_id)
            .cloned()
    }

    fn widget_tag_name_for_class_descriptor(&self, class_descriptor: &str) -> String {
        let mut current = Some(class_descriptor.to_owned());
        let mut visited = std::collections::HashSet::new();
        while let Some(descriptor) = current {
            if !visited.insert(descriptor.clone()) {
                break;
            }
            if let Some((_, tag)) = WIDGET_TAG_NAMES
                .iter()
                .find(|(candidate, _)| *candidate == descriptor)
            {
                return (*tag).to_owned();
            }
            current = self.superclass_descriptor(&descriptor);
        }
        "View".to_owned()
    }
}

fn view_id_from_layout_child_reference_key(key: &str) -> Option<u32> {
    let rest = key.strip_prefix("reference:")?;
    if !rest.contains(":layout/") {
        return None;
    }
    let (_, view_id) = rest.rsplit_once("/id/")?;
    view_id
        .parse::<i32>()
        .ok()
        .and_then(|id| u32::try_from(id).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_id_parses_from_a_layout_child_reference_key() {
        assert_eq!(
            view_id_from_layout_child_reference_key(
                "reference:Landroid/widget/LinearLayout;:layout/2130968576/id/2130903045"
            ),
            Some(2130903045)
        );
    }

    #[test]
    fn view_id_is_none_for_non_layout_reference_keys() {
        assert_eq!(
            view_id_from_layout_child_reference_key("reference:content"),
            None
        );
        assert_eq!(view_id_from_layout_child_reference_key("object:12"), None);
    }
}

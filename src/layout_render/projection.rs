use super::*;

pub(crate) fn render_text_frame_lines(elements: &[LayoutTextElement]) -> Vec<String> {
    let mut lines = Vec::new();

    for element in elements {
        let Some(frame) = element.frame else {
            continue;
        };
        let id = element
            .view_id
            .map(|view_id| format!(" id=0x{view_id:08x}"))
            .unwrap_or_default();
        let on_click = element
            .on_click
            .as_ref()
            .map(|method| format!(" onClick={method}"))
            .unwrap_or_default();
        let drawable = element
            .drawable
            .as_ref()
            .map(|drawable| format!(" drawable=0x{:08x}:{}", drawable.resource_id, drawable.path))
            .or_else(|| {
                element
                    .drawable_resource_id
                    .map(|resource_id| format!(" drawable=0x{resource_id:08x}"))
            })
            .unwrap_or_default();
        let text_size = element
            .text_size
            .map(|size| format!(" textSize={size}"))
            .unwrap_or_default();
        let text_color = element
            .text_color
            .map(|color| format!(" textColor=0x{color:08x}"))
            .unwrap_or_default();
        let background_color = element
            .background_color
            .map(|color| format!(" backgroundColor=0x{color:08x}"))
            .unwrap_or_default();

        lines.push(format!(
            "{} {:?}{}{}{}{}{}{} frame={}x{}+{}+{}",
            layout_text_kind_name(element.kind),
            element.text,
            id,
            on_click,
            drawable,
            text_size,
            text_color,
            background_color,
            frame.width,
            frame.height,
            frame.x,
            frame.y
        ));
    }

    lines
}

pub(crate) fn collect_text_elements(
    node: &ViewNode,
    depth: usize,
    frame: LayoutFrame,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    elements: &mut Vec<LayoutTextElement>,
) -> Result<()> {
    let mut cache = LayoutResolutionCache::default();
    collect_text_elements_cached(node, depth, frame, resources, theme, &mut cache, elements)
}

fn collect_text_elements_cached(
    node: &ViewNode,
    depth: usize,
    frame: LayoutFrame,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
    elements: &mut Vec<LayoutTextElement>,
) -> Result<()> {
    if !is_node_visible(node) {
        return Ok(());
    }

    if let Some(content) = node_render_content_cached(node, resources, theme, cache)? {
        elements.push(LayoutTextElement {
            text: content.text,
            kind: text_element_kind(node),
            view_id: node_id(node),
            on_click: node_on_click(node),
            drawable_resource_id: content.drawable_resource_id,
            drawable_padding: content.drawable_padding,
            drawable_tint: content.drawable_tint,
            drawable: None,
            text_size: content.text_size,
            text_padding_left: content.text_padding_left,
            text_padding_right: content.text_padding_right,
            text_color: content.text_color,
            background_color: content.background_color,
            background_corner_radius: content.background_corner_radius,
            background_border_color: content.background_border_color,
            background_border_width: content.background_border_width,
            elevation_px: content.elevation_px,
            alignment: node_text_alignment(node),
            editable: is_edit_text_node(node),
            depth,
            frame: Some(frame),
            clip: None,
        });
    }

    for (child, child_frame) in measure_child_frames(node, frame) {
        collect_text_elements_cached(
            child,
            depth + 1,
            child_frame,
            resources,
            theme,
            cache,
            elements,
        )?;
    }

    Ok(())
}

pub(crate) struct AdapterRowsRender {
    pub(crate) item_tree: ViewNode,
    pub(crate) rows: Vec<AdapterRowRender>,
    pub(crate) image_intrinsic_sizes: BTreeMap<u32, (u32, u32)>,
}

pub(crate) struct AdapterRowRender {
    pub(crate) click_id: u32,
    pub(crate) texts: BTreeMap<u32, String>,
    pub(crate) images: BTreeMap<u32, u32>,
    pub(crate) visibilities: BTreeMap<u32, i32>,
}

pub(crate) fn collect_adapter_view_frames(
    node: &ViewNode,
    frame: LayoutFrame,
    adapter_ids: &BTreeSet<u32>,
    frames: &mut BTreeMap<u32, LayoutFrame>,
) {
    if !is_node_visible(node) {
        return;
    }

    if let Some(id) = node_id(node) {
        if adapter_ids.contains(&id) {
            frames.insert(id, frame);
        }
    }

    for (child, child_frame) in measure_child_frames(node, frame) {
        collect_adapter_view_frames(child, child_frame, adapter_ids, frames);
    }
}

pub(crate) fn collect_adapter_rows_elements(
    adapter: &AdapterRowsRender,
    list_frame: LayoutFrame,
    window_height: u32,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    elements: &mut Vec<LayoutTextElement>,
) -> Result<()> {
    if adapter.rows.is_empty() {
        return Ok(());
    }

    let config = theme.map(|theme| theme.config).unwrap_or_default();
    let mut row_nodes = Vec::with_capacity(adapter.rows.len());
    for row in &adapter.rows {
        let mut tree = adapter.item_tree.clone();
        apply_adapter_row_overrides(&mut tree, row);
        apply_row_image_intrinsic_sizes(
            &mut tree,
            &adapter.image_intrinsic_sizes,
            resources,
            config,
        )?;
        row_nodes.push(tree);
    }

    let container = ViewNode {
        name: "LinearLayout".to_owned(),
        attributes: vec![ViewAttribute {
            name: "orientation".to_owned(),
            value: "1".to_owned(),
            typed_value: XmlValue::Integer(1),
            reference: None,
        }],
        children: row_nodes,
    };

    const ROW_HEIGHT_ESTIMATE: u32 = 160;
    let materialised_height = (adapter.rows.len() as u32).saturating_mul(ROW_HEIGHT_ESTIMATE);
    let container_frame = LayoutFrame {
        x: list_frame.x,
        y: list_frame.y,
        width: list_frame.width,
        height: window_height
            .saturating_sub(list_frame.y)
            .max(materialised_height),
    };

    const LIST_VIEWPORT_BOTTOM_GAP: u32 = 10;
    let viewport_bottom = elements
        .iter()
        .filter_map(|element| element.frame)
        .filter(|frame| frame.y > list_frame.y)
        .map(|frame| frame.y)
        .min()
        .unwrap_or(window_height)
        .min(window_height);
    let viewport = LayoutFrame {
        x: list_frame.x,
        y: list_frame.y,
        width: list_frame.width,
        height: viewport_bottom
            .saturating_sub(list_frame.y)
            .saturating_sub(LIST_VIEWPORT_BOTTOM_GAP),
    };

    for (row, (row_node, row_frame)) in adapter
        .rows
        .iter()
        .zip(measure_child_frames(&container, container_frame))
    {
        let mut click_target = row_click_target(row.click_id, row_frame);
        click_target.clip = Some(viewport);
        elements.push(click_target);

        let mut row_elements = Vec::new();
        collect_text_elements(row_node, 0, row_frame, resources, theme, &mut row_elements)?;
        for element in &mut row_elements {
            element.view_id = None;
            element.clip = Some(viewport);
        }
        elements.extend(row_elements);
    }

    Ok(())
}

// cite: frameworks-base ViewGroup.java#addView
// cite: frameworks-base LinearLayout.java#VERTICAL
pub(crate) fn collect_dynamic_children_elements(
    container: &crate::dex_run::DexDynamicViewGroup,
    list_frame: LayoutFrame,
    window_height: u32,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    elements: &mut Vec<LayoutTextElement>,
) -> Result<()> {
    if container.children.is_empty() {
        return Ok(());
    }

    // cite: frameworks-base View.java#GONE, INVISIBLE
    let visible_rows: Vec<&crate::dex_run::DexDynamicView> = container
        .children
        .iter()
        .filter(|row| row.visibility == ANDROID_VIEW_VISIBLE)
        .collect();
    if visible_rows.is_empty() {
        return Ok(());
    }

    let wrapper = ViewNode {
        name: "LinearLayout".to_owned(),
        attributes: vec![ViewAttribute {
            name: "orientation".to_owned(),
            value: "1".to_owned(),
            typed_value: XmlValue::Integer(1),
            reference: None,
        }],
        children: visible_rows
            .iter()
            .map(|row| dynamic_view_to_view_node(row))
            .collect(),
    };

    // cite: frameworks-base ScrollView.java#measureChild
    let content_frame = LayoutFrame {
        x: list_frame.x,
        y: list_frame.y,
        width: list_frame.width,
        height: window_height.saturating_sub(list_frame.y),
    };

    // cite: frameworks-base View.java#setOnClickListener, performClick
    for (row, (row_node, row_frame)) in visible_rows
        .into_iter()
        .zip(measure_child_frames(&wrapper, content_frame))
    {
        if let Some(click_id) = row.click_id {
            elements.push(row_click_target(click_id as u32, row_frame));
        }
        collect_text_elements(row_node, 0, row_frame, resources, theme, elements)?;
    }

    Ok(())
}

fn dynamic_view_to_view_node(view: &crate::dex_run::DexDynamicView) -> ViewNode {
    // cite: frameworks-base LinearLayout.java#generateDefaultLayoutParams
    let width_dimension = view
        .explicit_width
        .map(|width| width as i32)
        .unwrap_or(WRAP_CONTENT);
    let height_dimension = view
        .explicit_height
        .map(|height| height as i32)
        .unwrap_or(WRAP_CONTENT);
    let mut attributes = vec![
        ViewAttribute {
            name: "layout_width".to_owned(),
            value: width_dimension.to_string(),
            typed_value: XmlValue::Integer(width_dimension),
            reference: None,
        },
        ViewAttribute {
            name: "layout_height".to_owned(),
            value: height_dimension.to_string(),
            typed_value: XmlValue::Integer(height_dimension),
            reference: None,
        },
    ];

    if let Some(text) = &view.text {
        attributes.push(ViewAttribute {
            name: "text".to_owned(),
            value: text.clone(),
            typed_value: XmlValue::String(text.clone()),
            reference: None,
        });
    }
    if let Some(color) = view.background_color {
        attributes.push(ViewAttribute {
            name: "background".to_owned(),
            value: color.to_string(),
            typed_value: XmlValue::Color(color),
            reference: None,
        });
    }
    if let Some(resource_id) = view.drawable_resource_id {
        attributes.push(ViewAttribute {
            name: "src".to_owned(),
            value: String::new(),
            typed_value: XmlValue::Reference(resource_id),
            reference: Some(resource_id),
        });
    }

    let mut node = ViewNode {
        name: view.tag_name.clone(),
        attributes,
        children: view
            .children
            .iter()
            .map(dynamic_view_to_view_node)
            .collect(),
    };
    set_node_visibility(&mut node, view.visibility);

    node
}

fn row_click_target(click_id: u32, frame: LayoutFrame) -> LayoutTextElement {
    LayoutTextElement {
        text: String::new(),
        kind: LayoutTextKind::Label,
        view_id: Some(click_id),
        on_click: None,
        drawable_resource_id: None,
        drawable_padding: 0,
        drawable_tint: None,
        drawable: None,
        text_size: None,
        text_padding_left: 0,
        text_padding_right: 0,
        text_color: None,
        background_color: None,
        background_corner_radius: 0,
        background_border_color: None,
        background_border_width: 0,
        elevation_px: 0,
        alignment: LayoutTextAlignment::Start,
        editable: false,
        depth: 0,
        frame: Some(frame),
        clip: None,
    }
}

fn apply_row_image_intrinsic_sizes(
    node: &mut ViewNode,
    sizes: &BTreeMap<u32, (u32, u32)>,
    resources: Option<&[u8]>,
    config: resource_table::ResourceResolutionConfig,
) -> Result<()> {
    if (node.name.ends_with("ImageView") || node.name.ends_with("ImageButton"))
        && node_intrinsic_image_size(node).is_none()
    {
        if let Some(resource_id) = image_drawable_resource_id(node, resources, config)? {
            if let Some(&(width, height)) = sizes.get(&resource_id) {
                set_node_attribute(
                    node,
                    INTRINSIC_IMAGE_WIDTH_ATTR,
                    width.to_string(),
                    XmlValue::Integer(width as i32),
                    None,
                );
                set_node_attribute(
                    node,
                    INTRINSIC_IMAGE_HEIGHT_ATTR,
                    height.to_string(),
                    XmlValue::Integer(height as i32),
                    None,
                );
            }
        }
    }

    for child in &mut node.children {
        apply_row_image_intrinsic_sizes(child, sizes, resources, config)?;
    }

    Ok(())
}

fn apply_adapter_row_overrides(node: &mut ViewNode, row: &AdapterRowRender) {
    if let Some(id) = node_id(node) {
        if let Some(text) = row.texts.get(&id) {
            set_node_attribute(
                node,
                "text",
                text.clone(),
                XmlValue::String(text.clone()),
                None,
            );
        }
        if let Some(drawable_resource_id) = row.images.get(&id) {
            set_node_attribute(
                node,
                "src",
                String::new(),
                XmlValue::Reference(*drawable_resource_id),
                Some(*drawable_resource_id),
            );
        }
        if let Some(visibility) = row.visibilities.get(&id) {
            set_node_visibility(node, *visibility);
        }
    }

    for child in &mut node.children {
        apply_adapter_row_overrides(child, row);
    }
}

fn set_node_attribute(
    node: &mut ViewNode,
    name: &str,
    value: String,
    typed_value: XmlValue,
    reference: Option<u32>,
) {
    if let Some(attribute) = node
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        attribute.value = value;
        attribute.typed_value = typed_value;
        attribute.reference = reference;
        return;
    }

    node.attributes.push(ViewAttribute {
        name: name.to_owned(),
        value,
        typed_value,
        reference,
    });
}

pub(crate) fn collect_view_visibilities(node: &ViewNode, visibilities: &mut BTreeMap<u32, i32>) {
    if let (Some(view_id), Some(visibility)) = (node_id(node), node_visibility(node)) {
        visibilities.insert(view_id, visibility);
    }

    for child in &node.children {
        collect_view_visibilities(child, visibilities);
    }
}

pub(crate) fn apply_runtime_visibility(node: &mut ViewNode, visibilities: &BTreeMap<u32, i32>) {
    if let Some(view_id) = node_id(node) {
        if let Some(visibility) = visibilities.get(&view_id) {
            set_node_visibility(node, *visibility);
        }
    }

    for child in &mut node.children {
        apply_runtime_visibility(child, visibilities);
    }
}

// cite: frameworks-base TypedArray.java#getDimensionPixelSize, peekValue
// cite: androidx themes_base.xml#actionBarSize
pub(crate) fn apply_theme_attribute_dimensions(
    node: &mut ViewNode,
    resources: &[u8],
    theme: &LayoutTheme,
    cache: &mut LayoutResolutionCache,
) {
    for attribute in &mut node.attributes {
        let pixels = match attribute.typed_value {
            XmlValue::AttributeReference(attribute_id) => {
                let Some(primitive) =
                    resolve_layout_theme_attribute_cached(resources, theme, cache, attribute_id)
                else {
                    continue;
                };
                resolve_layout_dimension_primitive_cached(
                    resources,
                    theme,
                    cache,
                    attribute_id,
                    primitive,
                )
            }
            XmlValue::Reference(resource_id) => {
                resolve_layout_dimension_resource_inner(resources, Some(theme), cache, resource_id)
            }
            _ => continue,
        };
        let Ok(pixels) = pixels else {
            continue;
        };

        attribute.value = pixels.to_string();
        attribute.typed_value = XmlValue::Integer(pixels);
        attribute.reference = None;
    }

    for child in &mut node.children {
        apply_theme_attribute_dimensions(child, resources, theme, cache);
    }
}

// cite: frameworks-base TextView.java#setText
pub(crate) fn apply_runtime_texts(node: &mut ViewNode, texts: &BTreeMap<u32, String>) {
    if let Some(view_id) = node_id(node) {
        if let Some(text) = texts.get(&view_id) {
            if !text.is_empty() || attribute_value(node, "text").is_some() {
                set_node_text(node, text);
            }
        }
    }

    for child in &mut node.children {
        apply_runtime_texts(child, texts);
    }
}

pub(crate) fn set_node_text(node: &mut ViewNode, text: &str) {
    if let Some(attribute) = node
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name == "text")
    {
        attribute.value = text.to_owned();
        attribute.typed_value = XmlValue::String(text.to_owned());
        attribute.reference = None;
        return;
    }

    node.attributes.push(ViewAttribute {
        name: "text".to_owned(),
        value: text.to_owned(),
        typed_value: XmlValue::String(text.to_owned()),
        reference: None,
    });
}

pub(crate) fn set_node_visibility(node: &mut ViewNode, visibility: i32) {
    if let Some(attribute) = node
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name == "visibility")
    {
        attribute.value = visibility.to_string();
        attribute.typed_value = XmlValue::Integer(visibility);
        attribute.reference = None;
        return;
    }

    node.attributes.push(ViewAttribute {
        name: "visibility".to_owned(),
        value: visibility.to_string(),
        typed_value: XmlValue::Integer(visibility),
        reference: None,
    });
}

use super::*;

#[derive(Clone)]
pub(crate) struct ViewNode {
    pub(crate) name: String,
    pub(crate) attributes: Vec<ViewAttribute>,
    pub(crate) children: Vec<ViewNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ViewAttribute {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) typed_value: XmlValue,
    pub(crate) reference: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeRenderContent {
    pub(crate) text: String,
    pub(crate) drawable_resource_id: Option<u32>,
    pub(crate) drawable_padding: u32,
    pub(crate) drawable_tint: Option<LayoutDrawableTint>,
    pub(crate) text_size: Option<u32>,
    pub(crate) text_padding_left: u32,
    pub(crate) text_padding_right: u32,
    pub(crate) text_color: Option<u32>,
    pub(crate) background_color: Option<u32>,
    pub(crate) background_corner_radius: u32,
    pub(crate) background_border_color: Option<u32>,
    pub(crate) background_border_width: u32,
    pub(crate) elevation_px: u32,
}

pub(crate) fn load_layout_tree(
    apk: &mut ApkArchive,
    resources: &[u8],
    resource: &ResourceFile,
    theme: &LayoutTheme,
    depth: usize,
    active_resources: &mut BTreeSet<u32>,
) -> Result<ViewNode> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(WinedroidError::new(format!(
            "layout include depth exceeded {MAX_INCLUDE_DEPTH}"
        )));
    }

    if !active_resources.insert(resource.resource_id) {
        return Err(WinedroidError::new(format!(
            "layout include cycle reached resource 0x{:08x}",
            resource.resource_id
        )));
    }

    let events = read_layout_events(apk, resource)?;
    let tree =
        build_view_tree_for_config(&events, resources, theme.config).ok_or_else(|| {
            WinedroidError::new(format!(
                "layout resource 0x{:08x} at `{}` has no root element",
                resource.resource_id, resource.path
            ))
        })??;
    let mut expanded = expand_includes(apk, resources, tree, theme, depth, active_resources)?;
    attach_image_intrinsic_sizes(apk, resources, theme, &mut expanded)?;
    resolve_shape_drawable_backgrounds(apk, resources, theme, &mut expanded)?;

    active_resources.remove(&resource.resource_id);
    Ok(expanded)
}

pub(crate) fn build_view_tree_for_config(
    events: &[XmlEvent],
    resources: &[u8],
    config: resource_table::ResourceResolutionConfig,
) -> Option<Result<ViewNode>> {
    let mut stack = Vec::new();
    let mut root = None;

    for event in events {
        match event {
            XmlEvent::Start(element) => {
                let attributes = match view_attributes(element, resources, config) {
                    Ok(attributes) => attributes,
                    Err(error) => return Some(Err(error)),
                };

                stack.push(ViewNode {
                    name: element.name.clone(),
                    attributes,
                    children: Vec::new(),
                });
            }
            XmlEvent::End(element) => {
                let Some(node) = stack.pop() else {
                    return Some(Err(WinedroidError::new(format!(
                        "layout XML closed `{}` before any open element",
                        element.name
                    ))));
                };

                if node.name != element.name {
                    return Some(Err(WinedroidError::new(format!(
                        "layout XML closed `{}` while `{}` was open",
                        element.name, node.name
                    ))));
                }

                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                    continue;
                }

                if root.is_some() {
                    return Some(Err(WinedroidError::new(
                        "layout XML contains multiple root elements",
                    )));
                }

                root = Some(node);
            }
        }
    }

    if let Some(open) = stack.last() {
        return Some(Err(WinedroidError::new(format!(
            "layout XML ended before `{}` was closed",
            open.name
        ))));
    }

    root.map(Ok)
}

#[cfg(test)]
pub(crate) fn build_view_tree(events: &[XmlEvent], resources: &[u8]) -> Option<Result<ViewNode>> {
    build_view_tree_for_config(
        events,
        resources,
        resource_table::ResourceResolutionConfig::default(),
    )
}

pub(crate) fn view_attributes(
    element: &XmlStartElement,
    resources: &[u8],
    config: resource_table::ResourceResolutionConfig,
) -> Result<Vec<ViewAttribute>> {
    let mut attributes = Vec::with_capacity(element.attributes.len());

    for attribute in &element.attributes {
        attributes.push(ViewAttribute {
            name: attribute.name.clone(),
            value: render_attribute_value(attribute, resources, config)?,
            typed_value: attribute.value.clone(),
            reference: reference_value(attribute),
        });
    }

    Ok(attributes)
}

pub(crate) fn render_attribute_value(
    attribute: &XmlAttribute,
    resources: &[u8],
    config: resource_table::ResourceResolutionConfig,
) -> Result<String> {
    if attribute.name == "text" {
        if let XmlValue::Reference(resource_id) = attribute.value {
            return resolve_string_text(resources, resource_id, config);
        }
    }

    Ok(attribute.value.as_text())
}

pub(crate) fn resolve_string_text(
    resources: &[u8],
    resource_id: u32,
    config: resource_table::ResourceResolutionConfig,
) -> Result<String> {
    let Some(resource) =
        resource_table::resolve_resource_string_for_config(resources, resource_id, config)?
    else {
        return Err(WinedroidError::new(format!(
            "string resource 0x{resource_id:08x} was not found in resources.arsc"
        )));
    };

    if resource.type_name != "string" {
        return Err(WinedroidError::new(format!(
            "resource 0x{resource_id:08x} resolved to type `{}`, expected `string`",
            resource.type_name
        )));
    }

    Ok(resource.value)
}

pub(crate) fn reference_value(attribute: &XmlAttribute) -> Option<u32> {
    match attribute.value {
        XmlValue::Reference(value) => Some(value),
        _ => None,
    }
}

pub(crate) fn expand_includes(
    apk: &mut ApkArchive,
    resources: &[u8],
    mut node: ViewNode,
    theme: &LayoutTheme,
    depth: usize,
    active_resources: &mut BTreeSet<u32>,
) -> Result<ViewNode> {
    if node.name == "include" {
        let layout_id = include_layout_reference(&node)?;
        let resource = resolve_layout_resource(resources, layout_id, theme.config)?;
        return load_layout_tree(
            apk,
            resources,
            &resource,
            theme,
            depth + 1,
            active_resources,
        );
    }

    let mut expanded_children = Vec::with_capacity(node.children.len());

    for child in node.children {
        expanded_children.push(expand_includes(
            apk,
            resources,
            child,
            theme,
            depth,
            active_resources,
        )?);
    }

    node.children = expanded_children;
    Ok(node)
}

pub(crate) fn include_layout_reference(node: &ViewNode) -> Result<u32> {
    for attribute in &node.attributes {
        if attribute.name == "layout" {
            let Some(reference) = attribute.reference else {
                return Err(WinedroidError::new(
                    "<include> layout attribute is not a resource reference",
                ));
            };

            return Ok(reference);
        }
    }

    Err(WinedroidError::new(
        "<include> element is missing required layout attribute",
    ))
}

pub(crate) fn format_layout_summary(resource: &ResourceFile, events: &[XmlEvent]) -> String {
    let root = first_start_element_name(events).unwrap_or("empty");
    let names = start_element_names(events, 8);

    format!(
        "layout 0x{:08x} {} -> {} root={} elements={}",
        resource.resource_id,
        resource.key_name,
        resource.path,
        root,
        names.join(" > ")
    )
}

pub(crate) fn render_view_tree_lines(root: &ViewNode, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    render_view_tree_node(root, 0, limit, &mut lines);

    if lines.len() == limit {
        lines.push("  ...".to_owned());
    }

    lines
}

pub(crate) fn render_view_tree_node(
    node: &ViewNode,
    depth: usize,
    limit: usize,
    lines: &mut Vec<String>,
) {
    if lines.len() >= limit {
        return;
    }

    let indent = "  ".repeat(depth);
    let attributes = render_node_attributes(node);

    if attributes.is_empty() {
        lines.push(format!("{indent}view {}", node.name));
    } else {
        lines.push(format!("{indent}view {} {}", node.name, attributes));
    }

    for child in &node.children {
        render_view_tree_node(child, depth + 1, limit, lines);
    }
}

pub(crate) fn render_node_attributes(node: &ViewNode) -> String {
    let mut parts = Vec::new();

    for name in ["id", "text", "layout", "src", "visibility", "class"] {
        if let Some(attribute) = node
            .attributes
            .iter()
            .find(|attribute| attribute.name == name)
        {
            parts.push(format!("{name}={}", attribute.value));
        }
    }

    parts.join(" ")
}

// cite: frameworks-base GradientDrawable.java#updateGradientDrawableSolid, updateGradientDrawableStroke, setCornerRadii
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ShapeDrawableStyle {
    pub(crate) solid_color: Option<u32>,
    pub(crate) corner_radius_px: i32,
    pub(crate) stroke_color: Option<u32>,
    pub(crate) stroke_width_px: i32,
}

// cite: frameworks-base View.java#draw, drawBackground
fn resolve_shape_drawable_backgrounds(
    apk: &mut ApkArchive,
    resources: &[u8],
    theme: &LayoutTheme,
    node: &mut ViewNode,
) -> Result<()> {
    if let Some(attribute) = node
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name == "background")
    {
        if let Some(resource_id) = attribute.reference {
            if let Some(style) = shape_drawable_style(apk, resources, resource_id, theme)? {
                if let Some(color) = style.solid_color {
                    attribute.value = format!("#{color:08x}");
                    attribute.typed_value = XmlValue::Color(color);
                    attribute.reference = None;
                }
                if style.corner_radius_px > 0 {
                    node.attributes.push(ViewAttribute {
                        name: "winedroidBackgroundCornerRadius".to_owned(),
                        value: style.corner_radius_px.to_string(),
                        typed_value: XmlValue::Integer(style.corner_radius_px),
                        reference: None,
                    });
                }
                if let Some(stroke_color) = style.stroke_color {
                    node.attributes.push(ViewAttribute {
                        name: "winedroidBackgroundBorderColor".to_owned(),
                        value: format!("#{stroke_color:08x}"),
                        typed_value: XmlValue::Color(stroke_color),
                        reference: None,
                    });
                    node.attributes.push(ViewAttribute {
                        name: "winedroidBackgroundBorderWidth".to_owned(),
                        value: style.stroke_width_px.to_string(),
                        typed_value: XmlValue::Integer(style.stroke_width_px),
                        reference: None,
                    });
                }
            }
        }
    }

    for child in &mut node.children {
        resolve_shape_drawable_backgrounds(apk, resources, theme, child)?;
    }

    Ok(())
}

fn shape_drawable_style(
    apk: &mut ApkArchive,
    resources: &[u8],
    resource_id: u32,
    theme: &LayoutTheme,
) -> Result<Option<ShapeDrawableStyle>> {
    // cite: frameworks-base ResourceTypes.cpp#ResTable::getResource
    let Some(resource) = resource_table::resolve_resource_value_references_for_config(
        resources,
        resource_id,
        theme.config,
    )?
    else {
        // cite: frameworks-base ResourceTypes.cpp#ResTable::getResource
        if crate::android_framework::is_framework_package_resource_id(resource_id)
            && crate::android_framework::color_resource(resource_id).is_some()
        {
            return Ok(None);
        }
        return Err(WinedroidError::new(format!(
            "shape background resource 0x{resource_id:08x} was not found"
        )));
    };
    if !matches!(resource.type_name.as_str(), "drawable" | "mipmap")
        || !matches!(resource.value, resource_table::ResourcePrimitive::String(_))
    {
        return Ok(None);
    }

    let drawable = load_drawable_resource_from_apk(apk, resources, resource_id, theme)?;
    if !drawable.path.ends_with(".xml") {
        return Ok(None);
    }

    let events = crate::binary_xml::parse_binary_xml(&drawable.data).map_err(|error| {
        WinedroidError::with_source(
            format!(
                "failed to parse shape background resource 0x{resource_id:08x} at `{}`",
                drawable.path
            ),
            error,
        )
    })?;

    shape_drawable_style_from_events(resources, theme, &events)
}

fn shape_drawable_style_from_events(
    resources: &[u8],
    theme: &LayoutTheme,
    events: &[XmlEvent],
) -> Result<Option<ShapeDrawableStyle>> {
    let mut root_is_shape = false;
    let mut style = ShapeDrawableStyle::default();
    let mut cache = LayoutResolutionCache::default();
    for event in events {
        match event {
            XmlEvent::Start(element) if element.name == "shape" => {
                root_is_shape = true;
            }
            XmlEvent::Start(element) if element.name == "solid" && root_is_shape => {
                if let Some(attribute) = element.attribute("color") {
                    style.solid_color = resolve_xml_color_value_cached(
                        resources,
                        Some(theme),
                        &mut cache,
                        &attribute.value,
                    )?;
                }
            }
            XmlEvent::Start(element) if element.name == "corners" && root_is_shape => {
                if let Some(attribute) = element.attribute("radius") {
                    style.corner_radius_px = resolve_xml_shape_dimension_px(
                        resources,
                        theme,
                        &mut cache,
                        &attribute.value,
                    )?;
                }
            }
            XmlEvent::Start(element) if element.name == "stroke" && root_is_shape => {
                if let Some(attribute) = element.attribute("width") {
                    style.stroke_width_px = resolve_xml_shape_dimension_px(
                        resources,
                        theme,
                        &mut cache,
                        &attribute.value,
                    )?;
                }
                if let Some(attribute) = element.attribute("color") {
                    style.stroke_color = resolve_xml_color_value_cached(
                        resources,
                        Some(theme),
                        &mut cache,
                        &attribute.value,
                    )?;
                }
            }
            XmlEvent::Start(_) if !root_is_shape => return Ok(None),
            _ => {}
        }
    }

    Ok(Some(style))
}

#[cfg(test)]
pub(crate) fn test_shape_drawable_style(
    apk: &mut ApkArchive,
    resources: &[u8],
    resource_id: u32,
    theme: &LayoutTheme,
) -> Result<Option<ShapeDrawableStyle>> {
    shape_drawable_style(apk, resources, resource_id, theme)
}

#[cfg(test)]
pub(crate) fn test_shape_drawable_style_from_events(
    resources: &[u8],
    theme: &LayoutTheme,
    events: &[XmlEvent],
) -> Result<Option<ShapeDrawableStyle>> {
    shape_drawable_style_from_events(resources, theme, events)
}

fn resolve_xml_shape_dimension_px(
    resources: &[u8],
    theme: &LayoutTheme,
    cache: &mut LayoutResolutionCache,
    value: &XmlValue,
) -> Result<i32> {
    match value {
        XmlValue::Dimension(value) => Ok(complex_dimension_to_px(*value)),
        XmlValue::Integer(value) => Ok(*value),
        XmlValue::Reference(resource_id) => {
            resolve_layout_dimension_resource_inner(resources, Some(theme), cache, *resource_id)
        }
        XmlValue::AttributeReference(attribute_id) => {
            resolve_layout_theme_dimension_attribute_cached(
                resources,
                Some(theme),
                cache,
                *attribute_id,
            )
        }
        value => Err(WinedroidError::new(format!(
            "Android shape dimension has unsupported value `{value}`"
        ))),
    }
}

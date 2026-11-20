use super::*;

pub(crate) fn node_text(node: &ViewNode) -> Option<String> {
    let text = attribute_value(node, "text")?;

    Some(text.to_owned())
}

pub(crate) fn node_render_content(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
) -> Result<Option<NodeRenderContent>> {
    let mut cache = LayoutResolutionCache::default();
    node_render_content_cached(node, resources, theme, &mut cache)
}

pub(crate) fn node_render_content_cached(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
) -> Result<Option<NodeRenderContent>> {
    // cite: frameworks-base View.java#draw, drawBackground
    let background_corner_radius =
        dimension_attribute(node, "winedroidBackgroundCornerRadius").unwrap_or(0);
    let background_border_color = node_color_attribute_cached(
        node,
        resources,
        theme,
        cache,
        &["winedroidBackgroundBorderColor"],
        DrawableBackgroundTintHandling::SkipWhenDrawableBacked,
    )?;
    let background_border_width =
        dimension_attribute(node, "winedroidBackgroundBorderWidth").unwrap_or(0);
    // cite: frameworks-base TextView.java#readTextAppearance, setRawTextSize
    let text_size = dimension_attribute(node, "textSize");
    // cite: frameworks-base TextView.java#getCompoundPaddingLeft, getCompoundPaddingRight
    let text_padding = padding_edges(node);

    if let Some(text) = node_text(node) {
        return Ok(Some(NodeRenderContent {
            text,
            drawable_resource_id: compound_drawable_resource_id(node),
            drawable_padding: dimension_attribute(node, "drawablePadding").unwrap_or(0),
            drawable_tint: drawable_tint(node),
            text_size,
            text_padding_left: text_padding.left,
            text_padding_right: text_padding.right,
            text_color: node_text_color_cached(node, resources, theme, cache)?,
            background_color: node_background_color_cached(node, resources, theme, cache)?,
            background_corner_radius,
            background_border_color,
            background_border_width,
            elevation_px: 0,
        }));
    }

    if is_text_view_node(node) && node_id(node).is_some() {
        return Ok(Some(NodeRenderContent {
            text: String::new(),
            drawable_resource_id: compound_drawable_resource_id(node),
            drawable_padding: dimension_attribute(node, "drawablePadding").unwrap_or(0),
            drawable_tint: drawable_tint(node),
            text_size,
            text_padding_left: text_padding.left,
            text_padding_right: text_padding.right,
            text_color: node_text_color_cached(node, resources, theme, cache)?,
            background_color: node_background_color_cached(node, resources, theme, cache)?,
            background_corner_radius,
            background_border_color,
            background_border_width,
            elevation_px: 0,
        }));
    }

    if is_renderable_image_button(node) {
        if let Some(drawable_resource_id) =
            image_drawable_resource_id(node, resources, content_resource_config(theme))?
        {
            return Ok(Some(NodeRenderContent {
                text: String::new(),
                drawable_resource_id: Some(drawable_resource_id),
                drawable_padding: 0,
                drawable_tint: drawable_tint(node),
                text_size: None,
                text_padding_left: text_padding.left,
                text_padding_right: text_padding.right,
                text_color: node_text_color_cached(node, resources, theme, cache)?,
                background_color: node_background_color_cached(node, resources, theme, cache)?,
                background_corner_radius,
                background_border_color,
                background_border_width,
                elevation_px: 0,
            }));
        }

        return Ok(Some(NodeRenderContent {
            text: image_button_placeholder(node).to_owned(),
            drawable_resource_id: None,
            drawable_padding: 0,
            drawable_tint: None,
            text_size: None,
            text_padding_left: text_padding.left,
            text_padding_right: text_padding.right,
            text_color: node_text_color_cached(node, resources, theme, cache)?,
            background_color: node_background_color_cached(node, resources, theme, cache)?,
            background_corner_radius,
            background_border_color,
            background_border_width,
            elevation_px: 0,
        }));
    }

    if is_renderable_image_view(node) {
        return Ok(Some(NodeRenderContent {
            text: String::new(),
            drawable_resource_id: image_drawable_resource_id(
                node,
                resources,
                content_resource_config(theme),
            )?,
            drawable_padding: 0,
            drawable_tint: drawable_tint(node),
            text_size: None,
            text_padding_left: text_padding.left,
            text_padding_right: text_padding.right,
            text_color: node_text_color_cached(node, resources, theme, cache)?,
            background_color: node_background_color_cached(node, resources, theme, cache)?,
            background_corner_radius,
            background_border_color,
            background_border_width,
            elevation_px: 0,
        }));
    }

    if let Some(background_color) =
        node_container_background_color_cached(node, resources, theme, cache)?
    {
        return Ok(Some(NodeRenderContent {
            text: String::new(),
            drawable_resource_id: None,
            drawable_padding: 0,
            drawable_tint: None,
            text_size: None,
            text_padding_left: text_padding.left,
            text_padding_right: text_padding.right,
            text_color: None,
            background_color: Some(background_color),
            background_corner_radius,
            background_border_color,
            background_border_width,
            // cite: frameworks-base View.java#setElevation, View_elevation
            elevation_px: dimension_attribute(node, "elevation").unwrap_or(0),
        }));
    }

    Ok(None)
}

pub(crate) fn text_element_kind(node: &ViewNode) -> LayoutTextKind {
    if is_button_node(node) {
        return LayoutTextKind::Button;
    }

    LayoutTextKind::Label
}

pub(crate) fn node_on_click(node: &ViewNode) -> Option<String> {
    attribute_value(node, "onClick").map(str::to_owned)
}

pub(crate) fn is_button_node(node: &ViewNode) -> bool {
    node.name.ends_with("Button")
}

fn is_color_backed_button_node(node: &ViewNode) -> bool {
    is_button_node(node) && !node.name.ends_with("ImageButton")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DrawableBackgroundTintHandling {
    PaintAsBackground,
    SkipWhenDrawableBacked,
}

pub(crate) fn is_edit_text_node(node: &ViewNode) -> bool {
    node.name.ends_with("EditText")
}

pub(crate) fn is_text_view_node(node: &ViewNode) -> bool {
    if node.name.ends_with("ImageButton") {
        return false;
    }

    node.name.ends_with("TextView")
        || node.name.ends_with("EditText")
        || is_button_node(node)
        || node.name.ends_with("CheckBox")
        || node.name.ends_with("RadioButton")
        || node.name.ends_with("Switch")
        || node.name.ends_with("CheckedTextView")
}

pub(crate) fn is_renderable_image_button(node: &ViewNode) -> bool {
    node.name.ends_with("ImageButton")
        && (node_id(node).is_some()
            || node_on_click(node).is_some()
            || attribute_value(node, "src").is_some()
            || attribute_value(node, "srcCompat").is_some())
}

pub(crate) fn is_renderable_image_view(node: &ViewNode) -> bool {
    node.name.ends_with("ImageView")
        && (node_id(node).is_some()
            || attribute_value(node, "src").is_some()
            || attribute_value(node, "srcCompat").is_some())
}

pub(crate) fn is_renderable_image(node: &ViewNode) -> bool {
    is_renderable_image_button(node) || is_renderable_image_view(node)
}

pub(crate) fn image_button_placeholder(_node: &ViewNode) -> &'static str {
    "..."
}

fn content_resource_config(
    theme: Option<&LayoutTheme>,
) -> resource_table::ResourceResolutionConfig {
    theme.map(|theme| theme.config).unwrap_or_default()
}

pub(crate) fn image_resource_id_without_type_check(node: &ViewNode) -> Option<u32> {
    reference_attribute(node, "srcCompat")
        .or_else(|| reference_attribute(node, "src"))
        .or_else(|| reference_attribute(node, "background"))
}

pub(crate) fn image_drawable_resource_id(
    node: &ViewNode,
    resources: Option<&[u8]>,
    config: resource_table::ResourceResolutionConfig,
) -> Result<Option<u32>> {
    if let Some(resource_id) =
        reference_attribute(node, "srcCompat").or_else(|| reference_attribute(node, "src"))
    {
        return Ok(Some(resource_id));
    }

    background_drawable_resource_id(node, resources, config)
}

pub(crate) fn background_drawable_resource_id(
    node: &ViewNode,
    resources: Option<&[u8]>,
    config: resource_table::ResourceResolutionConfig,
) -> Result<Option<u32>> {
    let Some(resource_id) = reference_attribute(node, "background") else {
        return Ok(None);
    };
    let Some(resources) = resources else {
        return Ok(Some(resource_id));
    };

    if is_drawable_file_resource(resources, resource_id, config)? {
        return Ok(Some(resource_id));
    }

    Ok(None)
}

pub(crate) fn is_drawable_file_resource(
    resources: &[u8],
    resource_id: u32,
    config: resource_table::ResourceResolutionConfig,
) -> Result<bool> {
    // cite: frameworks-base ResourceTypes.cpp#ResTable::getResource
    let Some(resource) = resource_table::resolve_resource_value_references_for_config(
        resources,
        resource_id,
        config,
    )?
    else {
        return Ok(false);
    };

    Ok(matches!(resource.type_name.as_str(), "drawable" | "mipmap")
        && matches!(resource.value, resource_table::ResourcePrimitive::String(_)))
}

pub(crate) fn compound_drawable_resource_id(node: &ViewNode) -> Option<u32> {
    reference_attribute(node, "drawableStartCompat")
        .or_else(|| reference_attribute(node, "drawableLeftCompat"))
        .or_else(|| reference_attribute(node, "drawableStart"))
        .or_else(|| reference_attribute(node, "drawableLeft"))
}

pub(crate) fn drawable_tint(node: &ViewNode) -> Option<LayoutDrawableTint> {
    tint_attribute(node, "tint")
        .or_else(|| tint_attribute(node, "imageTint"))
        .or_else(|| tint_attribute(node, "color"))
}

pub(crate) fn tint_attribute(node: &ViewNode, name: &str) -> Option<LayoutDrawableTint> {
    let attribute = attribute(node, name)?;

    match attribute.typed_value {
        XmlValue::Color(value) | XmlValue::Hex(value) => Some(LayoutDrawableTint::Color(value)),
        XmlValue::Integer(value) => Some(LayoutDrawableTint::Color(value as u32)),
        XmlValue::Reference(resource_id) => Some(LayoutDrawableTint::Resource(resource_id)),
        XmlValue::AttributeReference(attribute_id) => {
            Some(LayoutDrawableTint::Attribute(attribute_id))
        }
        _ => None,
    }
}

pub(crate) fn node_text_color_cached(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
) -> Result<Option<u32>> {
    node_color_attribute_cached(
        node,
        resources,
        theme,
        cache,
        &["textColor", "color"],
        DrawableBackgroundTintHandling::SkipWhenDrawableBacked,
    )
}

pub(crate) fn node_background_color_cached(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
) -> Result<Option<u32>> {
    if is_color_backed_button_node(node) {
        return node_color_attribute_cached(
            node,
            resources,
            theme,
            cache,
            &["backgroundTint", "background", "color"],
            DrawableBackgroundTintHandling::PaintAsBackground,
        );
    }

    node_color_attribute_cached(
        node,
        resources,
        theme,
        cache,
        &["backgroundTint", "background"],
        DrawableBackgroundTintHandling::SkipWhenDrawableBacked,
    )
}

pub(crate) fn node_container_background_color_cached(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
) -> Result<Option<u32>> {
    node_color_attribute_cached(
        node,
        resources,
        theme,
        cache,
        &["background", "backgroundTint"],
        DrawableBackgroundTintHandling::SkipWhenDrawableBacked,
    )
}

fn node_color_attribute_cached(
    node: &ViewNode,
    resources: Option<&[u8]>,
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
    names: &[&str],
    drawable_background_tint_handling: DrawableBackgroundTintHandling,
) -> Result<Option<u32>> {
    let Some(resources) = resources else {
        return Ok(None);
    };

    let style_resource_id = reference_attribute(node, "style");

    for name in names {
        if let Some(attribute) = attribute(node, name) {
            if *name == "backgroundTint"
                && drawable_background_tint_handling
                    == DrawableBackgroundTintHandling::SkipWhenDrawableBacked
                && node_has_drawable_background_cached(
                    node,
                    resources,
                    theme,
                    cache,
                    style_resource_id,
                )?
            {
                continue;
            }
            if *name == "background"
                && xml_value_is_drawable_reference(resources, theme, cache, &attribute.typed_value)?
            {
                continue;
            }
            if let Some(color) =
                resolve_xml_color_value_cached(resources, theme, cache, &attribute.typed_value)?
            {
                return Ok(Some(color));
            }
        }
    }

    let Some(style_resource_id) = style_resource_id else {
        return Ok(None);
    };

    for name in names {
        if *name == "backgroundTint"
            && drawable_background_tint_handling
                == DrawableBackgroundTintHandling::SkipWhenDrawableBacked
            && style_has_drawable_background(resources, theme, cache, style_resource_id)?
        {
            continue;
        }
        let paint_drawable_background_tint =
            drawable_background_tint_handling == DrawableBackgroundTintHandling::PaintAsBackground;
        if let Some(color) = resolve_style_color_attribute_cached(
            resources,
            theme,
            cache,
            style_resource_id,
            name,
            paint_drawable_background_tint,
        )? {
            return Ok(Some(color));
        }
    }

    Ok(None)
}

fn node_has_drawable_background_cached(
    node: &ViewNode,
    resources: &[u8],
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
    style_resource_id: Option<u32>,
) -> Result<bool> {
    if let Some(background) = attribute(node, "background") {
        if xml_value_is_drawable_reference(resources, theme, cache, &background.typed_value)? {
            return Ok(true);
        }
    }

    let Some(style_resource_id) = style_resource_id else {
        return Ok(false);
    };

    style_has_drawable_background(resources, theme, cache, style_resource_id)
}

fn xml_value_is_drawable_reference(
    resources: &[u8],
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
    value: &XmlValue,
) -> Result<bool> {
    let Some(theme) = theme else {
        return xml_resource_value_is_drawable_reference(resources, theme, cache, value);
    };

    match value {
        XmlValue::AttributeReference(attribute_id) => {
            let Some(primitive) =
                resolve_layout_theme_attribute_cached(resources, theme, cache, *attribute_id)
            else {
                return Ok(false);
            };
            primitive_is_drawable_reference(resources, theme, cache, &primitive)
        }
        _ => xml_resource_value_is_drawable_reference(resources, Some(theme), cache, value),
    }
}

fn xml_resource_value_is_drawable_reference(
    resources: &[u8],
    theme: Option<&LayoutTheme>,
    cache: &mut LayoutResolutionCache,
    value: &XmlValue,
) -> Result<bool> {
    let XmlValue::Reference(resource_id) = value else {
        return Ok(false);
    };

    resource_id_is_drawable_reference(resources, theme, cache, *resource_id)
}

pub(crate) fn layout_text_kind_name(kind: LayoutTextKind) -> &'static str {
    match kind {
        LayoutTextKind::Label => "label",
        LayoutTextKind::Button => "button",
    }
}

pub(crate) fn attribute_value<'a>(node: &'a ViewNode, name: &str) -> Option<&'a str> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

pub(crate) fn node_id(node: &ViewNode) -> Option<u32> {
    reference_attribute(node, "id")
}

pub(crate) fn reference_attribute(node: &ViewNode, name: &str) -> Option<u32> {
    attribute(node, name).and_then(|attribute| attribute.reference)
}

pub(crate) fn bool_attribute(node: &ViewNode, name: &str) -> bool {
    let Some(attribute) = attribute(node, name) else {
        return false;
    };

    match &attribute.typed_value {
        XmlValue::Boolean(value) => *value,
        XmlValue::Integer(value) => *value != 0,
        XmlValue::Hex(value) => *value != 0,
        XmlValue::String(value) if value == "true" => true,
        XmlValue::String(value) if value == "false" => false,
        _ => false,
    }
}

pub(crate) fn int_attribute(node: &ViewNode, name: &str) -> Option<i32> {
    let attribute = attribute(node, name)?;

    match &attribute.typed_value {
        XmlValue::Integer(value) => Some(*value),
        XmlValue::Hex(value) => Some(*value as i32),
        XmlValue::Boolean(value) => Some(i32::from(*value)),
        XmlValue::String(value) => value.parse::<i32>().ok(),
        _ => None,
    }
}

pub(crate) fn float_attribute(node: &ViewNode, name: &str) -> Option<f32> {
    let attribute = attribute(node, name)?;

    match &attribute.typed_value {
        XmlValue::Float(value) => Some(f32::from_bits(*value)),
        XmlValue::Fraction(value) => Some(complex_to_float(*value)),
        XmlValue::Integer(value) => Some(*value as f32),
        XmlValue::Hex(value) => Some(*value as f32),
        XmlValue::String(value) => value.parse::<f32>().ok(),
        _ => None,
    }
}

pub(crate) fn dimension_attribute(node: &ViewNode, name: &str) -> Option<u32> {
    let value = signed_dimension_attribute(node, name)?;

    if value < 0 {
        return None;
    }

    Some(value as u32)
}

pub(crate) fn signed_dimension_attribute(node: &ViewNode, name: &str) -> Option<i32> {
    let attribute = attribute(node, name)?;

    match &attribute.typed_value {
        XmlValue::Integer(value) => Some(*value),
        XmlValue::Hex(value) => Some(*value as i32),
        XmlValue::Dimension(value) => Some(complex_dimension_to_px(*value)),
        XmlValue::String(value) if value == "match_parent" || value == "fill_parent" => {
            Some(MATCH_PARENT)
        }
        XmlValue::String(value) if value == "wrap_content" => Some(WRAP_CONTENT),
        XmlValue::String(value) => value.parse::<i32>().ok(),
        _ => None,
    }
}

pub(crate) fn attribute<'a>(node: &'a ViewNode, name: &str) -> Option<&'a ViewAttribute> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name == name)
}

pub(crate) fn complex_dimension_to_px(value: u32) -> i32 {
    let float_value = complex_to_float(value);
    let unit = value & 0x0f;
    let pixels = match unit {
        0 => float_value,
        1 | 2 => float_value,
        3 => float_value * 160.0 / 72.0,
        4 => float_value * 160.0,
        5 => float_value * 160.0 / 25.4,
        _ => float_value,
    };

    pixels.round() as i32
}

pub(crate) fn complex_to_float(value: u32) -> f32 {
    const RADIX_MULTIPLIERS: [f32; 4] = [
        1.0 / 256.0,
        1.0 / 32_768.0,
        1.0 / 8_388_608.0,
        1.0 / 2_147_483_648.0,
    ];

    let mantissa = (value & 0xffff_ff00) as i32;
    let radix = ((value >> 4) & 0x03) as usize;

    mantissa as f32 * RADIX_MULTIPLIERS[radix]
}

pub(crate) fn first_start_element_name(events: &[XmlEvent]) -> Option<&str> {
    events.iter().find_map(|event| match event {
        XmlEvent::Start(element) => Some(element.name.as_str()),
        XmlEvent::End(_) => None,
    })
}

pub(crate) fn start_element_names(events: &[XmlEvent], limit: usize) -> Vec<String> {
    let mut names = Vec::new();

    for event in events {
        let XmlEvent::Start(element) = event else {
            continue;
        };

        names.push(element.name.clone());

        if names.len() == limit {
            break;
        }
    }

    if names.is_empty() {
        names.push("none".to_owned());
    }

    names
}

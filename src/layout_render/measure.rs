use super::*;

pub(crate) fn measure_child_frames(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    if parent.children.is_empty() {
        return Vec::new();
    }

    if is_constraint_layout(parent) {
        return measure_constraint_layout_children(parent, frame);
    }

    if is_table_layout(parent) {
        return measure_table_layout_children(parent, frame);
    }

    if is_table_row(parent) {
        return measure_table_row_children(parent, frame);
    }

    match linear_layout_orientation(parent) {
        Some(LinearOrientation::Vertical) => measure_vertical_linear_children(parent, frame),
        Some(LinearOrientation::Horizontal) => measure_horizontal_linear_children(parent, frame),
        None => measure_relative_children(parent, frame),
    }
}

pub(crate) fn measure_constraint_layout_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    // cite: androidx ConstraintLayout.java#onViewAdded, applyConstraintsFromLayoutParams, onLayout
    // cite: androidx Guideline.java#Guideline
    let children: Vec<&ViewNode> = parent
        .children
        .iter()
        .filter(|child| is_node_visible(child) || is_constraint_guideline(child))
        .collect();
    let mut frames = vec![content; children.len()];
    let mut id_frames = BTreeMap::new();

    for (index, child) in children.iter().enumerate() {
        if let Some(guideline_frame) = measure_constraint_guideline(child, content) {
            frames[index] = guideline_frame;
            if let Some(id) = node_id(child) {
                id_frames.insert(id, guideline_frame);
            }
        }
    }

    for _ in 0..3 {
        for (index, child) in children.iter().enumerate() {
            let child_frame = measure_constraint_guideline(child, content)
                .unwrap_or_else(|| measure_constraint_child(child, content, &id_frames));

            frames[index] = child_frame;
            if let Some(id) = node_id(child) {
                id_frames.insert(id, child_frame);
            }
        }
    }

    children
        .into_iter()
        .zip(frames)
        .filter(|(child, _)| !is_constraint_guideline(child))
        .collect()
}

pub(crate) fn measure_constraint_guideline(
    node: &ViewNode,
    parent: LayoutFrame,
) -> Option<LayoutFrame> {
    if !is_constraint_guideline(node) {
        return None;
    }

    let horizontal = int_attribute(node, "orientation").unwrap_or(0) == 0;
    let percent = float_attribute(node, "layout_constraintGuide_percent");
    let begin = dimension_attribute(node, "layout_constraintGuide_begin");
    let end = dimension_attribute(node, "layout_constraintGuide_end");

    if horizontal {
        let y = if let Some(percent) = percent {
            parent
                .y
                .saturating_add(((parent.height as f32) * percent).round() as u32)
        } else if let Some(begin) = begin {
            parent.y.saturating_add(begin)
        } else if let Some(end) = end {
            parent.y.saturating_add(parent.height.saturating_sub(end))
        } else {
            parent.y
        };

        return Some(LayoutFrame {
            x: parent.x,
            y,
            width: parent.width,
            height: 0,
        });
    }

    let x = if let Some(percent) = percent {
        parent
            .x
            .saturating_add(((parent.width as f32) * percent).round() as u32)
    } else if let Some(begin) = begin {
        parent.x.saturating_add(begin)
    } else if let Some(end) = end {
        parent.x.saturating_add(parent.width.saturating_sub(end))
    } else {
        parent.x
    };

    Some(LayoutFrame {
        x,
        y: parent.y,
        width: 0,
        height: parent.height,
    })
}

pub(crate) fn measure_constraint_child(
    child: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> LayoutFrame {
    let margins = margin_edges(child);
    let left = constraint_left(child, parent, id_frames, margins);
    let right = constraint_right(child, parent, id_frames, margins);
    let top = constraint_top(child, parent, id_frames, margins);
    let bottom = constraint_bottom(child, parent, id_frames, margins);
    let available_left = left.unwrap_or_else(|| parent.x.saturating_add(margins.left));
    let available_right = right.unwrap_or_else(|| {
        parent
            .x
            .saturating_add(parent.width)
            .saturating_sub(margins.right)
    });
    let available_top = top.unwrap_or_else(|| parent.y.saturating_add(margins.top));
    let available_bottom = bottom.unwrap_or_else(|| {
        parent
            .y
            .saturating_add(parent.height)
            .saturating_sub(margins.bottom)
    });
    let available_width = available_right.saturating_sub(available_left);
    let available_height = available_bottom.saturating_sub(available_top);
    let width =
        constraint_measured_width(child, available_width, left.is_some() && right.is_some());
    let height = constraint_measured_height(
        child,
        available_height,
        width,
        top.is_some() && bottom.is_some(),
    );
    let x = match (left, right) {
        (None, Some(_)) => available_right.saturating_sub(width),
        (Some(_), Some(_)) if width < available_width => {
            available_left.saturating_add((available_width - width) / 2)
        }
        _ => available_left,
    };
    let y = match (top, bottom) {
        (None, Some(_)) => available_bottom.saturating_sub(height),
        (Some(_), Some(_)) if height < available_height => {
            available_top.saturating_add((available_height - height) / 2)
        }
        _ => available_top,
    };

    LayoutFrame {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn constraint_measured_width(
    node: &ViewNode,
    available_width: u32,
    constrained_on_both_sides: bool,
) -> u32 {
    match signed_dimension_attribute(node, "layout_width") {
        Some(0) if constrained_on_both_sides => available_width,
        Some(MATCH_PARENT) | None => available_width,
        Some(WRAP_CONTENT) => wrap_content_width(node, available_width),
        Some(value) if value >= 0 => (value as u32).min(available_width),
        Some(_) => available_width,
    }
}

pub(crate) fn constraint_measured_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
    constrained_on_both_sides: bool,
) -> u32 {
    match signed_dimension_attribute(node, "layout_height") {
        Some(0) if constrained_on_both_sides => available_height,
        Some(MATCH_PARENT) => available_height,
        Some(WRAP_CONTENT) | None => wrap_content_height(node, available_height, measured_width),
        Some(value) if value >= 0 => (value as u32).min(available_height),
        Some(_) => available_height,
    }
}

pub(crate) fn measure_table_layout_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    let visible_children: Vec<&ViewNode> = visible_children(parent).collect();
    let weighted_row_height = weighted_table_row_height(&visible_children, content.height);
    let mut cursor_y = content.y;
    let mut children = Vec::new();

    for child in visible_children {
        let margins = margin_edges(child);
        let available_width = content.width.saturating_sub(margins.horizontal());
        let available_height = content
            .y
            .saturating_add(content.height)
            .saturating_sub(cursor_y)
            .saturating_sub(margins.vertical());
        let height = table_layout_child_height(
            child,
            available_height,
            available_width,
            weighted_row_height,
        );
        let child_frame = LayoutFrame {
            x: content.x.saturating_add(margins.left),
            y: cursor_y.saturating_add(margins.top),
            width: available_width,
            height,
        };

        children.push((child, child_frame));
        cursor_y = child_frame
            .y
            .saturating_add(child_frame.height)
            .saturating_add(margins.bottom);
    }

    children
}

pub(crate) fn weighted_table_row_height(
    children: &[&ViewNode],
    content_height: u32,
) -> Option<u32> {
    if content_height == 0 {
        return None;
    }

    let weighted_rows = children
        .iter()
        .filter(|child| is_table_row(child) && layout_weight(child) > 0.0)
        .count() as u32;

    if weighted_rows == 0 {
        return None;
    }

    let fixed_height = children
        .iter()
        .filter(|child| !is_table_row(child) || layout_weight(child) <= 0.0)
        .map(|child| {
            let margins = margin_edges(child);
            table_layout_child_height(child, content_height, 0, None)
                .saturating_add(margins.vertical())
        })
        .sum::<u32>();
    let remaining_height = content_height.saturating_sub(fixed_height);

    if remaining_height == 0 {
        return None;
    }

    Some((remaining_height / weighted_rows).max(DEFAULT_VIEW_HEIGHT))
}

pub(crate) fn table_layout_child_height(
    child: &ViewNode,
    available_height: u32,
    available_width: u32,
    weighted_row_height: Option<u32>,
) -> u32 {
    if is_table_row(child) {
        if layout_weight(child) > 0.0 {
            if let Some(height) = weighted_row_height {
                return height.min(available_height);
            }
        }

        return match signed_dimension_attribute(child, "layout_height") {
            Some(value) if value > 0 => (value as u32).min(available_height),
            _ => default_table_row_height(available_height),
        };
    }

    nonzero_table_row_height(measured_height(child, available_height, available_width))
}

pub(crate) fn measure_table_row_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    let children: Vec<&ViewNode> = visible_children(parent).collect();

    if children.is_empty() {
        return Vec::new();
    }

    let column_count = children.len() as u32;
    let base_column_width = content.width / column_count;
    let mut cursor_x = content.x;
    let mut measured_children = Vec::with_capacity(children.len());

    for (index, child) in children.iter().enumerate() {
        let margins = margin_edges(child);
        let is_last = index + 1 == children.len();
        let column_width = if is_last {
            content
                .x
                .saturating_add(content.width)
                .saturating_sub(cursor_x)
        } else {
            base_column_width
        };
        let child_frame = LayoutFrame {
            x: cursor_x.saturating_add(margins.left),
            y: content.y.saturating_add(margins.top),
            width: column_width.saturating_sub(margins.horizontal()),
            height: content.height.saturating_sub(margins.vertical()),
        };

        measured_children.push((*child, child_frame));
        cursor_x = cursor_x.saturating_add(column_width);
    }

    measured_children
}

pub(crate) fn nonzero_table_row_height(height: u32) -> u32 {
    if height == 0 {
        return DEFAULT_VIEW_HEIGHT;
    }

    height
}

pub(crate) fn default_table_row_height(available_height: u32) -> u32 {
    if available_height == 0 {
        return DEFAULT_VIEW_HEIGHT;
    }

    DEFAULT_VIEW_HEIGHT.min(available_height)
}

pub(crate) fn measure_vertical_linear_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    let parent_gravity = int_attribute(parent, "gravity").unwrap_or(0);
    let children: Vec<&ViewNode> = visible_children(parent).collect();

    let mut measured: Vec<(u32, u32, Edges, f32)> = Vec::with_capacity(children.len());
    let mut total_used = 0u32;
    let mut total_weight = 0.0f32;
    for child in &children {
        let margins = margin_edges(child);
        let width = measured_width(child, content.width.saturating_sub(margins.horizontal()));
        let weight = layout_weight(child);
        let (natural, share) =
            if signed_dimension_attribute(child, "layout_height") == Some(MATCH_PARENT) {
                (0, if weight > 0.0 { weight } else { 1.0 })
            } else {
                (measured_height(child, content.height, width), weight)
            };
        total_used = total_used
            .saturating_add(natural)
            .saturating_add(margins.vertical());
        total_weight += share;
        measured.push((width, natural, margins, share));
    }
    let remaining = content.height.saturating_sub(total_used);

    let mut cursor_y = content.y;
    let mut result = Vec::with_capacity(children.len());
    for (child, (width, natural, margins, share)) in children.into_iter().zip(measured) {
        let extra = if total_weight > 0.0 && remaining > 0 {
            (remaining as f32 * (share / total_weight)).round() as u32
        } else {
            0
        };
        let height = natural.saturating_add(extra);
        let mut x = content.x.saturating_add(margins.left);
        if should_center_horizontally(child, parent_gravity) {
            x = centered_position(content.x, content.width, width, margins.left, margins.right);
        }

        let y = cursor_y.saturating_add(margins.top);
        result.push((
            child,
            LayoutFrame {
                x,
                y,
                width,
                height,
            },
        ));
        cursor_y = y.saturating_add(height).saturating_add(margins.bottom);
    }

    result
}

pub(crate) fn measure_horizontal_linear_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    let parent_gravity = int_attribute(parent, "gravity").unwrap_or(0);
    let children: Vec<&ViewNode> = visible_children(parent).collect();

    let mut measured: Vec<(u32, Edges, f32)> = Vec::with_capacity(children.len());
    let mut total_used = 0u32;
    let mut total_weight = 0.0f32;
    for child in &children {
        let margins = margin_edges(child);
        let natural = measured_width(child, content.width.saturating_sub(margins.horizontal()));
        let weight = layout_weight(child);
        total_used = total_used
            .saturating_add(natural)
            .saturating_add(margins.horizontal());
        total_weight += weight;
        measured.push((natural, margins, weight));
    }
    let remaining = content.width.saturating_sub(total_used);

    // cite: frameworks-base LinearLayout.java#layoutHorizontal
    let mut cursor_x = content.x;
    if total_weight == 0.0 {
        match parent_gravity & 0x0000_0007 {
            0x0000_0005 => cursor_x = content.x.saturating_add(remaining),
            0x0000_0001 => cursor_x = content.x.saturating_add(remaining / 2),
            _ => {}
        }
    }
    let mut result = Vec::with_capacity(children.len());
    for (child, (natural, margins, weight)) in children.into_iter().zip(measured) {
        let extra = if total_weight > 0.0 && remaining > 0 {
            (remaining as f32 * (weight / total_weight)).round() as u32
        } else {
            0
        };
        let width = natural.saturating_add(extra);
        let available_height = content.height.saturating_sub(margins.vertical());
        let height = measured_height(child, available_height, width);
        let x = cursor_x.saturating_add(margins.left);
        let mut y = content.y.saturating_add(margins.top);
        if should_center_vertically(child, parent_gravity) {
            y = centered_position(
                content.y,
                content.height,
                height,
                margins.top,
                margins.bottom,
            );
        }

        result.push((
            child,
            LayoutFrame {
                x,
                y,
                width,
                height,
            },
        ));
        cursor_x = x.saturating_add(width).saturating_add(margins.right);
    }

    result
}

pub(crate) fn measure_relative_children(
    parent: &ViewNode,
    frame: LayoutFrame,
) -> Vec<(&ViewNode, LayoutFrame)> {
    let padding = padding_edges(parent);
    let content = inset_frame(frame, padding);
    let mut id_frames = BTreeMap::new();
    let mut children = Vec::new();

    let parent_is_horizontal_scroll = parent.name.ends_with("HorizontalScrollView");
    for child in visible_children(parent) {
        let mut child_frame = measure_relative_child(child, content, &id_frames);
        if parent_is_horizontal_scroll && child_frame.width < content.width {
            let gravity = int_attribute(child, "layout_gravity")
                .or_else(|| int_attribute(child, "gravity"))
                .unwrap_or(0);
            if gravity & 0x0000_0007 == 0x0000_0005 {
                let margins = margin_edges(child);
                child_frame.x = content
                    .x
                    .saturating_add(content.width)
                    .saturating_sub(margins.right)
                    .saturating_sub(child_frame.width);
            }
        }

        if let Some(id) = node_id(child) {
            id_frames.insert(id, child_frame);
        }

        children.push((child, child_frame));
    }

    children
}

pub(crate) fn measure_relative_child(
    child: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> LayoutFrame {
    let margins = margin_edges(child);
    let available_width = parent.width.saturating_sub(margins.horizontal());
    let available_height = parent.height.saturating_sub(margins.vertical());

    // cite: frameworks-base RelativeLayout.java#LEFT_OF, RIGHT_OF
    let right_of_anchor = reference_attribute(child, "layout_toRightOf")
        .or_else(|| reference_attribute(child, "layout_toEndOf"))
        .and_then(|id| id_frames.get(&id))
        .map(|frame| {
            frame
                .x
                .saturating_add(frame.width)
                .saturating_add(margins.left)
        });
    let left_of_anchor = reference_attribute(child, "layout_toLeftOf")
        .or_else(|| reference_attribute(child, "layout_toStartOf"))
        .and_then(|id| id_frames.get(&id))
        .map(|frame| frame.x.saturating_sub(margins.right));

    let width = match (right_of_anchor, left_of_anchor) {
        (Some(left), Some(right)) => right.saturating_sub(left).min(available_width),
        _ => measured_width(child, available_width),
    };
    let height = measured_height(child, available_height, width);
    let mut x = parent.x.saturating_add(margins.left);
    let mut y = parent.y.saturating_add(margins.top);

    // cite: frameworks-base FrameLayout.java#layoutChildren
    let layout_gravity = int_attribute(child, "layout_gravity").unwrap_or(0);
    if layout_gravity & 0x0000_0007 == 0x0000_0005 {
        x = parent
            .x
            .saturating_add(parent.width)
            .saturating_sub(margins.right)
            .saturating_sub(width);
    }
    if gravity_centers_horizontally(layout_gravity) {
        x = centered_position(parent.x, parent.width, width, margins.left, margins.right);
    }
    if layout_gravity & 0x0000_0070 == 0x0000_0050 {
        y = parent
            .y
            .saturating_add(parent.height)
            .saturating_sub(margins.bottom)
            .saturating_sub(height);
    }
    if layout_gravity & 0x0000_0070 == 0x0000_0010 {
        y = centered_position(parent.y, parent.height, height, margins.top, margins.bottom);
    }

    if bool_attribute(child, "layout_centerInParent")
        || bool_attribute(child, "layout_centerHorizontal")
    {
        x = centered_position(parent.x, parent.width, width, margins.left, margins.right);
    }

    if bool_attribute(child, "layout_centerInParent")
        || bool_attribute(child, "layout_centerVertical")
    {
        y = centered_position(parent.y, parent.height, height, margins.top, margins.bottom);
    }

    if bool_attribute(child, "layout_alignParentEnd")
        || bool_attribute(child, "layout_alignParentRight")
    {
        x = parent
            .x
            .saturating_add(parent.width)
            .saturating_sub(margins.right)
            .saturating_sub(width);
    }

    if bool_attribute(child, "layout_alignParentBottom") {
        y = parent
            .y
            .saturating_add(parent.height)
            .saturating_sub(margins.bottom)
            .saturating_sub(height);
    }

    // cite: frameworks-base RelativeLayout.java#LEFT_OF, RIGHT_OF
    if let Some(left) = right_of_anchor {
        x = left;
    } else if let Some(right) = left_of_anchor {
        x = right.saturating_sub(width);
    }

    // cite: frameworks-base RelativeLayout.java#ALIGN_LEFT, ALIGN_RIGHT, ALIGN_START, ALIGN_END
    if let Some(reference) = reference_attribute(child, "layout_alignLeft")
        .or_else(|| reference_attribute(child, "layout_alignStart"))
    {
        if let Some(reference_frame) = id_frames.get(&reference) {
            x = reference_frame.x.saturating_add(margins.left);
        }
    }

    if let Some(reference) = reference_attribute(child, "layout_alignRight")
        .or_else(|| reference_attribute(child, "layout_alignEnd"))
    {
        if let Some(reference_frame) = id_frames.get(&reference) {
            x = reference_frame
                .x
                .saturating_add(reference_frame.width)
                .saturating_sub(width)
                .saturating_sub(margins.right);
        }
    }

    // cite: frameworks-base RelativeLayout.java#ALIGN_TOP, ALIGN_BOTTOM
    if let Some(reference) = reference_attribute(child, "layout_alignTop") {
        if let Some(reference_frame) = id_frames.get(&reference) {
            y = reference_frame.y.saturating_add(margins.top);
        }
    }

    if let Some(reference) = reference_attribute(child, "layout_alignBottom") {
        if let Some(reference_frame) = id_frames.get(&reference) {
            y = reference_frame
                .y
                .saturating_add(reference_frame.height)
                .saturating_sub(height)
                .saturating_sub(margins.bottom);
        }
    }

    if let Some(reference) = reference_attribute(child, "layout_below") {
        if let Some(reference_frame) = id_frames.get(&reference) {
            y = reference_frame
                .y
                .saturating_add(reference_frame.height)
                .saturating_add(margins.top);
        }
    }

    if let Some(reference) = reference_attribute(child, "layout_above") {
        if let Some(reference_frame) = id_frames.get(&reference) {
            y = reference_frame
                .y
                .saturating_sub(margins.bottom)
                .saturating_sub(height);
        }
    }

    LayoutFrame {
        x,
        y,
        width,
        height,
    }
}

fn constraint_left(
    node: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
    margins: Edges,
) -> Option<u32> {
    if let Some(reference) = constraint_reference(node, "layout_constraintStart_toStartOf")
        .or_else(|| constraint_reference(node, "layout_constraintLeft_toLeftOf"))
    {
        return constraint_reference_left(reference, parent, id_frames)
            .map(|position| position.saturating_add(margins.left));
    }

    if let Some(reference) = constraint_reference(node, "layout_constraintStart_toEndOf")
        .or_else(|| constraint_reference(node, "layout_constraintLeft_toRightOf"))
    {
        return constraint_reference_right(reference, parent, id_frames)
            .map(|position| position.saturating_add(margins.left));
    }

    None
}

fn constraint_right(
    node: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
    margins: Edges,
) -> Option<u32> {
    if let Some(reference) = constraint_reference(node, "layout_constraintEnd_toEndOf")
        .or_else(|| constraint_reference(node, "layout_constraintRight_toRightOf"))
    {
        return constraint_reference_right(reference, parent, id_frames)
            .map(|position| position.saturating_sub(margins.right));
    }

    if let Some(reference) = constraint_reference(node, "layout_constraintEnd_toStartOf")
        .or_else(|| constraint_reference(node, "layout_constraintRight_toLeftOf"))
    {
        return constraint_reference_left(reference, parent, id_frames)
            .map(|position| position.saturating_sub(margins.right));
    }

    None
}

fn constraint_top(
    node: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
    margins: Edges,
) -> Option<u32> {
    if let Some(reference) = constraint_reference(node, "layout_constraintTop_toTopOf") {
        return constraint_reference_top(reference, parent, id_frames)
            .map(|position| position.saturating_add(margins.top));
    }

    if let Some(reference) = constraint_reference(node, "layout_constraintTop_toBottomOf") {
        return constraint_reference_bottom(reference, parent, id_frames)
            .map(|position| position.saturating_add(margins.top));
    }

    None
}

fn constraint_bottom(
    node: &ViewNode,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
    margins: Edges,
) -> Option<u32> {
    if let Some(reference) = constraint_reference(node, "layout_constraintBottom_toBottomOf") {
        return constraint_reference_bottom(reference, parent, id_frames)
            .map(|position| position.saturating_sub(margins.bottom));
    }

    if let Some(reference) = constraint_reference(node, "layout_constraintBottom_toTopOf") {
        return constraint_reference_top(reference, parent, id_frames)
            .map(|position| position.saturating_sub(margins.bottom));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstraintReference {
    Parent,
    View(u32),
}

fn constraint_reference(node: &ViewNode, name: &str) -> Option<ConstraintReference> {
    let attribute = attribute(node, name)?;

    match attribute.typed_value {
        XmlValue::Reference(0) | XmlValue::Integer(0) | XmlValue::Hex(0) => {
            Some(ConstraintReference::Parent)
        }
        XmlValue::Reference(id) => Some(ConstraintReference::View(id)),
        _ => None,
    }
}

fn constraint_reference_left(
    reference: ConstraintReference,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> Option<u32> {
    match reference {
        ConstraintReference::Parent => Some(parent.x),
        ConstraintReference::View(id) => id_frames.get(&id).map(|frame| frame.x),
    }
}

fn constraint_reference_right(
    reference: ConstraintReference,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> Option<u32> {
    match reference {
        ConstraintReference::Parent => Some(parent.x.saturating_add(parent.width)),
        ConstraintReference::View(id) => id_frames
            .get(&id)
            .map(|frame| frame.x.saturating_add(frame.width)),
    }
}

fn constraint_reference_top(
    reference: ConstraintReference,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> Option<u32> {
    match reference {
        ConstraintReference::Parent => Some(parent.y),
        ConstraintReference::View(id) => id_frames.get(&id).map(|frame| frame.y),
    }
}

fn constraint_reference_bottom(
    reference: ConstraintReference,
    parent: LayoutFrame,
    id_frames: &BTreeMap<u32, LayoutFrame>,
) -> Option<u32> {
    match reference {
        ConstraintReference::Parent => Some(parent.y.saturating_add(parent.height)),
        ConstraintReference::View(id) => id_frames
            .get(&id)
            .map(|frame| frame.y.saturating_add(frame.height)),
    }
}

pub(crate) fn visible_children(node: &ViewNode) -> impl Iterator<Item = &ViewNode> {
    node.children.iter().filter(|child| is_node_visible(child))
}

pub(crate) fn layout_weight(node: &ViewNode) -> f32 {
    float_attribute(node, "layout_weight")
        .unwrap_or(0.0)
        .max(0.0)
}

pub(crate) fn is_constraint_layout(node: &ViewNode) -> bool {
    node.name.ends_with("ConstraintLayout")
}

pub(crate) fn is_constraint_guideline(node: &ViewNode) -> bool {
    node.name.ends_with("Guideline")
}

pub(crate) fn is_table_layout(node: &ViewNode) -> bool {
    node.name.ends_with("TableLayout")
}

pub(crate) fn is_table_row(node: &ViewNode) -> bool {
    node.name.ends_with("TableRow")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinearOrientation {
    Horizontal,
    Vertical,
}

fn linear_layout_orientation(node: &ViewNode) -> Option<LinearOrientation> {
    if !node.name.ends_with("LinearLayout") {
        return None;
    }

    match int_attribute(node, "orientation") {
        Some(1) => Some(LinearOrientation::Vertical),
        _ => Some(LinearOrientation::Horizontal),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Edges {
    pub(crate) left: u32,
    pub(crate) top: u32,
    pub(crate) right: u32,
    pub(crate) bottom: u32,
}

impl Edges {
    pub(crate) fn horizontal(self) -> u32 {
        self.left.saturating_add(self.right)
    }

    fn vertical(self) -> u32 {
        self.top.saturating_add(self.bottom)
    }
}

// cite: frameworks-base View.java#View_padding, View_paddingHorizontal, View_paddingVertical, View_paddingStart, View_paddingEnd
pub(crate) fn padding_edges(node: &ViewNode) -> Edges {
    let all = dimension_attribute(node, "padding").unwrap_or(0);
    let horizontal = dimension_attribute(node, "paddingHorizontal").unwrap_or(all);
    let vertical = dimension_attribute(node, "paddingVertical").unwrap_or(all);

    Edges {
        left: dimension_attribute(node, "paddingStart")
            .or_else(|| dimension_attribute(node, "paddingLeft"))
            .unwrap_or(horizontal),
        top: dimension_attribute(node, "paddingTop").unwrap_or(vertical),
        right: dimension_attribute(node, "paddingEnd")
            .or_else(|| dimension_attribute(node, "paddingRight"))
            .unwrap_or(horizontal),
        bottom: dimension_attribute(node, "paddingBottom").unwrap_or(vertical),
    }
}

fn margin_edges(node: &ViewNode) -> Edges {
    let all = dimension_attribute(node, "layout_margin").unwrap_or(0);

    Edges {
        left: dimension_attribute(node, "layout_marginLeft")
            .or_else(|| dimension_attribute(node, "layout_marginStart"))
            .unwrap_or(all),
        top: dimension_attribute(node, "layout_marginTop").unwrap_or(all),
        right: dimension_attribute(node, "layout_marginRight")
            .or_else(|| dimension_attribute(node, "layout_marginEnd"))
            .unwrap_or(all),
        bottom: dimension_attribute(node, "layout_marginBottom").unwrap_or(all),
    }
}

fn inset_frame(frame: LayoutFrame, edges: Edges) -> LayoutFrame {
    let x = frame.x.saturating_add(edges.left);
    let y = frame.y.saturating_add(edges.top);
    let width = frame.width.saturating_sub(edges.horizontal());
    let height = frame.height.saturating_sub(edges.vertical());

    LayoutFrame {
        x,
        y,
        width,
        height,
    }
}

pub(crate) const INTRINSIC_IMAGE_WIDTH_ATTR: &str = "winedroidIntrinsicImageWidth";
pub(crate) const INTRINSIC_IMAGE_HEIGHT_ATTR: &str = "winedroidIntrinsicImageHeight";

pub(crate) fn node_intrinsic_image_size(node: &ViewNode) -> Option<(u32, u32)> {
    let width = attribute_value(node, INTRINSIC_IMAGE_WIDTH_ATTR)?
        .parse()
        .ok()?;
    let height = attribute_value(node, INTRINSIC_IMAGE_HEIGHT_ATTR)?
        .parse()
        .ok()?;
    Some((width, height))
}

pub(crate) fn measured_width(node: &ViewNode, available_width: u32) -> u32 {
    match signed_dimension_attribute(node, "layout_width") {
        Some(MATCH_PARENT) | None => available_width,
        Some(WRAP_CONTENT) => wrap_content_width(node, available_width),
        Some(value) if value >= 0 => (value as u32).min(available_width),
        Some(_) => available_width,
    }
}

pub(crate) fn measured_height(node: &ViewNode, available_height: u32, measured_width: u32) -> u32 {
    match signed_dimension_attribute(node, "layout_height") {
        Some(MATCH_PARENT) => available_height,
        Some(WRAP_CONTENT) | None => wrap_content_height(node, available_height, measured_width),
        Some(value) if value >= 0 => (value as u32).min(available_height),
        Some(_) => available_height,
    }
}

pub(crate) fn wrap_content_width(node: &ViewNode, available_width: u32) -> u32 {
    if is_renderable_image(node) && image_resource_id_without_type_check(node).is_some() {
        if let Some((intrinsic_width, _)) = node_intrinsic_image_size(node) {
            return intrinsic_width.min(available_width);
        }
        return DEFAULT_VIEW_HEIGHT.min(available_width);
    }

    if let Some(content) = node_render_content(node, None, None).ok().flatten() {
        return measured_text_width(node, &content.text).min(available_width);
    }

    available_width
}

pub(crate) fn wrap_content_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
) -> u32 {
    if node_text(node).is_some() {
        return text_height(node).min(available_height);
    }

    if node.name.ends_with("ImageView") {
        if let Some((intrinsic_width, intrinsic_height)) = node_intrinsic_image_size(node) {
            if intrinsic_width > 0 && measured_width < intrinsic_width {
                let scaled = (u64::from(intrinsic_height) * u64::from(measured_width)
                    / u64::from(intrinsic_width)) as u32;
                return scaled.min(available_height);
            }
            return intrinsic_height.min(available_height);
        }
        return DEFAULT_IMAGE_HEIGHT.min(available_height);
    }

    if node.name.ends_with("ProgressBar") {
        return DEFAULT_PROGRESS_HEIGHT.min(available_height);
    }

    match linear_layout_orientation(node) {
        Some(LinearOrientation::Vertical) => {
            return vertical_wrap_content_height(node, available_height, measured_width);
        }
        Some(LinearOrientation::Horizontal) => {
            return horizontal_wrap_content_height(node, available_height, measured_width);
        }
        None => {}
    }

    if is_constraint_layout(node) {
        return constraint_wrap_content_height(node, available_height, measured_width);
    }

    if node.name.ends_with("RelativeLayout") {
        return relative_wrap_content_height(node, available_height, measured_width);
    }

    DEFAULT_VIEW_HEIGHT.min(available_height)
}

// cite: frameworks-base RelativeLayout.java#onMeasure
fn relative_wrap_content_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
) -> u32 {
    let padding = padding_edges(node);
    let frame = LayoutFrame {
        x: 0,
        y: 0,
        width: measured_width,
        height: available_height,
    };
    let children = measure_relative_children(node, frame);
    let max_bottom = children
        .iter()
        .map(|(_, child_frame)| child_frame.y.saturating_add(child_frame.height))
        .max()
        .unwrap_or(0);

    max_bottom
        .saturating_add(padding.bottom)
        .min(available_height)
}

pub(crate) fn constraint_wrap_content_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
) -> u32 {
    let padding = padding_edges(node);
    let provisional = LayoutFrame {
        x: 0,
        y: 0,
        width: measured_width,
        height: available_height,
    };
    let content = inset_frame(provisional, padding);
    let mut id_frames = BTreeMap::new();
    let mut needed = 0u32;

    // cite: androidx ConstraintLayout.java#onViewAdded, applyConstraintsFromLayoutParams, onLayout
    // cite: androidx Guideline.java#Guideline
    for child in node
        .children
        .iter()
        .filter(|child| is_node_visible(child) || is_constraint_guideline(child))
    {
        if is_constraint_guideline(child) {
            if let Some(frame) = measure_constraint_guideline(child, content) {
                if let Some(view_id) = node_id(child) {
                    id_frames.insert(view_id, frame);
                }
            }
            continue;
        }

        let frame = measure_constraint_child(child, content, &id_frames);
        if let Some(view_id) = node_id(child) {
            id_frames.insert(view_id, frame);
        }

        let margins = margin_edges(child);
        let top = constraint_top(child, content, &id_frames, margins);
        let top_is_parent = matches!(
            constraint_reference(child, "layout_constraintTop_toTopOf"),
            Some(ConstraintReference::Parent)
        );
        let bottom_to_parent = matches!(
            constraint_reference(child, "layout_constraintBottom_toBottomOf"),
            Some(ConstraintReference::Parent)
        );
        let extent = match top {
            Some(_) if top_is_parent && bottom_to_parent => {
                frame.height.saturating_add(margins.vertical())
            }
            Some(top_position) => top_position
                .saturating_add(frame.height)
                .saturating_sub(content.y)
                .saturating_add(margins.bottom),
            None => frame.height.saturating_add(margins.vertical()),
        };

        needed = needed.max(extent);
    }

    needed
        .saturating_add(padding.vertical())
        .min(available_height)
}

pub(crate) fn horizontal_wrap_content_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
) -> u32 {
    let padding = padding_edges(node);
    let content_width = measured_width.saturating_sub(padding.horizontal());
    let inner_available = available_height.saturating_sub(padding.vertical());
    let mut content_max = 0;
    let mut image_max = 0;

    for child in visible_children(node) {
        if signed_dimension_attribute(child, "layout_height") == Some(MATCH_PARENT) {
            continue;
        }

        let margins = margin_edges(child);
        let child_width = measured_width_for_wrap_child(child, content_width, margins);
        let child_height =
            measured_height(child, inner_available, child_width).saturating_add(margins.vertical());

        if child.name.ends_with("ImageView") && node_intrinsic_image_size(child).is_none() {
            image_max = image_max.max(child_height);
        } else {
            content_max = content_max.max(child_height);
        }
    }

    let content_height = if content_max > 0 {
        content_max
    } else {
        image_max
    };

    content_height
        .saturating_add(padding.vertical())
        .min(available_height)
}

pub(crate) fn vertical_wrap_content_height(
    node: &ViewNode,
    available_height: u32,
    measured_width: u32,
) -> u32 {
    let padding = padding_edges(node);
    let content_width = measured_width.saturating_sub(padding.horizontal());
    let mut height = padding.vertical();

    for child in visible_children(node) {
        let margins = margin_edges(child);
        let child_width = measured_width_for_wrap_child(child, content_width, margins);
        let child_height = measured_height(child, available_height, child_width);

        height = height
            .saturating_add(margins.top)
            .saturating_add(child_height)
            .saturating_add(margins.bottom);
    }

    height.min(available_height)
}

fn measured_width_for_wrap_child(child: &ViewNode, content_width: u32, margins: Edges) -> u32 {
    measured_width(child, content_width.saturating_sub(margins.horizontal()))
}

pub(crate) fn text_height(node: &ViewNode) -> u32 {
    let line_height = dimension_attribute(node, "textSize")
        .map(|size| size.saturating_add(10))
        .unwrap_or(DEFAULT_TEXT_HEIGHT);

    // cite: frameworks-base StaticLayout.java#generate
    let line_count = node_text(node)
        .map(|text| text.lines().count().max(1) as u32)
        .unwrap_or(1);

    line_height.saturating_mul(line_count)
}

// cite: frameworks-base TextView.java#onMeasure, getCompoundPaddingLeft, getCompoundPaddingRight
// cite: frameworks-base Layout.java#getDesiredWidthWithLimit
pub(crate) fn measured_text_width(node: &ViewNode, text: &str) -> u32 {
    let text_size = dimension_attribute(node, "textSize").unwrap_or(DEFAULT_TEXT_SIZE);
    let padding = padding_edges(node);

    crate::skia_renderer::measured_label_text_width(text, text_size as f32)
        .saturating_add(padding.horizontal())
}

pub(crate) fn centered_position(
    parent_start: u32,
    parent_size: u32,
    child_size: u32,
    leading_margin: u32,
    trailing_margin: u32,
) -> u32 {
    let available_size = parent_size.saturating_sub(leading_margin.saturating_add(trailing_margin));
    let offset = available_size.saturating_sub(child_size) / 2;

    parent_start
        .saturating_add(leading_margin)
        .saturating_add(offset)
}

pub(crate) fn should_center_horizontally(node: &ViewNode, parent_gravity: i32) -> bool {
    gravity_centers_horizontally(parent_gravity)
        || int_attribute(node, "layout_gravity")
            .map(gravity_centers_horizontally)
            .unwrap_or(false)
}

pub(crate) fn node_text_alignment(node: &ViewNode) -> LayoutTextAlignment {
    if let Some(alignment) = int_attribute(node, "textAlignment").and_then(text_alignment_value) {
        return alignment;
    }

    if let Some(gravity) = int_attribute(node, "gravity") {
        return gravity_text_alignment(gravity);
    }

    LayoutTextAlignment::Start
}

pub(crate) fn text_alignment_value(value: i32) -> Option<LayoutTextAlignment> {
    match value {
        2 | 5 => Some(LayoutTextAlignment::Start),
        3 | 6 => Some(LayoutTextAlignment::End),
        4 => Some(LayoutTextAlignment::Center),
        _ => None,
    }
}

pub(crate) fn gravity_text_alignment(value: i32) -> LayoutTextAlignment {
    if gravity_centers_horizontally(value) {
        return LayoutTextAlignment::Center;
    }

    match value & 0x0000_0007 {
        0x0000_0005 => LayoutTextAlignment::End,
        _ => LayoutTextAlignment::Start,
    }
}

pub(crate) fn should_center_vertically(node: &ViewNode, parent_gravity: i32) -> bool {
    gravity_centers_vertically(parent_gravity)
        || int_attribute(node, "layout_gravity")
            .map(gravity_centers_vertically)
            .unwrap_or(false)
}

pub(crate) fn gravity_centers_horizontally(value: i32) -> bool {
    value & 0x0000_0007 == 0x0000_0001
}

pub(crate) fn gravity_centers_vertically(value: i32) -> bool {
    value & 0x0000_0070 == 0x0000_0010
}

pub(crate) fn is_node_visible(node: &ViewNode) -> bool {
    node_visibility(node).unwrap_or(ANDROID_VIEW_VISIBLE) == ANDROID_VIEW_VISIBLE
}

pub(crate) fn node_visibility(node: &ViewNode) -> Option<i32> {
    int_attribute(node, "visibility").map(android_visibility_from_layout_value)
}

pub(crate) fn android_visibility_from_layout_value(value: i32) -> i32 {
    match value {
        0 => ANDROID_VIEW_VISIBLE,
        1 => ANDROID_VIEW_INVISIBLE,
        2 => ANDROID_VIEW_GONE,
        4 => ANDROID_VIEW_INVISIBLE,
        8 => ANDROID_VIEW_GONE,
        other => other,
    }
}

use std::env;

use crate::dex_run;
use crate::layout_render;
use crate::trace;

pub(super) fn view_snapshot_detail_lines(snapshot: &dex_run::DexViewSnapshot) -> Vec<String> {
    vec![
        format!(
            "content layouts: {}",
            display_i32_list(&snapshot.content_layouts)
        ),
        format!(
            "inflated layouts: {}",
            display_i32_list(&snapshot.inflated_layouts)
        ),
        format!(
            "runtime view roots: {}",
            display_runtime_view_roots(&snapshot.runtime_view_roots)
        ),
        format!(
            "runtime canvas frames: {}",
            display_runtime_canvas_frames(&snapshot.runtime_canvas_frames)
        ),
        format!(
            "view lookups: {}",
            display_i32_list(&snapshot.view_lookup_ids)
        ),
        format!("view texts: {}", display_view_texts(&snapshot.view_texts)),
        format!(
            "view images: {}",
            display_view_images(&snapshot.view_images)
        ),
        format!(
            "view visibilities: {}",
            display_view_visibilities(&snapshot.view_visibilities)
        ),
        format!(
            "view text cursors: {}",
            display_view_text_cursors(&snapshot.view_text_cursors)
        ),
        format!(
            "click listeners: {}",
            display_click_listeners(&snapshot.click_listeners)
        ),
        format!(
            "XML onClick handlers: {}",
            display_declared_on_click_handlers(&snapshot.declared_on_click_handlers)
        ),
    ]
}

pub(super) fn print_layout_render_summary(summary: &layout_render::LayoutRenderSummary) {
    if !trace::enabled("ui") {
        return;
    }
    println!("Layout rendering bootstrap");

    for line in &summary.lines {
        println!("  {line}");
    }

    println!();
}

pub(super) fn activity_window_detail_lines(
    snapshot: &dex_run::DexViewSnapshot,
    summary: &layout_render::LayoutRenderSummary,
) -> Vec<String> {
    let mut lines = view_snapshot_detail_lines(snapshot);
    lines.extend(summary.lines.clone());
    lines
}

pub(super) fn trace_runtime_canvas_frames(snapshot: &dex_run::DexViewSnapshot) {
    if !trace::enabled("render") {
        return;
    }

    let mut summary = RuntimeCanvasOpSummary::default();
    for frame in &snapshot.runtime_canvas_frames {
        summary.record_ops(&frame.ops);
    }

    trace::event(
        "render",
        format!(
            "runtime canvas frames={} ops={} save={} restore={} translate={} scale={} concat_matrix={} clip_rect={} clip_path={} rect={} round_rect={} path={} text={} bitmap={} render_node={}",
            snapshot.runtime_canvas_frames.len(),
            summary.total(),
            summary.save,
            summary.restore,
            summary.translate,
            summary.scale,
            summary.concat_matrix,
            summary.clip_rect,
            summary.clip_path,
            summary.rect,
            summary.round_rect,
            summary.path,
            summary.text,
            summary.bitmap,
            summary.render_node
        ),
    );

    for (index, frame) in snapshot.runtime_canvas_frames.iter().enumerate() {
        let mut frame_summary = RuntimeCanvasOpSummary::default();
        frame_summary.record_ops(&frame.ops);
        trace::event(
            "render",
            format!(
                "runtime canvas frame #{index}: {} {}x{} root={} ops={}",
                frame.class_descriptor,
                frame.width,
                frame.height,
                frame.root_view_key,
                frame_summary.total()
            ),
        );
        trace_runtime_canvas_ops(index, &frame.ops);
    }
}

fn trace_runtime_canvas_ops(frame_index: usize, ops: &[dex_run::DexRuntimeCanvasOp]) {
    if !trace::enabled("render.ops") {
        return;
    }

    let mut lines = Vec::new();
    collect_runtime_canvas_op_lines(ops, 0, &mut lines);

    for (index, line) in lines.iter().enumerate() {
        trace::event(
            "render.ops",
            format!("frame #{frame_index} op #{index}: {line}"),
        );
    }
}

fn collect_runtime_canvas_op_lines(
    ops: &[dex_run::DexRuntimeCanvasOp],
    depth: usize,
    lines: &mut Vec<String>,
) {
    for op in ops {
        lines.push(runtime_canvas_op_line(op, depth));

        match op {
            dex_run::DexRuntimeCanvasOp::Bitmap { ops, .. }
            | dex_run::DexRuntimeCanvasOp::RenderNode { ops, .. } => {
                collect_runtime_canvas_op_lines(ops, depth + 1, lines);
            }
            dex_run::DexRuntimeCanvasOp::Save
            | dex_run::DexRuntimeCanvasOp::Restore
            | dex_run::DexRuntimeCanvasOp::Translate { .. }
            | dex_run::DexRuntimeCanvasOp::Scale { .. }
            | dex_run::DexRuntimeCanvasOp::Rotate { .. }
            | dex_run::DexRuntimeCanvasOp::ConcatMatrix { .. }
            | dex_run::DexRuntimeCanvasOp::ClipRect { .. }
            | dex_run::DexRuntimeCanvasOp::ClipPath { .. }
            | dex_run::DexRuntimeCanvasOp::Line { .. }
            | dex_run::DexRuntimeCanvasOp::Rect { .. }
            | dex_run::DexRuntimeCanvasOp::Circle { .. }
            | dex_run::DexRuntimeCanvasOp::RoundRect { .. }
            | dex_run::DexRuntimeCanvasOp::Path { .. }
            | dex_run::DexRuntimeCanvasOp::Text { .. } => {}
        }
    }
}

fn runtime_canvas_op_line(op: &dex_run::DexRuntimeCanvasOp, depth: usize) -> String {
    let indent = "  ".repeat(depth);

    match op {
        dex_run::DexRuntimeCanvasOp::Save => format!("{indent}save"),
        dex_run::DexRuntimeCanvasOp::Restore => format!("{indent}restore"),
        dex_run::DexRuntimeCanvasOp::Translate { dx, dy } => {
            format!("{indent}translate dx={dx:.2} dy={dy:.2}")
        }
        dex_run::DexRuntimeCanvasOp::Scale { sx, sy } => {
            format!("{indent}scale sx={sx:.3} sy={sy:.3}")
        }
        dex_run::DexRuntimeCanvasOp::Rotate { degrees } => {
            format!("{indent}rotate degrees={degrees:.2}")
        }
        dex_run::DexRuntimeCanvasOp::ConcatMatrix { values } => {
            format!("{indent}concat_matrix values={values:?}")
        }
        dex_run::DexRuntimeCanvasOp::ClipRect {
            left,
            top,
            right,
            bottom,
            op,
        } => format!(
            "{indent}clip_rect left={left:.2} top={top:.2} right={right:.2} bottom={bottom:.2} op={op:?}"
        ),
        dex_run::DexRuntimeCanvasOp::ClipPath { ops, op } => {
            format!("{indent}clip_path op={op:?} path_ops={}", ops.len())
        }
        dex_run::DexRuntimeCanvasOp::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            blend_mode,
            stroke_width,
            stroke_cap,
        } => format!(
            "{indent}line x1={x1:.2} y1={y1:.2} x2={x2:.2} y2={y2:.2} color=0x{color:08x} blend={blend_mode:?} stroke_width={stroke_width:.2} cap={stroke_cap:?}"
        ),
        dex_run::DexRuntimeCanvasOp::Rect {
            left,
            top,
            right,
            bottom,
            color,
            blend_mode,
            paint_style,
        } => format!(
            "{indent}rect left={left:.2} top={top:.2} right={right:.2} bottom={bottom:.2} color=0x{color:08x} blend={blend_mode:?} style={paint_style:?}"
        ),
        dex_run::DexRuntimeCanvasOp::Circle {
            cx,
            cy,
            radius,
            color,
            blend_mode,
            paint_style,
            stroke_width,
        } => format!(
            "{indent}circle cx={cx:.2} cy={cy:.2} radius={radius:.2} color=0x{color:08x} blend={blend_mode:?} style={paint_style:?} stroke_width={stroke_width:.2}"
        ),
        dex_run::DexRuntimeCanvasOp::RoundRect {
            left,
            top,
            right,
            bottom,
            rx,
            ry,
            color,
            blend_mode,
            paint_style,
        } => format!(
            "{indent}round_rect left={left:.2} top={top:.2} right={right:.2} bottom={bottom:.2} rx={rx:.2} ry={ry:.2} color=0x{color:08x} blend={blend_mode:?} style={paint_style:?}"
        ),
        dex_run::DexRuntimeCanvasOp::Path {
            ops,
            color,
            blend_mode,
            paint_style,
        } => {
            format!(
                "{indent}path color=0x{color:08x} blend={blend_mode:?} style={paint_style:?} path_ops={}",
                ops.len()
            )
        }
        dex_run::DexRuntimeCanvasOp::Text {
            text,
            x,
            y,
            color,
            text_size,
        } => format!(
            "{indent}text {:?} x={x:.2} y={y:.2} color=0x{color:08x} size={text_size:.2}",
            text
        ),
        dex_run::DexRuntimeCanvasOp::Bitmap {
            left,
            top,
            right,
            bottom,
            source_left,
            source_top,
            source_right,
            source_bottom,
            bitmap_width,
            bitmap_height,
            color,
            blend_mode,
            color_filter,
            alpha_mask,
            encoded_image,
            ops,
        } => format!(
            "{indent}bitmap dst={left:.2},{top:.2},{right:.2},{bottom:.2} src={source_left:.2},{source_top:.2},{source_right:.2},{source_bottom:.2} size={}x{} color=0x{color:08x} blend={blend_mode:?} color_filter={} alpha_mask={alpha_mask} encoded_bytes={} nested_ops={}",
            bitmap_width,
            bitmap_height,
            color_filter
                .map(|filter| format!("0x{:08x}/{:?}", filter.color, filter.blend_mode))
                .unwrap_or_else(|| "none".to_owned()),
            encoded_image.as_ref().map(Vec::len).unwrap_or(0),
            ops.len()
        ),
        dex_run::DexRuntimeCanvasOp::RenderNode {
            left,
            top,
            right,
            bottom,
            alpha,
            scale_x,
            scale_y,
            clip_to_bounds,
            ops,
        } => format!(
            "{indent}render_node bounds={left:.2},{top:.2},{right:.2},{bottom:.2} alpha={alpha:.2} scale={scale_x:.2},{scale_y:.2} clip_to_bounds={clip_to_bounds} nested_ops={}",
            ops.len()
        ),
    }
}

pub(super) fn display_i32_list(values: &[i32]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_runtime_view_roots(values: &[dex_run::DexRuntimeViewRoot]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|root| format!("{} -> {}", root.view_key, root.class_descriptor))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_runtime_canvas_frames(values: &[dex_run::DexRuntimeCanvasFrame]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|frame| {
            format!(
                "{} {}x{} ops:{}{}",
                frame.class_descriptor,
                frame.width,
                frame.height,
                frame.ops.len(),
                runtime_canvas_frame_summary_suffix(&frame.ops)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn runtime_canvas_frame_summary_suffix(ops: &[dex_run::DexRuntimeCanvasOp]) -> String {
    if env::var_os("WINEDROID_RUNTIME_CANVAS_OP_SUMMARY").is_none() {
        return String::new();
    }

    let mut summary = RuntimeCanvasOpSummary::default();
    summary.record_ops(ops);
    format!(
        " save:{} restore:{} translate:{} scale:{} rotate:{} concat_matrix:{} clip_rect:{} clip_path:{} line:{} rect:{} circle:{} round_rect:{} path:{} text:{} bitmap:{} render_node:{}",
        summary.save,
        summary.restore,
        summary.translate,
        summary.scale,
        summary.rotate,
        summary.concat_matrix,
        summary.clip_rect,
        summary.clip_path,
        summary.line,
        summary.rect,
        summary.circle,
        summary.round_rect,
        summary.path,
        summary.text,
        summary.bitmap,
        summary.render_node
    )
}

#[derive(Debug, Default)]
pub(super) struct RuntimeCanvasOpSummary {
    save: usize,
    restore: usize,
    translate: usize,
    scale: usize,
    rotate: usize,
    concat_matrix: usize,
    clip_rect: usize,
    clip_path: usize,
    line: usize,
    rect: usize,
    circle: usize,
    round_rect: usize,
    path: usize,
    text: usize,
    bitmap: usize,
    render_node: usize,
}

impl RuntimeCanvasOpSummary {
    fn total(&self) -> usize {
        self.save
            + self.restore
            + self.translate
            + self.scale
            + self.rotate
            + self.concat_matrix
            + self.clip_rect
            + self.clip_path
            + self.line
            + self.rect
            + self.circle
            + self.round_rect
            + self.path
            + self.text
            + self.bitmap
            + self.render_node
    }

    fn record_ops(&mut self, ops: &[dex_run::DexRuntimeCanvasOp]) {
        for op in ops {
            self.record_op(op);
        }
    }

    fn record_op(&mut self, op: &dex_run::DexRuntimeCanvasOp) {
        match op {
            dex_run::DexRuntimeCanvasOp::Save => self.save += 1,
            dex_run::DexRuntimeCanvasOp::Restore => self.restore += 1,
            dex_run::DexRuntimeCanvasOp::Translate { .. } => self.translate += 1,
            dex_run::DexRuntimeCanvasOp::Scale { .. } => self.scale += 1,
            dex_run::DexRuntimeCanvasOp::Rotate { .. } => self.rotate += 1,
            dex_run::DexRuntimeCanvasOp::ConcatMatrix { .. } => self.concat_matrix += 1,
            dex_run::DexRuntimeCanvasOp::ClipRect { .. } => self.clip_rect += 1,
            dex_run::DexRuntimeCanvasOp::ClipPath { .. } => self.clip_path += 1,
            dex_run::DexRuntimeCanvasOp::Line { .. } => self.line += 1,
            dex_run::DexRuntimeCanvasOp::Rect { .. } => self.rect += 1,
            dex_run::DexRuntimeCanvasOp::Circle { .. } => self.circle += 1,
            dex_run::DexRuntimeCanvasOp::RoundRect { .. } => self.round_rect += 1,
            dex_run::DexRuntimeCanvasOp::Path { .. } => self.path += 1,
            dex_run::DexRuntimeCanvasOp::Text { .. } => self.text += 1,
            dex_run::DexRuntimeCanvasOp::Bitmap { ops, .. } => {
                self.bitmap += 1;
                self.record_ops(ops);
            }
            dex_run::DexRuntimeCanvasOp::RenderNode { ops, .. } => {
                self.render_node += 1;
                self.record_ops(ops);
            }
        }
    }
}

pub(super) fn display_click_listeners(values: &[dex_run::DexViewClickListener]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|listener| format!("{} -> {}", listener.view_id, listener.listener))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_view_texts(values: &[dex_run::DexViewText]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|text| format!("{} -> {:?}", text.view_id, text.text))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_view_images(values: &[dex_run::DexViewImage]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|image| format!("{} -> 0x{:08x}", image.view_id, image.drawable_resource_id))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_view_text_cursors(values: &[dex_run::DexViewTextCursor]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|cursor| {
            format!(
                "{} -> {}..{} visible={}",
                cursor.view_id, cursor.selection_start, cursor.selection_end, cursor.cursor_visible
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_view_visibilities(values: &[dex_run::DexViewVisibility]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|visibility| format!("{} -> {}", visibility.view_id, visibility.visibility))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn display_declared_on_click_handlers(
    values: &[dex_run::DexDeclaredOnClickHandler],
) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }

    values
        .iter()
        .map(|handler| format!("{} -> {}", handler.view_id, handler.method_name))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn print_session_header(
    package: &str,
    version_name: Option<&str>,
    version_code: Option<&str>,
    native_plan: &crate::native_runtime::NativeRuntimePlan,
) {
    let started = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!("{} (Unix milliseconds, UTC)", duration.as_millis()),
        Err(_) => "unavailable (clock precedes Unix epoch)".to_owned(),
    };
    let mode = if crate::missing::strict_enabled() {
        "strict"
    } else {
        "best-effort"
    };
    println!(
        "winedroid {} | {} | {mode}",
        env!("CARGO_PKG_VERSION"),
        native_plan.host
    );
    println!("  app: {package}");
    println!(
        "  version: {:?} (code {:?})",
        version_name.unwrap_or("unknown"),
        version_code.unwrap_or("unknown")
    );
    println!("  started: {started}");
}

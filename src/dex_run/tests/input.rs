use super::super::*;

fn read_int(program: &DexProgram, view: &DexValue, field: DexFieldId) -> Option<i32> {
    match program.read_stored_instance_field(view, &field) {
        Some(DexValue::Int(value)) => Some(value),
        _ => None,
    }
}

#[test]
fn focus_is_single_and_clears_the_previous_views_caret() {
    let program = DexProgram::new(Vec::new());
    let first = view_reference(101);
    let second = view_reference(102);
    program.write_view_text(101, "a").unwrap();
    program.write_view_text(102, "b").unwrap();

    assert!(program.set_focused_view(Some(first.clone())));
    assert_eq!(read_int(&program, &first, view_focused_field()), Some(1));
    assert_eq!(
        read_int(&program, &first, text_view_cursor_visible_field()),
        Some(1)
    );

    assert!(program.set_focused_view(Some(second.clone())));
    assert_eq!(read_int(&program, &first, view_focused_field()), Some(0));
    assert_eq!(
        read_int(&program, &first, text_view_cursor_visible_field()),
        Some(0)
    );
    assert_eq!(read_int(&program, &second, view_focused_field()), Some(1));

    assert!(!program.set_focused_view(Some(second)));
}

#[test]
fn typing_inserts_deletes_and_moves_the_caret() {
    let program = DexProgram::new(Vec::new());
    let view = view_reference(200);
    program.write_view_text(200, "ab").unwrap();
    program.set_focused_view(Some(view));

    assert_eq!(program.handle_key_input("X", InputKey::Text, 0).unwrap(), 1);
    assert_eq!(program.view_text(200).unwrap(), Some("abX".to_owned()));

    assert_eq!(
        program
            .handle_key_input("", InputKey::DeleteBackward, 0)
            .unwrap(),
        1
    );
    assert_eq!(program.view_text(200).unwrap(), Some("ab".to_owned()));

    assert_eq!(program.handle_key_input("", InputKey::Left, 0).unwrap(), 1);
    assert_eq!(program.handle_key_input("Z", InputKey::Text, 0).unwrap(), 1);
    assert_eq!(program.view_text(200).unwrap(), Some("aZb".to_owned()));
}

#[test]
fn key_input_without_focus_is_ignored() {
    let program = DexProgram::new(Vec::new());
    assert_eq!(program.handle_key_input("X", InputKey::Text, 0).unwrap(), 0);
}

#[test]
fn enter_on_a_single_line_field_is_consumed_without_inserting_a_newline() {
    let program = DexProgram::new(Vec::new());
    let view = view_reference(400);
    program.write_view_text(400, "12").unwrap();
    program.set_focused_view(Some(view));

    assert_eq!(
        program.handle_key_input("\r", InputKey::Enter, 0).unwrap(),
        1
    );
    assert_eq!(program.view_text(400).unwrap(), Some("12".to_owned()));
}

#[test]
fn caret_renders_only_for_the_focused_view() {
    let program = DexProgram::new(Vec::new());
    program.write_view_text(301, "ab").unwrap();
    program.write_view_text(302, "cd").unwrap();
    program.write_text_view_selection(&view_reference(301), 1, 1);
    program.set_focused_view(Some(view_reference(302)));

    let cursor_ids: Vec<i32> = program
        .view_snapshot()
        .view_text_cursors
        .iter()
        .map(|cursor| cursor.view_id)
        .collect();
    assert_eq!(cursor_ids, vec![302]);
}

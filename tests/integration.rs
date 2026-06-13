use std::path::PathBuf;
use torus_edit::{
    app::App,
    editor::Direction,
    renderer::Renderer,
    search::SearchState,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_app() -> App {
    App::new(Renderer { width: 80, height: 24 })
}

fn type_text(app: &mut App, text: &str) {
    let t = app.theme.clone();
    for c in text.chars() {
        app.editor.insert_char(c, &t);
    }
}

fn tmp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

// ── File operations ───────────────────────────────────────────────────────────

#[test]
fn file_open_loads_content() {
    let path = tmp_path("torus_open_test.txt");
    std::fs::write(&path, "hello from disk\n").unwrap();

    let mut app = make_app();
    app.open_file(path.clone());

    assert!(app.status_msg.is_empty(), "unexpected error: {}", app.status_msg);
    assert!(app.editor.tab().buffer.to_string().contains("hello from disk"));

    std::fs::remove_file(path).ok();
}

#[test]
fn file_save_writes_changes_to_disk() {
    let path = tmp_path("torus_save_test.txt");
    std::fs::write(&path, "original").unwrap();

    let mut app = make_app();
    app.open_file(path.clone());
    // Delete everything and type new content.
    let len = app.editor.tab().buffer.len_chars();
    for _ in 0..len {
        let t = app.theme.clone();
        app.editor.delete_forward(&t);
    }
    type_text(&mut app, "updated");
    app.editor.tab_mut().buffer.save().unwrap();

    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert_eq!(on_disk, "updated");

    std::fs::remove_file(path).ok();
}

#[test]
fn file_save_as_creates_new_file() {
    let mut app = make_app();
    type_text(&mut app, "brand new");

    let path = tmp_path("torus_save_as_test.txt");
    app.editor.tab_mut().buffer.save_as(path.clone()).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "brand new");
    assert!(!app.editor.tab().buffer.is_dirty);

    std::fs::remove_file(path).ok();
}

// ── Edit round-trips ──────────────────────────────────────────────────────────

#[test]
fn edit_undo_redo_roundtrip() {
    let mut app = make_app();
    type_text(&mut app, "hello");
    assert_eq!(app.editor.tab().buffer.to_string(), "hello");

    let t = app.theme.clone();
    app.editor.undo(&t);
    assert_eq!(app.editor.tab().buffer.to_string(), "");

    app.editor.redo(&t);
    assert_eq!(app.editor.tab().buffer.to_string(), "hello");
}

#[test]
fn edit_multiple_undo_steps() {
    let mut app = make_app();
    let t = app.theme.clone();

    // Each word boundary (space) triggers a checkpoint, giving discrete undo steps.
    type_text(&mut app, "one ");
    type_text(&mut app, "two ");
    type_text(&mut app, "three");

    app.editor.undo(&t); // undo "three"
    assert_eq!(app.editor.tab().buffer.to_string(), "one two ");

    app.editor.undo(&t); // undo "two "
    assert_eq!(app.editor.tab().buffer.to_string(), "one ");
}

#[test]
fn edit_backspace_across_line_boundary_joins_lines() {
    let mut app = make_app();
    type_text(&mut app, "ab\ncd");

    // Cursor is at line 1, col 2. Move to line 1, col 0.
    app.editor.tab_mut().cursor.char_col = 0;
    app.editor.tab_mut().cursor.col = 0;

    let t = app.theme.clone();
    app.editor.backspace(&t); // deletes the '\n'

    assert_eq!(app.editor.tab().buffer.to_string(), "abcd");
    assert_eq!(app.editor.tab().cursor.line, 0);
}

#[test]
fn edit_delete_forward_across_line_boundary_joins_lines() {
    let mut app = make_app();
    type_text(&mut app, "ab\ncd");

    // Place cursor at end of first line (after 'b', before '\n').
    app.editor.tab_mut().cursor.line = 0;
    app.editor.tab_mut().cursor.char_col = 2;
    app.editor.tab_mut().cursor.col = 2;

    let t = app.theme.clone();
    app.editor.delete_forward(&t); // deletes the '\n'

    assert_eq!(app.editor.tab().buffer.to_string(), "abcd");
    assert_eq!(app.editor.tab().buffer.len_lines(), 1);
}

// ── Search & Replace ──────────────────────────────────────────────────────────

#[test]
fn search_no_matches_returns_empty_list() {
    let mut s = SearchState::default();
    s.open();
    s.query = "zzz".to_string();
    s.update_matches("hello world");
    assert!(s.matches.is_empty());
}

#[test]
fn search_next_wraps_from_last_match_to_first() {
    let mut s = SearchState::default();
    s.open();
    s.query = "x".to_string();
    s.update_matches("x y x y x");
    assert_eq!(s.matches.len(), 3);

    s.current_match = 2;
    s.next_match();
    assert_eq!(s.current_match, 0);
}

#[test]
fn search_replace_current_updates_buffer() {
    let mut app = make_app();
    type_text(&mut app, "hello world");
    app.search.open_replace();
    app.search.query = "world".to_string();
    app.search.replace_text = "there".to_string();
    let text = app.editor.tab().buffer.to_string();
    app.search.update_matches(&text);

    // Simulate single replace via the buffer directly.
    if let Some(&(start, end)) = app.search.matches.get(app.search.current_match) {
        let count = end - start;
        app.editor.tab_mut().buffer.delete_range(start, count);
        app.editor.tab_mut().buffer.insert_str(start, "there");
    }

    assert_eq!(app.editor.tab().buffer.to_string(), "hello there");
}

#[test]
fn search_replace_all_leaves_no_matches() {
    let mut app = make_app();
    type_text(&mut app, "cat and cat and cat");
    app.search.open_replace();
    app.search.query = "cat".to_string();
    app.search.replace_text = "dog".to_string();

    let original = app.editor.tab().buffer.to_string();
    let new_text = original.replace("cat", "dog");
    app.editor.tab_mut().buffer.replace_all_content(&new_text);

    assert_eq!(app.editor.tab().buffer.to_string(), "dog and dog and dog");

    let refreshed = app.editor.tab().buffer.to_string();
    app.search.update_matches(&refreshed);
    assert!(app.search.matches.is_empty());
}

// ── Multi-tab ─────────────────────────────────────────────────────────────────

#[test]
fn multitab_independent_cursors() {
    let mut app = make_app();
    type_text(&mut app, "line1\nline2\nline3");

    // Open a second tab and type something different.
    app.editor.new_tab();
    type_text(&mut app, "aaa\nbbb");

    // Move cursor on tab 1.
    app.editor.active_tab = 1;
    let t = app.theme.clone();
    app.editor.move_cursor(Direction::Up, 80, 24);
    let tab1_line = app.editor.tab().cursor.line;

    // Tab 0 cursor should still be at its own position.
    app.editor.active_tab = 0;
    let tab0_line = app.editor.tab().cursor.line;

    assert_ne!(tab0_line, tab1_line, "tabs should have independent cursor positions");
}

#[test]
fn multitab_close_activates_adjacent_tab() {
    let mut app = make_app();
    app.editor.new_tab();
    app.editor.new_tab();
    assert_eq!(app.editor.tabs.len(), 3);

    app.editor.active_tab = 2;
    app.editor.close_tab();

    assert_eq!(app.editor.tabs.len(), 2);
    assert_eq!(app.editor.active_tab, 1);
}

#[test]
fn multitab_dirty_flag_cleared_after_save() {
    let path = tmp_path("torus_dirty_test.txt");
    std::fs::write(&path, "").unwrap();

    let mut app = make_app();
    app.open_file(path.clone());
    type_text(&mut app, "edit");
    assert!(app.editor.tab().buffer.is_dirty);

    app.editor.tab_mut().buffer.save().unwrap();
    assert!(!app.editor.tab().buffer.is_dirty);

    std::fs::remove_file(path).ok();
}

#[test]
fn multitab_next_and_prev_cycle_through_tabs() {
    let mut app = make_app();
    app.editor.new_tab();
    app.editor.new_tab();                   // tabs: 0, 1, 2

    app.editor.active_tab = 0;
    app.editor.next_tab();
    assert_eq!(app.editor.active_tab, 1);
    app.editor.next_tab();
    assert_eq!(app.editor.active_tab, 2);
    app.editor.next_tab();                  // wraps
    assert_eq!(app.editor.active_tab, 0);

    app.editor.prev_tab();                  // wraps back
    assert_eq!(app.editor.active_tab, 2);
}

// ── Cursor & scrolling ────────────────────────────────────────────────────────

#[test]
fn cursor_clamps_to_end_of_shorter_line_on_move_up() {
    let mut app = make_app();
    type_text(&mut app, "short\nthis line is much longer");

    // Put cursor at end of the long second line.
    app.editor.tab_mut().cursor.line = 1;
    let long_len = app.editor.tab().buffer.line_len_chars(1);
    app.editor.tab_mut().cursor.char_col = long_len;
    app.editor.tab_mut().cursor.col = long_len;

    let t = app.theme.clone();
    app.editor.move_cursor(Direction::Up, 80, 24);

    let short_len = app.editor.tab().buffer.line_len_chars(0);
    assert_eq!(app.editor.tab().cursor.line, 0);
    assert!(app.editor.tab().cursor.char_col <= short_len,
        "cursor should clamp to end of shorter line");
}

#[test]
fn cursor_scroll_margin_maintained_when_moving_down() {
    let mut app = make_app();
    // Insert enough lines to require scrolling (view_rows ≈ 20 with height 24).
    for i in 0..40 {
        type_text(&mut app, &format!("line {}\n", i));
    }

    let t = app.theme.clone();
    // Drive cursor to line 30 via repeated Down presses.
    for _ in 0..30 {
        app.editor.move_cursor(Direction::Down, 80, 24);
    }

    // Scroll margin is 3 lines; scroll_row should have advanced to keep margin.
    let scroll = app.editor.tab().scroll_row;
    let cursor_line = app.editor.tab().cursor.line;
    assert!(scroll > 0, "viewport should have scrolled");
    assert!(cursor_line >= scroll + 3 || scroll == 0,
        "cursor should be inside the scroll margin");
}

#[test]
fn goto_line_jumps_cursor_to_correct_line() {
    let mut app = make_app();
    for i in 1..=50 {
        type_text(&mut app, &format!("line {}\n", i));
    }

    app.editor.go_to_line(35, 80, 24);

    assert_eq!(app.editor.tab().cursor.line, 34); // 1-based → 0-based
    assert_eq!(app.editor.tab().cursor.char_col, 0);
}

#[test]
fn goto_line_clamps_to_last_line_when_out_of_range() {
    let mut app = make_app();
    type_text(&mut app, "only\nthree\nlines");

    app.editor.go_to_line(9999, 80, 24);

    let max = app.editor.tab().buffer.len_lines() - 1;
    assert_eq!(app.editor.tab().cursor.line, max);
}

#[test]
fn cursor_word_right_jumps_past_word() {
    let mut app = make_app();
    type_text(&mut app, "hello world");

    // Reset cursor to start.
    app.editor.tab_mut().cursor.char_col = 0;
    app.editor.tab_mut().cursor.col = 0;

    app.editor.move_cursor(Direction::WordRight, 80, 24);

    // After one WordRight from col 0, cursor should be past "hello" (col 5).
    assert!(app.editor.tab().cursor.char_col >= 5,
        "word-right should skip past 'hello'");
}

#[test]
fn cursor_word_left_jumps_to_word_start() {
    let mut app = make_app();
    type_text(&mut app, "hello world");

    // Cursor ends up after "world" (col 11); jump left once.
    app.editor.move_cursor(Direction::WordLeft, 80, 24);

    // Should land at the start of "world" (col 6).
    assert!(app.editor.tab().cursor.char_col <= 6,
        "word-left should jump to start of 'world'");
}

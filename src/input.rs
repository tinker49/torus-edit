use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::{
    app::{App, AppCommand},
    editor::Direction,
    menu::MenuAction,
    search::SearchFocus,
};

// ── Top-level event dispatcher ────────────────────────────────────────────────

pub fn handle_event(event: Event, app: &mut App) -> Option<AppCommand> {
    match event {
        Event::Key(key)       => handle_key(key, app),
        Event::Resize(w, h)   => { app.renderer.on_resize(w, h); None }
        _                     => None,
    }
}

// ── Normal key handler ────────────────────────────────────────────────────────

fn handle_key(key: KeyEvent, app: &mut App) -> Option<AppCommand> {
    let ctrl  = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt   = key.modifiers.contains(KeyModifiers::ALT);

    // ── Menu open: route everything to the menu handler ───────────────────────
    if app.menu.is_open() {
        return handle_menu_key(key, app);
    }

    // ── Alt+letter: try to open a dropdown ───────────────────────────────────
    if alt {
        if let KeyCode::Char(c) = key.code {
            app.menu.open_by_key(c.to_ascii_lowercase());
        }
        return None;
    }

    // ── Search bar active ─────────────────────────────────────────────────────
    if app.search.active {
        return handle_search_key(key, app);
    }

    // ── Go-to-line prompt active ──────────────────────────────────────────────
    if app.goto_active {
        return handle_goto_key(key, app);
    }

    // ── Ctrl shortcuts ────────────────────────────────────────────────────────
    if ctrl {
        match key.code {
            // File operations
            KeyCode::Char('n') => { app.editor.new_tab(); return None; }
            KeyCode::Char('o') => return Some(AppCommand::PromptOpenFile),
            KeyCode::Char('s') => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    return Some(AppCommand::PromptSaveAs);
                }
                match app.editor.tab_mut().buffer.save() {
                    Ok(())  => app.set_status("Saved."),
                    Err(e)  => app.set_status(format!("Save error: {e}")),
                }
                return None;
            }
            KeyCode::Char('w') => { app.editor.close_tab(); return None; }
            KeyCode::Char('q') => return Some(AppCommand::Quit),

            // Edit operations
            KeyCode::Char('z') => {
                let t = app.theme.clone();
                app.editor.undo(&t);
                return None;
            }
            KeyCode::Char('y') => {
                let t = app.theme.clone();
                app.editor.redo(&t);
                return None;
            }
            KeyCode::Char('f') => { app.search.open();         return None; }
            KeyCode::Char('h') => { app.search.open_replace(); return None; }
            KeyCode::Char('g') => {
                app.goto_active = true;
                app.goto_input.clear();
                return None;
            }

            // Navigation
            KeyCode::Left  => {
                app.editor.move_cursor(Direction::WordLeft,  app.renderer.width, app.renderer.height);
                return None;
            }
            KeyCode::Right => {
                app.editor.move_cursor(Direction::WordRight, app.renderer.width, app.renderer.height);
                return None;
            }
            KeyCode::Tab   => { app.editor.next_tab(); return None; }

            _ => {}
        }
    }

    // ── Plain editing keys ────────────────────────────────────────────────────
    // Clear any transient status message on the first printable keypress.
    if matches!(key.code, KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete) {
        app.status_msg.clear();
    }

    let w = app.renderer.width;
    let h = app.renderer.height;

    match key.code {
        KeyCode::Char(c) => {
            let t = app.theme.clone();
            app.editor.insert_char(c, &t);
        }
        KeyCode::Enter => {
            let t = app.theme.clone();
            app.editor.insert_char('\n', &t);
        }
        KeyCode::Backspace => {
            let t = app.theme.clone();
            app.editor.backspace(&t);
        }
        KeyCode::Delete => {
            let t = app.theme.clone();
            app.editor.delete_forward(&t);
        }
        KeyCode::Left      => app.editor.move_cursor(Direction::Left,      w, h),
        KeyCode::Right     => app.editor.move_cursor(Direction::Right,     w, h),
        KeyCode::Up        => app.editor.move_cursor(Direction::Up,        w, h),
        KeyCode::Down      => app.editor.move_cursor(Direction::Down,      w, h),
        KeyCode::Home      => app.editor.move_cursor(Direction::Home,      w, h),
        KeyCode::End       => app.editor.move_cursor(Direction::End,       w, h),
        KeyCode::PageUp    => app.editor.move_cursor(Direction::PageUp,    w, h),
        KeyCode::PageDown  => app.editor.move_cursor(Direction::PageDown,  w, h),
        KeyCode::Esc       => { app.status_msg.clear(); }
        _ => {}
    }

    None
}

// ── Menu key handler ──────────────────────────────────────────────────────────

fn handle_menu_key(key: KeyEvent, app: &mut App) -> Option<AppCommand> {
    match key.code {
        KeyCode::Esc                     => { app.menu.close(); }
        KeyCode::Up                      => { app.menu.move_up(); }
        KeyCode::Down                    => { app.menu.move_down(); }
        KeyCode::Left                    => { app.menu.move_left(); }
        KeyCode::Right                   => { app.menu.move_right(); }
        KeyCode::Enter                   => {
            if let Some(action) = app.menu.activate() {
                return dispatch_action(action, app);
            }
        }
        // Alt+letter while a menu is open: switch to that menu.
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => {
            app.menu.open_by_key(c.to_ascii_lowercase());
        }
        _ => {}
    }
    None
}

// ── Search key handler ────────────────────────────────────────────────────────

fn handle_search_key(key: KeyEvent, app: &mut App) -> Option<AppCommand> {
    let bare = key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT;

    match key.code {
        KeyCode::Esc => {
            app.search.close();
        }

        // Tab toggles focus between Query and Replace fields.
        KeyCode::Tab if app.search.replace_mode => {
            app.search.focus = match app.search.focus {
                SearchFocus::Query   => SearchFocus::Replace,
                SearchFocus::Replace => SearchFocus::Query,
            };
        }

        KeyCode::Enter => {
            if app.search.replace_mode && app.search.focus == SearchFocus::Query {
                // Move focus to the Replace field.
                app.search.focus = SearchFocus::Replace;
            } else if app.search.replace_mode && app.search.focus == SearchFocus::Replace {
                // Replace current match and advance.
                perform_replace(app, false);
            } else {
                app.search.next_match();
                jump_to_current_match(app);
            }
        }

        // F3 / F4: next match / replace-all
        KeyCode::F(3) => {
            app.search.next_match();
            jump_to_current_match(app);
        }
        KeyCode::F(4) if app.search.replace_mode => {
            perform_replace(app, true);
        }

        // Typing into the active field.
        KeyCode::Char(c) if bare => {
            match app.search.focus {
                SearchFocus::Query => {
                    app.search.query.push(c);
                    let text = app.editor.tab().buffer.to_string();
                    app.search.update_matches(&text);
                    jump_to_current_match(app);
                }
                SearchFocus::Replace => {
                    app.search.replace_text.push(c);
                }
            }
        }

        KeyCode::Backspace => {
            match app.search.focus {
                SearchFocus::Query => {
                    app.search.query.pop();
                    let text = app.editor.tab().buffer.to_string();
                    app.search.update_matches(&text);
                    jump_to_current_match(app);
                }
                SearchFocus::Replace => { app.search.replace_text.pop(); }
            }
        }

        _ => {}
    }

    None
}

// ── Go-to-line handler ────────────────────────────────────────────────────────

fn handle_goto_key(key: KeyEvent, app: &mut App) -> Option<AppCommand> {
    match key.code {
        KeyCode::Esc => {
            app.goto_active = false;
            app.goto_input.clear();
        }
        KeyCode::Enter => {
            if let Ok(n) = app.goto_input.parse::<usize>() {
                app.editor.go_to_line(n, app.renderer.width, app.renderer.height);
            }
            app.goto_active = false;
            app.goto_input.clear();
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            app.goto_input.push(c);
        }
        KeyCode::Backspace => { app.goto_input.pop(); }
        _ => {}
    }
    None
}

// ── Action dispatcher ─────────────────────────────────────────────────────────

fn dispatch_action(action: MenuAction, app: &mut App) -> Option<AppCommand> {
    match action {
        MenuAction::Quit              => return Some(AppCommand::Quit),
        MenuAction::NewFile           => app.editor.new_tab(),
        MenuAction::OpenFile          => return Some(AppCommand::PromptOpenFile),
        MenuAction::Save              => {
            match app.editor.tab_mut().buffer.save() {
                Ok(())  => app.set_status("Saved."),
                Err(e)  => app.set_status(format!("Save error: {e}")),
            }
        }
        MenuAction::SaveAs            => return Some(AppCommand::PromptSaveAs),
        MenuAction::CloseTab          => { app.editor.close_tab(); }
        MenuAction::Undo              => { let t = app.theme.clone(); app.editor.undo(&t); }
        MenuAction::Redo              => { let t = app.theme.clone(); app.editor.redo(&t); }
        MenuAction::Find              => app.search.open(),
        MenuAction::Replace           => app.search.open_replace(),
        MenuAction::GoToLine          => { app.goto_active = true; app.goto_input.clear(); }
        MenuAction::ToggleLineNumbers => { app.editor.show_line_numbers ^= true; }
        MenuAction::NextTab           => app.editor.next_tab(),
        MenuAction::PrevTab           => app.editor.prev_tab(),
    }
    None
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn jump_to_current_match(app: &mut App) {
    if let Some(&(start, _)) = app.search.matches.get(app.search.current_match) {
        let buf  = &app.editor.tab().buffer;
        let safe = start.min(buf.len_chars().saturating_sub(1));
        let line = buf.char_to_line(safe);
        let col  = start - buf.line_to_char(line);
        let w    = app.renderer.width;
        let h    = app.renderer.height;
        let tab  = app.editor.tab_mut();
        tab.cursor.line     = line;
        tab.cursor.char_col = col;
        tab.cursor.col      = col;
        tab.ensure_cursor_visible(w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use crate::{app::App, menu::MenuAction, renderer::Renderer};

    fn make_app() -> App {
        App::new(Renderer { width: 80, height: 24 })
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    // ── handle_event ─────────────────────────────────────────────────────────

    #[test]
    fn test_handle_event_resize_updates_renderer() {
        let mut app = make_app();
        handle_event(Event::Resize(120, 40), &mut app);
        assert_eq!(app.renderer.width, 120);
        assert_eq!(app.renderer.height, 40);
    }

    // ── handle_key ───────────────────────────────────────────────────────────

    #[test]
    fn test_handle_key_char_inserts_into_buffer() {
        let mut app = make_app();
        handle_key(key(KeyCode::Char('a')), &mut app);
        assert_eq!(app.editor.tab().buffer.to_string(), "a");
    }

    #[test]
    fn test_handle_key_ctrl_q_returns_quit() {
        let mut app = make_app();
        let cmd = handle_key(ctrl(KeyCode::Char('q')), &mut app);
        assert!(matches!(cmd, Some(AppCommand::Quit)));
    }

    #[test]
    fn test_handle_key_ctrl_n_opens_new_tab() {
        let mut app = make_app();
        handle_key(ctrl(KeyCode::Char('n')), &mut app);
        assert_eq!(app.editor.tabs.len(), 2);
    }

    #[test]
    fn test_handle_key_ctrl_g_activates_goto() {
        let mut app = make_app();
        handle_key(ctrl(KeyCode::Char('g')), &mut app);
        assert!(app.goto_active);
    }

    #[test]
    fn test_handle_key_ctrl_f_opens_search() {
        let mut app = make_app();
        handle_key(ctrl(KeyCode::Char('f')), &mut app);
        assert!(app.search.active);
    }

    #[test]
    fn test_handle_key_esc_clears_status() {
        let mut app = make_app();
        app.set_status("some message");
        handle_key(key(KeyCode::Esc), &mut app);
        assert!(app.status_msg.is_empty());
    }

    // ── handle_menu_key ───────────────────────────────────────────────────────

    #[test]
    fn test_handle_menu_key_esc_closes_menu() {
        let mut app = make_app();
        app.menu.open_by_key('f');
        assert!(app.menu.is_open());
        handle_menu_key(key(KeyCode::Esc), &mut app);
        assert!(!app.menu.is_open());
    }

    // ── handle_search_key ────────────────────────────────────────────────────

    #[test]
    fn test_handle_search_key_esc_closes_search() {
        let mut app = make_app();
        app.search.open();
        handle_search_key(key(KeyCode::Esc), &mut app);
        assert!(!app.search.active);
    }

    #[test]
    fn test_handle_search_key_char_appends_to_query() {
        let mut app = make_app();
        app.search.open();
        handle_search_key(key(KeyCode::Char('x')), &mut app);
        assert_eq!(app.search.query, "x");
    }

    // ── handle_goto_key ───────────────────────────────────────────────────────

    #[test]
    fn test_handle_goto_key_digit_appends_to_input() {
        let mut app = make_app();
        app.goto_active = true;
        handle_goto_key(key(KeyCode::Char('5')), &mut app);
        assert_eq!(app.goto_input, "5");
    }

    #[test]
    fn test_handle_goto_key_enter_navigates_and_clears() {
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "line1\nline2\nline3".chars() { app.editor.insert_char(c, &t); }
        app.goto_active = true;
        app.goto_input = "2".to_string();
        handle_goto_key(key(KeyCode::Enter), &mut app);
        assert!(!app.goto_active);
        assert_eq!(app.editor.tab().cursor.line, 1);
    }

    // ── dispatch_action ───────────────────────────────────────────────────────

    #[test]
    fn test_dispatch_action_new_file_opens_tab() {
        let mut app = make_app();
        dispatch_action(MenuAction::NewFile, &mut app);
        assert_eq!(app.editor.tabs.len(), 2);
    }

    #[test]
    fn test_dispatch_action_quit_returns_quit_command() {
        let mut app = make_app();
        let cmd = dispatch_action(MenuAction::Quit, &mut app);
        assert!(matches!(cmd, Some(AppCommand::Quit)));
    }

    #[test]
    fn test_dispatch_action_toggle_line_numbers() {
        let mut app = make_app();
        assert!(app.editor.show_line_numbers);
        dispatch_action(MenuAction::ToggleLineNumbers, &mut app);
        assert!(!app.editor.show_line_numbers);
    }

    // ── jump_to_current_match ────────────────────────────────────────────────

    #[test]
    fn test_jump_to_current_match_moves_cursor() {
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "hello world".chars() { app.editor.insert_char(c, &t); }
        app.search.open();
        app.search.query = "world".to_string();
        let text = app.editor.tab().buffer.to_string();
        app.search.update_matches(&text);
        jump_to_current_match(&mut app);
        // "world" starts at char index 6
        assert_eq!(app.editor.tab().cursor.char_col, 6);
    }

    // ── perform_replace ───────────────────────────────────────────────────────

    #[test]
    fn test_perform_replace_single_replaces_current_match() {
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "hello world".chars() { app.editor.insert_char(c, &t); }
        app.search.open_replace();
        app.search.query        = "world".to_string();
        app.search.replace_text = "there".to_string();
        let text = app.editor.tab().buffer.to_string();
        app.search.update_matches(&text);
        perform_replace(&mut app, false);
        assert_eq!(app.editor.tab().buffer.to_string(), "hello there");
    }

    #[test]
    fn test_perform_replace_all_replaces_every_occurrence() {
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "aaa".chars() { app.editor.insert_char(c, &t); }
        app.search.open_replace();
        app.search.query        = "a".to_string();
        app.search.replace_text = "b".to_string();
        let text = app.editor.tab().buffer.to_string();
        app.search.update_matches(&text);
        perform_replace(&mut app, true);
        assert_eq!(app.editor.tab().buffer.to_string(), "bbb");
    }
}

fn perform_replace(app: &mut App, replace_all: bool) {
    let query   = app.search.query.clone();
    let replace = app.search.replace_text.clone();
    let t       = app.theme.clone();

    if replace_all {
        let original = app.editor.tab().buffer.to_string();
        let new_text = original.replace(&query, &replace);
        let count    = original.matches(&query).count();
        app.editor.tab_mut().buffer.replace_all_content(&new_text);
        app.editor.tab_mut().rehighlight(&t);
        let refreshed = app.editor.tab().buffer.to_string();
        app.search.update_matches(&refreshed);
        app.set_status(format!("Replaced {count} occurrence(s) of '{query}'"));
    } else if let Some(&(start, end)) = app.search.matches.get(app.search.current_match) {
        let count = end - start;
        app.editor.tab_mut().buffer.delete_range(start, count);
        app.editor.tab_mut().buffer.insert_str(start, &replace);
        app.editor.tab_mut().buffer.checkpoint();
        app.editor.tab_mut().rehighlight(&t);
        let refreshed = app.editor.tab().buffer.to_string();
        app.search.update_matches(&refreshed);
        jump_to_current_match(app);
    }
}

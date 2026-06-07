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

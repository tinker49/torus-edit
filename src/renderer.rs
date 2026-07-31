use std::io::Write;
use crossterm::{
    cursor,
    style::{Color, SetForegroundColor, SetBackgroundColor, Print, Attribute, SetAttribute},
    terminal,
    QueueableCommand,
};
use crate::{
    app::App,
    config::{Theme, MENU_HEIGHT, TAB_BAR_HEIGHT, STATUS_HEIGHT, GUTTER_WIDTH},
    editor::Editor,
    menu::MenuBar,
    search::SearchState,
};

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    pub width:  u16,
    pub height: u16,
}

impl Renderer {
    pub fn new() -> std::io::Result<Self> {
        let (w, h) = terminal::size()?;
        Ok(Self { width: w, height: h })
    }

    pub fn on_resize(&mut self, w: u16, h: u16) {
        self.width  = w;
        self.height = h;
    }

    /// Render the full UI into `out`.  Called once per event loop tick.
    pub fn render_all(&self, out: &mut impl Write, app: &App) -> std::io::Result<()> {
        out.queue(cursor::Hide)?;
        self.render_menu_bar(out, &app.menu, &app.theme)?;
        self.render_tab_bar(out, &app.editor, &app.theme)?;
        self.render_editor(out, &app.editor, &app.search, &app.theme)?;
        self.render_status(out, app)?;
        self.render_dropdown(out, &app.menu, &app.theme)?;
        self.render_cursor(out, app)?;
        out.flush()
    }

    // ── Menu bar ──────────────────────────────────────────────────────────────

    fn render_menu_bar(
        &self,
        out:   &mut impl Write,
        menu:  &MenuBar,
        theme: &Theme,
    ) -> std::io::Result<()> {
        out.queue(cursor::MoveTo(0, 0))?;
        out.queue(SetBackgroundColor(theme.menu_bg))?;
        out.queue(SetForegroundColor(theme.menu_fg))?;

        let mut x = 1u16;
        for (i, m) in menu.menus.iter().enumerate() {
            let active = menu.open == Some(i);
            if active {
                out.queue(SetBackgroundColor(theme.menu_active_bg))?;
                out.queue(SetForegroundColor(theme.menu_active_fg))?;
            } else {
                out.queue(SetBackgroundColor(theme.menu_bg))?;
                out.queue(SetForegroundColor(theme.menu_fg))?;
            }
            let label = format!(" {} ", m.label);
            out.queue(cursor::MoveTo(x, 0))?;
            out.queue(Print(&label))?;
            x += label.len() as u16;
        }

        // Fill the rest of the row.
        out.queue(SetBackgroundColor(theme.menu_bg))?;
        out.queue(SetForegroundColor(theme.menu_fg))?;
        let fill = " ".repeat(self.width.saturating_sub(x) as usize);
        out.queue(Print(fill))?;

        Ok(())
    }

    // ── Dropdown overlay ──────────────────────────────────────────────────────

    fn render_dropdown(
        &self,
        out:   &mut impl Write,
        menu:  &MenuBar,
        theme: &Theme,
    ) -> std::io::Result<()> {
        if let Some(mi) = menu.open {
            let m          = &menu.menus[mi];
            let menu_col   = menu.menu_x(mi);
            const DW: u16  = 32; // dropdown width

            for (row, item) in m.items.iter().enumerate() {
                let y   = MENU_HEIGHT + row as u16;
                let sel = row == menu.selected;

                out.queue(cursor::MoveTo(menu_col, y))?;
                if sel {
                    out.queue(SetBackgroundColor(theme.dropdown_selected_bg))?;
                    out.queue(SetForegroundColor(theme.dropdown_selected_fg))?;
                } else {
                    out.queue(SetBackgroundColor(theme.dropdown_bg))?;
                    out.queue(SetForegroundColor(theme.dropdown_fg))?;
                }

                let sc  = item.shortcut.as_deref().unwrap_or("");
                // label (left) + shortcut (right-aligned) within DW columns
                let lbl_max = (DW as usize).saturating_sub(sc.len() + 3);
                let label   = truncate(&item.label, lbl_max);
                let gap     = DW as usize - 2 - label.len() - sc.len();
                let row_str = format!(" {}{}{} ", label, " ".repeat(gap), sc);
                let row_str: String = row_str.chars().take(DW as usize).collect();

                if !sel && !sc.is_empty() {
                    // Print label in normal colour, shortcut in accent colour.
                    let label_part = format!(" {}", label);
                    out.queue(Print(&label_part))?;
                    out.queue(SetForegroundColor(theme.dropdown_shortcut_fg))?;
                    let sc_part = format!(
                        "{}{} ",
                        " ".repeat(gap),
                        sc
                    );
                    out.queue(Print(sc_part))?;
                } else {
                    out.queue(Print(&row_str))?;
                }
            }
        }

        Ok(())
    }

    // ── Tab bar ───────────────────────────────────────────────────────────────

    fn render_tab_bar(
        &self,
        out:    &mut impl Write,
        editor: &Editor,
        theme:  &Theme,
    ) -> std::io::Result<()> {
        out.queue(cursor::MoveTo(0, MENU_HEIGHT))?;

        let mut x = 0u16;
        for (i, tab) in editor.tabs.iter().enumerate() {
            let active = i == editor.active_tab;
            let dirty  = tab.buffer.is_dirty;
            let name   = tab.buffer.name();

            let label = format!(" {} {} ", name, if dirty { "●" } else { " " });
            let lw    = label.chars().count() as u16;
            if x + lw > self.width { break; }

            out.queue(cursor::MoveTo(x, MENU_HEIGHT))?;
            if active {
                out.queue(SetBackgroundColor(theme.tab_active_bg))?;
                out.queue(SetForegroundColor(theme.tab_active_fg))?;
            } else {
                out.queue(SetBackgroundColor(theme.tab_bg))?;
                out.queue(SetForegroundColor(theme.tab_inactive_fg))?;
            }

            // Print name, then dirty indicator in accent colour.
            out.queue(Print(format!(" {} ", name)))?;
            if dirty {
                out.queue(SetForegroundColor(theme.tab_dirty_indicator))?;
            }
            out.queue(Print(if dirty { "● " } else { "  " }))?;
            x += lw;
        }

        // Fill remainder of tab bar.
        out.queue(SetBackgroundColor(theme.tab_bg))?;
        out.queue(SetForegroundColor(theme.tab_inactive_fg))?;
        let fill = " ".repeat(self.width.saturating_sub(x) as usize);
        out.queue(Print(fill))?;

        Ok(())
    }

    // ── Editor area ───────────────────────────────────────────────────────────

    fn render_editor(
        &self,
        out:    &mut impl Write,
        editor: &Editor,
        search: &SearchState,
        theme:  &Theme,
    ) -> std::io::Result<()> {
        let tab    = editor.tab();
        let buf    = &tab.buffer;
        let gutter = if editor.show_line_numbers { GUTTER_WIDTH } else { 0 };

        let top_y      = MENU_HEIGHT + TAB_BAR_HEIGHT;
        // Reserve one extra row for the search bar when active.
        let extra      = if search.active { 1u16 } else { 0 };
        let view_rows  = self.height.saturating_sub(top_y + STATUS_HEIGHT + extra) as usize;
        let view_cols  = self.width.saturating_sub(gutter) as usize;

        for screen_row in 0..view_rows {
            let line_idx = tab.scroll_row + screen_row;
            let y        = top_y + screen_row as u16;

            out.queue(cursor::MoveTo(0, y))?;

            if line_idx >= buf.len_lines() {
                // Render tilde and blank space beyond end of file.
                if gutter > 0 {
                    out.queue(SetBackgroundColor(theme.gutter_bg))?;
                    out.queue(SetForegroundColor(theme.gutter_fg))?;
                    let s = format!("{:>width$} ", "~", width = (gutter - 1) as usize);
                    out.queue(Print(s))?;
                }
                out.queue(SetBackgroundColor(theme.editor_bg))?;
                out.queue(Print(" ".repeat(view_cols)))?;
                continue;
            }

            // ── Gutter ────────────────────────────────────────────────────────
            if gutter > 0 {
                let is_cursor_line = line_idx == tab.cursor.line;
                out.queue(SetBackgroundColor(theme.gutter_bg))?;
                out.queue(SetForegroundColor(
                    if is_cursor_line { theme.gutter_active_fg } else { theme.gutter_fg }
                ))?;
                let num = format!("{:>width$} ", line_idx + 1, width = (gutter - 1) as usize);
                out.queue(Print(num))?;
            }

            // ── Text content ──────────────────────────────────────────────────
            let line_start_char = buf.line_to_char(line_idx);
            let line_slice      = buf.rope.line(line_idx);
            let mut col         = 0usize;   // visual column
            let mut rendered    = 0usize;   // rendered cells so far

            for (ci, ch) in line_slice.chars().enumerate() {
                if ch == '\n' || ch == '\r' { break; }

                // Skip columns left of the scroll offset.
                if col < tab.scroll_col {
                    col += if ch == '\t' { tab_width(col) } else { 1 };
                    continue;
                }
                if rendered >= view_cols { break; }

                let char_abs  = line_start_char + ci;
                let byte_pos  = buf.rope.char_to_byte(char_abs);

                // Determine background (search match overrides syntax).
                let (bg, fg) = if search.active {
                    match search.match_at(char_abs) {
                        Some(true)  => (theme.search_current_bg, theme.editor_bg),
                        Some(false) => (theme.search_match_bg,   theme.editor_bg),
                        None => {
                            let syn = tab.highlighter.color_at(byte_pos).unwrap_or(theme.editor_fg);
                            (theme.editor_bg, syn)
                        }
                    }
                } else {
                    let syn = tab.highlighter.color_at(byte_pos).unwrap_or(theme.editor_fg);
                    (theme.editor_bg, syn)
                };

                out.queue(SetBackgroundColor(bg))?;
                out.queue(SetForegroundColor(fg))?;

                if ch == '\t' {
                    let spaces = tab_width(col).min(view_cols - rendered);
                    out.queue(Print(" ".repeat(spaces)))?;
                    rendered += spaces;
                    col      += spaces;
                } else {
                    out.queue(Print(ch))?;
                    rendered += 1;
                    col      += 1;
                }
            }

            // Fill remaining cells on this row with the editor background.
            if rendered < view_cols {
                out.queue(SetBackgroundColor(theme.editor_bg))?;
                out.queue(Print(" ".repeat(view_cols - rendered)))?;
            }
        }

        Ok(())
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn render_status(&self, out: &mut impl Write, app: &App) -> std::io::Result<()> {
        let t   = &app.theme;
        let tab = app.editor.tab();
        let buf = &tab.buffer;

        // ── Search bar (floats just above the status bar) ─────────────────────
        if app.search.active {
            let search_y = self.height - STATUS_HEIGHT - 1;
            out.queue(cursor::MoveTo(0, search_y))?;
            out.queue(SetBackgroundColor(t.search_bar_bg))?;
            out.queue(SetForegroundColor(t.search_bar_fg))?;

            let prompt = if app.search.replace_mode {
                if app.search.focus == crate::search::SearchFocus::Replace {
                    "Replace ▶ "
                } else {
                    "Search  ▶ "
                }
            } else {
                "Search  ▶ "
            };

            let count_str = if !app.search.matches.is_empty() {
                format!(
                    "  [{}/{}]",
                    app.search.current_match + 1,
                    app.search.matches.len()
                )
            } else if !app.search.query.is_empty() {
                "  [no matches]".to_string()
            } else {
                String::new()
            };

            let display = if app.search.replace_mode
                && app.search.focus == crate::search::SearchFocus::Replace
            {
                format!("{}{}{}",  prompt, app.search.replace_text, count_str)
            } else {
                format!("{}{}{}", prompt, app.search.query, count_str)
            };

            let padded: String = format!("{:<width$}", display, width = self.width as usize)
                .chars()
                .take(self.width as usize)
                .collect();
            out.queue(Print(padded))?;
        }

        // ── Status bar ────────────────────────────────────────────────────────
        let y = self.height - STATUS_HEIGHT;
        out.queue(cursor::MoveTo(0, y))?;
        out.queue(SetBackgroundColor(t.status_bg))?;
        out.queue(SetForegroundColor(t.status_fg))?;

        let display = if !app.status_msg.is_empty() {
            // Transient status message takes over the whole bar.
            format!("  {}", app.status_msg)
        } else {
            let file  = buf.name();
            let dirty = if buf.is_dirty { " [+]" } else { "" };
            let lang  = buf.language.as_deref().unwrap_or("text");
            let line  = tab.cursor.line + 1;
            let col   = tab.cursor.col  + 1;
            let total = buf.len_lines();

            let left  = format!("  {}{}  │  {} ", file, dirty, lang);
            let right = format!("  Ln {}, Col {}  │  {} lines  ", line, col, total);
            let avail = self.width as usize;

            if left.len() + right.len() <= avail {
                let mid = " ".repeat(avail - left.len() - right.len());
                format!("{}{}{}", left, mid, right)
            } else {
                left
            }
        };

        let padded: String = format!("{:<width$}", display, width = self.width as usize)
            .chars()
            .take(self.width as usize)
            .collect();
        out.queue(Print(padded))?;

        Ok(())
    }

    // ── Cursor positioning ────────────────────────────────────────────────────

    fn render_cursor(&self, out: &mut impl Write, app: &App) -> std::io::Result<()> {
        // Hide cursor while the menu is open (selection is shown via background colour).
        if app.menu.is_open() {
            out.queue(cursor::Hide)?;
            return Ok(());
        }

        // Show cursor in the search bar when searching.
        if app.search.active {
            let y       = self.height - STATUS_HEIGHT - 1;
            let prompt  = if app.search.replace_mode
                && app.search.focus == crate::search::SearchFocus::Replace
            {
                "Replace ▶ "
            } else {
                "Search  ▶ "
            };
            let text_len = if app.search.replace_mode
                && app.search.focus == crate::search::SearchFocus::Replace
            {
                app.search.replace_text.len()
            } else {
                app.search.query.len()
            };
            let x = prompt.chars().count() as u16 + text_len as u16;
            out.queue(cursor::MoveTo(x, y))?;
            out.queue(cursor::SetCursorStyle::SteadyBar)?;
            out.queue(cursor::Show)?;
            return Ok(());
        }

        // Show cursor at the goto prompt when active.
        if app.goto_active {
            let y = self.height - STATUS_HEIGHT;
            let x = 2 + "Go to line: ".len() as u16 + app.goto_input.len() as u16;
            out.queue(cursor::MoveTo(x, y))?;
            out.queue(cursor::SetCursorStyle::SteadyBar)?;
            out.queue(cursor::Show)?;
            return Ok(());
        }

        // Normal editor cursor.
        let tab     = app.editor.tab();
        let gutter  = if app.editor.show_line_numbers { GUTTER_WIDTH } else { 0 };
        let top_y   = MENU_HEIGHT + TAB_BAR_HEIGHT;
        let extra   = if app.search.active { 1u16 } else { 0 };

        let screen_row = (tab.cursor.line as isize - tab.scroll_row as isize) as u16;
        let screen_col = (tab.cursor.col  as isize - tab.scroll_col as isize) as u16 + gutter;

        out.queue(cursor::MoveTo(screen_col, top_y + screen_row))?;
        out.queue(cursor::SetCursorStyle::SteadyBar)?;
        out.queue(cursor::Show)?;

        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Visual width of a tab stop at column `col` (4-space tabs).
#[inline]
fn tab_width(col: usize) -> usize { 4 - (col % 4) }

/// Truncate a string to at most `max_chars` characters.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars.saturating_sub(1)).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::App, menu::MenuBar, search::SearchState};

    fn make_renderer() -> Renderer {
        Renderer { width: 80, height: 24 }
    }

    fn make_app() -> App {
        App::new(make_renderer())
    }

    fn output_contains(buf: &[u8], needle: &str) -> bool {
        buf.windows(needle.len()).any(|w| w == needle.as_bytes())
    }

    /// Strip ANSI escape sequences so plain text can be searched.
    fn strip_ansi(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
                i += 2;
                while i < bytes.len() && !bytes[i].is_ascii_alphabetic() { i += 1; }
                if i < bytes.len() { i += 1; }
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        out
    }

    // ── new ───────────────────────────────────────────────────────────────────

    #[test]
    #[ignore = "requires a real PTY for terminal::size()"]
    fn test_new_reads_terminal_size() {}

    // ── on_resize ─────────────────────────────────────────────────────────────

    #[test]
    fn test_on_resize_updates_dimensions() {
        let mut r = make_renderer();
        r.on_resize(120, 40);
        assert_eq!(r.width, 120);
        assert_eq!(r.height, 40);
    }

    // ── render_all ────────────────────────────────────────────────────────────

    #[test]
    fn test_render_all_writes_bytes() {
        let r   = make_renderer();
        let mut app = make_app();
        let mut out = Vec::new();
        r.render_all(&mut out, &mut app).unwrap();
        assert!(!out.is_empty());
    }

    // ── render_menu_bar ───────────────────────────────────────────────────────

    #[test]
    fn test_render_menu_bar_contains_menu_labels() {
        let r     = make_renderer();
        let menu  = MenuBar::new();
        let theme = Theme::default();
        let mut out = Vec::new();
        r.render_menu_bar(&mut out, &menu, &theme).unwrap();
        assert!(output_contains(&out, "File"), "menu bar should contain 'File'");
        assert!(output_contains(&out, "Edit"), "menu bar should contain 'Edit'");
    }

    // ── render_tab_bar ────────────────────────────────────────────────────────

    #[test]
    fn test_render_tab_bar_contains_buffer_name() {
        let r      = make_renderer();
        let editor = Editor::new();
        let theme  = Theme::default();
        let mut out = Vec::new();
        r.render_tab_bar(&mut out, &editor, &theme).unwrap();
        // New untitled buffer is named "[No Name]"
        assert!(output_contains(&out, "[No Name]"));
    }

    // ── render_editor ────────────────────────────────────────────────────────

    #[test]
    fn test_render_editor_writes_text_content() {
        let r      = make_renderer();
        let theme  = Theme::default();
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "hello".chars() { app.editor.insert_char(c, &t); }
        let mut out = Vec::new();
        r.render_editor(&mut out, &app.editor, &app.search, &theme).unwrap();
        // crossterm emits color codes between each character, so strip before searching
        let plain = strip_ansi(&out);
        assert!(output_contains(&plain, "hello"), "rendered editor should contain buffer text");
    }

    // ── render_status ─────────────────────────────────────────────────────────

    #[test]
    fn test_render_status_contains_cursor_position() {
        let r   = make_renderer();
        let mut app = make_app();
        let mut out = Vec::new();
        r.render_status(&mut out, &app).unwrap();
        // Default cursor is Ln 1, Col 1
        assert!(output_contains(&out, "Ln 1"), "status bar should show line number");
    }

    #[test]
    fn test_render_status_shows_transient_message() {
        let r   = make_renderer();
        let mut app = make_app();
        app.set_status("File saved!");
        let mut out = Vec::new();
        r.render_status(&mut out, &app).unwrap();
        assert!(output_contains(&out, "File saved!"));
    }

    // ── render_cursor ─────────────────────────────────────────────────────────

    #[test]
    fn test_render_cursor_writes_bytes() {
        let r   = make_renderer();
        let mut app = make_app();
        let mut out = Vec::new();
        r.render_cursor(&mut out, &app).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn test_render_cursor_hides_when_menu_open() {
        let r   = make_renderer();
        let mut app = make_app();
        app.menu.open_by_key('f');
        let mut out = Vec::new();
        r.render_cursor(&mut out, &app).unwrap();
        // When menu is open, render_cursor calls cursor::Hide and returns early.
        // The Hide command emits ESC [ ? 25 l — check for the escape byte.
        assert!(out.contains(&0x1b), "should emit escape sequence when hiding cursor");
    }

    // ── tab_width ─────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_width_at_column_zero_is_four() {
        assert_eq!(tab_width(0), 4);
    }

    #[test]
    fn test_tab_width_fills_to_next_tab_stop() {
        assert_eq!(tab_width(1), 3);
        assert_eq!(tab_width(2), 2);
        assert_eq!(tab_width(3), 1);
        assert_eq!(tab_width(4), 4); // next stop
    }

    // ── render_dropdown ──────────────────────────────────────────────────────

    #[test]
    fn test_render_dropdown_empty_when_closed() {
        let r     = make_renderer();
        let menu  = MenuBar::new();
        let theme = Theme::default();
        let mut out = Vec::new();
        r.render_dropdown(&mut out, &menu, &theme).unwrap();
        assert!(out.is_empty(), "closed menu should produce no output");
    }

    #[test]
    fn test_render_dropdown_shows_item_labels_when_open() {
        let r     = make_renderer();
        let mut menu  = MenuBar::new();
        let theme = Theme::default();
        menu.open_by_key('f');
        let mut out = Vec::new();
        r.render_dropdown(&mut out, &menu, &theme).unwrap();
        let plain = strip_ansi(&out);
        assert!(output_contains(&plain, "New File"), "dropdown should show 'New File'");
        assert!(output_contains(&plain, "Save"),     "dropdown should show 'Save'");
        assert!(output_contains(&plain, "Quit"),     "dropdown should show 'Quit'");
    }

    #[test]
    fn test_render_dropdown_shows_shortcuts() {
        let r     = make_renderer();
        let mut menu  = MenuBar::new();
        let theme = Theme::default();
        menu.open_by_key('f');
        let mut out = Vec::new();
        r.render_dropdown(&mut out, &menu, &theme).unwrap();
        let plain = strip_ansi(&out);
        assert!(output_contains(&plain, "Ctrl+S"), "dropdown should show 'Ctrl+S' shortcut");
    }

    #[test]
    fn test_render_dropdown_appears_after_editor_in_render_all() {
        // Regression test: dropdown must be rendered last so the editor does not
        // overwrite it.  Verify the item text appears after the editor content
        // in the raw byte stream.
        let r   = make_renderer();
        let mut app = make_app();
        let t = app.theme.clone();
        for c in "hello".chars() { app.editor.insert_char(c, &t); }
        app.menu.open_by_key('f');

        let mut out = Vec::new();
        r.render_all(&mut out, &mut app).unwrap();
        let plain = strip_ansi(&out);

        let editor_pos  = plain.windows(5).position(|w| w == b"hello")
            .expect("editor text 'hello' not found in output");
        let dropdown_pos = plain.windows(8).position(|w| w == b"New File")
            .expect("dropdown item 'New File' not found in output");

        assert!(
            dropdown_pos > editor_pos,
            "dropdown (pos {dropdown_pos}) must be rendered after editor content (pos {editor_pos})"
        );
    }

    // ── truncate ─────────────────────────────────────────────────────────────

    #[test]
    fn test_truncate_short_string_unchanged() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn test_truncate_long_string_gets_ellipsis() {
        let result = truncate("hello world", 6);
        assert!(result.ends_with('…'), "should end with ellipsis");
        assert!(result.chars().count() <= 6);
    }
}

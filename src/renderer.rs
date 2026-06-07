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

        // ── Dropdown ─────────────────────────────────────────────────────────
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
                let gap     = DW as usize - 1 - label.len() - sc.len();
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

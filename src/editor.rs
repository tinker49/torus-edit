use std::path::PathBuf;
use crate::{
    buffer::Buffer,
    config::{Theme, MENU_HEIGHT, TAB_BAR_HEIGHT, STATUS_HEIGHT, GUTTER_WIDTH, SCROLL_MARGIN},
    highlight::Highlighter,
};

// ── Cursor ────────────────────────────────────────────────────────────────────

#[derive(Default, Clone, Copy)]
pub struct Cursor {
    /// Zero-based line index.
    pub line:     usize,
    /// Visual column (accounts for tab expansion in display).
    pub col:      usize,
    /// Character offset within the line (does not expand tabs).
    pub char_col: usize,
}

// ── Tab ───────────────────────────────────────────────────────────────────────

pub struct Tab {
    pub buffer:      Buffer,
    pub highlighter: Highlighter,
    pub cursor:      Cursor,
    /// Index of the top-visible line (vertical scroll offset).
    pub scroll_row:  usize,
    /// Index of the left-visible column (horizontal scroll offset).
    pub scroll_col:  usize,
}

impl Tab {
    pub fn new_empty() -> Self {
        Self {
            buffer:      Buffer::new(),
            highlighter: Highlighter::new(),
            cursor:      Cursor::default(),
            scroll_row:  0,
            scroll_col:  0,
        }
    }

    pub fn from_buffer(mut buf: Buffer, theme: &Theme) -> Self {
        let mut h = Highlighter::new();
        if let Some(lang) = &buf.language.clone() {
            h.set_language(lang);
            h.parse(&buf.to_string(), theme);
        }
        Self {
            buffer:      buf,
            highlighter: h,
            cursor:      Cursor::default(),
            scroll_row:  0,
            scroll_col:  0,
        }
    }

    /// Re-parse syntax highlights after a buffer edit.
    pub fn rehighlight(&mut self, theme: &Theme) {
        let text = self.buffer.to_string();
        self.highlighter.reparse(&text, theme);
    }

    /// Adjust `scroll_row` / `scroll_col` so the cursor is visible.
    pub fn ensure_cursor_visible(&mut self, cols: u16, rows: u16) {
        let view_rows = rows
            .saturating_sub(MENU_HEIGHT + TAB_BAR_HEIGHT + STATUS_HEIGHT + 1)
            as usize;
        let gutter    = GUTTER_WIDTH as usize;
        let view_cols = cols.saturating_sub(GUTTER_WIDTH + 1) as usize;

        // Vertical ────────────────────────────────────────────────────────────
        if self.cursor.line < self.scroll_row + SCROLL_MARGIN {
            self.scroll_row = self.cursor.line.saturating_sub(SCROLL_MARGIN);
        } else if self.cursor.line + SCROLL_MARGIN >= self.scroll_row + view_rows {
            self.scroll_row = (self.cursor.line + SCROLL_MARGIN + 1).saturating_sub(view_rows);
        }

        // Horizontal ──────────────────────────────────────────────────────────
        if self.cursor.col < self.scroll_col {
            self.scroll_col = self.cursor.col;
        } else if self.cursor.col >= self.scroll_col + view_cols {
            self.scroll_col = self.cursor.col + 1 - view_cols;
        }
    }
}

// ── Editor ────────────────────────────────────────────────────────────────────

pub struct Editor {
    pub tabs:              Vec<Tab>,
    pub active_tab:        usize,
    pub show_line_numbers: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            tabs:              vec![Tab::new_empty()],
            active_tab:        0,
            show_line_numbers: true,
        }
    }

    // ── Tab management ────────────────────────────────────────────────────────

    pub fn open_file(&mut self, path: PathBuf, theme: &Theme) -> std::io::Result<()> {
        // Reuse existing tab if already open.
        for (i, t) in self.tabs.iter().enumerate() {
            if t.buffer.path.as_deref() == Some(&path) {
                self.active_tab = i;
                return Ok(());
            }
        }
        let buf = Buffer::from_path(path)?;
        self.tabs.push(Tab::from_buffer(buf, theme));
        self.active_tab = self.tabs.len() - 1;
        Ok(())
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(Tab::new_empty());
        self.active_tab = self.tabs.len() - 1;
    }

    /// Close the active tab.  If it was the only tab, replace it with an empty one.
    pub fn close_tab(&mut self) {
        if self.tabs.len() == 1 {
            self.tabs[0] = Tab::new_empty();
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn tab(&self)     -> &Tab     { &self.tabs[self.active_tab] }
    pub fn tab_mut(&mut self) -> &mut Tab { &mut self.tabs[self.active_tab] }

    // ── Editing ───────────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, c: char, theme: &Theme) {
        let tab      = &mut self.tabs[self.active_tab];
        let char_idx = tab.buffer.line_to_char(tab.cursor.line) + tab.cursor.char_col;

        tab.buffer.insert_str(char_idx, &c.to_string());

        if c == '\n' {
            tab.cursor.line    += 1;
            tab.cursor.char_col = 0;
            tab.cursor.col      = 0;
            tab.buffer.checkpoint();
        } else {
            tab.cursor.char_col += 1;
            tab.cursor.col      += 1;
            // Checkpoint at natural word boundaries for fine-grained undo.
            if matches!(c, ' ' | '.' | ',' | ';' | ':' | '!' | '?' | '{' | '}' | '(' | ')') {
                tab.buffer.checkpoint();
            }
        }

        tab.rehighlight(theme);
    }

    pub fn backspace(&mut self, theme: &Theme) {
        let tab      = &mut self.tabs[self.active_tab];
        let char_idx = tab.buffer.line_to_char(tab.cursor.line) + tab.cursor.char_col;
        if char_idx == 0 { return; }

        let deleted = tab.buffer.rope.char(char_idx - 1);
        tab.buffer.delete_range(char_idx - 1, 1);

        if deleted == '\n' {
            if tab.cursor.line > 0 {
                tab.cursor.line    -= 1;
                tab.cursor.char_col = tab.buffer.line_len_chars(tab.cursor.line);
                tab.cursor.col      = tab.cursor.char_col;
            }
            tab.buffer.checkpoint();
        } else {
            tab.cursor.char_col = tab.cursor.char_col.saturating_sub(1);
            tab.cursor.col      = tab.cursor.col.saturating_sub(1);
        }

        tab.rehighlight(theme);
    }

    pub fn delete_forward(&mut self, theme: &Theme) {
        let tab      = &mut self.tabs[self.active_tab];
        let char_idx = tab.buffer.line_to_char(tab.cursor.line) + tab.cursor.char_col;
        if char_idx >= tab.buffer.len_chars() { return; }
        tab.buffer.delete_range(char_idx, 1);
        tab.rehighlight(theme);
    }

    // ── Cursor movement ───────────────────────────────────────────────────────

    pub fn move_cursor(&mut self, dir: Direction, cols: u16, rows: u16) {
        let tab = &mut self.tabs[self.active_tab];
        let buf = &tab.buffer;

        match dir {
            Direction::Left => {
                if tab.cursor.char_col > 0 {
                    tab.cursor.char_col -= 1;
                    tab.cursor.col       = tab.cursor.char_col;
                } else if tab.cursor.line > 0 {
                    tab.cursor.line    -= 1;
                    let len             = buf.line_len_chars(tab.cursor.line);
                    tab.cursor.char_col = len;
                    tab.cursor.col      = len;
                }
            }
            Direction::Right => {
                let len = buf.line_len_chars(tab.cursor.line);
                if tab.cursor.char_col < len {
                    tab.cursor.char_col += 1;
                    tab.cursor.col       = tab.cursor.char_col;
                } else if tab.cursor.line + 1 < buf.len_lines() {
                    tab.cursor.line    += 1;
                    tab.cursor.char_col = 0;
                    tab.cursor.col      = 0;
                }
            }
            Direction::Up => {
                if tab.cursor.line > 0 {
                    tab.cursor.line    -= 1;
                    let len             = buf.line_len_chars(tab.cursor.line);
                    tab.cursor.char_col = tab.cursor.col.min(len);
                }
            }
            Direction::Down => {
                let nlines = buf.len_lines();
                if tab.cursor.line + 1 < nlines {
                    tab.cursor.line    += 1;
                    let len             = buf.line_len_chars(tab.cursor.line);
                    tab.cursor.char_col = tab.cursor.col.min(len);
                }
            }
            Direction::Home => {
                // Jump to first non-whitespace, then to column 0 on second press
                let line = buf.rope.line(tab.cursor.line).to_string();
                let first_non_ws = line.chars().take_while(|c| c.is_whitespace() && *c != '\n').count();
                if tab.cursor.char_col != first_non_ws {
                    tab.cursor.char_col = first_non_ws;
                } else {
                    tab.cursor.char_col = 0;
                }
                tab.cursor.col = tab.cursor.char_col;
            }
            Direction::End => {
                let len             = buf.line_len_chars(tab.cursor.line);
                tab.cursor.char_col = len;
                tab.cursor.col      = len;
            }
            Direction::PageUp => {
                let view = rows.saturating_sub(MENU_HEIGHT + TAB_BAR_HEIGHT + STATUS_HEIGHT + 1) as usize;
                tab.cursor.line    = tab.cursor.line.saturating_sub(view);
                let len             = buf.line_len_chars(tab.cursor.line);
                tab.cursor.char_col = tab.cursor.col.min(len);
            }
            Direction::PageDown => {
                let view   = rows.saturating_sub(MENU_HEIGHT + TAB_BAR_HEIGHT + STATUS_HEIGHT + 1) as usize;
                let max    = buf.len_lines().saturating_sub(1);
                tab.cursor.line    = (tab.cursor.line + view).min(max);
                let len             = buf.line_len_chars(tab.cursor.line);
                tab.cursor.char_col = tab.cursor.col.min(len);
            }
            Direction::WordLeft => {
                let abs = buf.line_to_char(tab.cursor.line) + tab.cursor.char_col;
                let new = word_boundary_left(&buf.rope, abs);
                let line = buf.char_to_line(new);
                let col  = new - buf.line_to_char(line);
                tab.cursor.line     = line;
                tab.cursor.char_col = col;
                tab.cursor.col      = col;
            }
            Direction::WordRight => {
                let abs  = buf.line_to_char(tab.cursor.line) + tab.cursor.char_col;
                let new  = word_boundary_right(&buf.rope, abs);
                let capped = new.min(buf.len_chars().saturating_sub(1));
                let line = buf.char_to_line(capped);
                let col  = capped - buf.line_to_char(line);
                tab.cursor.line     = line;
                tab.cursor.char_col = col;
                tab.cursor.col      = col;
            }
        }

        tab.ensure_cursor_visible(cols, rows);
    }

    // ── Undo / Redo ───────────────────────────────────────────────────────────

    pub fn undo(&mut self, theme: &Theme) {
        let tab = &mut self.tabs[self.active_tab];
        if let Some(pos) = tab.buffer.undo() {
            jump_to_char(tab, pos);
        }
        self.tabs[self.active_tab].rehighlight(theme);
    }

    pub fn redo(&mut self, theme: &Theme) {
        let tab = &mut self.tabs[self.active_tab];
        if let Some(pos) = tab.buffer.redo() {
            jump_to_char(tab, pos);
        }
        self.tabs[self.active_tab].rehighlight(theme);
    }

    // ── Go to line ────────────────────────────────────────────────────────────

    pub fn go_to_line(&mut self, one_based: usize, cols: u16, rows: u16) {
        let tab = &mut self.tabs[self.active_tab];
        let max = tab.buffer.len_lines().saturating_sub(1);
        tab.cursor.line     = one_based.saturating_sub(1).min(max);
        tab.cursor.char_col = 0;
        tab.cursor.col      = 0;
        tab.ensure_cursor_visible(cols, rows);
    }
}

// ── Direction ─────────────────────────────────────────────────────────────────

pub enum Direction {
    Left, Right, Up, Down,
    Home, End,
    PageUp, PageDown,
    WordLeft, WordRight,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn jump_to_char(tab: &mut Tab, pos: usize) {
    let max  = tab.buffer.len_chars().saturating_sub(1);
    let pos  = pos.min(max);
    let line = tab.buffer.char_to_line(pos);
    let col  = pos - tab.buffer.line_to_char(line);
    tab.cursor.line     = line;
    tab.cursor.char_col = col;
    tab.cursor.col      = col;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Theme;

    fn theme() -> Theme { Theme::default() }

    fn editor_with_text(text: &str) -> Editor {
        let mut ed = Editor::new();
        let t = theme();
        for c in text.chars() {
            ed.insert_char(c, &t);
        }
        ed
    }

    // ── Tab ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_tab_new_empty() {
        let tab = Tab::new_empty();
        assert_eq!(tab.buffer.len_chars(), 0);
        assert_eq!(tab.cursor.line, 0);
        assert_eq!(tab.cursor.char_col, 0);
        assert_eq!(tab.scroll_row, 0);
    }

    #[test]
    fn test_tab_from_buffer() {
        let buf = Buffer::from_path(PathBuf::from("Cargo.toml")).unwrap();
        let tab = Tab::from_buffer(buf, &theme());
        assert!(tab.buffer.len_chars() > 0);
        assert_eq!(tab.cursor.line, 0);
    }

    #[test]
    fn test_tab_rehighlight_does_not_panic() {
        let mut tab = Tab::new_empty();
        tab.buffer.insert_str(0, "fn main() {}");
        tab.rehighlight(&theme()); // just verify no panic
    }

    #[test]
    fn test_tab_ensure_cursor_visible_scrolls_down() {
        let mut tab = Tab::new_empty();
        // Insert enough lines so cursor.line=25 is valid.
        for _ in 0..30 {
            tab.buffer.insert_str(tab.buffer.len_chars(), "line\n");
        }
        tab.cursor.line = 25;
        tab.ensure_cursor_visible(80, 24);
        assert!(tab.scroll_row > 0, "scroll_row should advance when cursor is below viewport");
    }

    // ── Editor ────────────────────────────────────────────────────────────────

    #[test]
    fn test_editor_new() {
        let ed = Editor::new();
        assert_eq!(ed.tabs.len(), 1);
        assert_eq!(ed.active_tab, 0);
        assert!(ed.show_line_numbers);
    }

    #[test]
    fn test_editor_open_file() {
        let mut ed = Editor::new();
        ed.open_file(PathBuf::from("Cargo.toml"), &theme()).unwrap();
        assert_eq!(ed.tabs.len(), 2);
        assert_eq!(ed.active_tab, 1);
    }

    #[test]
    fn test_editor_open_file_reuses_existing_tab() {
        let mut ed = Editor::new();
        ed.open_file(PathBuf::from("Cargo.toml"), &theme()).unwrap();
        let tabs_after_first = ed.tabs.len();
        ed.open_file(PathBuf::from("Cargo.toml"), &theme()).unwrap();
        assert_eq!(ed.tabs.len(), tabs_after_first, "should not open a duplicate tab");
    }

    #[test]
    fn test_editor_new_tab() {
        let mut ed = Editor::new();
        ed.new_tab();
        assert_eq!(ed.tabs.len(), 2);
        assert_eq!(ed.active_tab, 1);
    }

    #[test]
    fn test_editor_close_tab_reduces_count() {
        let mut ed = Editor::new();
        ed.new_tab();
        assert_eq!(ed.tabs.len(), 2);
        ed.close_tab();
        assert_eq!(ed.tabs.len(), 1);
    }

    #[test]
    fn test_editor_close_last_tab_resets_to_empty() {
        let mut ed = editor_with_text("hello");
        assert_eq!(ed.tabs.len(), 1);
        ed.close_tab();
        assert_eq!(ed.tabs.len(), 1);
        assert_eq!(ed.tab().buffer.len_chars(), 0);
    }

    #[test]
    fn test_editor_next_tab_wraps() {
        let mut ed = Editor::new();
        ed.new_tab();
        ed.active_tab = 0;
        ed.next_tab();
        assert_eq!(ed.active_tab, 1);
        ed.next_tab();
        assert_eq!(ed.active_tab, 0, "should wrap back to 0");
    }

    #[test]
    fn test_editor_prev_tab_wraps() {
        let mut ed = Editor::new();
        ed.new_tab();
        ed.active_tab = 0;
        ed.prev_tab();
        assert_eq!(ed.active_tab, 1, "should wrap to last tab");
    }

    #[test]
    fn test_editor_tab_returns_active() {
        let mut ed = Editor::new();
        ed.new_tab();
        ed.active_tab = 1;
        ed.tab_mut().buffer.insert_str(0, "hello");
        assert_eq!(ed.tab().buffer.to_string(), "hello");
    }

    #[test]
    fn test_editor_tab_mut_mutates_active() {
        let mut ed = Editor::new();
        ed.tab_mut().buffer.insert_str(0, "x");
        assert_eq!(ed.tabs[0].buffer.to_string(), "x");
    }

    #[test]
    fn test_editor_insert_char_updates_buffer_and_cursor() {
        let mut ed = Editor::new();
        ed.insert_char('a', &theme());
        assert_eq!(ed.tab().buffer.to_string(), "a");
        assert_eq!(ed.tab().cursor.char_col, 1);
    }

    #[test]
    fn test_editor_insert_newline_advances_line() {
        let mut ed = editor_with_text("ab");
        ed.insert_char('\n', &theme());
        assert_eq!(ed.tab().cursor.line, 1);
        assert_eq!(ed.tab().cursor.char_col, 0);
    }

    #[test]
    fn test_editor_backspace_removes_char() {
        let mut ed = editor_with_text("ab");
        ed.backspace(&theme());
        assert_eq!(ed.tab().buffer.to_string(), "a");
        assert_eq!(ed.tab().cursor.char_col, 1);
    }

    #[test]
    fn test_editor_delete_forward_removes_char_at_cursor() {
        let mut ed = editor_with_text("ab");
        ed.tab_mut().cursor.char_col = 0;
        ed.tab_mut().cursor.col = 0;
        ed.delete_forward(&theme());
        assert_eq!(ed.tab().buffer.to_string(), "b");
    }

    #[test]
    fn test_editor_move_cursor_right() {
        let mut ed = editor_with_text("abc");
        ed.tab_mut().cursor.char_col = 0;
        ed.tab_mut().cursor.col = 0;
        ed.move_cursor(Direction::Right, 80, 24);
        assert_eq!(ed.tab().cursor.char_col, 1);
    }

    #[test]
    fn test_editor_undo_reverts_insert() {
        let mut ed = editor_with_text("hi");
        ed.undo(&theme());
        assert_eq!(ed.tab().buffer.to_string(), "");
    }

    #[test]
    fn test_editor_redo_reapplies_insert() {
        let mut ed = editor_with_text("hi");
        ed.undo(&theme());
        ed.redo(&theme());
        assert_eq!(ed.tab().buffer.to_string(), "hi");
    }

    #[test]
    fn test_editor_go_to_line() {
        let mut ed = editor_with_text("line1\nline2\nline3");
        ed.go_to_line(2, 80, 24);
        assert_eq!(ed.tab().cursor.line, 1);
        assert_eq!(ed.tab().cursor.char_col, 0);
    }
}

fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn word_boundary_left(rope: &ropey::Rope, char_idx: usize) -> usize {
    if char_idx == 0 { return 0; }
    let mut i = char_idx - 1;
    // Skip whitespace to the left.
    while i > 0 && rope.char(i).is_whitespace() { i -= 1; }
    let pivot = rope.char(i);
    if is_word_char(pivot) {
        while i > 0 && is_word_char(rope.char(i - 1)) { i -= 1; }
    } else {
        while i > 0 && !rope.char(i - 1).is_whitespace() && !is_word_char(rope.char(i - 1)) {
            i -= 1;
        }
    }
    i
}

fn word_boundary_right(rope: &ropey::Rope, char_idx: usize) -> usize {
    let len = rope.len_chars();
    if char_idx >= len { return len; }
    let mut i = char_idx;
    // Skip whitespace to the right.
    while i < len && rope.char(i).is_whitespace() { i += 1; }
    if i < len && is_word_char(rope.char(i)) {
        while i < len && is_word_char(rope.char(i)) { i += 1; }
    } else {
        while i < len && !rope.char(i).is_whitespace() && !is_word_char(rope.char(i)) {
            i += 1;
        }
    }
    i
}

use ropey::Rope;
use std::path::PathBuf;

// ── Edit operations (for undo / redo) ─────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum EditOp {
    /// Text was inserted at `char_idx`.
    Insert { char_idx: usize, text: String },
    /// Text was deleted starting at `char_idx`.
    Delete { char_idx: usize, text: String },
}

impl EditOp {
    fn invert(&self) -> Self {
        match self {
            Self::Insert { char_idx, text } => Self::Delete { char_idx: *char_idx, text: text.clone() },
            Self::Delete { char_idx, text } => Self::Insert { char_idx: *char_idx, text: text.clone() },
        }
    }
}

// ── Buffer ────────────────────────────────────────────────────────────────────

/// A single open document backed by a `ropey::Rope`.
pub struct Buffer {
    pub rope:     Rope,
    pub path:     Option<PathBuf>,
    pub is_dirty: bool,
    /// Detected language name (e.g. "rust", "python").
    pub language: Option<String>,

    // History stacks hold *groups* of operations that are undone/redone atomically.
    undo_stack:    Vec<Vec<EditOp>>,
    redo_stack:    Vec<Vec<EditOp>>,
    // Accumulates edits until the next checkpoint() call.
    pending_group: Vec<EditOp>,
}

impl Buffer {
    // ── Constructors ──────────────────────────────────────────────────────────

    pub fn new() -> Self {
        Self {
            rope:          Rope::new(),
            path:          None,
            is_dirty:      false,
            language:      None,
            undo_stack:    Vec::new(),
            redo_stack:    Vec::new(),
            pending_group: Vec::new(),
        }
    }

    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let text     = std::fs::read_to_string(&path)?;
        let language = detect_language(&path);
        Ok(Self {
            rope: Rope::from_str(&text),
            path: Some(path),
            is_dirty: false,
            language,
            undo_stack:    Vec::new(),
            redo_stack:    Vec::new(),
            pending_group: Vec::new(),
        })
    }

    // ── Identity ──────────────────────────────────────────────────────────────

    pub fn name(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
    }

    // ── Primitive edits ───────────────────────────────────────────────────────

    /// Insert `text` at character index `char_idx`.
    pub fn insert_str(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
        self.pending_group.push(EditOp::Insert {
            char_idx,
            text: text.to_string(),
        });
        self.is_dirty = true;
        self.redo_stack.clear();
    }

    /// Delete `char_count` characters starting at `char_idx`.
    pub fn delete_range(&mut self, char_idx: usize, char_count: usize) {
        if char_count == 0 { return; }
        let end = (char_idx + char_count).min(self.rope.len_chars());
        if char_idx >= end { return; }
        let deleted: String = self.rope.slice(char_idx..end).to_string();
        self.rope.remove(char_idx..end);
        self.pending_group.push(EditOp::Delete {
            char_idx,
            text: deleted,
        });
        self.is_dirty = true;
        self.redo_stack.clear();
    }

    /// Replace the entire buffer contents (used by Replace All).
    pub fn replace_all_content(&mut self, new_text: &str) {
        self.rope = Rope::from_str(new_text);
        self.is_dirty = true;
        // Drop history — Replace All is not individually reversible.
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.pending_group.clear();
    }

    // ── Undo / Redo ───────────────────────────────────────────────────────────

    /// Flush the pending edit group onto the undo stack.
    /// Call this at natural edit boundaries (Enter, space, delimiter, explicit save-point).
    pub fn checkpoint(&mut self) {
        if self.pending_group.is_empty() { return; }
        let group = std::mem::take(&mut self.pending_group);
        self.undo_stack.push(group);
        // Cap history to avoid unbounded memory growth.
        const MAX_HISTORY: usize = 2_000;
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the most recent committed edit group.
    /// Returns the character index the cursor should jump to (if any).
    pub fn undo(&mut self) -> Option<usize> {
        self.checkpoint(); // flush any pending edits first
        let group = self.undo_stack.pop()?;
        let mut cursor_char = None;
        let mut redo_group  = Vec::with_capacity(group.len());

        for op in group.iter().rev() {
            match op {
                EditOp::Insert { char_idx, text } => {
                    let count = text.chars().count();
                    self.rope.remove(*char_idx..*char_idx + count);
                    redo_group.push(EditOp::Delete { char_idx: *char_idx, text: text.clone() });
                    cursor_char = Some(*char_idx);
                }
                EditOp::Delete { char_idx, text } => {
                    self.rope.insert(*char_idx, text);
                    redo_group.push(EditOp::Insert { char_idx: *char_idx, text: text.clone() });
                    cursor_char = Some(*char_idx + text.chars().count());
                }
            }
        }

        redo_group.reverse();
        self.redo_stack.push(redo_group);
        self.is_dirty = true;
        cursor_char
    }

    /// Redo the most recently undone edit group.
    /// Returns the character index the cursor should jump to (if any).
    pub fn redo(&mut self) -> Option<usize> {
        let group = self.redo_stack.pop()?;
        let mut cursor_char = None;
        let mut undo_group  = Vec::with_capacity(group.len());

        for op in &group {
            match op {
                EditOp::Insert { char_idx, text } => {
                    self.rope.insert(*char_idx, text);
                    undo_group.push(EditOp::Delete { char_idx: *char_idx, text: text.clone() });
                    cursor_char = Some(*char_idx + text.chars().count());
                }
                EditOp::Delete { char_idx, text } => {
                    let count = text.chars().count();
                    self.rope.remove(*char_idx..*char_idx + count);
                    undo_group.push(EditOp::Insert { char_idx: *char_idx, text: text.clone() });
                    cursor_char = Some(*char_idx);
                }
            }
        }

        self.undo_stack.push(undo_group);
        self.is_dirty = true;
        cursor_char
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    pub fn save(&mut self) -> std::io::Result<()> {
        let path = self.path.clone().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No file path set — use Save As")
        })?;
        std::fs::write(&path, self.rope.to_string())?;
        self.is_dirty = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.language = detect_language(&path);
        self.path     = Some(path);
        self.save()
    }

    // ── Rope helpers ──────────────────────────────────────────────────────────

    pub fn len_chars(&self) -> usize { self.rope.len_chars() }
    pub fn len_lines(&self) -> usize { self.rope.len_lines() }
    pub fn char_to_line(&self, c: usize) -> usize { self.rope.char_to_line(c) }
    pub fn line_to_char(&self, l: usize) -> usize { self.rope.line_to_char(l) }

    /// Length of line `l` in characters, *excluding* trailing `\r\n`.
    pub fn line_len_chars(&self, l: usize) -> usize {
        if l >= self.rope.len_lines() { return 0; }
        let s   = self.rope.line(l);
        let mut len = s.len_chars();
        while len > 0 {
            let last = s.char(len - 1);
            if last == '\n' || last == '\r' { len -= 1; } else { break; }
        }
        len
    }

    /// Clone the full text as a `String` (needed by tree-sitter and search).
    pub fn to_string(&self) -> String { self.rope.to_string() }
}

// ── Language detection ────────────────────────────────────────────────────────

fn detect_language(path: &PathBuf) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    Some(match ext {
        "rs"                  => "rust",
        "py" | "pyw"          => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "c"  | "h"            => "c",
        "sh" | "bash"         => "bash",
        "toml"                => "toml",
        _                     => return None,
    }.to_string())
}

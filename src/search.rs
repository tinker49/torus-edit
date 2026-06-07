/// Which text field in the search bar has keyboard focus.
#[derive(Default, PartialEq, Clone, Copy)]
pub enum SearchFocus {
    #[default]
    Query,
    Replace,
}

/// All state for the search / replace widget.
#[derive(Default)]
pub struct SearchState {
    pub query:        String,
    pub replace_text: String,
    pub active:       bool,
    pub replace_mode: bool,
    pub focus:        SearchFocus,

    /// Pre-computed (start_char, end_char) matches within the current buffer.
    pub matches:       Vec<(usize, usize)>,
    /// Index into `matches` that is currently highlighted.
    pub current_match: usize,
}

impl SearchState {
    // ── Lifecycle ─────────────────────────────────────────────────────────────

    pub fn open(&mut self) {
        self.active       = true;
        self.replace_mode = false;
        self.focus        = SearchFocus::Query;
        self.current_match = 0;
    }

    pub fn open_replace(&mut self) {
        self.active       = true;
        self.replace_mode = true;
        self.focus        = SearchFocus::Query;
        self.current_match = 0;
    }

    pub fn close(&mut self) {
        self.active  = false;
        self.matches.clear();
        self.current_match = 0;
    }

    // ── Match management ──────────────────────────────────────────────────────

    /// Re-scan the full text for the current query (case-insensitive).
    /// Call whenever the query changes or the buffer content changes.
    pub fn update_matches(&mut self, text: &str) {
        self.matches.clear();
        if self.query.is_empty() { return; }

        let q_lower: Vec<char> = self.query.to_lowercase().chars().collect();
        let t_lower: Vec<char> = text.to_lowercase().chars().collect();
        let qlen = q_lower.len();
        let tlen = t_lower.len();

        let mut i = 0;
        while i + qlen <= tlen {
            if t_lower[i..i + qlen] == q_lower[..] {
                self.matches.push((i, i + qlen));
                i += qlen; // non-overlapping matches
            } else {
                i += 1;
            }
        }

        // Keep current_match in bounds.
        if self.current_match >= self.matches.len() {
            self.current_match = 0;
        }
    }

    pub fn next_match(&mut self) {
        if self.matches.is_empty() { return; }
        self.current_match = (self.current_match + 1) % self.matches.len();
    }

    pub fn prev_match(&mut self) {
        if self.matches.is_empty() { return; }
        self.current_match = if self.current_match == 0 {
            self.matches.len() - 1
        } else {
            self.current_match - 1
        };
    }

    /// True if `char_idx` falls inside any match; also returns whether it is
    /// the *current* highlighted match.
    pub fn match_at(&self, char_idx: usize) -> Option<bool> {
        // Binary search: find first match whose end > char_idx
        let pos = self.matches.partition_point(|&(_, e)| e <= char_idx);
        if let Some(&(s, e)) = self.matches.get(pos) {
            if char_idx >= s && char_idx < e {
                return Some(pos == self.current_match);
            }
        }
        None
    }
}

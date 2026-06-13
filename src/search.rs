/// Which text field in the search bar has keyboard focus.
#[derive(Default, PartialEq, Clone, Copy, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_matches(query: &str, text: &str) -> SearchState {
        let mut s = SearchState::default();
        s.open();
        s.query = query.to_string();
        s.update_matches(text);
        s
    }

    #[test]
    fn test_open_activates_find_mode() {
        let mut s = SearchState::default();
        s.open();
        assert!(s.active);
        assert!(!s.replace_mode);
        assert_eq!(s.focus, SearchFocus::Query);
        assert_eq!(s.current_match, 0);
    }

    #[test]
    fn test_open_replace_activates_replace_mode() {
        let mut s = SearchState::default();
        s.open_replace();
        assert!(s.active);
        assert!(s.replace_mode);
        assert_eq!(s.focus, SearchFocus::Query);
    }

    #[test]
    fn test_close_deactivates_and_clears_matches() {
        let mut s = state_with_matches("hi", "hi there hi");
        s.close();
        assert!(!s.active);
        assert!(s.matches.is_empty());
        assert_eq!(s.current_match, 0);
    }

    #[test]
    fn test_update_matches_finds_all_occurrences() {
        let mut s = SearchState::default();
        s.query = "ab".to_string();
        s.update_matches("ab cd ab ef ab");
        assert_eq!(s.matches.len(), 3);
    }

    #[test]
    fn test_update_matches_is_case_insensitive() {
        let mut s = SearchState::default();
        s.query = "hello".to_string();
        s.update_matches("Hello HELLO hello");
        assert_eq!(s.matches.len(), 3);
    }

    #[test]
    fn test_update_matches_empty_query_produces_no_matches() {
        let mut s = SearchState::default();
        s.query = String::new();
        s.update_matches("some text");
        assert!(s.matches.is_empty());
    }

    #[test]
    fn test_update_matches_non_overlapping() {
        let mut s = SearchState::default();
        s.query = "aa".to_string();
        s.update_matches("aaaa"); // "aa" at 0..2, then "aa" at 2..4 (non-overlapping)
        assert_eq!(s.matches.len(), 2);
        assert_eq!(s.matches[0], (0, 2));
        assert_eq!(s.matches[1], (2, 4));
    }

    #[test]
    fn test_next_match_advances_index() {
        let mut s = state_with_matches("x", "x y x y x");
        assert_eq!(s.current_match, 0);
        s.next_match();
        assert_eq!(s.current_match, 1);
    }

    #[test]
    fn test_next_match_wraps_to_first() {
        let mut s = state_with_matches("x", "x y x");
        s.current_match = 1;
        s.next_match();
        assert_eq!(s.current_match, 0);
    }

    #[test]
    fn test_prev_match_decrements_index() {
        let mut s = state_with_matches("x", "x y x y x");
        s.current_match = 2;
        s.prev_match();
        assert_eq!(s.current_match, 1);
    }

    #[test]
    fn test_prev_match_wraps_to_last() {
        let mut s = state_with_matches("x", "x y x");
        s.current_match = 0;
        s.prev_match();
        assert_eq!(s.current_match, 1);
    }

    #[test]
    fn test_match_at_returns_some_inside_match() {
        let s = state_with_matches("hi", "say hi now");
        // "hi" starts at char 4
        assert!(s.match_at(4).is_some());
        assert!(s.match_at(5).is_some());
    }

    #[test]
    fn test_match_at_returns_none_outside_match() {
        let s = state_with_matches("hi", "say hi now");
        assert!(s.match_at(0).is_none());
        assert!(s.match_at(6).is_none());
    }

    #[test]
    fn test_match_at_distinguishes_current_match() {
        let mut s = state_with_matches("x", "x y x");
        s.current_match = 0;
        assert_eq!(s.match_at(0), Some(true));  // current
        assert_eq!(s.match_at(4), Some(false)); // non-current
    }
}

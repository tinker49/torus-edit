// ── Actions ───────────────────────────────────────────────────────────────────

/// Every action that can be triggered from the menu bar.
/// The input handler maps both keyboard shortcuts and menu selections to these.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuAction {
    // File menu
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    CloseTab,
    Quit,
    // Edit menu
    Undo,
    Redo,
    Find,
    Replace,
    GoToLine,
    // View menu
    ToggleLineNumbers,
    NextTab,
    PrevTab,
}

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MenuItem {
    pub label:    String,
    pub shortcut: Option<String>,
    pub action:   MenuAction,
}

#[derive(Clone)]
pub struct Menu {
    pub label: String,
    /// Alt + this character opens the menu.
    pub key:   char,
    pub items: Vec<MenuItem>,
}

// ── MenuBar state machine ─────────────────────────────────────────────────────

pub struct MenuBar {
    pub menus:    Vec<Menu>,
    /// Index into `menus` of the currently open dropdown, or `None`.
    pub open:     Option<usize>,
    /// Selected row within the open dropdown.
    pub selected: usize,
}

impl MenuBar {
    pub fn new() -> Self {
        let item = |label: &str, shortcut: Option<&str>, action: MenuAction| MenuItem {
            label:    label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
            action,
        };

        let menus = vec![
            Menu {
                label: "File".into(),
                key:   'f',
                items: vec![
                    item("New File",  Some("Ctrl+N"), MenuAction::NewFile),
                    item("Open…",     Some("Ctrl+O"), MenuAction::OpenFile),
                    item("Save",      Some("Ctrl+S"), MenuAction::Save),
                    item("Save As…",  Some("Ctrl+D"), MenuAction::SaveAs),
                    item("Close Tab", Some("Ctrl+W"), MenuAction::CloseTab),
                    item("Quit",      Some("Ctrl+Q"), MenuAction::Quit),
                ],
            },
            Menu {
                label: "Edit".into(),
                key:   'e',
                items: vec![
                    item("Undo",        Some("Ctrl+Z"), MenuAction::Undo),
                    item("Redo",        Some("Ctrl+Y"), MenuAction::Redo),
                    item("Find",        Some("Ctrl+F"), MenuAction::Find),
                    item("Replace",     Some("Ctrl+H"), MenuAction::Replace),
                    item("Go to Line",  Some("Ctrl+G"), MenuAction::GoToLine),
                ],
            },
            Menu {
                label: "View".into(),
                key:   'v',
                items: vec![
                    item("Toggle Line Numbers", None,                   MenuAction::ToggleLineNumbers),
                    item("Next Tab",            Some("Ctrl+Right"),     MenuAction::NextTab),
                    item("Prev Tab",            Some("Ctrl+Left"),      MenuAction::PrevTab),
                ],
            },
        ];

        Self { menus, open: None, selected: 0 }
    }

    // ── State transitions ─────────────────────────────────────────────────────

    /// Open the menu whose `key == ch`.  Returns `true` if found.
    pub fn open_by_key(&mut self, ch: char) -> bool {
        for (i, m) in self.menus.iter().enumerate() {
            if m.key == ch {
                self.open     = Some(i);
                self.selected = 0;
                return true;
            }
        }
        false
    }

    pub fn close(&mut self) {
        self.open     = None;
        self.selected = 0;
    }

    pub fn is_open(&self) -> bool { self.open.is_some() }

    pub fn move_up(&mut self) {
        if let Some(idx) = self.open {
            let len = self.menus[idx].items.len();
            self.selected = if self.selected == 0 { len - 1 } else { self.selected - 1 };
        }
    }

    pub fn move_down(&mut self) {
        if let Some(idx) = self.open {
            let len      = self.menus[idx].items.len();
            self.selected = (self.selected + 1) % len;
        }
    }

    pub fn move_left(&mut self) {
        if let Some(idx) = self.open {
            let new       = if idx == 0 { self.menus.len() - 1 } else { idx - 1 };
            self.open     = Some(new);
            self.selected = 0;
        }
    }

    pub fn move_right(&mut self) {
        if let Some(idx) = self.open {
            let new       = (idx + 1) % self.menus.len();
            self.open     = Some(new);
            self.selected = 0;
        }
    }

    /// Confirm the highlighted item, close the dropdown, return the action.
    pub fn activate(&mut self) -> Option<MenuAction> {
        let idx    = self.open?;
        let action = self.menus[idx].items.get(self.selected)?.action.clone();
        self.close();
        Some(action)
    }

    // ── Geometry helpers (used by the renderer) ───────────────────────────────

    /// Column at which menu `i` starts on the menu bar (1-indexed, 0 = left padding).
    pub fn menu_x(&self, i: usize) -> u16 {
        let mut x = 1u16;
        for j in 0..i {
            x += self.menus[j].label.len() as u16 + 2; // " Label "
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_starts_closed() {
        let mb = MenuBar::new();
        assert!(!mb.is_open());
        assert_eq!(mb.selected, 0);
        assert!(!mb.menus.is_empty());
    }

    #[test]
    fn test_open_by_key_known_opens_correct_menu() {
        let mut mb = MenuBar::new();
        assert!(mb.open_by_key('f'));
        assert_eq!(mb.open, Some(0));
        assert_eq!(mb.selected, 0);
    }

    #[test]
    fn test_open_by_key_unknown_returns_false() {
        let mut mb = MenuBar::new();
        assert!(!mb.open_by_key('z'));
        assert!(!mb.is_open());
    }

    #[test]
    fn test_close_resets_state() {
        let mut mb = MenuBar::new();
        mb.open_by_key('e');
        mb.close();
        assert!(!mb.is_open());
        assert_eq!(mb.selected, 0);
    }

    #[test]
    fn test_is_open_reflects_state() {
        let mut mb = MenuBar::new();
        assert!(!mb.is_open());
        mb.open_by_key('v');
        assert!(mb.is_open());
        mb.close();
        assert!(!mb.is_open());
    }

    #[test]
    fn test_move_up_wraps_to_last_item() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f'); // File menu has 6 items
        assert_eq!(mb.selected, 0);
        mb.move_up();
        assert_eq!(mb.selected, mb.menus[0].items.len() - 1);
    }

    #[test]
    fn test_move_down_advances_selection() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f');
        mb.move_down();
        assert_eq!(mb.selected, 1);
    }

    #[test]
    fn test_move_down_wraps_to_first_item() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f');
        let len = mb.menus[0].items.len();
        mb.selected = len - 1;
        mb.move_down();
        assert_eq!(mb.selected, 0);
    }

    #[test]
    fn test_move_left_switches_to_previous_menu() {
        let mut mb = MenuBar::new();
        mb.open_by_key('e'); // Edit = index 1
        mb.move_left();
        assert_eq!(mb.open, Some(0)); // File
        assert_eq!(mb.selected, 0);
    }

    #[test]
    fn test_move_left_wraps_from_first_to_last_menu() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f'); // File = index 0
        mb.move_left();
        assert_eq!(mb.open, Some(mb.menus.len() - 1));
    }

    #[test]
    fn test_move_right_switches_to_next_menu() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f'); // File = index 0
        mb.move_right();
        assert_eq!(mb.open, Some(1)); // Edit
        assert_eq!(mb.selected, 0);
    }

    #[test]
    fn test_move_right_wraps_from_last_to_first_menu() {
        let mut mb = MenuBar::new();
        mb.open_by_key('v'); // View = last menu
        mb.move_right();
        assert_eq!(mb.open, Some(0)); // File
    }

    #[test]
    fn test_activate_returns_action_and_closes() {
        let mut mb = MenuBar::new();
        mb.open_by_key('f');
        mb.selected = 0; // "New File"
        let action = mb.activate();
        assert_eq!(action, Some(MenuAction::NewFile));
        assert!(!mb.is_open());
    }

    #[test]
    fn test_activate_when_closed_returns_none() {
        let mut mb = MenuBar::new();
        assert_eq!(mb.activate(), None);
    }

    #[test]
    fn test_menu_x_first_menu_starts_at_one() {
        let mb = MenuBar::new();
        assert_eq!(mb.menu_x(0), 1);
    }

    #[test]
    fn test_menu_x_second_menu_offset() {
        let mb = MenuBar::new();
        // "File" = 4 chars + 2 padding → offset = 1 + 6 = 7
        assert_eq!(mb.menu_x(1), 1 + mb.menus[0].label.len() as u16 + 2);
    }
}

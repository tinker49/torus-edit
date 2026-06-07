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

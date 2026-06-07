// ── Layout constants ──────────────────────────────────────────────────────────
/// Height of the menu bar row (top of screen).
pub const MENU_HEIGHT: u16 = 1;
/// Height of the tab bar row (below menu).
pub const TAB_BAR_HEIGHT: u16 = 1;
/// Height of the status bar row (bottom of screen).
pub const STATUS_HEIGHT: u16 = 1;
/// Width of the line-number gutter (supports up to 9 999 lines: "9999 ").
pub const GUTTER_WIDTH: u16 = 5;
/// Vertical scroll margin: keep cursor this many lines from the top/bottom edge.
pub const SCROLL_MARGIN: usize = 3;

// ── Color theme ───────────────────────────────────────────────────────────────
use crossterm::style::Color;

/// All colours used by the renderer.  Based on Catppuccin Mocha.
#[derive(Clone)]
pub struct Theme {
    // Editor area
    pub editor_bg:            Color,
    pub editor_fg:            Color,
    // Gutter (line numbers)
    pub gutter_bg:            Color,
    pub gutter_fg:            Color,
    pub gutter_active_fg:     Color,
    // Menu bar
    pub menu_bg:              Color,
    pub menu_fg:              Color,
    pub menu_active_bg:       Color,
    pub menu_active_fg:       Color,
    // Dropdown menus
    pub dropdown_bg:          Color,
    pub dropdown_fg:          Color,
    pub dropdown_selected_bg: Color,
    pub dropdown_selected_fg: Color,
    pub dropdown_shortcut_fg: Color,
    // Tab bar
    pub tab_bg:               Color,
    pub tab_active_bg:        Color,
    pub tab_active_fg:        Color,
    pub tab_inactive_fg:      Color,
    pub tab_dirty_indicator:  Color,
    // Status bar
    pub status_bg:            Color,
    pub status_fg:            Color,
    // Search / selection
    pub search_match_bg:      Color,
    pub search_current_bg:    Color,
    pub search_bar_bg:        Color,
    pub search_bar_fg:        Color,
    // Syntax highlighting
    pub syn_keyword:          Color,
    pub syn_string:           Color,
    pub syn_comment:          Color,
    pub syn_number:           Color,
    pub syn_function:         Color,
    pub syn_type:             Color,
    pub syn_operator:         Color,
    pub syn_punctuation:      Color,
    pub syn_attribute:        Color,
    pub syn_constant:         Color,
    pub syn_variable:         Color,
}

impl Default for Theme {
    fn default() -> Self {
        // Catppuccin Mocha palette
        let rgb = |r, g, b| Color::Rgb { r, g, b };
        Self {
            editor_bg:            rgb(30,  30,  46),
            editor_fg:            rgb(205, 214, 244),
            gutter_bg:            rgb(24,  24,  37),
            gutter_fg:            rgb(88,  91,  112),
            gutter_active_fg:     rgb(166, 173, 200),
            menu_bg:              rgb(24,  24,  37),
            menu_fg:              rgb(205, 214, 244),
            menu_active_bg:       rgb(137, 180, 250),
            menu_active_fg:       rgb(24,  24,  37),
            dropdown_bg:          rgb(36,  39,  58),
            dropdown_fg:          rgb(205, 214, 244),
            dropdown_selected_bg: rgb(137, 180, 250),
            dropdown_selected_fg: rgb(24,  24,  37),
            dropdown_shortcut_fg: rgb(166, 227, 161),
            tab_bg:               rgb(24,  24,  37),
            tab_active_bg:        rgb(30,  30,  46),
            tab_active_fg:        rgb(205, 214, 244),
            tab_inactive_fg:      rgb(88,  91,  112),
            tab_dirty_indicator:  rgb(243, 139, 168),
            status_bg:            rgb(137, 180, 250),
            status_fg:            rgb(24,  24,  37),
            search_match_bg:      rgb(249, 226, 175),
            search_current_bg:    rgb(250, 179, 135),
            search_bar_bg:        rgb(36,  39,  58),
            search_bar_fg:        rgb(205, 214, 244),
            syn_keyword:          rgb(203, 166, 247),
            syn_string:           rgb(166, 227, 161),
            syn_comment:          rgb(88,  91,  112),
            syn_number:           rgb(250, 179, 135),
            syn_function:         rgb(137, 180, 250),
            syn_type:             rgb(249, 226, 175),
            syn_operator:         rgb(137, 220, 235),
            syn_punctuation:      rgb(166, 173, 200),
            syn_attribute:        rgb(245, 194, 231),
            syn_constant:         rgb(250, 179, 135),
            syn_variable:         rgb(205, 214, 244),
        }
    }
}

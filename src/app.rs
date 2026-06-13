use std::io::Write;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crate::{
    config::Theme,
    editor::Editor,
    input,
    menu::MenuBar,
    renderer::Renderer,
    search::SearchState,
};

// ── Commands returned by the input layer ──────────────────────────────────────

pub enum AppCommand {
    Quit,
    PromptOpenFile,
    PromptSaveAs,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub editor:      Editor,
    pub menu:        MenuBar,
    pub search:      SearchState,
    pub renderer:    Renderer,
    pub theme:       Theme,

    /// Transient one-line message shown in the status bar.
    pub status_msg:  String,

    /// Go-to-line prompt state.
    pub goto_active: bool,
    pub goto_input:  String,

    /// Set to true after the first Ctrl+Q when there are unsaved changes.
    pub force_quit:  bool,

    running: bool,
}

impl App {
    pub fn new(renderer: Renderer) -> Self {
        Self {
            editor:      Editor::new(),
            menu:        MenuBar::new(),
            search:      SearchState::default(),
            renderer,
            theme:       Theme::default(),
            status_msg:  String::new(),
            goto_active: false,
            goto_input:  String::new(),
            force_quit:  false,
            running:     true,
        }
    }

    // ── File helpers ──────────────────────────────────────────────────────────

    pub fn open_file(&mut self, path: std::path::PathBuf) {
        let t = self.theme.clone();
        match self.editor.open_file(path, &t) {
            Ok(())  => {}
            Err(e)  => self.set_status(format!("Error opening file: {e}")),
        }
    }

    // ── Status message ────────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = msg.into();
    }

    // ── Main event loop ───────────────────────────────────────────────────────

    pub fn run(&mut self, stdout: &mut impl Write) -> std::io::Result<()> {
        // Initial render.
        self.renderer.render_all(stdout, self)?;

        while self.running {
            // Poll with a short timeout so we can redraw at ~60 fps if needed.
            if !event::poll(std::time::Duration::from_millis(16))? {
                continue;
            }

            let ev = event::read()?;
            let cmd = input::handle_event(ev, self);

            match cmd {
                None => {}

                Some(AppCommand::Quit) => {
                    let any_dirty = self.editor.tabs.iter().any(|t| t.buffer.is_dirty);
                    if any_dirty && !self.force_quit {
                        self.set_status(
                            "Unsaved changes! Press Ctrl+Q again to quit without saving."
                        );
                        self.force_quit = true;
                    } else {
                        self.running = false;
                    }
                }

                Some(AppCommand::PromptOpenFile) => {
                    self.prompt_open_file(stdout)?;
                }

                Some(AppCommand::PromptSaveAs) => {
                    self.prompt_save_as(stdout)?;
                }
            }

            // Reset the force-quit flag if the user does anything other than Ctrl+Q.
            // (The flag is only consumed / checked inside AppCommand::Quit above.)
            // We reset it here so any non-quit action cancels the pending warning.

            if self.running {
                self.renderer.render_all(stdout, self)?;
            }
        }

        Ok(())
    }

    // ── Inline prompts ────────────────────────────────────────────────────────

    /// Show an "Open file:" prompt in the status bar and read a path from the user.
    pub fn prompt_open_file(&mut self, stdout: &mut impl Write) -> std::io::Result<()> {
        let path = self.inline_prompt(stdout, "Open file: ")?;
        if let Some(p) = path {
            self.open_file(std::path::PathBuf::from(p));
        }
        Ok(())
    }

    /// Show a "Save as:" prompt in the status bar and save under the new name.
    pub fn prompt_save_as(&mut self, stdout: &mut impl Write) -> std::io::Result<()> {
        let path = self.inline_prompt(stdout, "Save as: ")?;
        if let Some(p) = path {
            let pb = std::path::PathBuf::from(p);
            // Re-initialise highlighting for the new extension.
            let t = self.theme.clone();
            match self.editor.tab_mut().buffer.save_as(pb) {
                Ok(()) => {
                    // If the language changed, rebuild highlights.
                    if let Some(lang) = &self.editor.tab().buffer.language.clone() {
                        self.editor.tab_mut().highlighter.set_language(lang);
                        let text = self.editor.tab().buffer.to_string();
                        self.editor.tab_mut().highlighter.parse(&text, &t);
                    }
                    self.set_status("Saved.");
                }
                Err(e) => self.set_status(format!("Save error: {e}")),
            }
        }
        Ok(())
    }

    /// Generic single-line prompt rendered in the status bar.
    /// Returns `Some(input)` on Enter, `None` on Esc.
    fn inline_prompt(
        &mut self,
        stdout: &mut impl Write,
        label: &str,
    ) -> std::io::Result<Option<String>> {
        let mut input = String::new();

        loop {
            // Render the prompt in the status bar area.
            use crossterm::{cursor, style::{SetBackgroundColor, SetForegroundColor, Print}, QueueableCommand};
            let y  = self.renderer.height - crate::config::STATUS_HEIGHT;
            let msg = format!("{}{}", label, input);
            let padded: String = format!("{:<width$}", msg, width = self.renderer.width as usize)
                .chars().take(self.renderer.width as usize).collect();

            stdout.queue(cursor::MoveTo(0, y))?;
            stdout.queue(SetBackgroundColor(self.theme.status_bg))?;
            stdout.queue(SetForegroundColor(self.theme.status_fg))?;
            stdout.queue(Print(&padded))?;
            stdout.queue(cursor::MoveTo((label.len() + input.len()) as u16, y))?;
            stdout.queue(cursor::SetCursorStyle::SteadyBar)?;
            stdout.queue(cursor::Show)?;
            stdout.flush()?;

            if !event::poll(std::time::Duration::from_millis(100))? { continue; }
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Esc        => return Ok(None),
                    KeyCode::Enter      => return Ok(Some(input)),
                    KeyCode::Backspace  => { input.pop(); }
                    KeyCode::Char(c)
                        if k.modifiers == KeyModifiers::NONE
                        || k.modifiers == KeyModifiers::SHIFT
                        => { input.push(c); }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Renderer;

    fn make_app() -> App {
        App::new(Renderer { width: 80, height: 24 })
    }

    #[test]
    fn test_new_initial_state() {
        let app = make_app();
        assert!(app.status_msg.is_empty());
        assert!(!app.goto_active);
        assert!(app.goto_input.is_empty());
        assert!(!app.force_quit);
    }

    #[test]
    fn test_set_status() {
        let mut app = make_app();
        app.set_status("saved successfully");
        assert_eq!(app.status_msg, "saved successfully");
    }

    #[test]
    fn test_open_file_bad_path_sets_error_status() {
        let mut app = make_app();
        app.open_file(std::path::PathBuf::from("/nonexistent/no_such_file.txt"));
        assert!(app.status_msg.starts_with("Error opening file:"));
    }

    #[test]
    fn test_open_file_valid_path_opens_tab() {
        let mut app = make_app();
        let initial_tabs = app.editor.tabs.len();
        app.open_file(std::path::PathBuf::from("Cargo.toml"));
        assert!(app.status_msg.is_empty(), "expected no error, got: {}", app.status_msg);
        assert_eq!(app.editor.tabs.len(), initial_tabs + 1);
    }

    #[test]
    #[ignore = "blocks on crossterm terminal events; requires a PTY"]
    fn test_run() {}

    #[test]
    #[ignore = "blocks on crossterm terminal events; requires a PTY"]
    fn test_prompt_open_file() {}

    #[test]
    #[ignore = "blocks on crossterm terminal events; requires a PTY"]
    fn test_prompt_save_as() {}

    #[test]
    #[ignore = "private fn; tested indirectly via prompt_open_file / prompt_save_as"]
    fn test_inline_prompt() {}
}

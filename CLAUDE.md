# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Torus Edit** is a minimal, fast text editor written in Rust with limited dependencies. It uses tree-sitter for syntax highlighting, crossterm for terminal handling, and ropey for efficient text manipulation.

## Build and Development Commands

### Build
```bash
cargo build
cargo build --release
```

### Run
```bash
cargo run -- [file_path]
```

### Clean
```bash
cargo clean
```

## Project Structure and Architecture

### Core Component Layers

The editor follows a layered architecture with clear separation of concerns:

1. **Application Layer** (`app.rs`): Main event loop and state management
   - `App` struct holds editor, menu, search state, renderer, and theme
   - Runs at ~60 fps polling with 16ms timeout per event
   - Routes events through input handler and dispatches commands
   - Manages transient status messages and prompts (Go-to-line, Open File, Save As)

2. **Editor Layer** (`editor.rs`): Document and cursor management
   - `Editor` manages a vector of `Tab`s with multi-tab support
   - `Tab` contains: Buffer (text content), Cursor (position), Highlighter, scroll offsets
   - `Cursor` tracks: line, visual column (accounts for tab expansion), and character column
   - Provides all text editing operations (insert, delete, backspace) with cursor management

3. **Buffer Layer** (`buffer.rs`): Text storage and persistence
   - `Buffer` wraps `ropey::Rope` for efficient text manipulation
   - Tracks dirty state, file path, and detected language
   - Implements undo/redo via stacked edit groups (max 2,000 groups)
   - Edit operations: `insert_str()`, `delete_range()`, `checkpoint()`
   - Language detection via file extension (supports: rust, python, javascript, c, bash, toml)

4. **Highlighting Layer** (`highlight.rs`): Syntax highlighting
   - `Highlighter` uses tree-sitter parser with language-specific grammars
   - Stores `Span`s: byte-ranges with associated colors
   - Binary-searchable span list (`color_at()`) for O(log n) lookup
   - Uses incremental re-parsing on edits for performance
   - Node type → color mapping is language-aware

5. **Search Layer** (`search.rs`): Find and replace
   - `SearchState` manages query, matches, and replace text
   - Case-insensitive substring matching with non-overlapping match detection
   - Supports find-only mode and find+replace mode with tab-switching between fields
   - Maintains current match index for highlighting

6. **Input Layer** (`input.rs`): Event handling
   - Dispatches crossterm events to appropriate handlers
   - Normal key handler: text editing, navigation, Ctrl shortcuts
   - Menu handler: dropdown navigation and selection
   - Search handler: query/replace field input and match navigation
   - Go-to-line handler: numeric input parsing
   - Delegates to `MenuAction` dispatch for menu items

7. **Rendering Layer** (`renderer.rs`): Terminal output
   - `Renderer` manages terminal dimensions and coordinating multi-part renders
   - Renders: menu bar, tab bar, editor viewport with gutter, status bar, cursor
   - Uses crossterm's `QueueableCommand` for batched I/O
   - Respects layout constants: MENU_HEIGHT, TAB_BAR_HEIGHT, STATUS_HEIGHT, GUTTER_WIDTH, SCROLL_MARGIN

8. **Menu Layer** (`menu.rs`): Menu bar and dropdowns
   - `MenuBar` tracks open/closed state and selected row
   - `Menu` items map to `MenuAction` enums
   - Three menus: File, Edit, View with keyboard shortcuts
   - Alt+letter opens menu, arrows navigate, Enter activates

9. **Configuration Layer** (`config.rs`): Theme and constants
   - `Theme` struct: all UI colors (based on Catppuccin Mocha)
   - Layout constants define screen regions
   - Syntax highlighting color mapping per language and token type

### Data Flow

1. **Keystroke path**: `crossterm::event::read()` → `input::handle_event()` → command dispatch → editor/buffer modification → `renderer::render_all()`
2. **Text edit path**: Insert/delete at character index → rope manipulation → undo stack push → rehighlight → render
3. **Cursor movement**: Update line/col in Tab::cursor → `ensure_cursor_visible()` updates scroll offsets → render
4. **Search**: Query typed → `update_matches()` scans full text → jump to first match → highlight + render

### Key Abstractions

- **Rope-based text**: Character/line indices are logical, not byte-based (except in highlighting spans)
- **Edit groups**: Multiple keystrokes grouped at natural boundaries (Enter, space, punctuation) for fine-grained undo
- **Incremental highlighting**: Tree-sitter incremental parse on edits, then span rebuild
- **Scroll margin**: Cursor kept 3 lines from viewport edges for readable context
- **Tab expansion**: Visual column tracking for display, character column for text operations

## Common Development Tasks

### Adding a New Keybinding
1. Add case to `input::handle_key()` or the appropriate handler (search, goto, menu)
2. If it's a menu action, add variant to `MenuAction` enum in `menu.rs` and corresponding menu item
3. Implement the action dispatch in `input::dispatch_action()`

### Adding Syntax Highlighting for a New Language
1. Add language crate dependency to `Cargo.toml` (e.g., `tree-sitter-yaml`)
2. Update `buffer.rs` detect_language() with file extension mapping
3. Add language case to `highlight.rs` set_language() and create language-specific color function (e.g., `yaml_color()`)
4. Map node types to colors in the language-specific function using `Theme` colors

### Modifying the UI Layout
- Screen layout is computed in `renderer.rs` based on constants in `config.rs`
- Scroll viewport calculations in `editor.rs` ensure_cursor_visible() and renderer
- Gutter width (5 chars) supports up to 9,999 lines

### Understanding Undo/Redo
- `Buffer.pending_group` accumulates edits until `checkpoint()` called
- `checkpoint()` moves group to `undo_stack`, clears `redo_stack`
- `undo()/redo()` reverse operations and swap with opposite stack
- Max history: 2,000 groups per buffer to bound memory

## Dependencies

- **crossterm** 0.27: Terminal manipulation (raw mode, colors, events, cursor)
- **ropey** 1.6: Rope data structure for efficient text storage and manipulation
- **tree-sitter** 0.20: Incremental parser and AST queries
- **tree-sitter-[language]** 0.20: Language grammars for rust, python, javascript, c, bash, toml
- **libc** 0.2: C library bindings for terminal control (fallback/experimental)

## Terminal Handling

The app enters raw mode (crossterm) in `main.rs`, which:
- Disables canonical mode and echo
- Enables 16ms polling for event timeout
- Installs panic hook to restore terminal before backtrace
- Uses BufWriter for batched stdout output

The `torus/terminal_handler.rs` contains experimental raw mode wrapper using libc (not currently used in main app).

## Design Philosophy

- Minimal dependencies: Only crossterm, ropey, tree-sitter, and libc
- Fast startup and editing responsiveness
- Language detection via file extension
- Efficient text storage via rope structure
- Incremental syntax highlighting
- Natural edit grouping for fine-grained undo

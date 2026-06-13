use tree_sitter::{Parser, Tree};
use crossterm::style::Color;
use crate::config::Theme;

// ── Highlight span ────────────────────────────────────────────────────────────

/// A coloured byte-range within the source text.
#[derive(Clone, Debug)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte:   usize,
    pub color:      Color,
}

// ── Highlighter ───────────────────────────────────────────────────────────────

pub struct Highlighter {
    parser:        Parser,
    tree:          Option<Tree>,
    pub spans:     Vec<Span>,
    language_name: Option<String>,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            parser:        Parser::new(),
            tree:          None,
            spans:         Vec::new(),
            language_name: None,
        }
    }

    /// Set the grammar for the given language name.  Returns `true` on success.
    pub fn set_language(&mut self, lang: &str) -> bool {
        let language = match lang {
            "rust"       => tree_sitter_rust::language,
            "python"     => tree_sitter_python::language,
            "javascript" => tree_sitter_javascript::language,
            "c"          => tree_sitter_c::language,
            "bash"       => tree_sitter_bash::language,
            "toml"       => tree_sitter_toml::language,
            _            => return false,
        };
        if (self.parser.set_language(language()).is_ok()) {
            self.language_name = Some(lang.to_string());
            true
        } else {
            false
        }
    }

    /// Full parse — call when the language changes or the document is first loaded.
    pub fn parse(&mut self, text: &str, theme: &Theme) {
        self.tree = self.parser.parse(text, None);
        self.build_spans(text, theme);
    }

    /// Incremental re-parse — call after each edit (fast for small changes).
    pub fn reparse(&mut self, text: &str, theme: &Theme) {
        self.tree = self.parser.parse(text, self.tree.as_ref());
        self.build_spans(text, theme);
    }

    /// Return the syntax colour for the character at `byte_offset`, if any.
    /// Uses binary search over the pre-sorted span list — O(log n).
    pub fn color_at(&self, byte_offset: usize) -> Option<Color> {
        // Find the first span whose end_byte > byte_offset
        let idx = self.spans.partition_point(|s| s.end_byte <= byte_offset);
        self.spans.get(idx).and_then(|s| {
            if s.start_byte <= byte_offset { Some(s.color) } else { None }
        })
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    fn build_spans(&mut self, _text: &str, theme: &Theme) {
        self.spans.clear();
        let tree = match &self.tree { Some(t) => t, None => return };
        let lang = self.language_name.as_deref().unwrap_or("");

        // Iterative depth-first traversal using an explicit stack.
        // When a node maps to a colour we emit one span for the whole node
        // and do NOT recurse into its children (e.g. a string_literal gets one
        // colour even though its children include quote tokens and content).
        let mut stack = vec![tree.root_node()];

        while let Some(node) = stack.pop() {
            if let Some(color) = node_color(lang, node.kind(), theme) {
                self.spans.push(Span {
                    start_byte: node.start_byte(),
                    end_byte:   node.end_byte(),
                    color,
                });
                // Do not recurse — children inherit the parent colour.
            } else {
                // Push children in reverse so leftmost is processed first.
                for i in (0..node.child_count()).rev() {
                    if let Some(child) = node.child(i) {
                        stack.push(child);
                    }
                }
            }
        }

        // Sort by start position so binary search in color_at() works correctly.
        self.spans.sort_by_key(|s| s.start_byte);
    }
}

// ── Node-type → colour mapping ────────────────────────────────────────────────

fn node_color(lang: &str, kind: &str, t: &Theme) -> Option<Color> {
    // ── Cross-language leaf tokens ────────────────────────────────────────────
    match kind {
        // Strings (composite nodes coloured as a unit)
        | "string_literal"
        | "raw_string_literal"
        | "string"
        | "char_literal"
        | "interpreted_string_literal"
        | "raw_string"
        | "heredoc_body"
        | "concatenated_string"
        => return Some(t.syn_string),

        // Comments
        | "line_comment"
        | "block_comment"
        | "comment"
        | "doc_comment"
        | "shebang"
        => return Some(t.syn_comment),

        // Numbers
        | "integer_literal"
        | "float_literal"
        | "number_literal"
        | "number"
        | "octal_literal"
        | "hex_literal"
        | "binary_literal"
        => return Some(t.syn_number),

        _ => {}
    }

    match lang {
        "rust"       => rust_color(kind, t),
        "python"     => python_color(kind, t),
        "javascript" => js_color(kind, t),
        "c"          => c_color(kind, t),
        "bash"       => bash_color(kind, t),
        "toml"       => toml_color(kind, t),
        _            => None,
    }
}

// ── Per-language mappings ─────────────────────────────────────────────────────

fn rust_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        // Keywords
        | "fn" | "let" | "mut" | "pub" | "use" | "mod" | "struct" | "enum" | "impl"
        | "trait" | "where" | "for" | "if" | "else" | "match" | "return" | "self"
        | "super" | "in" | "loop" | "while" | "break" | "continue" | "const" | "static"
        | "type" | "unsafe" | "extern" | "crate" | "ref" | "move" | "async" | "await"
        | "dyn" | "box" | "as" | "Self"
        => Some(t.syn_keyword),

        // Types
        | "type_identifier" | "primitive_type" | "lifetime"
        => Some(t.syn_type),

        // Attributes  (#[...] and #![...])
        | "attribute_item" | "inner_attribute_item"
        => Some(t.syn_attribute),

        // Constants / boolean literals
        | "true" | "false"
        => Some(t.syn_constant),

        // Operators
        | "+" | "-" | "*" | "/" | "%" | "=" | "==" | "!=" | "<" | ">"
        | "<=" | ">=" | "&&" | "||" | "!" | "&" | "|" | "^" | "<<"
        | ">>" | "->" | "=>" | ".." | "..=" | "?"
        => Some(t.syn_operator),

        // Punctuation
        | "{" | "}" | "(" | ")" | "[" | "]" | ";" | "," | "." | ":" | "::"
        => Some(t.syn_punctuation),

        _ => None,
    }
}

fn python_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        | "def" | "class" | "return" | "import" | "from" | "as" | "if" | "elif"
        | "else" | "for" | "while" | "in" | "not" | "and" | "or" | "is"
        | "lambda" | "with" | "yield" | "del" | "pass" | "break" | "continue"
        | "try" | "except" | "finally" | "raise" | "global" | "nonlocal"
        | "assert" | "async" | "await" | "match" | "case"
        => Some(t.syn_keyword),

        | "true" | "false" | "none" | "True" | "False" | "None"
        => Some(t.syn_constant),

        | "decorator"
        => Some(t.syn_attribute),

        | "=" | "+" | "-" | "*" | "/" | "%" | "**" | "==" | "!="
        | "<" | ">" | "<=" | ">=" | "->" | ":" | ":=" | "//"
        => Some(t.syn_operator),

        | "{" | "}" | "(" | ")" | "[" | "]" | "," | "."
        => Some(t.syn_punctuation),

        _ => None,
    }
}

fn js_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        | "const" | "let" | "var" | "function" | "return" | "if" | "else" | "for"
        | "while" | "do" | "switch" | "case" | "break" | "continue" | "new"
        | "delete" | "typeof" | "instanceof" | "in" | "of" | "class" | "extends"
        | "import" | "export" | "from" | "default" | "this" | "super" | "try"
        | "catch" | "finally" | "throw" | "async" | "await" | "yield" | "void"
        => Some(t.syn_keyword),

        | "true" | "false" | "null" | "undefined"
        => Some(t.syn_constant),

        | "=" | "+" | "-" | "*" | "/" | "%" | "==" | "===" | "!=" | "!=="
        | "<" | ">" | "<=" | ">=" | "&&" | "||" | "!" | "=>" | "?." | "??" | "..."
        => Some(t.syn_operator),

        | "{" | "}" | "(" | ")" | "[" | "]" | "," | "." | ";"
        => Some(t.syn_punctuation),

        _ => None,
    }
}

fn c_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        | "auto" | "break" | "case" | "char" | "const" | "continue" | "default"
        | "do" | "double" | "else" | "enum" | "extern" | "float" | "for" | "goto"
        | "if" | "int" | "long" | "register" | "return" | "short" | "signed"
        | "sizeof" | "static" | "struct" | "switch" | "typedef" | "union"
        | "unsigned" | "void" | "volatile" | "while"
        => Some(t.syn_keyword),

        | "type_identifier"
        => Some(t.syn_type),

        // Preprocessor directives
        | "preproc_include" | "preproc_def" | "preproc_function_def"
        | "preproc_ifdef" | "preproc_if" | "preproc_directive"
        | "#include" | "#define" | "#ifdef" | "#ifndef" | "#endif" | "#if" | "#else"
        => Some(t.syn_attribute),

        | "true" | "false" | "NULL"
        => Some(t.syn_constant),

        | "=" | "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | ">"
        | "<=" | ">=" | "&&" | "||" | "!" | "&" | "|" | "^" | "~"
        | "<<" | ">>" | "->" | "++" | "--" | "+=" | "-=" | "*=" | "/="
        => Some(t.syn_operator),

        | "{" | "}" | "(" | ")" | "[" | "]" | "," | "." | ";" | ":"
        => Some(t.syn_punctuation),

        _ => None,
    }
}

fn bash_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        | "if" | "then" | "else" | "elif" | "fi" | "for" | "while" | "do"
        | "done" | "case" | "esac" | "in" | "function" | "return" | "local"
        | "export" | "readonly" | "source" | "declare" | "typeset" | "until"
        | "select" | "time"
        => Some(t.syn_keyword),

        | "variable_name"
        => Some(t.syn_variable),

        | "command_name"
        => Some(t.syn_function),

        | "=" | "==" | "!=" | "-eq" | "-ne" | "-lt" | "-gt" | "-le" | "-ge"
        => Some(t.syn_operator),

        _ => None,
    }
}

fn toml_color(kind: &str, t: &Theme) -> Option<Color> {
    match kind {
        | "table"       | "array_table"  => Some(t.syn_type),
        | "bare_key"    | "quoted_key"   => Some(t.syn_keyword),
        | "true"        | "false"        => Some(t.syn_constant),
        | "="                            => Some(t.syn_operator),
        | "[" | "]"  | "[[" | "]]" | "," | "." => Some(t.syn_punctuation),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme { Theme::default() }

    #[test]
    fn test_new_has_no_spans_or_language() {
        let h = Highlighter::new();
        assert!(h.spans.is_empty());
    }

    #[test]
    fn test_set_language_known_returns_true() {
        let mut h = Highlighter::new();
        assert!(h.set_language("rust"));
    }

    #[test]
    fn test_set_language_unknown_returns_false() {
        let mut h = Highlighter::new();
        assert!(!h.set_language("brainfuck"));
    }

    #[test]
    fn test_parse_populates_spans_for_rust() {
        let mut h = Highlighter::new();
        h.set_language("rust");
        h.parse("fn main() {}", &theme());
        assert!(!h.spans.is_empty(), "parse should produce spans for Rust code");
    }

    #[test]
    fn test_reparse_updates_spans() {
        let mut h = Highlighter::new();
        h.set_language("rust");
        h.parse("fn main() {}", &theme());
        let count_before = h.spans.len();
        h.reparse("fn main() { let x = 1; }", &theme());
        let count_after = h.spans.len();
        assert!(count_after >= count_before, "reparse should produce at least as many spans for longer code");
    }

    #[test]
    fn test_color_at_returns_color_inside_span() {
        let mut h = Highlighter::new();
        h.set_language("rust");
        h.parse("fn main() {}", &theme());
        // "fn" starts at byte 0; byte 0 should map to syn_keyword
        assert!(h.color_at(0).is_some(), "byte 0 of 'fn' should have a color");
    }

    #[test]
    fn test_color_at_returns_none_when_no_spans() {
        let h = Highlighter::new();
        assert!(h.color_at(0).is_none());
    }
}

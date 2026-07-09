//! Lexer for asciiscript DSL.
//!
//! Tokenizes source into a stream of tokens for the parser.

use crate::asciiscript::ast::{Position, Span};
use std::fmt;

/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords - Containers
    Layout,
    Window,
    Box,
    Section,
    Row,
    Column,

    // Keywords - Primitives
    Text,
    Input,
    Checkbox,
    Radio,
    Select,
    Button,
    Link,
    Code,
    Raw,
    Separator,
    Spacer,
    Progress,
    Alert,

    // Keywords - Table
    Table,
    Header,
    Tr,
    Td,
    Col,

    // Keywords - List
    List,
    Item,

    // Flags
    Checked,
    Selected,

    // Literals
    String(String),      // "..."
    MultilineString(String), // ```...```
    Number(u32),
    Ident(String),       // for attribute names and enum values

    // Symbols
    LBrace,  // {
    RBrace,  // }
    Colon,   // :

    // Special
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Layout => write!(f, "layout"),
            TokenKind::Window => write!(f, "window"),
            TokenKind::Box => write!(f, "box"),
            TokenKind::Section => write!(f, "section"),
            TokenKind::Row => write!(f, "row"),
            TokenKind::Column => write!(f, "column"),
            TokenKind::Text => write!(f, "text"),
            TokenKind::Input => write!(f, "input"),
            TokenKind::Checkbox => write!(f, "checkbox"),
            TokenKind::Radio => write!(f, "radio"),
            TokenKind::Select => write!(f, "select"),
            TokenKind::Button => write!(f, "button"),
            TokenKind::Link => write!(f, "link"),
            TokenKind::Code => write!(f, "code"),
            TokenKind::Raw => write!(f, "raw"),
            TokenKind::Separator => write!(f, "separator"),
            TokenKind::Spacer => write!(f, "spacer"),
            TokenKind::Progress => write!(f, "progress"),
            TokenKind::Alert => write!(f, "alert"),
            TokenKind::Table => write!(f, "table"),
            TokenKind::Header => write!(f, "header"),
            TokenKind::Tr => write!(f, "tr"),
            TokenKind::Td => write!(f, "td"),
            TokenKind::Col => write!(f, "col"),
            TokenKind::List => write!(f, "list"),
            TokenKind::Item => write!(f, "item"),
            TokenKind::Checked => write!(f, "checked"),
            TokenKind::Selected => write!(f, "selected"),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::MultilineString(s) => write!(f, "```{}```", s),
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Lexer error with location.
#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    pub message: String,
    pub position: Position,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.position.line, self.position.column, self.message)
    }
}

impl std::error::Error for LexerError {}

/// Lexer state.
pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: u32,
    column: u32,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            current_pos: 0,
        }
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(source: &str) -> Result<Vec<Token>, LexerError> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn position(&self) -> Position {
        Position::new(self.line, self.column)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some((pos, ch)) = self.chars.next() {
            self.current_pos = pos;
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Skip comments
            if self.peek() == Some('#') {
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<String, LexerError> {
        let start_pos = self.position();
        self.advance(); // consume opening "

        let mut result = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(result),
                Some('\\') => {
                    // Handle escape sequences
                    match self.advance() {
                        Some('n') => result.push('\n'),
                        Some('t') => result.push('\t'),
                        Some('\\') => result.push('\\'),
                        Some('"') => result.push('"'),
                        Some(ch) => result.push(ch),
                        None => {
                            return Err(LexerError {
                                message: "Unterminated string".to_string(),
                                position: start_pos,
                            })
                        }
                    }
                }
                Some(ch) => result.push(ch),
                None => {
                    return Err(LexerError {
                        message: "Unterminated string".to_string(),
                        position: start_pos,
                    })
                }
            }
        }
    }

    fn read_multiline_string(&mut self) -> Result<String, LexerError> {
        let start_pos = self.position();

        // Consume opening ```
        self.advance(); // `
        self.advance(); // `
        self.advance(); // `

        // Skip optional newline after opening ```
        if self.peek() == Some('\n') {
            self.advance();
        }

        let mut result = String::new();
        let mut backtick_count = 0;

        loop {
            match self.advance() {
                Some('`') => {
                    backtick_count += 1;
                    if backtick_count == 3 {
                        // Remove the two backticks we added
                        result.pop();
                        result.pop();
                        // Trim trailing newline if present
                        if result.ends_with('\n') {
                            result.pop();
                        }
                        return Ok(result);
                    }
                    result.push('`');
                }
                Some(ch) => {
                    backtick_count = 0;
                    result.push(ch);
                }
                None => {
                    return Err(LexerError {
                        message: "Unterminated multiline string".to_string(),
                        position: start_pos,
                    })
                }
            }
        }
    }

    fn read_number(&mut self) -> u32 {
        let mut result = 0u32;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                result = result * 10 + (ch as u32 - '0' as u32);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_lowercase() || ch == '_' {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn keyword_or_ident(&self, s: &str) -> TokenKind {
        match s {
            // Containers
            "layout" => TokenKind::Layout,
            "window" => TokenKind::Window,
            "box" => TokenKind::Box,
            "section" => TokenKind::Section,
            "row" => TokenKind::Row,
            "column" => TokenKind::Column,
            // Primitives
            "text" => TokenKind::Text,
            "input" => TokenKind::Input,
            "checkbox" => TokenKind::Checkbox,
            "radio" => TokenKind::Radio,
            "select" => TokenKind::Select,
            "button" => TokenKind::Button,
            "link" => TokenKind::Link,
            "code" => TokenKind::Code,
            "raw" => TokenKind::Raw,
            "separator" => TokenKind::Separator,
            "spacer" => TokenKind::Spacer,
            "progress" => TokenKind::Progress,
            "alert" => TokenKind::Alert,
            // Table
            "table" => TokenKind::Table,
            "header" => TokenKind::Header,
            "tr" => TokenKind::Tr,
            "td" => TokenKind::Td,
            "col" => TokenKind::Col,
            // List
            "list" => TokenKind::List,
            "item" => TokenKind::Item,
            // Flags
            "checked" => TokenKind::Checked,
            "selected" => TokenKind::Selected,
            // Identifier
            _ => TokenKind::Ident(s.to_string()),
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexerError> {
        self.skip_whitespace_and_comments();

        let start = self.position();

        let Some(ch) = self.peek() else {
            return Ok(Token::new(TokenKind::Eof, Span::new(start, start)));
        };

        let kind = match ch {
            '{' => {
                self.advance();
                TokenKind::LBrace
            }
            '}' => {
                self.advance();
                TokenKind::RBrace
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            '"' => {
                let s = self.read_string()?;
                TokenKind::String(s)
            }
            '`' => {
                // Check for ```
                let mut is_multiline = false;
                let mut chars_copy = self.chars.clone();
                if chars_copy.next().map(|(_, c)| c) == Some('`') {
                    if chars_copy.next().map(|(_, c)| c) == Some('`') {
                        if chars_copy.next().map(|(_, c)| c) == Some('`') {
                            is_multiline = true;
                        }
                    }
                }

                if is_multiline {
                    let s = self.read_multiline_string()?;
                    TokenKind::MultilineString(s)
                } else {
                    return Err(LexerError {
                        message: "Unexpected character '`'. Did you mean '```' for multiline string?".to_string(),
                        position: start,
                    });
                }
            }
            '0'..='9' => {
                let n = self.read_number();
                TokenKind::Number(n)
            }
            'a'..='z' | '_' => {
                let ident = self.read_identifier();
                self.keyword_or_ident(&ident)
            }
            _ => {
                return Err(LexerError {
                    message: format!("Unexpected character '{}'", ch),
                    position: start,
                });
            }
        };

        let end = self.position();
        Ok(Token::new(kind, Span::new(start, end)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> Vec<TokenKind> {
        Lexer::tokenize(source)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    fn tokenize_err(source: &str) -> LexerError {
        Lexer::tokenize(source).unwrap_err()
    }

    // ========== Basic token tests ==========

    #[test]
    fn empty_source_returns_eof() {
        assert_eq!(tokenize(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn whitespace_only_returns_eof() {
        assert_eq!(tokenize("   \n\t  "), vec![TokenKind::Eof]);
    }

    #[test]
    fn comment_only_returns_eof() {
        assert_eq!(tokenize("# this is a comment"), vec![TokenKind::Eof]);
    }

    #[test]
    fn comment_with_content_after() {
        assert_eq!(
            tokenize("# comment\nlayout"),
            vec![TokenKind::Layout, TokenKind::Eof]
        );
    }

    // ========== Symbol tests ==========

    #[test]
    fn braces() {
        assert_eq!(
            tokenize("{ }"),
            vec![TokenKind::LBrace, TokenKind::RBrace, TokenKind::Eof]
        );
    }

    #[test]
    fn colon() {
        assert_eq!(
            tokenize(":"),
            vec![TokenKind::Colon, TokenKind::Eof]
        );
    }

    // ========== Keyword tests ==========

    #[test]
    fn container_keywords() {
        assert_eq!(
            tokenize("layout window box section row column"),
            vec![
                TokenKind::Layout,
                TokenKind::Window,
                TokenKind::Box,
                TokenKind::Section,
                TokenKind::Row,
                TokenKind::Column,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn primitive_keywords() {
        assert_eq!(
            tokenize("text input checkbox radio select button link"),
            vec![
                TokenKind::Text,
                TokenKind::Input,
                TokenKind::Checkbox,
                TokenKind::Radio,
                TokenKind::Select,
                TokenKind::Button,
                TokenKind::Link,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn more_primitive_keywords() {
        assert_eq!(
            tokenize("code raw separator spacer progress alert"),
            vec![
                TokenKind::Code,
                TokenKind::Raw,
                TokenKind::Separator,
                TokenKind::Spacer,
                TokenKind::Progress,
                TokenKind::Alert,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn table_keywords() {
        assert_eq!(
            tokenize("table header tr td col"),
            vec![
                TokenKind::Table,
                TokenKind::Header,
                TokenKind::Tr,
                TokenKind::Td,
                TokenKind::Col,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn list_keywords() {
        assert_eq!(
            tokenize("list item"),
            vec![TokenKind::List, TokenKind::Item, TokenKind::Eof]
        );
    }

    #[test]
    fn flag_keywords() {
        assert_eq!(
            tokenize("checked selected"),
            vec![TokenKind::Checked, TokenKind::Selected, TokenKind::Eof]
        );
    }

    // ========== String tests ==========

    #[test]
    fn simple_string() {
        assert_eq!(
            tokenize(r#""hello""#),
            vec![TokenKind::String("hello".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_with_spaces() {
        assert_eq!(
            tokenize(r#""hello world""#),
            vec![TokenKind::String("hello world".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(
            tokenize(r#""line1\nline2""#),
            vec![TokenKind::String("line1\nline2".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_with_escaped_quote() {
        assert_eq!(
            tokenize(r#""say \"hi\"""#),
            vec![TokenKind::String("say \"hi\"".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_string_error() {
        let err = tokenize_err(r#""hello"#);
        assert!(err.message.contains("Unterminated string"));
    }

    // ========== Multiline string tests ==========

    #[test]
    fn multiline_string_simple() {
        let source = "```\nhello\nworld\n```";
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::MultilineString("hello\nworld".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn multiline_string_preserves_whitespace() {
        let source = "```\n  indented\n    more\n```";
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::MultilineString("  indented\n    more".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn multiline_string_no_leading_newline() {
        let source = "```hello```";
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::MultilineString("hello".to_string()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn unterminated_multiline_string_error() {
        let err = tokenize_err("```\nhello");
        assert!(err.message.contains("Unterminated multiline string"));
    }

    // ========== Number tests ==========

    #[test]
    fn single_digit() {
        assert_eq!(
            tokenize("5"),
            vec![TokenKind::Number(5), TokenKind::Eof]
        );
    }

    #[test]
    fn multi_digit() {
        assert_eq!(
            tokenize("123"),
            vec![TokenKind::Number(123), TokenKind::Eof]
        );
    }

    #[test]
    fn zero() {
        assert_eq!(
            tokenize("0"),
            vec![TokenKind::Number(0), TokenKind::Eof]
        );
    }

    // ========== Identifier tests ==========

    #[test]
    fn identifier_simple() {
        assert_eq!(
            tokenize("foo"),
            vec![TokenKind::Ident("foo".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn identifier_with_underscore() {
        assert_eq!(
            tokenize("foo_bar"),
            vec![TokenKind::Ident("foo_bar".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn identifier_all_underscore() {
        assert_eq!(
            tokenize("_"),
            vec![TokenKind::Ident("_".to_string()), TokenKind::Eof]
        );
    }

    // ========== Complex expression tests ==========

    #[test]
    fn attribute_expression() {
        assert_eq!(
            tokenize("width:40"),
            vec![
                TokenKind::Ident("width".to_string()),
                TokenKind::Colon,
                TokenKind::Number(40),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn simple_layout() {
        let source = r#"layout {
            window "Title" {
            }
        }"#;
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::Layout,
                TokenKind::LBrace,
                TokenKind::Window,
                TokenKind::String("Title".to_string()),
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn row_with_elements() {
        let source = r#"row { text "Label:" input width:20 }"#;
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::Row,
                TokenKind::LBrace,
                TokenKind::Text,
                TokenKind::String("Label:".to_string()),
                TokenKind::Input,
                TokenKind::Ident("width".to_string()),
                TokenKind::Colon,
                TokenKind::Number(20),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn checkbox_with_flag() {
        let source = r#"checkbox "Enable" checked"#;
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::Checkbox,
                TokenKind::String("Enable".to_string()),
                TokenKind::Checked,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn table_structure() {
        let source = r#"table {
            header { col "Name" width:10 }
            tr { td "value" }
        }"#;
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::Table,
                TokenKind::LBrace,
                TokenKind::Header,
                TokenKind::LBrace,
                TokenKind::Col,
                TokenKind::String("Name".to_string()),
                TokenKind::Ident("width".to_string()),
                TokenKind::Colon,
                TokenKind::Number(10),
                TokenKind::RBrace,
                TokenKind::Tr,
                TokenKind::LBrace,
                TokenKind::Td,
                TokenKind::String("value".to_string()),
                TokenKind::RBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn code_with_multiline() {
        let source = r#"code ```
fn main() {
    println!("hi");
}
```"#;
        assert_eq!(
            tokenize(source),
            vec![
                TokenKind::Code,
                TokenKind::MultilineString("fn main() {\n    println!(\"hi\");\n}".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    // ========== Error tests ==========

    #[test]
    fn unexpected_character_error() {
        let err = tokenize_err("@");
        assert!(err.message.contains("Unexpected character '@'"));
        assert_eq!(err.position.line, 1);
        assert_eq!(err.position.column, 1);
    }

    #[test]
    fn lone_backtick_error() {
        let err = tokenize_err("`");
        assert!(err.message.contains("Did you mean '```'"));
    }

    // ========== Position tracking tests ==========

    #[test]
    fn token_positions() {
        let tokens = Lexer::tokenize("layout {\n  window\n}").unwrap();

        // layout starts at 1:1
        assert_eq!(tokens[0].span.start.line, 1);
        assert_eq!(tokens[0].span.start.column, 1);

        // { at 1:8
        assert_eq!(tokens[1].span.start.line, 1);
        assert_eq!(tokens[1].span.start.column, 8);

        // window at 2:3
        assert_eq!(tokens[2].span.start.line, 2);
        assert_eq!(tokens[2].span.start.column, 3);

        // } at 3:1
        assert_eq!(tokens[3].span.start.line, 3);
        assert_eq!(tokens[3].span.start.column, 1);
    }
}

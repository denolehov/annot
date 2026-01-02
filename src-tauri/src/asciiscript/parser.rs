//! Parser for asciiscript DSL.
//!
//! Parses a token stream into an AST.

use crate::asciiscript::ast::*;
use crate::asciiscript::lexer::{Lexer, LexerError, Token, TokenKind};
use std::fmt;

/// Parser error with location.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.span, self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError {
            message: err.message,
            span: Span::new(err.position, err.position),
        }
    }
}

/// Parser state.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse source string into a Program.
    pub fn parse(source: &str) -> Result<Program, ParseError> {
        let tokens = Lexer::tokenize(source)?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn current(&self) -> &Token {
        // tokens always ends with Eof, so this is safe
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek(&self) -> &TokenKind {
        &self.current().kind
    }

    fn advance(&mut self) -> &Token {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        // Return previous token (the one we just passed)
        &self.tokens[self.pos.saturating_sub(1)]
    }

    fn expect(&mut self, expected: TokenKind) -> Result<&Token, ParseError> {
        if *self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("Expected {}, found {}", expected, self.peek()),
                span: self.current().span,
            })
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    // ========== Parsing methods ==========

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let layout = self.parse_layout()?;

        if !self.is_at_end() {
            return Err(ParseError {
                message: format!("Unexpected token after layout: {}", self.peek()),
                span: self.current().span,
            });
        }

        Ok(Program { layout })
    }

    fn parse_layout(&mut self) -> Result<Layout, ParseError> {
        let start = self.current().span;
        self.expect(TokenKind::Layout)?;
        self.expect(TokenKind::LBrace)?;

        let statements = self.parse_statements()?;

        let end_token = self.expect(TokenKind::RBrace)?;
        let span = start.merge(end_token.span);

        Ok(Layout { statements, span })
    }

    fn parse_statements(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            // Containers
            TokenKind::Window | TokenKind::Box | TokenKind::Section
            | TokenKind::Row | TokenKind::Column => {
                Ok(Statement::Container(self.parse_container()?))
            }
            // Primitives
            TokenKind::Text => Ok(Statement::Primitive(self.parse_text()?)),
            TokenKind::Input => Ok(Statement::Primitive(self.parse_input()?)),
            TokenKind::Checkbox => Ok(Statement::Primitive(self.parse_checkbox()?)),
            TokenKind::Radio => Ok(Statement::Primitive(self.parse_radio()?)),
            TokenKind::Select => Ok(Statement::Primitive(self.parse_select()?)),
            TokenKind::Button => Ok(Statement::Primitive(self.parse_button()?)),
            TokenKind::Link => Ok(Statement::Primitive(self.parse_link()?)),
            TokenKind::Code => Ok(Statement::Primitive(self.parse_code()?)),
            TokenKind::Raw => Ok(Statement::Primitive(self.parse_raw()?)),
            TokenKind::Separator => Ok(Statement::Primitive(self.parse_separator()?)),
            TokenKind::Spacer => Ok(Statement::Primitive(self.parse_spacer()?)),
            TokenKind::Progress => Ok(Statement::Primitive(self.parse_progress()?)),
            TokenKind::Alert => Ok(Statement::Primitive(self.parse_alert()?)),
            TokenKind::Table => Ok(Statement::Primitive(self.parse_table()?)),
            TokenKind::List => Ok(Statement::Primitive(self.parse_list()?)),
            _ => Err(ParseError {
                message: format!("Unexpected token: {}", self.peek()),
                span: self.current().span,
            }),
        }
    }

    fn parse_container(&mut self) -> Result<Container, ParseError> {
        let start = self.current().span;
        let kind = match self.peek() {
            TokenKind::Window => { self.advance(); ContainerKind::Window }
            TokenKind::Box => { self.advance(); ContainerKind::Box }
            TokenKind::Section => { self.advance(); ContainerKind::Section }
            TokenKind::Row => { self.advance(); ContainerKind::Row }
            TokenKind::Column => { self.advance(); ContainerKind::Column }
            _ => unreachable!(),
        };

        // Parse optional title (required for section)
        let title = if let TokenKind::String(s) = self.peek().clone() {
            self.advance();
            Some(s)
        } else if kind == ContainerKind::Section {
            return Err(ParseError {
                message: "section requires a title".to_string(),
                span: self.current().span,
            });
        } else {
            None
        };

        // Parse modifiers
        let modifiers = self.parse_modifiers()?;

        // Parse block
        self.expect(TokenKind::LBrace)?;
        let statements = self.parse_statements()?;
        let end_token = self.expect(TokenKind::RBrace)?;

        let span = start.merge(end_token.span);

        Ok(Container {
            kind,
            title,
            modifiers,
            statements,
            span,
        })
    }

    fn parse_modifiers(&mut self) -> Result<Modifiers, ParseError> {
        let mut modifiers = Modifiers::new();

        loop {
            match self.peek() {
                TokenKind::Ident(name) => {
                    let name = name.clone();
                    let attr_span = self.current().span;
                    self.advance();

                    // Expect colon
                    self.expect(TokenKind::Colon)?;

                    // Parse value
                    match name.as_str() {
                        "width" => {
                            if let TokenKind::Number(n) = self.peek() {
                                modifiers.width = Some(*n);
                                self.advance();
                            } else {
                                return Err(ParseError {
                                    message: format!("width requires a number, found {}", self.peek()),
                                    span: self.current().span,
                                });
                            }
                        }
                        "height" => {
                            if let TokenKind::Number(n) = self.peek() {
                                modifiers.height = Some(*n);
                                self.advance();
                            } else {
                                return Err(ParseError {
                                    message: format!("height requires a number, found {}", self.peek()),
                                    span: self.current().span,
                                });
                            }
                        }
                        "padding" => {
                            if let TokenKind::Number(n) = self.peek() {
                                modifiers.padding = Some(*n);
                                self.advance();
                            } else {
                                return Err(ParseError {
                                    message: format!("padding requires a number, found {}", self.peek()),
                                    span: self.current().span,
                                });
                            }
                        }
                        "gap" => {
                            if let TokenKind::Number(n) = self.peek() {
                                modifiers.gap = Some(*n);
                                self.advance();
                            } else {
                                return Err(ParseError {
                                    message: format!("gap requires a number, found {}", self.peek()),
                                    span: self.current().span,
                                });
                            }
                        }
                        "align" => {
                            match self.peek() {
                                TokenKind::Ident(v) if v == "left" => {
                                    modifiers.align = Some(Align::Left);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "center" => {
                                    modifiers.align = Some(Align::Center);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "right" => {
                                    modifiers.align = Some(Align::Right);
                                    self.advance();
                                }
                                _ => {
                                    return Err(ParseError {
                                        message: format!("align requires left/center/right, found {}", self.peek()),
                                        span: self.current().span,
                                    });
                                }
                            }
                        }
                        "style" => {
                            match self.peek() {
                                TokenKind::Ident(v) if v == "bold" => {
                                    modifiers.style = Some(Style::Bold);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "dim" => {
                                    modifiers.style = Some(Style::Dim);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "danger" => {
                                    modifiers.style = Some(Style::Danger);
                                    self.advance();
                                }
                                _ => {
                                    return Err(ParseError {
                                        message: format!("style requires bold/dim/danger, found {}", self.peek()),
                                        span: self.current().span,
                                    });
                                }
                            }
                        }
                        "type" => {
                            match self.peek() {
                                TokenKind::Ident(v) if v == "error" => {
                                    modifiers.alert_type = Some(AlertType::Error);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "warn" => {
                                    modifiers.alert_type = Some(AlertType::Warn);
                                    self.advance();
                                }
                                TokenKind::Ident(v) if v == "info" => {
                                    modifiers.alert_type = Some(AlertType::Info);
                                    self.advance();
                                }
                                _ => {
                                    return Err(ParseError {
                                        message: format!("type requires error/warn/info, found {}", self.peek()),
                                        span: self.current().span,
                                    });
                                }
                            }
                        }
                        "placeholder" => {
                            if let TokenKind::String(s) = self.peek().clone() {
                                modifiers.placeholder = Some(s);
                                self.advance();
                            } else {
                                return Err(ParseError {
                                    message: format!("placeholder requires a string, found {}", self.peek()),
                                    span: self.current().span,
                                });
                            }
                        }
                        _ => {
                            return Err(ParseError {
                                message: format!("Unknown attribute: {}", name),
                                span: attr_span,
                            });
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(modifiers)
    }

    // ========== Primitive parsers ==========

    fn parse_text(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'text'

        let content = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("text requires a string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let modifiers = self.parse_modifiers()?;
        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Text {
            content,
            modifiers,
            span,
        })
    }

    fn parse_input(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'input'

        let modifiers = self.parse_modifiers()?;

        // Validate: width is required
        if modifiers.width.is_none() {
            return Err(ParseError {
                message: "input requires width attribute".to_string(),
                span: start,
            });
        }

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Input { modifiers, span })
    }

    fn parse_checkbox(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'checkbox'

        let label = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("checkbox requires a label string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let checked = if self.check(&TokenKind::Checked) {
            self.advance();
            true
        } else {
            false
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Checkbox {
            label,
            checked,
            span,
        })
    }

    fn parse_radio(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'radio'

        let label = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("radio requires a label string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let selected = if self.check(&TokenKind::Selected) {
            self.advance();
            true
        } else {
            false
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Radio {
            label,
            selected,
            span,
        })
    }

    fn parse_select(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'select'

        let value = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("select requires a value string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let modifiers = self.parse_modifiers()?;

        // Validate: width is required
        if modifiers.width.is_none() {
            return Err(ParseError {
                message: "select requires width attribute".to_string(),
                span: start,
            });
        }

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Select {
            value,
            modifiers,
            span,
        })
    }

    fn parse_button(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'button'

        let label = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("button requires a label string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let modifiers = self.parse_modifiers()?;
        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Button {
            label,
            modifiers,
            span,
        })
    }

    fn parse_link(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'link'

        let text = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("link requires a text string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Link { text, span })
    }

    fn parse_code(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'code'

        let content = match self.peek().clone() {
            TokenKind::String(s) | TokenKind::MultilineString(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("code requires a string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Code { content, span })
    }

    fn parse_raw(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'raw'

        let content = match self.peek().clone() {
            TokenKind::String(s) | TokenKind::MultilineString(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("raw requires a string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Raw { content, span })
    }

    fn parse_separator(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'separator'

        Ok(Primitive::Separator { span: start })
    }

    fn parse_spacer(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'spacer'

        Ok(Primitive::Spacer { span: start })
    }

    fn parse_progress(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'progress'

        let value = match self.peek() {
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                if n > 100 {
                    return Err(ParseError {
                        message: format!("progress value must be 0-100, found {}", n),
                        span: self.current().span,
                    });
                }
                n as u8
            }
            _ => {
                return Err(ParseError {
                    message: format!("progress requires a number, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(Primitive::Progress { value, span })
    }

    fn parse_alert(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'alert'

        let modifiers = self.parse_modifiers()?;

        // Validate: type is required
        let alert_type = modifiers.alert_type.ok_or_else(|| ParseError {
            message: "alert requires type attribute (error/warn/info)".to_string(),
            span: start,
        })?;

        self.expect(TokenKind::LBrace)?;
        let statements = self.parse_statements()?;
        let end_token = self.expect(TokenKind::RBrace)?;

        let span = start.merge(end_token.span);

        Ok(Primitive::Alert {
            alert_type,
            statements,
            span,
        })
    }

    fn parse_table(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'table'

        let modifiers = self.parse_modifiers()?;

        self.expect(TokenKind::LBrace)?;

        // Parse optional header
        let header = if self.check(&TokenKind::Header) {
            Some(self.parse_table_header()?)
        } else {
            None
        };

        // Parse rows
        let mut rows = Vec::new();
        while self.check(&TokenKind::Tr) {
            rows.push(self.parse_table_row()?);
        }

        let end_token = self.expect(TokenKind::RBrace)?;
        let span = start.merge(end_token.span);

        Ok(Primitive::Table(Table {
            header,
            rows,
            modifiers,
            span,
        }))
    }

    fn parse_table_header(&mut self) -> Result<TableHeader, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'header'

        self.expect(TokenKind::LBrace)?;

        let mut columns = Vec::new();
        while self.check(&TokenKind::Col) {
            columns.push(self.parse_table_column()?);
        }

        if columns.is_empty() {
            return Err(ParseError {
                message: "header requires at least one col".to_string(),
                span: start,
            });
        }

        let end_token = self.expect(TokenKind::RBrace)?;
        let span = start.merge(end_token.span);

        Ok(TableHeader { columns, span })
    }

    fn parse_table_column(&mut self) -> Result<TableColumn, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'col'

        let label = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("col requires a label string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let modifiers = self.parse_modifiers()?;

        // Validate: width is required
        if modifiers.width.is_none() {
            return Err(ParseError {
                message: "col requires width attribute".to_string(),
                span: start,
            });
        }

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(TableColumn {
            label,
            modifiers,
            span,
        })
    }

    fn parse_table_row(&mut self) -> Result<TableRow, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'tr'

        self.expect(TokenKind::LBrace)?;

        let mut cells = Vec::new();
        while self.check(&TokenKind::Td) {
            cells.push(self.parse_table_cell()?);
        }

        if cells.is_empty() {
            return Err(ParseError {
                message: "tr requires at least one td".to_string(),
                span: start,
            });
        }

        let end_token = self.expect(TokenKind::RBrace)?;
        let span = start.merge(end_token.span);

        Ok(TableRow { cells, span })
    }

    fn parse_table_cell(&mut self) -> Result<TableCell, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'td'

        let content = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("td requires a content string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let modifiers = self.parse_modifiers()?;
        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(TableCell {
            content,
            modifiers,
            span,
        })
    }

    fn parse_list(&mut self) -> Result<Primitive, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'list'

        self.expect(TokenKind::LBrace)?;

        let mut items = Vec::new();
        while self.check(&TokenKind::Item) {
            items.push(self.parse_list_item()?);
        }

        if items.is_empty() {
            return Err(ParseError {
                message: "list requires at least one item".to_string(),
                span: start,
            });
        }

        let end_token = self.expect(TokenKind::RBrace)?;
        let span = start.merge(end_token.span);

        Ok(Primitive::List(List { items, span }))
    }

    fn parse_list_item(&mut self) -> Result<ListItem, ParseError> {
        let start = self.current().span;
        self.advance(); // consume 'item'

        let content = match self.peek().clone() {
            TokenKind::String(s) => {
                self.advance();
                s
            }
            _ => {
                return Err(ParseError {
                    message: format!("item requires a content string, found {}", self.peek()),
                    span: self.current().span,
                });
            }
        };

        let selected = if self.check(&TokenKind::Selected) {
            self.advance();
            true
        } else {
            false
        };

        let span = start.merge(self.tokens[self.pos.saturating_sub(1)].span);

        Ok(ListItem {
            content,
            selected,
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Program {
        Parser::parse(source).unwrap()
    }

    fn parse_err(source: &str) -> ParseError {
        Parser::parse(source).unwrap_err()
    }

    // ========== Basic layout tests ==========

    #[test]
    fn empty_layout() {
        let program = parse("layout { }");
        assert!(program.layout.statements.is_empty());
    }

    #[test]
    fn layout_required() {
        let err = parse_err("window { }");
        assert!(err.message.contains("Expected layout"));
    }

    // ========== Container tests ==========

    #[test]
    fn window_with_title() {
        let program = parse(r#"layout { window "My Window" { } }"#);
        assert_eq!(program.layout.statements.len(), 1);

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Window);
            assert_eq!(c.title, Some("My Window".to_string()));
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn window_without_title() {
        let program = parse("layout { window { } }");

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Window);
            assert_eq!(c.title, None);
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn box_with_title() {
        let program = parse(r#"layout { box "Section" { } }"#);

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Box);
            assert_eq!(c.title, Some("Section".to_string()));
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn section_requires_title() {
        let err = parse_err("layout { section { } }");
        assert!(err.message.contains("section requires a title"));
    }

    #[test]
    fn section_with_title() {
        let program = parse(r#"layout { section "Options" { } }"#);

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Section);
            assert_eq!(c.title, Some("Options".to_string()));
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn row_container() {
        let program = parse("layout { row { } }");

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Row);
            assert_eq!(c.title, None);
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn column_container() {
        let program = parse("layout { column { } }");

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.kind, ContainerKind::Column);
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn container_with_width() {
        let program = parse("layout { window width:50 { } }");

        if let Statement::Container(c) = &program.layout.statements[0] {
            assert_eq!(c.modifiers.width, Some(50));
        } else {
            panic!("Expected container");
        }
    }

    #[test]
    fn nested_containers() {
        let program = parse(r#"layout {
            window "Outer" {
                box "Inner" {
                    row { }
                }
            }
        }"#);

        if let Statement::Container(window) = &program.layout.statements[0] {
            assert_eq!(window.kind, ContainerKind::Window);
            assert_eq!(window.statements.len(), 1);

            if let Statement::Container(box_c) = &window.statements[0] {
                assert_eq!(box_c.kind, ContainerKind::Box);
                assert_eq!(box_c.statements.len(), 1);

                if let Statement::Container(row) = &box_c.statements[0] {
                    assert_eq!(row.kind, ContainerKind::Row);
                }
            }
        }
    }

    // ========== Primitive tests ==========

    #[test]
    fn text_simple() {
        let program = parse(r#"layout { text "Hello" }"#);

        if let Statement::Primitive(Primitive::Text { content, .. }) = &program.layout.statements[0] {
            assert_eq!(content, "Hello");
        } else {
            panic!("Expected text primitive");
        }
    }

    #[test]
    fn text_with_modifiers() {
        let program = parse(r#"layout { text "Hello" align:center style:bold }"#);

        if let Statement::Primitive(Primitive::Text { content, modifiers, .. }) = &program.layout.statements[0] {
            assert_eq!(content, "Hello");
            assert_eq!(modifiers.align, Some(Align::Center));
            assert_eq!(modifiers.style, Some(Style::Bold));
        } else {
            panic!("Expected text primitive");
        }
    }

    #[test]
    fn input_requires_width() {
        let err = parse_err("layout { input }");
        assert!(err.message.contains("input requires width"));
    }

    #[test]
    fn input_with_width() {
        let program = parse("layout { input width:20 }");

        if let Statement::Primitive(Primitive::Input { modifiers, .. }) = &program.layout.statements[0] {
            assert_eq!(modifiers.width, Some(20));
        } else {
            panic!("Expected input primitive");
        }
    }

    #[test]
    fn input_with_placeholder() {
        let program = parse(r#"layout { input width:20 placeholder:"email" }"#);

        if let Statement::Primitive(Primitive::Input { modifiers, .. }) = &program.layout.statements[0] {
            assert_eq!(modifiers.width, Some(20));
            assert_eq!(modifiers.placeholder, Some("email".to_string()));
        } else {
            panic!("Expected input primitive");
        }
    }

    #[test]
    fn checkbox_unchecked() {
        let program = parse(r#"layout { checkbox "Enable" }"#);

        if let Statement::Primitive(Primitive::Checkbox { label, checked, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "Enable");
            assert!(!checked);
        } else {
            panic!("Expected checkbox primitive");
        }
    }

    #[test]
    fn checkbox_checked() {
        let program = parse(r#"layout { checkbox "Enable" checked }"#);

        if let Statement::Primitive(Primitive::Checkbox { label, checked, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "Enable");
            assert!(checked);
        } else {
            panic!("Expected checkbox primitive");
        }
    }

    #[test]
    fn radio_unselected() {
        let program = parse(r#"layout { radio "Option A" }"#);

        if let Statement::Primitive(Primitive::Radio { label, selected, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "Option A");
            assert!(!selected);
        } else {
            panic!("Expected radio primitive");
        }
    }

    #[test]
    fn radio_selected() {
        let program = parse(r#"layout { radio "Option A" selected }"#);

        if let Statement::Primitive(Primitive::Radio { label, selected, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "Option A");
            assert!(selected);
        } else {
            panic!("Expected radio primitive");
        }
    }

    #[test]
    fn select_requires_width() {
        let err = parse_err(r#"layout { select "Value" }"#);
        assert!(err.message.contains("select requires width"));
    }

    #[test]
    fn select_with_width() {
        let program = parse(r#"layout { select "Mono" width:12 }"#);

        if let Statement::Primitive(Primitive::Select { value, modifiers, .. }) = &program.layout.statements[0] {
            assert_eq!(value, "Mono");
            assert_eq!(modifiers.width, Some(12));
        } else {
            panic!("Expected select primitive");
        }
    }

    #[test]
    fn button_simple() {
        let program = parse(r#"layout { button "OK" }"#);

        if let Statement::Primitive(Primitive::Button { label, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "OK");
        } else {
            panic!("Expected button primitive");
        }
    }

    #[test]
    fn button_with_style() {
        let program = parse(r#"layout { button "Delete" style:danger }"#);

        if let Statement::Primitive(Primitive::Button { label, modifiers, .. }) = &program.layout.statements[0] {
            assert_eq!(label, "Delete");
            assert_eq!(modifiers.style, Some(Style::Danger));
        } else {
            panic!("Expected button primitive");
        }
    }

    #[test]
    fn link_simple() {
        let program = parse(r#"layout { link "Click here" }"#);

        if let Statement::Primitive(Primitive::Link { text, .. }) = &program.layout.statements[0] {
            assert_eq!(text, "Click here");
        } else {
            panic!("Expected link primitive");
        }
    }

    #[test]
    fn separator_simple() {
        let program = parse("layout { separator }");

        if let Statement::Primitive(Primitive::Separator { .. }) = &program.layout.statements[0] {
            // OK
        } else {
            panic!("Expected separator primitive");
        }
    }

    #[test]
    fn spacer_simple() {
        let program = parse("layout { spacer }");

        if let Statement::Primitive(Primitive::Spacer { .. }) = &program.layout.statements[0] {
            // OK
        } else {
            panic!("Expected spacer primitive");
        }
    }

    #[test]
    fn progress_valid() {
        let program = parse("layout { progress 75 }");

        if let Statement::Primitive(Primitive::Progress { value, .. }) = &program.layout.statements[0] {
            assert_eq!(*value, 75);
        } else {
            panic!("Expected progress primitive");
        }
    }

    #[test]
    fn progress_zero() {
        let program = parse("layout { progress 0 }");

        if let Statement::Primitive(Primitive::Progress { value, .. }) = &program.layout.statements[0] {
            assert_eq!(*value, 0);
        } else {
            panic!("Expected progress primitive");
        }
    }

    #[test]
    fn progress_hundred() {
        let program = parse("layout { progress 100 }");

        if let Statement::Primitive(Primitive::Progress { value, .. }) = &program.layout.statements[0] {
            assert_eq!(*value, 100);
        } else {
            panic!("Expected progress primitive");
        }
    }

    #[test]
    fn progress_over_100_error() {
        let err = parse_err("layout { progress 101 }");
        assert!(err.message.contains("0-100"));
    }

    // ========== Code/Raw tests ==========

    #[test]
    fn code_single_line() {
        let program = parse(r#"layout { code "fn main() {}" }"#);

        if let Statement::Primitive(Primitive::Code { content, .. }) = &program.layout.statements[0] {
            assert_eq!(content, "fn main() {}");
        } else {
            panic!("Expected code primitive");
        }
    }

    #[test]
    fn code_multiline() {
        let program = parse(r#"layout {
            code ```
fn main() {
    println!("hi");
}
```
        }"#);

        if let Statement::Primitive(Primitive::Code { content, .. }) = &program.layout.statements[0] {
            assert!(content.contains("fn main()"));
            assert!(content.contains("println!"));
        } else {
            panic!("Expected code primitive");
        }
    }

    #[test]
    fn raw_multiline() {
        let program = parse(r#"layout {
            raw ```
┌───┐
│ A │
└───┘
```
        }"#);

        if let Statement::Primitive(Primitive::Raw { content, .. }) = &program.layout.statements[0] {
            assert!(content.contains("┌───┐"));
            assert!(content.contains("│ A │"));
        } else {
            panic!("Expected raw primitive");
        }
    }

    // ========== Alert tests ==========

    #[test]
    fn alert_requires_type() {
        let err = parse_err("layout { alert { } }");
        assert!(err.message.contains("alert requires type"));
    }

    #[test]
    fn alert_error() {
        let program = parse(r#"layout { alert type:error { text "Error!" } }"#);

        if let Statement::Primitive(Primitive::Alert { alert_type, statements, .. }) = &program.layout.statements[0] {
            assert_eq!(*alert_type, AlertType::Error);
            assert_eq!(statements.len(), 1);
        } else {
            panic!("Expected alert primitive");
        }
    }

    #[test]
    fn alert_warn() {
        let program = parse(r#"layout { alert type:warn { text "Warning" } }"#);

        if let Statement::Primitive(Primitive::Alert { alert_type, .. }) = &program.layout.statements[0] {
            assert_eq!(*alert_type, AlertType::Warn);
        } else {
            panic!("Expected alert primitive");
        }
    }

    #[test]
    fn alert_info() {
        let program = parse(r#"layout { alert type:info { text "Info" } }"#);

        if let Statement::Primitive(Primitive::Alert { alert_type, .. }) = &program.layout.statements[0] {
            assert_eq!(*alert_type, AlertType::Info);
        } else {
            panic!("Expected alert primitive");
        }
    }

    // ========== Table tests ==========

    #[test]
    fn table_with_header_and_rows() {
        let program = parse(r#"layout {
            table {
                header { col "Name" width:10 col "Age" width:5 }
                tr { td "Alice" td "30" }
                tr { td "Bob" td "25" }
            }
        }"#);

        if let Statement::Primitive(Primitive::Table(table)) = &program.layout.statements[0] {
            assert!(table.header.is_some());
            let header = table.header.as_ref().unwrap();
            assert_eq!(header.columns.len(), 2);
            assert_eq!(header.columns[0].label, "Name");
            assert_eq!(header.columns[0].modifiers.width, Some(10));
            assert_eq!(header.columns[1].label, "Age");
            assert_eq!(header.columns[1].modifiers.width, Some(5));

            assert_eq!(table.rows.len(), 2);
            assert_eq!(table.rows[0].cells.len(), 2);
            assert_eq!(table.rows[0].cells[0].content, "Alice");
            assert_eq!(table.rows[0].cells[1].content, "30");
        } else {
            panic!("Expected table primitive");
        }
    }

    #[test]
    fn table_col_requires_width() {
        let err = parse_err(r#"layout { table { header { col "Name" } } }"#);
        assert!(err.message.contains("col requires width"));
    }

    #[test]
    fn table_header_requires_col() {
        let err = parse_err("layout { table { header { } } }");
        assert!(err.message.contains("header requires at least one col"));
    }

    #[test]
    fn table_row_requires_cell() {
        let err = parse_err("layout { table { tr { } } }");
        assert!(err.message.contains("tr requires at least one td"));
    }

    #[test]
    fn table_cell_with_align() {
        let program = parse(r#"layout {
            table {
                header { col "Value" width:10 align:right }
                tr { td "123" align:right }
            }
        }"#);

        if let Statement::Primitive(Primitive::Table(table)) = &program.layout.statements[0] {
            let header = table.header.as_ref().unwrap();
            assert_eq!(header.columns[0].modifiers.align, Some(Align::Right));
            assert_eq!(table.rows[0].cells[0].modifiers.align, Some(Align::Right));
        } else {
            panic!("Expected table primitive");
        }
    }

    // ========== List tests ==========

    #[test]
    fn list_with_items() {
        let program = parse(r#"layout {
            list {
                item "First"
                item "Second" selected
                item "Third"
            }
        }"#);

        if let Statement::Primitive(Primitive::List(list)) = &program.layout.statements[0] {
            assert_eq!(list.items.len(), 3);
            assert_eq!(list.items[0].content, "First");
            assert!(!list.items[0].selected);
            assert_eq!(list.items[1].content, "Second");
            assert!(list.items[1].selected);
            assert_eq!(list.items[2].content, "Third");
            assert!(!list.items[2].selected);
        } else {
            panic!("Expected list primitive");
        }
    }

    #[test]
    fn list_requires_item() {
        let err = parse_err("layout { list { } }");
        assert!(err.message.contains("list requires at least one item"));
    }

    // ========== Modifier validation tests ==========

    #[test]
    fn unknown_attribute_error() {
        let err = parse_err("layout { window foo:bar { } }");
        assert!(err.message.contains("Unknown attribute"));
    }

    #[test]
    fn invalid_align_value() {
        let err = parse_err(r#"layout { text "hi" align:middle }"#);
        assert!(err.message.contains("align requires left/center/right"));
    }

    #[test]
    fn invalid_style_value() {
        let err = parse_err(r#"layout { text "hi" style:italic }"#);
        assert!(err.message.contains("style requires bold/dim/danger"));
    }

    #[test]
    fn invalid_type_value() {
        let err = parse_err(r#"layout { alert type:critical { } }"#);
        assert!(err.message.contains("type requires error/warn/info"));
    }

    #[test]
    fn width_requires_number() {
        let err = parse_err(r#"layout { window width:"big" { } }"#);
        assert!(err.message.contains("width requires a number"));
    }

    // ========== Complex structure tests ==========

    #[test]
    fn login_form() {
        let program = parse(r#"layout {
            window "Login" {
                row { text "Username:" input width:20 }
                row { text "Password:" input width:20 }
                separator
                row { spacer button "Cancel" button "Login" }
            }
        }"#);

        if let Statement::Container(window) = &program.layout.statements[0] {
            assert_eq!(window.title, Some("Login".to_string()));
            assert_eq!(window.statements.len(), 4);
        } else {
            panic!("Expected window container");
        }
    }

    #[test]
    fn settings_panel() {
        let program = parse(r#"layout {
            window "Preferences" width:45 {
                section "Appearance" {
                    row { text "Theme:" radio "Dark" selected radio "Light" }
                    checkbox "Line numbers" checked
                    checkbox "Word wrap"
                }
                section "Editor" {
                    row { text "Tab size:" input width:4 placeholder:"4" }
                    row { text "Font:" select "Mono" width:12 }
                }
                separator
                row { spacer button "Apply" }
            }
        }"#);

        if let Statement::Container(window) = &program.layout.statements[0] {
            assert_eq!(window.title, Some("Preferences".to_string()));
            assert_eq!(window.modifiers.width, Some(45));
            // 2 sections + separator + 1 row = 4
            assert_eq!(window.statements.len(), 4);
        } else {
            panic!("Expected window container");
        }
    }

    #[test]
    fn build_status() {
        let program = parse(r#"layout {
            window "Build" width:35 {
                text "Compiling project..."
                progress 65
                separator
                alert type:warn {
                    text "unused variable"
                }
            }
        }"#);

        if let Statement::Container(window) = &program.layout.statements[0] {
            assert_eq!(window.statements.len(), 4);
        } else {
            panic!("Expected window container");
        }
    }
}

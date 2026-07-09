//! AST types for asciiscript DSL.
//!
//! The AST closely mirrors the grammar defined in docs/asciiscript-spec.md.

use std::fmt;

/// A complete asciiscript program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub layout: Layout,
}

/// The root layout container.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// A statement in a block - either a container or primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Container(Container),
    Primitive(Primitive),
}

/// Container types that hold other statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub kind: ContainerKind,
    pub title: Option<String>,
    pub modifiers: Modifiers,
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Window,
    Box,
    Section,
    Row,
    Column,
}

impl fmt::Display for ContainerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerKind::Window => write!(f, "window"),
            ContainerKind::Box => write!(f, "box"),
            ContainerKind::Section => write!(f, "section"),
            ContainerKind::Row => write!(f, "row"),
            ContainerKind::Column => write!(f, "column"),
        }
    }
}

/// Primitive elements (leaf nodes).
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Text {
        content: String,
        modifiers: Modifiers,
        span: Span,
    },
    Input {
        modifiers: Modifiers,
        span: Span,
    },
    Checkbox {
        label: String,
        checked: bool,
        span: Span,
    },
    Radio {
        label: String,
        selected: bool,
        span: Span,
    },
    Select {
        value: String,
        modifiers: Modifiers,
        span: Span,
    },
    Button {
        label: String,
        modifiers: Modifiers,
        span: Span,
    },
    Link {
        text: String,
        span: Span,
    },
    Code {
        content: String,
        span: Span,
    },
    Raw {
        content: String,
        span: Span,
    },
    Separator {
        span: Span,
    },
    Spacer {
        span: Span,
    },
    Progress {
        value: u8, // 0-100
        span: Span,
    },
    Alert {
        alert_type: AlertType,
        statements: Vec<Statement>,
        span: Span,
    },
    Table(Table),
    List(List),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    Error,
    Warn,
    Info,
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertType::Error => write!(f, "error"),
            AlertType::Warn => write!(f, "warn"),
            AlertType::Info => write!(f, "info"),
        }
    }
}

/// Table structure with header and rows.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub header: Option<TableHeader>,
    pub rows: Vec<TableRow>,
    pub modifiers: Modifiers,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableHeader {
    pub columns: Vec<TableColumn>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub label: String,
    pub modifiers: Modifiers,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub content: String,
    pub modifiers: Modifiers,
    pub span: Span,
}

/// List with selectable items.
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub items: Vec<ListItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    pub content: String,
    pub selected: bool,
    pub span: Span,
}

/// Modifiers (attributes + flags).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Modifiers {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub padding: Option<u32>,
    pub gap: Option<u32>,
    pub align: Option<Align>,
    pub style: Option<Style>,
    pub alert_type: Option<AlertType>,
    pub placeholder: Option<String>,
}

impl Modifiers {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Align::Left => write!(f, "left"),
            Align::Center => write!(f, "center"),
            Align::Right => write!(f, "right"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Bold,
    Dim,
    Danger,
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Style::Bold => write!(f, "bold"),
            Style::Dim => write!(f, "dim"),
            Style::Danger => write!(f, "danger"),
        }
    }
}

/// Source location for error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_positions(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start: Position::new(start_line, start_col),
            end: Position::new(end_line, end_col),
        }
    }

    pub fn merge(self, other: Span) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,   // 1-indexed
    pub column: u32, // 1-indexed
}

impl Position {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start.line == self.end.line {
            write!(
                f,
                "{}:{}-{}",
                self.start.line, self.start.column, self.end.column
            )
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

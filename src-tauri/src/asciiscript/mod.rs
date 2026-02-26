//! asciiscript DSL parser and renderer.
//!
//! A declarative DSL for agents to describe UI mockups that compile to ASCII box art.
//! See docs/asciiscript-spec.md for the full specification.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod render;

#[cfg(test)]
mod tests;

pub use ast::*;
pub use lexer::{Lexer, LexerError, Token, TokenKind};
pub use parser::{ParseError, Parser};
pub use render::render;

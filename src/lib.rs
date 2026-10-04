//! Lossless, pre-macro HSP formatting for byte and UTF-8 input.

pub mod config;
mod document;
mod encoding;
mod error;
mod formatter;
mod lexer;
mod options;
mod parser;
mod util;

pub use encoding::{detect_encoding, Encoding};
pub use error::Error;
pub use formatter::{format, format_utf8};
pub use lexer::{lex, Kind, Token};
pub use options::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    AmbiguousLabelOrMultiplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// One-based physical line in the original source.
    pub line: usize,
    /// Original line bytes, excluding its newline.
    pub source: Vec<u8>,
    /// Zero-based offset of the original line, including any BOM or indentation.
    pub byte_offset: usize,
    pub kind: DiagnosticKind,
}

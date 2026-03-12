use super::span::Span;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

pub type Result<T, E = ParseError> = std::result::Result<T, E>;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: span
        write!(f, "Parse error at {:?}: {}", self.span, self.message)
    }
}

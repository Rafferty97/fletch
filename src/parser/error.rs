use super::span::Span;
use line_index::LineIndex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub line_index: Arc<LineIndex>,
}

pub type Result<T, E = ParseError> = std::result::Result<T, E>;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line_col = self.line_index.line_col(self.span.start());
        write!(f, "Parse error at {:?}: {}", line_col, self.message)
    }
}

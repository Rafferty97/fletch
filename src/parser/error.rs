use super::span::Span;
use crate::parser::lexer::LexError;
use line_index::LineIndex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub line_index: Option<Arc<LineIndex>>,
}

pub type Result<T, E = ParseError> = std::result::Result<T, E>;

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.line_index {
            Some(idx) => {
                let line_col = idx.line_col(self.span.start());
                write!(f, "Parse error at {:?}: {}", line_col, self.message)
            }
            None => write!(f, "Parse error at {:?}: {}", self.span, self.message),
        }
    }
}

impl From<LexError> for ParseError {
    fn from(err: LexError) -> Self {
        Self {
            message: err.message.into(),
            span: err.span,
            line_index: None,
        }
    }
}

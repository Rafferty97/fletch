use crate::util::span::Span;
use line_index::LineIndex;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub struct Error {
    pub message: String,
    pub span: Span,
    pub line_index: Option<Arc<LineIndex>>,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl std::fmt::Display for Error {
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

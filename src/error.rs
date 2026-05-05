use crate::{ast::BinOp, typecheck::Ty, util::span::Span};
use line_index::LineIndex;
use std::{num::ParseIntError, sync::Arc};
use thiserror::Error;

#[derive(Error, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
    pub line_index: Option<Arc<LineIndex>>,
}

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("expected {expected}, got {got}")]
    MismatchedType { expected: String, got: String },
    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    pub fn new_type_mismatch(expected: Ty, got: Ty, span: Span) -> Self {
        let kind =
            ErrorKind::MismatchedType { expected: expected.to_string(), got: got.to_string() };
        Self { kind, span, line_index: None }
    }

    pub fn new_binop(op: BinOp, lhs: Ty, rhs: Ty, span: Span) -> Self {
        let kind = ErrorKind::Other(format!("no implementation for {} {} {}", lhs, rhs, op));
        Self { kind, span, line_index: None }
    }

    pub fn new_other(msg: impl Into<String>, span: Span) -> Self {
        let kind = ErrorKind::Other(msg.into());
        Self { kind, span, line_index: None }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.line_index {
            Some(idx) => {
                let line_col = idx.line_col(self.span.start());
                write!(f, "Error at {:?}: {}", line_col, self.kind)
            }
            None => write!(f, "Error at {:?}: {}", self.span, self.kind),
        }
    }
}

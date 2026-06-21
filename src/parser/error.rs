use thiserror::Error;

use crate::parser::escape::{UnescapeError, UnescapeErrorKind};
use crate::span::Span;

use super::Parser;
use super::lexer::Token;

#[derive(Error, Clone, Debug)]
#[error("Syntax error: {kind}")]
pub struct ParseError<'a> {
    pub kind: ParseErrorKind<'a>,
    pub span: Span,
}

pub type Result<'a, T> = std::result::Result<T, ParseError<'a>>;

#[derive(Error, Clone, Debug)]
pub enum ParseErrorKind<'a> {
    #[error("lex error")]
    Lex,
    #[error("unexpected {act}, expected {exp}")]
    ExpectedToken { act: Token<'a>, exp: Token<'a> },
    #[error("unexpected {act}")]
    UnexpectedToken { act: Token<'a> },
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("{0}")]
    UnescapeStr(UnescapeErrorKind),
}

impl<'a, 'sym> Parser<'a, 'sym> {
    /// Creates a parser error with the span of the token that was just consumed
    pub(super) fn error(&self, kind: ParseErrorKind<'a>) -> ParseError<'a> {
        ParseError { kind, span: self.previous.span }
    }

    /// Creates an unexpected token error from the current (unconsumed) token
    pub(super) fn unexpected_curr(&self) -> ParseError<'a> {
        let kind = ParseErrorKind::UnexpectedToken { act: self.current.token };
        ParseError { kind, span: self.current.span }
    }

    /// Creates an unexpected token error from the previously consumed token
    pub(super) fn unexpected_prev(&self) -> ParseError<'a> {
        let kind = ParseErrorKind::UnexpectedToken { act: self.previous.token };
        ParseError { kind, span: self.previous.span }
    }
}

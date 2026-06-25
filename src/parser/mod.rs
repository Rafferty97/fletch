use std::iter::Peekable;
use std::sync::atomic::{AtomicU32, Ordering};

use bumpalo::Bump;
use codespan_reporting::diagnostic;
use logos::{Lexer, Logos, SpannedIter};

use crate::ast::span::{Span, Spanned};
use crate::ast::{Expr, NodeId, Program, Symbol};
use crate::diagnostics::{Diagnostic, DiagnosticReporter};
use crate::interner::IndexedInterner;
use crate::parser::error::Result;
use crate::parser::lexer::Token;
use crate::util::IdGen;

pub mod error;
pub mod escape;
mod expr;
mod helpers;
mod lexer;
mod program;
mod test;

#[derive(Copy, Clone)]
pub struct ParseCtx<'a, 'sym> {
    pub arena: &'sym Bump,
    pub sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
    /// The sink for diagnostics
    pub errors: &'a dyn DiagnosticReporter,
}

pub struct Parser<'a, 'sym> {
    /// The parsing context
    ctx: ParseCtx<'a, 'sym>,
    /// The lexer
    lexer: SpannedIter<'a, Token<'a>>,
    /// The next token to be consumed
    current: SpannedToken<'a>,
    /// The previously consumed token
    previous: SpannedToken<'a>,
    /// Generates node IDs
    node_ids: IdGen<NodeId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SpannedToken<'a> {
    token: Token<'a>,
    span: Span,
}

impl<'a, 'sym> ParseCtx<'a, 'sym> {
    pub fn new(
        arena: &'sym Bump,
        sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
        errors: &'a dyn DiagnosticReporter,
    ) -> Self {
        Self { arena, sym_interner, errors }
    }
}

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn new(ctx: ParseCtx<'a, 'sym>, src: &'a str) -> Self {
        let mut parser = Self {
            ctx,
            lexer: Token::lexer(src).spanned(),
            current: SpannedToken::dummy(),
            previous: SpannedToken::dummy(),
            node_ids: IdGen::new(NodeId),
        };
        parser.consume();
        parser
    }
}

impl<'a> SpannedToken<'a> {
    pub fn dummy() -> Self {
        Self { token: Token::dummy(), span: Span::dummy() }
    }
}

use std::iter::Peekable;

use bumpalo::Bump;
use logos::{Lexer, Logos, SpannedIter};

use crate::ast::Symbol;
use crate::interner::IndexedInterner;
use crate::span::{Span, Spanned};

use self::error::ParseErrorKind;
use self::error::{ParseError, Result};
use self::lexer::Token;

pub mod error;
mod escape;
mod expr;
mod lexer;
mod program;
mod test;

#[derive(Copy, Clone, Debug)]
pub struct ParseCtx<'a, 'sym> {
    pub arena: &'sym Bump,
    pub sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SpannedToken<'a> {
    token: Token<'a>,
    span: Span,
}

impl<'a, 'sym> ParseCtx<'a, 'sym> {
    pub fn new(arena: &'sym Bump, sym_interner: &'a IndexedInterner<'sym, Symbol, str>) -> Self {
        Self { arena, sym_interner }
    }
}

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn new(ctx: ParseCtx<'a, 'sym>, src: &'a str) -> Self {
        Self {
            ctx,
            lexer: Token::lexer(src).spanned(),
            current: SpannedToken::dummy(),
            previous: SpannedToken::dummy(),
        }
    }

    fn make_symbol(&self, raw: &str) -> Symbol {
        self.ctx.sym_interner.intern_str(self.ctx.arena, raw)
    }

    fn curr_pos(&self) -> u32 {
        self.current.span.lo()
    }

    fn make_span(&self, start: u32) -> Span {
        Span::new(start, self.curr_pos())
    }

    fn make_spanned<T>(&self, start: u32, node: T) -> Spanned<T> {
        let span = self.make_span(start);
        Spanned { node, span }
    }

    /// Returns the current token without consuming it
    fn peek(&self) -> Token<'a> {
        self.current.token
    }

    /// Consumes the current token and returns it
    fn consume(&mut self) -> Result<'a, SpannedToken<'a>> {
        self.previous = self.current;
        self.current = match self.lexer.next() {
            Some((Ok(token), span)) => SpannedToken { token, span: span.into() },
            Some((Err(_), span)) => Err(ParseError { kind: ParseErrorKind::Lex, span: span.into() })?,
            None => {
                let pos = self.previous.span.hi();
                SpannedToken { token: Token::Eof, span: Span::new(pos, pos) }
            }
        };
        Ok(self.previous)
    }

    /// Consumes the current token, checks that it matches the expected token, and returns it
    fn expect(&mut self, expected: Token<'a>) -> Result<'a, SpannedToken<'a>> {
        let token = self.consume()?;
        if token.token != expected {
            Err(self.error(ParseErrorKind::ExpectedToken { act: token.token, exp: expected }))?;
        }
        Ok(token)
    }

    /// Checks whether the next token matches the predicate, and if so, consumes and returns it
    fn consume_if(&mut self, pred: impl FnOnce(Token<'a>) -> bool) -> Result<'a, Option<SpannedToken<'a>>> {
        Ok(if pred(self.current.token) {
            Some(self.consume()?)
        } else {
            None
        })
    }

    /// Checks whether the next token matches the predicate, and if so, consumes it
    fn check(&mut self, pred: impl FnOnce(Token<'a>) -> bool) -> Result<'a, bool> {
        Ok(self.consume_if(pred)?.is_some())
    }
}

impl<'a> SpannedToken<'a> {
    pub fn dummy() -> Self {
        Self { token: Token::dummy(), span: Span::dummy() }
    }
}

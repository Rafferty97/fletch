use std::iter::Peekable;
use std::sync::atomic::{AtomicU32, Ordering};

use bumpalo::Bump;
use logos::{Lexer, Logos, SpannedIter};

use crate::ast::span::{Span, Spanned};
use crate::ast::{Expr, NodeId, Program, Symbol};
use crate::interner::IndexedInterner;

use self::error::ParseErrorKind;
use self::error::{ParseError, Result};
use self::lexer::Token;

pub mod error;
pub mod escape;
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
    /// The next `NodeId`
    next_id: AtomicU32,
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

    pub fn parse_program(self, src: &'a str) -> Result<'a, Program> {
        let mut parser = Parser::new(self, src);
        parser.consume()?;
        parser.parse_program()
    }
}

impl<'a, 'sym> Parser<'a, 'sym> {
    fn new(ctx: ParseCtx<'a, 'sym>, src: &'a str) -> Self {
        Self {
            ctx,
            lexer: Token::lexer(src).spanned(),
            current: SpannedToken::dummy(),
            previous: SpannedToken::dummy(),
            next_id: AtomicU32::new(0),
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

    fn make_spanned<T>(&mut self, start: u32, node: T) -> Spanned<T> {
        let id = self.next_id();
        let span = self.make_span(start);
        Spanned { id, node, span }
    }

    fn next_id(&self) -> NodeId {
        NodeId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the current token without consuming it
    fn peek(&self) -> Token<'a> {
        self.current.token
    }

    /// Consumes the current token and returns it
    fn consume(&mut self) -> Result<'a, SpannedToken<'a>> {
        self.previous = self.current;
        self.current = loop {
            let token = match self.lexer.next() {
                Some((Ok(token), span)) => SpannedToken { token, span: span.into() },
                Some((Err(_), span)) => Err(ParseError { kind: ParseErrorKind::Lex, span: span.into() })?,
                None => {
                    let pos = self.previous.span.hi();
                    SpannedToken { token: Token::Eof, span: Span::new(pos, pos) }
                }
            };
            if token.token == Token::Newline {
                if self.should_insert_semi() {
                    break SpannedToken { token: Token::Semi, ..token };
                }
            } else {
                break token;
            }
        };
        Ok(self.previous)
    }

    fn should_insert_semi(&self) -> bool {
        match self.previous.token {
            Token::Ident(_) => true,
            Token::Null => true,
            Token::False | Token::True => true,
            Token::Number(_) => true,
            Token::Str(_) => true,
            Token::RightParen => true,
            // Token::RightBrace => true,
            _ => false,
        }
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

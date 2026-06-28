use crate::ast::Symbol;
use crate::ast::span::{Span, Spanned};
use crate::diagnostics::Diagnostic;
use crate::parser::SpannedToken;
use crate::parser::error::Result;
use crate::parser::{Parser, lexer::Token};

impl<'a, 'sym> Parser<'a, 'sym> {
    pub(super) fn make_symbol(&self, raw: &str) -> Symbol {
        self.ctx.sym_interner.intern_str(self.ctx.arena, raw)
    }

    pub(super) fn curr_pos(&self) -> u32 {
        self.current.span.lo()
    }

    pub(super) fn make_span(&self, start: u32) -> Span {
        Span::new(start, self.curr_pos())
    }

    pub(super) fn make_spanned<T>(&mut self, start: u32, node: T) -> Spanned<T> {
        let id = self.node_ids.next();
        let span = self.make_span(start);
        Spanned { id, node, span }
    }

    /// Returns the current token without consuming it
    pub(super) fn peek(&self) -> Token<'a> {
        self.current.token
    }

    /// Consumes the current token and returns it
    pub(super) fn consume(&mut self) -> SpannedToken<'a> {
        self.previous = self.current;
        self.current = loop {
            let token = match self.lexer.next() {
                Some((Ok(token), span)) => SpannedToken { token, span: span.into() },
                Some((Err(_), span)) => SpannedToken { token: Token::Err('\0'), span: span.into() },
                None => {
                    let pos = self.previous.span.hi();
                    SpannedToken { token: Token::Eof, span: Span::new(pos, pos) }
                }
            };
            // if matches!(token.token, Token::Newline | Token::Eof) {
            //     if self.should_insert_semi() {
            //         break SpannedToken { token: Token::Semi, ..token };
            //     }
            // }
            if token.token != Token::Newline {
                break token;
            }
        };
        self.previous
    }

    pub(super) fn should_insert_semi(&self) -> bool {
        match self.previous.token {
            Token::Ident(_) => true,
            Token::Null => true,
            Token::False | Token::True => true,
            Token::Number(_) => true,
            Token::Str(_) => true,
            Token::RightParen => true,
            Token::RightBracket => true,
            Token::RightBrace => true,
            _ => false,
        }
    }

    /// Consumes the current token, checks that it matches the expected token, and returns it
    pub(super) fn expect(&mut self, expected: Token<'a>) -> Result<SpannedToken<'a>> {
        let token = self.consume();
        if token.token != expected {
            let diagnostic = Diagnostic::error(format!("expected {expected}"), token.span);
            Err(self.report_err(diagnostic))?;
        }
        Ok(token)
    }

    /// Checks whether the next token matches the predicate, and if so, consumes and returns it
    pub(super) fn consume_if(&mut self, pred: impl FnOnce(Token<'a>) -> bool) -> Option<SpannedToken<'a>> {
        if pred(self.current.token) {
            Some(self.consume())
        } else {
            None
        }
    }

    /// Checks whether the next token matches the predicate, and if so, consumes it
    pub(super) fn check(&mut self, pred: impl FnOnce(Token<'a>) -> bool) -> bool {
        self.consume_if(pred).is_some()
    }
}

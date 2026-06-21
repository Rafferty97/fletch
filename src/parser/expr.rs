use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol};
use crate::parser::SpannedToken;
use crate::parser::escape::unescape;
use crate::span::{Span, Spanned};

use super::Parser;
use super::error::{ParseError, ParseErrorKind, Result};
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub(super) fn parse_expr(&mut self) -> Result<'a, Expr> {
        let expr = self.parse_prefix()?;

        // FIXME
        match self.peek() {
            Token::Plus => {
                let lhs = expr;
                self.consume()?;
                let rhs = self.parse_prefix()?;
                let span = Span::cover(lhs.span, rhs.span);
                let node = ExprKind::Binary(BinOp::Add, lhs.into(), rhs.into());
                Ok(Expr { node, span })
            }
            _ => Ok(expr),
        }
    }

    pub(super) fn parse_prefix(&mut self) -> Result<'a, Expr> {
        let start = self.curr_pos();

        let kind = match self.peek() {
            Token::Null => {
                self.consume()?;
                ExprKind::Lit(Lit::Null)
            }
            Token::False => {
                self.consume()?;
                ExprKind::Lit(Lit::Bool(false))
            }
            Token::True => {
                self.consume()?;
                ExprKind::Lit(Lit::Bool(true))
            }
            Token::Number(raw) => {
                self.consume()?;
                let lit = if raw.contains('.') {
                    Lit::Float(self.make_symbol(raw))
                } else {
                    Lit::Int(self.make_symbol(raw))
                };
                ExprKind::Lit(lit)
            }
            Token::Str(raw) => {
                self.consume()?;
                let unescaped = match unescape(&raw[1..raw.len() - 1]) {
                    Ok(str) => self.make_symbol(&str),
                    Err(err) => {
                        let kind = ParseErrorKind::UnescapeStr(err.kind);
                        let span = Span::at(start + err.pos);
                        Err(ParseError { kind, span })?
                    }
                };
                ExprKind::Lit(Lit::Str(unescaped))
            }
            Token::Ident(raw) => ExprKind::Var(self.parse_ident()?),
            _ => Err(self.unexpected_curr())?,
        };

        Ok(self.make_spanned(start, kind))
    }
}

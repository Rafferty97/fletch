use codespan_reporting::diagnostic;

use crate::ast::span::{Span, Spanned};
use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol};
use crate::diagnostics::Diagnostic;
use crate::parser::SpannedToken;
use crate::parser::escape::unescape;
use crate::parser::lexer::Token::RightParen;

use super::Parser;
use super::error::Result;
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub(super) fn parse_expr(&mut self) -> Result<Expr> {
        let start = self.curr_pos();
        let expr = self.parse_prefix()?;

        // FIXME
        let node = match self.peek() {
            Token::Plus => {
                let lhs = expr;
                self.consume();
                let rhs = self.parse_prefix()?;
                ExprKind::Binary(BinOp::Add, lhs.into(), rhs.into())
            }
            Token::Minus => {
                let lhs = expr;
                self.consume();
                let rhs = self.parse_prefix()?;
                ExprKind::Binary(BinOp::Sub, lhs.into(), rhs.into())
            }
            Token::LeftParen => {
                let func = expr;
                self.consume();
                let args = self.parse_args(Token::RightParen)?;
                ExprKind::Call(func.into(), args)
            }
            _ => return Ok(expr),
        };

        Ok(self.make_spanned(start, node))
    }

    pub fn parse_prefix(&mut self) -> Result<Expr> {
        let start = self.curr_pos();

        let kind = match self.peek() {
            Token::Null => {
                self.consume();
                ExprKind::Lit(Lit::Null)
            }
            Token::False => {
                self.consume();
                ExprKind::Lit(Lit::Bool(false))
            }
            Token::True => {
                self.consume();
                ExprKind::Lit(Lit::Bool(true))
            }
            Token::Number(raw) => {
                self.consume();
                let lit = if raw.contains('.') {
                    Lit::Float(self.make_symbol(raw))
                } else {
                    Lit::Int(self.make_symbol(raw))
                };
                ExprKind::Lit(lit)
            }
            Token::Str(raw) => {
                self.consume();
                let unescaped = match unescape(&raw[1..raw.len() - 1]) {
                    Ok(str) => self.make_symbol(&str),
                    Err(err) => {
                        let span = Span::at(start + err.pos);
                        let diagnostic = Diagnostic::error(err.kind.to_string(), span);
                        self.report_err(diagnostic);
                        self.make_symbol("err")
                    }
                };
                ExprKind::Lit(Lit::Str(unescaped))
            }
            Token::Ident(raw) => ExprKind::Var(self.parse_ident()?),
            Token::LeftParen => {
                self.consume();
                let expr = self.parse_expr()?.into();
                self.expect(Token::RightParen)?;
                ExprKind::Grouped(expr)
            }
            Token::LeftBracket => {
                self.consume();
                let exprs = self.parse_args(Token::RightBracket)?;
                ExprKind::Array(exprs)
            }
            _ => Err(self.unexpected_curr())?,
        };

        Ok(self.make_spanned(start, kind))
    }

    fn parse_args(&mut self, terminator: Token) -> Result<Vec<Expr>> {
        let mut args = vec![];

        loop {
            if self.peek() == terminator {
                self.consume();
                break;
            }

            args.push(self.parse_expr()?);

            match self.consume().token {
                Token::Comma => {}
                t if t == terminator => break,
                _ => Err(self.unexpected_prev())?,
            }
        }

        Ok(args)
    }
}

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
        self.parse_precedence(Precedence::None)
    }

    fn parse_precedence(&mut self, min_prec: Precedence) -> Result<Expr> {
        let start = self.curr_pos();

        let mut expr = self.parse_prefix()?;

        loop {
            let prec = match self.peek() {
                Token::Plus => Precedence::Term,
                Token::Minus => Precedence::Term,
                Token::LeftParen => Precedence::Call,
                Token::LeftBracket => Precedence::Call,
                _ => break,
            };
            if prec < min_prec {
                break;
            }

            let node = match self.peek() {
                Token::Plus => {
                    let lhs = expr;
                    let op = self.consume();
                    let rhs = self.parse_prefix()?;
                    ExprKind::Binary(BinOp::Add, lhs.into(), rhs.into(), op.span)
                }
                Token::Minus => {
                    let lhs = expr;
                    let op = self.consume();
                    let rhs = self.parse_prefix()?;
                    ExprKind::Binary(BinOp::Sub, lhs.into(), rhs.into(), op.span)
                }
                Token::LeftParen => {
                    let func = expr;
                    let args = self.parse_list(Token::LeftParen, Token::RightParen)?;
                    ExprKind::Call(func.into(), args)
                }
                Token::LeftBracket => {
                    let array = expr;
                    self.consume();
                    let index = self.parse_expr()?;
                    self.expect(Token::RightBracket)?;
                    ExprKind::Index(array.into(), index.into())
                }
                _ => unreachable!(),
            };

            expr = self.make_spanned(start, node);
        }

        Ok(expr)
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
                let exprs = self.parse_list(Token::LeftBracket, Token::RightBracket)?;
                ExprKind::Array(exprs)
            }
            _ => Err(self.unexpected_curr())?,
        };

        Ok(self.make_spanned(start, kind))
    }

    fn parse_list(&mut self, start: Token<'a>, terminator: Token<'a>) -> Result<Vec<Expr>> {
        let mut args = vec![];

        self.expect(start)?;
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

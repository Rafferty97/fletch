use codespan_reporting::diagnostic;

use crate::ast::span::{Span, Spanned};
use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol, UnaryOp};
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
                Token::Asterisk => Precedence::Factor,
                Token::Solidus => Precedence::Factor,
                Token::EqEq | Token::BangEq => Precedence::Equality,
                Token::Lt | Token::LtEq | Token::Gt | Token::GtEq => Precedence::Comparison,
                Token::LeftParen => Precedence::Call,
                Token::LeftBracket => Precedence::Call,
                _ => break,
            };
            if prec < min_prec {
                break;
            }

            expr = match self.peek() {
                Token::Plus => self.parse_binary(expr, BinOp::Add, Precedence::Term)?,
                Token::Minus => self.parse_binary(expr, BinOp::Sub, Precedence::Term)?,
                Token::Asterisk => self.parse_binary(expr, BinOp::Mul, Precedence::Factor)?,
                Token::Solidus => self.parse_binary(expr, BinOp::Div, Precedence::Factor)?,
                Token::EqEq => self.parse_binary(expr, BinOp::Eq, Precedence::Equality)?,
                Token::BangEq => self.parse_binary(expr, BinOp::NotEq, Precedence::Equality)?,
                Token::Lt => self.parse_binary(expr, BinOp::Lt, Precedence::Comparison)?,
                Token::LtEq => self.parse_binary(expr, BinOp::LtEq, Precedence::Comparison)?,
                Token::Gt => self.parse_binary(expr, BinOp::Gt, Precedence::Comparison)?,
                Token::GtEq => self.parse_binary(expr, BinOp::GtEq, Precedence::Comparison)?,
                Token::LeftParen => {
                    let func = expr;
                    let arg_start = self.curr_pos();
                    let args = self.parse_list(Token::LeftParen, Token::RightParen)?;
                    let arg_span = self.make_span(arg_start);
                    let node = ExprKind::Call(func.into(), args, arg_span);
                    self.make_spanned(start, node)
                }
                Token::LeftBracket => {
                    let array = expr;
                    self.consume();
                    let index = self.parse_expr()?;
                    self.expect(Token::RightBracket)?;
                    let node = ExprKind::Index(array.into(), index.into());
                    self.make_spanned(start, node)
                }
                _ => unreachable!(),
            };
        }

        Ok(expr)
    }

    fn parse_prefix(&mut self) -> Result<Expr> {
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
            Token::Bang => {
                let span = self.consume().span;
                let expr = self.parse_precedence(Precedence::Unary)?.into();
                ExprKind::Unary(UnaryOp::Not, expr, span)
            }
            Token::Minus => {
                let span = self.consume().span;
                let expr = self.parse_precedence(Precedence::Unary)?.into();
                ExprKind::Unary(UnaryOp::Negate, expr, span)
            }
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
            Token::If => {
                self.consume();
                let cond = self.parse_expr()?.into();
                let then = self.parse_block_expr()?.into();
                let r#else = if self.check(|t| t == Token::Else) {
                    Some(self.parse_block_expr()?.into())
                } else {
                    None
                };
                ExprKind::If { cond, then, r#else }
            }
            Token::LeftBrace => return self.parse_block_expr(),
            _ => Err(self.unexpected_curr())?,
        };

        Ok(self.make_spanned(start, kind))
    }

    fn parse_binary(&mut self, lhs: Expr, op: BinOp, prec: Precedence) -> Result<Expr> {
        let op_token = self.consume();
        let rhs = self.parse_precedence(prec)?;
        let id = self.node_ids.next();
        let span = Span::cover(lhs.span, rhs.span);
        let node = ExprKind::Binary(op, lhs.into(), rhs.into(), op_token.span);
        Ok(Expr { id, node, span })
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

    fn parse_block_expr(&mut self) -> Result<Expr> {
        let start = self.curr_pos();
        let block = self.parse_block()?;
        let node = ExprKind::Block(block);
        Ok(self.make_spanned(start, node))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

use crate::ast::span::{Span, Spanned};
use crate::ast::{Block, Expr, ExprKind, Func, Ident, Lit, Mutability, Program, Stmt, StmtKind, Symbol, Ty};
use crate::diagnostics::Diagnostic;
use crate::parser::{SpannedToken, expr};

use super::Parser;
use super::error::Result;
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn parse_program(&mut self) -> Program {
        let mut funcs = vec![];

        while self.peek() != Token::Eof {
            match self.parse_top_level_item() {
                Ok(func) => funcs.push(func),
                _ => loop {
                    match self.peek() {
                        Token::Func => break,
                        Token::Eof => break,
                        _ => {}
                    }
                    self.consume();
                },
            }
        }

        Program { funcs }
    }

    fn parse_top_level_item(&mut self) -> Result<Func> {
        self.parse_func()
    }

    fn parse_func(&mut self) -> Result<Func> {
        self.expect(Token::Func)?;
        let name = self.parse_ident()?;
        let params = self.parse_params()?;
        let ret = if self.check(|t| t == Token::ThinArrow) {
            Some(self.parse_ty()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Func { name, params, ret, body })
    }

    fn parse_params(&mut self) -> Result<Vec<(Ident, Ty)>> {
        self.expect(Token::LeftParen)?;

        let mut params = vec![];
        loop {
            if self.peek() == Token::RightParen {
                self.consume();
                break;
            }

            let name = self.parse_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_ty()?;
            params.push((name, ty));

            match self.consume().token {
                Token::Comma => {}
                t if t == Token::RightParen => break,
                _ => Err(self.unexpected_prev())?,
            }
        }

        Ok(params)
    }

    pub(super) fn parse_block(&mut self) -> Result<Block> {
        self.expect(Token::LeftBrace)?;

        let mut stmts = vec![];
        let tail = loop {
            if matches!(self.peek(), Token::RightBrace | Token::Eof) {
                break None;
            }
            match self.parse_stmt() {
                Ok(StmtOrTail::Stmt(stmt)) => stmts.push(stmt),
                Ok(StmtOrTail::Tail(expr)) => break Some(expr.into()),
                Err(err) => self.sync_block(),
            }
        };
        self.expect(Token::RightBrace).ok();

        Ok(Block { stmts, tail })
    }

    fn sync_block(&mut self) {
        let mut level = 0;
        loop {
            if self.peek() == Token::RightBrace && level == 0 {
                return;
            }
            match self.consume().token {
                Token::LeftBrace => {
                    level += 1;
                }
                Token::RightBrace => {
                    level -= 1;
                }
                Token::Semi if level == 0 => return,
                Token::Eof => return,
                _ => {}
            }
        }
    }

    fn parse_stmt(&mut self) -> Result<StmtOrTail> {
        let start = self.curr_pos();
        let kind = match self.peek() {
            Token::Let | Token::Var => {
                let mutability = match self.consume().token {
                    Token::Let => Mutability::Not,
                    Token::Var => Mutability::Mut,
                    _ => unreachable!(),
                };
                let name = self.parse_ident()?;
                let ty = self
                    .consume_if(|t| t == Token::Colon)
                    .map(|_| self.parse_ty())
                    .transpose()?
                    .map(Into::into);
                self.expect(Token::Eq)?;
                let value = self.parse_expr()?.into();
                self.expect(Token::Semi)?;
                StmtKind::Let(name, ty, value, mutability)
            }
            _ => {
                let has_block = matches!(self.peek(), Token::If);
                let expr = self.parse_expr()?;
                match self.peek() {
                    Token::Eq => {
                        self.consume();
                        let lhs = self.convert_expr_to_place(expr)?;
                        let rhs = self.parse_expr()?.into();
                        self.expect(Token::Semi)?;
                        StmtKind::Assign(lhs, rhs)
                    }
                    Token::Semi => {
                        self.consume();
                        StmtKind::Expr(expr.into())
                    }
                    Token::RightBrace => {
                        return Ok(StmtOrTail::Tail(expr));
                    }
                    _ if has_block => StmtKind::Expr(expr.into()),
                    _ => Err(self.unexpected_prev())?,
                }
            }
        };
        Ok(StmtOrTail::Stmt(self.make_spanned(start, kind)))
    }

    pub(crate) fn parse_ident(&mut self) -> Result<Ident> {
        let token = self.consume();
        let Token::Ident(raw) = token.token else {
            let diagnostic = Diagnostic::error("expected an identifier", token.span);
            Err(self.report_err(diagnostic))?
        };
        let id = self.node_ids.next();
        let sym = self.make_symbol(raw);
        let span = token.span;
        Ok(Ident { id, sym, span })
    }

    fn convert_expr_to_place(&self, expr: Expr) -> Result<Ident> {
        match expr.node {
            ExprKind::Var(ident) => Ok(ident),
            _ => {
                let diagnostic = Diagnostic::error("this expression can not be assigned to", expr.span);
                Err(self.report_err(diagnostic))
            }
        }
    }
}

enum StmtOrTail {
    Stmt(Stmt),
    Tail(Expr),
}

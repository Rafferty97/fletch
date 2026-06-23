use crate::ast::span::{Span, Spanned};
use crate::ast::{Block, Expr, ExprKind, Func, Ident, Lit, Mutability, Program, Stmt, StmtKind, Symbol};
use crate::diagnostics::Diagnostic;
use crate::parser::SpannedToken;

use super::Parser;
use super::error::Result;
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn parse_program(&mut self) -> Program {
        // Expect a single main function
        let main = self.parse_func().unwrap_or_else(|_| {
            // FIXME
            let name = Ident { id: self.node_ids.next(), sym: self.make_symbol("main"), span: Span::dummy() };
            let body = Default::default();
            Func { name, body }
        });

        // Expect EOF
        self.expect(Token::Eof).ok();

        Program { main }
    }

    fn parse_func(&mut self) -> Result<Func> {
        self.expect(Token::Func)?;

        // Function name
        let name = self.parse_ident()?;

        // Parameter list
        self.expect(Token::LeftParen)?;
        self.expect(Token::RightParen)?;

        // Body
        let body = self.parse_block();

        Ok(Func { name, body })
    }

    fn parse_block(&mut self) -> Block {
        match self.expect(Token::LeftBrace) {
            Ok(_) => {}
            Err(_) => return Block::default(),
        }

        let mut stmts = vec![];
        'outer: while !self.check(|t| t == Token::RightBrace) {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => loop {
                    match self.consume().token {
                        Token::Semi => break,
                        Token::RightBrace => break 'outer,
                        _ => {}
                    }
                },
            }
        }

        Block { stmts, tail: None }
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        let start = self.curr_pos();
        let kind = match self.peek() {
            Token::Let | Token::Var => {
                let mutability = match self.consume().token {
                    Token::Let => Mutability::Not,
                    Token::Var => Mutability::Mut,
                    _ => unreachable!(),
                };
                let name = self.parse_ident()?;
                self.expect(Token::Eq)?;
                let value = self.parse_expr()?.into();
                self.expect(Token::Semi)?;
                StmtKind::Let(name, value, mutability)
            }
            _ => {
                let expr = self.parse_expr()?;
                match self.consume().token {
                    Token::Eq => {
                        let lhs = self.convert_expr_to_place(expr)?;
                        let rhs = self.parse_expr()?.into();
                        self.expect(Token::Semi)?;
                        StmtKind::Assign(lhs, rhs)
                    }
                    Token::Semi => StmtKind::Expr(expr.into()),
                    _ => Err(self.unexpected_prev())?,
                }
            }
        };
        Ok(self.make_spanned(start, kind))
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

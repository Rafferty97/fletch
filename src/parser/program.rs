use crate::ast::{Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol};
use crate::parser::SpannedToken;
use crate::span::{Span, Spanned};

use super::Parser;
use super::error::{ParseError, ParseErrorKind, Result};
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn parse_program(&mut self) -> Result<'a, Program> {
        // Expect a single main function
        let func = self.parse_func()?;

        // Expect EOF
        self.expect(Token::Eof)?;

        Ok(Program { main: func })
    }

    fn parse_func(&mut self) -> Result<'a, Func> {
        self.expect(Token::Func)?;

        // Function name
        let name = self.parse_ident()?;

        // Parameter list
        self.expect(Token::LeftParen)?;
        self.expect(Token::RightParen)?;

        // Body
        let body = self.parse_block()?;

        Ok(Func { name, body })
    }

    fn parse_block(&mut self) -> Result<'a, Block> {
        self.expect(Token::LeftBrace)?;

        let mut stmts = vec![];
        while !self.check(|t| t == Token::RightBrace)? {
            stmts.push(self.parse_stmt()?);
        }

        Ok(Block { stmts, tail: None })
    }

    fn parse_stmt(&mut self) -> Result<'a, Stmt> {
        let start = self.curr_pos();
        let kind = match self.peek() {
            Token::Let => {
                self.consume()?;
                let name = self.parse_ident()?;
                self.expect(Token::Eq)?;
                let value = self.parse_expr()?.into();
                self.expect(Token::Semi)?;
                StmtKind::Let(name, value)
            }
            _ => {
                let expr = self.parse_expr()?;
                match self.consume()?.token {
                    Token::Eq => {
                        let lhs = expr.into();
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

    pub(crate) fn parse_ident(&mut self) -> Result<'a, Ident> {
        let token = self.consume()?;
        let Token::Ident(raw) = token.token else {
            Err(self.error(ParseErrorKind::ExpectedToken { act: token.token, exp: Token::Ident("") }))?
        };
        Ok(Ident { sym: self.make_symbol(raw) })
    }
}

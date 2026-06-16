use crate::ast::{Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol};
use crate::parser::SpannedToken;
use crate::span::{Span, Spanned};

use super::Parser;
use super::error::{ParseError, ParseErrorKind, Result};
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub fn parse_program(&mut self) -> Result<'a, Program> {
        // Consume the initial dummy token
        self.consume()?;

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
        match self.peek() {
            Token::Print => {
                self.consume()?;
                self.expect(Token::LeftParen)?;
                let expr = self.parse_expr()?.into();
                self.expect(Token::RightParen)?;
                Ok(self.make_spanned(start, StmtKind::Print(expr)))
            }
            _ => Err(self.unexpected_token()),
        }
    }

    fn parse_expr(&mut self) -> Result<'a, Expr> {
        let start = self.curr_pos();
        match self.peek() {
            Token::Integer(raw) => {
                self.consume()?;
                let lit = Lit::Int(self.make_symbol(raw));
                Ok(self.make_spanned(start, ExprKind::Lit(lit)))
            }
            _ => Err(self.unexpected_token()),
        }
    }

    fn parse_ident(&mut self) -> Result<'a, Ident> {
        let token = self.consume()?;
        let Token::Ident(raw) = token.token else {
            Err(self.error(ParseErrorKind::ExpectedToken { act: token.token, exp: Token::Ident("") }))?
        };
        Ok(Ident { sym: self.make_symbol(raw) })
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
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;

    use crate::{interner::IndexedInterner, parser::ParseCtx};

    use super::*;

    #[test]
    fn parse_simple_arithmetic() {
        let src = r#"
            fn main() {
                print(4)
            }"#;

        let arena = &Bump::new();
        let sym_interner = &IndexedInterner::new();
        let ctx = ParseCtx { arena, sym_interner };
        let mut parser = Parser::new(ctx, src);

        parser.parse_program().unwrap();
    }
}

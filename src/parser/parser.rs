use super::error::Result;
use super::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    previous: Token<'a>,
    current: Token<'a>,
}

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> Result<()> {
        Ok(())
    }
}

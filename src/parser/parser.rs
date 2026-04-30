use super::error::{ParseError, Result};
use super::lexer::{Lexer, Token};
use crate::parser::lexer::TokenKind;
use line_index::{LineIndex, TextRange, TextSize};
use std::cell::OnceCell;
use std::sync::Arc;

pub struct Parser<'a> {
    src: &'a str,
    lexer: Lexer<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    line_index: OnceCell<Arc<LineIndex>>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            lexer: Lexer::new(src),
            previous: Token::default(),
            current: Token::default(),
            line_index: OnceCell::new(),
        }
    }

    pub fn parse_expr(&mut self) -> Result<()> {
        Err(self.error(
            "not implemented",
            TextRange::new(TextSize::new(0), TextSize::new(0)),
        ))
    }

    fn advance(&mut self) -> Result<(), ParseError> {
        self.previous = self.current;
        loop {
            self.current = self.lexer.next()?;
            if self.current.kind != TokenKind::Comment {
                break;
            }
        }
        Ok(())
    }

    fn error(&self, msg: impl Into<String>, span: TextRange) -> ParseError {
        // let line_index = self
        //     .line_index
        //     .get_or_init(|| Arc::new(line_index::LineIndex::new(self.src)))
        //     .clone();
        ParseError { message: msg.into(), span, line_index: None }
    }
}

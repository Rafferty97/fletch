use super::error::{ParseError, Result};
use super::lexer::{Lexer, Token};
use crate::ast::{BinOp, Expr, ExprKind, Ident, Program};
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
        let mut parser = Self {
            src,
            lexer: Lexer::new(src),
            previous: Token::default(),
            current: Token::default(),
            line_index: OnceCell::new(),
        };
        parser.advance().unwrap();
        parser
    }

    pub fn parse_program(&mut self) -> Result<Program> {
        Err(self.error(
            "not implemented",
            TextRange::new(TextSize::new(0), TextSize::new(0)),
        ))
    }

    pub fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_precedence(Precedence::Lowest)
    }

    fn parse_precedence(&mut self, prec: Precedence) -> Result<Expr> {
        self.advance()?;

        let mut expr = match self.previous.kind {
            TokenKind::Identifier => Expr {
                kind: ExprKind::Ident(Ident(self.previous.raw.into())),
                span: self.previous.span(),
            },
            _ => panic!("{:?}", self.previous.kind),
        };

        loop {
            let curr_prec = match self.current.kind {
                TokenKind::Plus => Precedence::Term,
                TokenKind::Minus => Precedence::Term,
                TokenKind::Asterisk => Precedence::Factor,
                TokenKind::Solidus => Precedence::Factor,
                _ => break Ok(expr),
            };
            if prec > curr_prec {
                break Ok(expr);
            }
            self.advance()?;
            expr = match self.previous.kind {
                TokenKind::Plus => self.parse_binary(expr, BinOp::Add, curr_prec)?,
                TokenKind::Minus => self.parse_binary(expr, BinOp::Sub, curr_prec)?,
                TokenKind::Asterisk => self.parse_binary(expr, BinOp::Mul, curr_prec)?,
                TokenKind::Solidus => self.parse_binary(expr, BinOp::Div, curr_prec)?,
                _ => panic!("{:?}", self.current.kind),
            };
        }
    }

    fn parse_binary(&mut self, lhs: Expr, op: BinOp, prec: Precedence) -> Result<Expr> {
        let rhs = self.parse_precedence(prec.succ())?;

        let op_span = self.previous.span();
        let span = TextRange::cover(lhs.span, rhs.span);
        let kind = ExprKind::Binary(op, lhs.into(), rhs.into(), op_span);
        Ok(Expr { kind, span })
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    fn matches(&mut self, kind: TokenKind) -> Result<bool, ParseError> {
        if self.current.kind == kind {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
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

    fn consume(&mut self, kind: TokenKind, message: &'static str) -> Result<()> {
        if self.current.kind == kind {
            self.advance()
        } else {
            Err(self.error_at_current(message))
        }
    }

    fn error(&self, msg: impl Into<String>, span: TextRange) -> ParseError {
        // let line_index = self
        //     .line_index
        //     .get_or_init(|| Arc::new(line_index::LineIndex::new(self.src)))
        //     .clone();
        ParseError { message: msg.into(), span, line_index: None }
    }

    fn error_at(&self, message: impl Into<String>, token: &Token) -> ParseError {
        self.error(message, token.span())
    }

    fn error_at_current(&self, message: &'static str) -> ParseError {
        self.error_at(message, &self.current)
    }

    fn error_at_previous(&self, message: &'static str) -> ParseError {
        self.error_at(message, &self.previous)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Debug)]
#[repr(u8)]
enum Precedence {
    Lowest,
    Coalesce,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
    Highest,
}

impl Precedence {
    fn succ(self) -> Self {
        match self {
            Self::Highest => panic!("no successor for highest precedence"),
            _ => unsafe {
                /// SAFETY: the `#[repr(u8)]` and check above ensure this is valid
                std::mem::transmute(self as u8 + 1)
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ast::{BinOp, Expr, ExprKind, Ident};

    #[test]
    fn simple_expr() {
        let mut parser = Parser::new("x + y");
        let ast = parser.parse_expr().unwrap();

        let (lhs, rhs) = match ast {
            Expr { kind: ExprKind::Binary(BinOp::Add, lhs, rhs, _), .. } => (*lhs, *rhs),
            _ => panic!("unexpected ast: {:?}", ast),
        };

        match lhs {
            Expr { kind: ExprKind::Ident(Ident(ident)), .. } => {
                assert_eq!(ident, "x");
            }
            _ => panic!("unexpected ast"),
        }

        match rhs {
            Expr { kind: ExprKind::Ident(Ident(ident)), .. } => {
                assert_eq!(ident, "y");
            }
            _ => panic!("unexpected ast"),
        }
    }
}

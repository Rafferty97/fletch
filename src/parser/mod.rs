use crate::arena::Ctx;
use crate::ast::{BinOpKind, Expr, ExprKind, Ident, Lit};
use crate::diagnostics::Diagnostic;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::span::{Span, Spanned};

type Result<T> = std::result::Result<T, Diagnostic>;

pub struct Parser<'cx, 'src> {
    ctx: Ctx<'cx>,
    lexer: Lexer<'src>,
    current: Token,
    previous: Token,
}

impl<'cx, 'src> Parser<'cx, 'src> {
    pub fn new(ctx: Ctx<'cx>, src: &'src str) -> Self {
        let lexer = Lexer::new(src);
        let (current, previous) = Default::default();
        let mut parser = Self { ctx, lexer, current, previous };
        parser.advance();
        parser
    }

    pub fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_precedence(Precedence::Lowest)
    }

    fn parse_precedence(&mut self, prec: Precedence) -> Result<Expr> {
        self.advance();

        let mut expr = match self.previous.kind {
            TokenKind::Ident => Expr {
                kind: ExprKind::Var(self.ident(self.previous)),
                span: self.previous.span,
            },
            TokenKind::Lit(_) => Expr {
                kind: ExprKind::Lit(self.literal(self.previous)),
                span: self.previous.span,
            },
            TokenKind::LeftParen => {
                let start = self.previous.span.start();
                let expr = self.parse_expr()?;
                self.consume(TokenKind::RightParen, "expected ')' after expression")?;
                let span = Span::new(start, self.previous.span.end());
                Expr { kind: ExprKind::Paren(expr.into()), span }
            }
            _ => todo!("{:?}", self.previous.kind),
        };

        loop {
            let curr_prec = match self.current.kind {
                TokenKind::Plus => Precedence::Term,
                TokenKind::Minus => Precedence::Term,
                TokenKind::Star => Precedence::Factor,
                TokenKind::Slash => Precedence::Factor,
                _ => break Ok(expr),
            };
            if prec > curr_prec {
                break Ok(expr);
            }

            self.advance();
            expr = match self.previous.kind {
                TokenKind::Plus => self.parse_binary(expr, BinOpKind::Add, curr_prec)?,
                TokenKind::Minus => self.parse_binary(expr, BinOpKind::Sub, curr_prec)?,
                TokenKind::Star => self.parse_binary(expr, BinOpKind::Mul, curr_prec)?,
                TokenKind::Slash => self.parse_binary(expr, BinOpKind::Div, curr_prec)?,
                _ => todo!("{:?}", self.current.kind),
            };
        }
    }

    fn parse_binary(&mut self, lhs: Expr, op: BinOpKind, prec: Precedence) -> Result<Expr> {
        let rhs = self.parse_precedence(prec.succ())?;

        let op = Spanned::new(op, self.previous.span);
        let span = Span::cover(lhs.span, rhs.span);
        let kind = ExprKind::Binary(op, lhs.into(), rhs.into());
        Ok(Expr { kind, span })
    }

    fn ident(&mut self, token: Token) -> Ident {
        debug_assert!(token.kind == TokenKind::Ident);
        let span = token.span;
        let str = self.lexer.get_raw(span);
        let sym = self.ctx.intern_str(str);
        Ident { sym, span }
    }

    fn literal(&mut self, token: Token) -> Lit {
        let TokenKind::Lit(kind) = token.kind else {
            unreachable!();
        };
        let span = token.span;
        let str = self.lexer.get_raw(span);
        let sym = self.ctx.intern_str(str);
        Lit { kind, sym }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if self.current.kind == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;
        self.current = loop {
            let token = self.lexer.next();
            match token.kind {
                TokenKind::Whitespace => continue,
                _ => break token,
            }
        };
    }

    fn consume(&mut self, kind: TokenKind, msg: impl Into<String>) -> Result<()> {
        if self.current.kind == kind {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at_current(msg))
        }
    }

    fn error_at_current(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(msg, self.current.span)
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
                // SAFETY: the `#[repr(u8)]` and check above ensure this is valid
                std::mem::transmute(self as u8 + 1)
            },
        }
    }
}

#[cfg(test)]
mod test {
    use bumpalo::Bump;

    use crate::diagnostics::Diagnostics;

    use super::*;

    #[test]
    fn lex_nested_arithmetic() {
        let src = "2 + (40 * (12/3) - 9)";

        let arena = Bump::new();
        let mut handler = Diagnostics::new();
        let ctx = Ctx::new(&arena, &mut handler);

        let mut parser = Parser::new(ctx, src);
        parser.advance();
        let expr = parser.parse_expr().unwrap();

        // assert!(handler.diagnostics.is_empty());
        assert!(matches!(expr.kind, ExprKind::Binary(..)));
    }
}

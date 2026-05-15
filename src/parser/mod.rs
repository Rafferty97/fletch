use crate::arena::Ctx;
use crate::ast::{
    BinOpKind, Block, Expr, ExprKind, Func, Ident, Item, ItemKind, LetStmt, Lit, NodeId, Stmt,
    StmtKind, Ty, TyKind,
};
use crate::diagnostics::Diagnostic;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::span::{Span, Spanned};

type Result<T> = std::result::Result<T, Diagnostic>;

pub struct Parser<'cx, 'src> {
    ctx: Ctx<'cx>,
    lexer: Lexer<'src>,
    current: Token,
    previous: Token,
    next_id: u32,
}

impl<'cx, 'src> Parser<'cx, 'src> {
    pub fn new(ctx: Ctx<'cx>, src: &'src str) -> Self {
        let lexer = Lexer::new(src);
        let (current, previous) = Default::default();
        let next_id = 0;
        let mut parser = Self { ctx, lexer, current, previous, next_id };
        parser.advance();
        parser
    }

    pub fn parse_toplevel_item(&mut self) -> Result<Item> {
        let result = self.parse_item()?;
        self.consume(TokenKind::Eof, "unexpected token")?;
        Ok(result)
    }

    pub fn parse_toplevel_stmt(&mut self) -> Result<Stmt> {
        let result = self.parse_stmt()?;
        self.consume(TokenKind::Eof, "unexpected token")?;
        Ok(result)
    }

    pub fn parse_toplevel_expr(&mut self) -> Result<Expr> {
        let result = self.parse_expr()?;
        self.consume(TokenKind::Eof, "unexpected token")?;
        Ok(result)
    }

    pub fn is_eof(&self) -> bool {
        self.current.kind == TokenKind::Eof
    }

    fn parse_item(&mut self) -> Result<Item> {
        let id = self.next_id();

        let kind = match self.current.kind {
            TokenKind::Func => {
                self.advance();
                ItemKind::Func(self.parse_func()?)
            }
            _ => ItemKind::Stmt(self.parse_stmt()?), // FIXME: temp
        };
        // _ => Err(self.error_at_current("unexpected token"))?,

        Ok(Item { id, kind })
    }

    fn parse_func(&mut self) -> Result<Func> {
        // Function name
        self.consume(TokenKind::Ident, "expected function name")?;
        let name = self.ident(self.previous);

        // Parameter list
        self.consume(TokenKind::LeftParen, "expected '('")?;
        let mut params = vec![];
        while !self.matches(TokenKind::RightParen) {
            self.consume(TokenKind::Ident, "expected parameter name")?;
            let name = self.ident(self.previous);

            self.consume(TokenKind::Colon, "expected ':'")?;
            let ty = self.parse_ty()?;

            params.push((name, ty));

            self.advance();
            match self.previous.kind {
                TokenKind::Comma => continue,
                TokenKind::RightParen => break,
                _ => Err(self.error_at_previous("expected ',' or ')'"))?,
            }
        }

        // Return type
        let ret = self.parse_ty()?;

        // Function body
        let body = self.parse_body()?;

        Ok(Func { name, params, ret, body })
    }

    fn parse_body(&mut self) -> Result<Block> {
        self.consume(TokenKind::LeftBracket, "expected '{'")?;

        let mut stmts = vec![];

        let tail = loop {
            if self.matches(TokenKind::RightBracket) {
                break None;
            }

            let stmt = self.parse_stmt()?;

            self.advance();
            match self.previous.kind {
                TokenKind::Semi => stmts.push(stmt),
                TokenKind::RightBracket => break Some(stmt),
                _ => Err(self.error_at_previous("expected ';' or '}'"))?,
            }
        };

        let tail = match tail {
            Some(Stmt { kind: StmtKind::Expr(expr), .. }) => Some(*expr),
            Some(_) => Err(self.error_at_previous("tail statement must be an expression"))?,
            None => None,
        };

        Ok(Block { stmts, tail })
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        let id = self.next_id();
        let start = self.current.span.start();

        let kind = match self.current.kind {
            TokenKind::Let => StmtKind::Let(self.parse_let()?),
            _ => StmtKind::Expr(self.parse_expr()?.into()),
        };

        let end = self.previous.span.end();
        let span = Span::new(start, end);

        Ok(Stmt { id, kind, span })
    }

    fn parse_let(&mut self) -> Result<LetStmt> {
        self.advance();

        self.consume(TokenKind::Ident, "expected an identifier")?;
        let name = self.ident(self.previous);

        let ty = if self.matches(TokenKind::Colon) {
            Some(self.parse_ty()?)
        } else {
            None
        };

        self.consume(TokenKind::Equals, "expected '=' after identifer")?;

        let expr = self.parse_expr()?.into();

        Ok(LetStmt { name, ty, expr })
    }

    fn parse_ty(&mut self) -> Result<Ty> {
        self.consume(TokenKind::Ident, "unexpected token")?;
        let ident = self.ident(self.previous);
        let span = ident.span;
        let mut ty = Ty { kind: TyKind::Var(ident), span };

        if self.matches(TokenKind::QuestionMark) {
            let span = Span::cover(ty.span, self.previous.span);
            ty = Ty { kind: TyKind::Nullable(ty.into()), span };
        }

        Ok(ty)
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_precedence(Precedence::Lowest)
    }

    fn parse_precedence(&mut self, prec: Precedence) -> Result<Expr> {
        self.advance();

        let mut expr = match self.previous.kind {
            TokenKind::Ident => Expr {
                id: self.next_id(),
                kind: ExprKind::Var(self.ident(self.previous)),
                span: self.previous.span,
            },
            TokenKind::Lit(_) => Expr {
                id: self.next_id(),
                kind: ExprKind::Lit(self.literal(self.previous)),
                span: self.previous.span,
            },
            TokenKind::LeftParen => {
                let start = self.previous.span.start();
                let expr = self.parse_expr()?;
                self.consume(TokenKind::RightParen, "expected ')' after expression")?;
                let span = Span::new(start, self.previous.span.end());
                Expr { id: self.next_id(), kind: ExprKind::Paren(expr.into()), span }
            }
            // _ => todo!("{:?}", self.previous.kind),
            _ => Err(self.error_at_previous("unexpected token"))?,
        };

        loop {
            let curr_prec = match self.current.kind {
                TokenKind::Plus => Precedence::Term,
                TokenKind::Minus => Precedence::Term,
                TokenKind::Star => Precedence::Factor,
                TokenKind::Slash => Precedence::Factor,
                TokenKind::LeftParen => Precedence::Call,
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
                TokenKind::LeftParen => self.parse_call(expr)?,
                _ => todo!("{:?}", self.current.kind),
            };
        }
    }

    fn parse_binary(&mut self, lhs: Expr, op: BinOpKind, prec: Precedence) -> Result<Expr> {
        let rhs = self.parse_precedence(prec.succ())?;

        let op = Spanned::new(op, self.previous.span);
        let span = Span::cover(lhs.span, rhs.span);
        let kind = ExprKind::Binary(op, lhs.into(), rhs.into());
        Ok(Expr { id: self.next_id(), kind, span })
    }

    fn parse_call(&mut self, callee: Expr) -> Result<Expr> {
        let mut args = vec![];
        while !self.matches(TokenKind::RightParen) {
            args.push(self.parse_expr()?);

            self.advance();
            match self.previous.kind {
                TokenKind::Comma => continue,
                TokenKind::RightParen => break,
                _ => Err(self.error_at_previous("expected ',' or ')'"))?,
            }
        }

        let span = Span::cover(callee.span, self.previous.span);
        let kind = ExprKind::Call(callee.into(), args);

        Ok(Expr { id: self.next_id(), kind, span })
    }

    fn raw(&self, token: Token) -> &str {
        self.lexer.get_raw(token.span)
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

    fn next_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
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

    fn error_at_previous(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(msg, self.previous.span)
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
        let expr = parser.parse_expr().unwrap();

        // assert!(handler.diagnostics.is_empty());
        assert!(matches!(expr.kind, ExprKind::Binary(..)));
    }
}

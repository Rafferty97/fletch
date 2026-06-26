use codespan_reporting::diagnostic;

use crate::ast::span::{Span, Spanned};
use crate::ast::{BinOp, Block, Expr, ExprKind, Func, Ident, Lit, Program, Stmt, StmtKind, Symbol, Ty, TyKind};
use crate::diagnostics::Diagnostic;
use crate::parser::SpannedToken;
use crate::parser::escape::unescape;
use crate::parser::lexer::Token::RightParen;

use super::Parser;
use super::error::Result;
use super::lexer::Token;

impl<'a, 'sym> Parser<'a, 'sym> {
    pub(super) fn parse_ty(&mut self) -> Result<Ty> {
        let id = self.node_ids.next();
        let mut ty = match self.peek() {
            Token::Ident(_) => {
                let ident = self.parse_ident()?;
                let span = ident.span;
                Ty { id, node: TyKind::Var(ident), span }
            }
            Token::LeftBracket => {
                let start = self.consume().span;
                let inner = self.parse_ty()?.into();
                let end = self.expect(Token::RightBracket)?.span;
                let span = Span::cover(start, end);
                Ty { id, node: TyKind::Array(inner), span }
            }
            Token::LeftParen => {
                let ty = self.parse_ty()?;
                self.expect(Token::RightParen)?;
                ty
            }
            _ => Err(self.unexpected_prev())?,
        };

        if let Some(token) = self.consume_if(|t| t == Token::Question) {
            let id = self.node_ids.next();
            let span = Span::cover(ty.span, token.span);
            let node = TyKind::Nullable(ty.into());
            ty = Ty { id, node, span };
        }

        Ok(ty)
    }
}

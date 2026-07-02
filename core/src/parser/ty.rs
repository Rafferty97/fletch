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
        let start = self.curr_pos();
        let mut ty = match self.peek() {
            Token::Ident(_) => {
                let ident = self.parse_ident()?;
                let span = ident.span;
                self.make_spanned(start, TyKind::Var(ident))
            }
            Token::LeftBracket => {
                self.consume();
                let inner = self.parse_ty()?.into();
                self.expect(Token::RightBracket)?.span;
                self.make_spanned(start, TyKind::Array(inner))
            }
            Token::LeftParen => {
                self.consume();

                let mut elements = vec![];
                let mut trailing_comma = false;
                while self.peek() != Token::RightParen {
                    elements.push(self.parse_ty()?);
                    trailing_comma = self.consume_if(|t| t == Token::Comma).is_some();
                }
                self.expect(Token::RightParen)?;

                match (elements.len(), trailing_comma) {
                    (1, false) => elements.pop().unwrap(),
                    _ => self.make_spanned(start, TyKind::Tuple(elements)),
                }
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

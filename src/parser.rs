use chumsky::extra::SimpleState;
use chumsky::input::{MapExtra, Stream, ValueInput};
use chumsky::pratt::*;
use chumsky::prelude::*;
use logos::Logos;

use crate::ast::{BinOp, Block, Expr, ExprKind, Node, NodeId, Stmt, StmtKind};
use crate::lexer::Token;

struct ParseState {
    next_id: u32,
}

impl ParseState {
    fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }
}

type Extra<'src> = extra::Full<Rich<'src, Token<'src>>, SimpleState<ParseState>, ()>;

fn make_node<'src, I, T>(kind: T, e: &mut MapExtra<'src, '_, I, Extra<'src>>) -> Node<T>
where
    I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
{
    Node {
        id: e.state().alloc_id(),
        kind,
        span: e.span().into(),
    }
}

fn expr_parser<'src, I>() -> impl Parser<'src, I, Expr, Extra<'src>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
{
    recursive(|expr| {
        let atom = select! {
            Token::Ident(raw) => ExprKind::Var(raw.into()),
            Token::IntLiteral(raw) => ExprKind::IntLiteral(raw.into()),
            Token::FloatLiteral(raw) => ExprKind::FloatLiteral(raw.into()),
        };

        let atom = atom
            .map_with(make_node)
            .or(expr.delimited_by(just(Token::OpenParen), just(Token::CloseParen)));

        atom.pratt((
            infix(left(2), just(Token::Asterisk), |l, _, r, e| {
                make_node(ExprKind::Binary(BinOp::Mul, Box::new(l), Box::new(r)), e)
            }),
            infix(left(2), just(Token::Solidus), |l, _, r, e| {
                make_node(ExprKind::Binary(BinOp::Div, Box::new(l), Box::new(r)), e)
            }),
            infix(left(1), just(Token::Plus), |l, _, r, e| {
                make_node(ExprKind::Binary(BinOp::Add, Box::new(l), Box::new(r)), e)
            }),
            infix(left(1), just(Token::Minus), |l, _, r, e| {
                make_node(ExprKind::Binary(BinOp::Sub, Box::new(l), Box::new(r)), e)
            }),
        ))
    })
}

fn stmt_parser<'src, I>(
    expr: impl Parser<'src, I, Expr, Extra<'src>>,
) -> impl Parser<'src, I, Stmt, Extra<'src>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
{
    let keyword = just(Token::Let);
    let name = select! { Token::Ident(s) => s }.labelled("variable name");
    // let annotation = just(Token::Colon).ignore_then(ty).or_not();
    let init = just(Token::Eq).ignore_then(expr);

    keyword
        .ignore_then(name)
        // .then(annotation)
        .then(init)
        .then_ignore(just(Token::Semi))
        .map(|(name, expr)| StmtKind::Let(name.into(), None, expr.into()))
        .map_with(make_node)
}

fn parser<'src, I>(
    stmt: impl Parser<'src, I, Stmt, Extra<'src>>,
) -> impl Parser<'src, I, Block, Extra<'src>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
{
    stmt.repeated().collect().map(|stmts| Block { stmts })
}

pub fn parse(src: &'_ str) -> Result<Block, Vec<Rich<'_, Token<'_>>>> {
    let token_iter = Token::lexer(src)
        .spanned()
        .map(|(tok, span)| match tok {
            Ok(tok) => (tok, span.into()),
            Err(()) => (Token::Error, span.into()),
        })
        .filter(|(tok, _)| !matches!(tok, Token::Newline));

    let token_stream = Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s)| (t, s));

    let mut state = SimpleState::from(ParseState { next_id: 0 });

    let expr = expr_parser();
    let stmt = stmt_parser(expr);
    let block = parser(stmt);

    block
        .parse_with_state(token_stream, &mut state)
        .into_result()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_expr() {
        let src = "let x = 2 + (4 * 8);";
        parse(src).unwrap();
    }
}

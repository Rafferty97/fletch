use crate::lexer::Token;
use chumsky::input::{Stream, ValueInput};
use chumsky::pratt::*;
use chumsky::prelude::*;
use logos::Logos;

#[derive(Clone, Debug)]
pub enum Expr {
    Var(Box<str>),
    IntLiteral(Box<str>),
    FloatLiteral(Box<str>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub fn parser<'src, I>() -> impl Parser<'src, I, Expr, extra::Err<Rich<'src, Token<'src>>>>
where
    I: ValueInput<'src, Token = Token<'src>, Span = SimpleSpan>,
{
    recursive(|expr| {
        let atom = select! {
            Token::Ident(raw) => Expr::Var(raw.into()),
            Token::IntLiteral(raw) => Expr::IntLiteral(raw.into()),
            Token::FloatLiteral(raw) => Expr::FloatLiteral(raw.into()),
        };

        let atom = atom.or(expr.delimited_by(just(Token::OpenParen), just(Token::CloseParen)));

        atom.pratt((
            infix(left(2), just(Token::Asterisk), |l, _, r, _| {
                Expr::Binary(BinOp::Mul, Box::new(l), Box::new(r))
            }),
            infix(left(2), just(Token::Solidus), |l, _, r, _| {
                Expr::Binary(BinOp::Div, Box::new(l), Box::new(r))
            }),
            infix(left(1), just(Token::Plus), |l, _, r, _| {
                Expr::Binary(BinOp::Add, Box::new(l), Box::new(r))
            }),
            infix(left(1), just(Token::Minus), |l, _, r, _| {
                Expr::Binary(BinOp::Sub, Box::new(l), Box::new(r))
            }),
        ))
    })
}

pub fn parse(src: &'_ str) -> Result<Expr, Vec<Rich<'_, Token<'_>>>> {
    let token_iter = Token::lexer(src).spanned().map(|(tok, span)| match tok {
        Ok(tok) => (tok, span.into()),
        Err(()) => (Token::Error, span.into()),
    });

    let token_stream = Stream::from_iter(token_iter).map((0..src.len()).into(), |(t, s)| (t, s));

    parser().parse(token_stream).into_result()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_expr() {
        let src = "2 + (4 * 8)";
        let result = parse(src).unwrap();
        println!("{result:?}");
    }
}

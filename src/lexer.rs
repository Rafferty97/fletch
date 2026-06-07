use logos::Logos;

#[derive(Clone, Copy, Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\f]+")]
pub enum Token<'a> {
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*", |lex| lex.slice())]
    Ident(&'a str),
    #[regex("[0-9]+", |lex| lex.slice())]
    IntLiteral(&'a str),
    #[regex("[0-9]+\\.[0-9]+", |lex| lex.slice())]
    FloatLiteral(&'a str),
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Solidus,
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[regex("//[^\n]*", allow_greedy = true)]
    Comment,
    #[token("let")]
    Let,
    #[token("=")]
    Eq,
    #[token(";")]
    Semi,
    #[regex("\n+")]
    Newline,
    #[logos(error = Token)]
    Error,
}

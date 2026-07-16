use logos::Logos;

use crate::diagnostics::ErrGuaranteed;

#[derive(Logos, Copy, Clone, PartialEq, Eq, Debug)]
#[logos(skip r"[ \t\r\f]+")]
pub enum Token<'a> {
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident(&'a str),
    #[regex("'[a-zA-Z_][a-zA-Z0-9_]*")]
    Tag(&'a str),
    #[regex("-?[0-9]+(\\.[0-9]*)?")]
    Number(&'a str),
    #[regex(r#""([^"\\]|\\.)*""#)]
    Str(&'a str),
    #[token("null")]
    Null,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("fn")]
    Func,
    #[token("let")]
    Let,
    #[token("var")]
    Var,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("_", priority = 3)]
    Underscore,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Solidus,
    #[token("!")]
    Bang,
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<")]
    Lt,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token(">=")]
    GtEq,
    #[token(",")]
    Comma,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token("?")]
    Question,
    #[token("->")]
    ThinArrow,
    #[token("=>")]
    FatArrow,
    #[token("\n")]
    Newline,
    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    Comment,
    Eof,
    #[regex(r".", |lex| lex.slice().chars().next().unwrap(), priority = 0)]
    Err(char),
}

impl<'a> Token<'a> {
    /// An arbitrary token that won't actually be consumed
    pub fn dummy() -> Self {
        Self::Semi
    }
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Ident(_) => "identifer",
            Self::Tag(_) => "tag",
            Self::Number(_) => "number",
            Self::Str(_) => "string",
            Self::Null => "'null'",
            Self::True => "'true'",
            Self::False => "'false'",
            Self::Func => "'fn'",
            Self::Let => "'let'",
            Self::Var => "'var'",
            Self::If => "'if'",
            Self::Else => "'else'",
            Self::Underscore => "'_'",
            Self::LeftParen => "'('",
            Self::RightParen => "')'",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::LeftBracket => "'['",
            Self::RightBracket => "']'",
            Self::Plus => "'+'",
            Self::Minus => "'-'",
            Self::Asterisk => "'*'",
            Self::Solidus => "'/'",
            Self::Bang => "'!'",
            Self::Eq => "'='",
            Self::EqEq => "'=='",
            Self::BangEq => "'!='",
            Self::Lt => "'<'",
            Self::LtEq => "'<='",
            Self::Gt => "'>'",
            Self::GtEq => "'>='",
            Self::Semi => "';'",
            Self::Comma => "','",
            Self::Colon => "':'",
            Self::Question => "'?'",
            Self::ThinArrow => "'->'",
            Self::FatArrow => "'=>'",
            Self::Newline => "new line",
            Self::Comment => "comment",
            Self::Eof => "end of input",
            Self::Err(c) => "'{c}'",
        };
        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lex_simple_arithmetic() {
        let src = r#"
            fn main() {
                print(2 + 2)
            }"#;

        let mut lexer = Token::lexer(src.trim());
        let mut next = || lexer.next().unwrap().unwrap();

        assert_eq!(next(), Token::Func);
        assert_eq!(next(), Token::Ident("main"));
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::LeftBrace);
        assert_eq!(next(), Token::Newline);
        assert_eq!(next(), Token::Ident("print"));
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::Number("2"));
        assert_eq!(next(), Token::Plus);
        assert_eq!(next(), Token::Number("2"));
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::Newline);
        assert_eq!(next(), Token::RightBrace);
        assert_eq!(lexer.next(), None);
    }
}

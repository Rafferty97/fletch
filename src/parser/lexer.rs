use logos::Logos;

#[derive(Logos, Copy, Clone, PartialEq, Eq, Debug)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token<'a> {
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident(&'a str),
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
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Solidus,
    #[token("=")]
    Eq,
    #[token(",")]
    Comma,
    #[token(";")]
    Semi,
    Eof,
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
            Self::Number(_) => "number",
            Self::Str(_) => "string",
            Self::Null => "'null'",
            Self::True => "'true'",
            Self::False => "'false'",
            Self::Func => "'fn'",
            Self::Let => "'let'",
            Self::LeftParen => "'('",
            Self::RightParen => "')'",
            Self::LeftBrace => "'{'",
            Self::RightBrace => "'}'",
            Self::Plus => "'+'",
            Self::Minus => "'-'",
            Self::Asterisk => "'*'",
            Self::Solidus => "'/'",
            Self::Eq => "'='",
            Self::Semi => "';'",
            Self::Comma => "','",
            Self::Eof => "end of input",
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

        let mut lexer = Token::lexer(src);
        let mut next = || lexer.next().unwrap().unwrap();

        assert_eq!(next(), Token::Func);
        assert_eq!(next(), Token::Ident("main"));
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::LeftBrace);
        assert_eq!(next(), Token::Ident("print"));
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::Number("2"));
        assert_eq!(next(), Token::Plus);
        assert_eq!(next(), Token::Number("2"));
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::RightBrace);
        assert_eq!(lexer.next(), None);
    }
}

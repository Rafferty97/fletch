use logos::Logos;

#[derive(Logos, Debug, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
enum Token<'a> {
    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident(&'a str),
    #[regex("-?[0-9]+")]
    Integer(&'a str),
    #[token("fn")]
    Fn,
    #[token("print")]
    Print, // FIXME: remove
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
    #[token(";")]
    Semi,
}

// fn main() {
//     print(3 + 8)
// }

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

        assert_eq!(next(), Token::Fn);
        assert_eq!(next(), Token::Ident("main"));
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::LeftBrace);
        assert_eq!(next(), Token::Print);
        assert_eq!(next(), Token::LeftParen);
        assert_eq!(next(), Token::Integer("2"));
        assert_eq!(next(), Token::Plus);
        assert_eq!(next(), Token::Integer("2"));
        assert_eq!(next(), Token::RightParen);
        assert_eq!(next(), Token::RightBrace);
        assert_eq!(lexer.next(), None);
    }
}

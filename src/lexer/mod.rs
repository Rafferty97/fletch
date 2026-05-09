use std::str::Chars;

use crate::span::{BytePos, Span};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TokenKind {
    Whitespace,
    Ident,
    Lit(LitKind),
    LeftParen,
    RightParen,
    Plus,
    Minus,
    Star,
    Slash,
    Unknown,
    #[default]
    Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LitKind {
    Bool,
    Integer,
    Float,
    Str,
}

pub struct Lexer<'a> {
    src: &'a str,
    chars: Chars<'a>,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { src, chars: src.chars() }
    }

    pub fn next(&mut self) -> Token {
        let start = self.pos();

        let Some(first) = self.advance() else {
            return Token::new(TokenKind::Eof, Span::default());
        };

        let kind = match first {
            c if c.is_whitespace() => {
                self.advance_while(char::is_whitespace);
                TokenKind::Whitespace
            }
            c if c.is_alphabetic() || c == '_' => {
                self.advance_while(|c| c.is_alphanumeric() || c == '_');
                match self.get_raw(self.mk_span(start)) {
                    "false" => TokenKind::Lit(LitKind::Bool),
                    "true" => TokenKind::Lit(LitKind::Bool),
                    _ => TokenKind::Ident,
                }
            }
            '0'..='9' => {
                self.advance_while(|c| c.is_ascii_digit());
                if self.peek() == Some('.') {
                    self.advance();
                    self.advance_while(|c| c.is_ascii_digit());
                    TokenKind::Lit(LitKind::Float)
                } else {
                    TokenKind::Lit(LitKind::Integer)
                }
            }
            '"' => loop {
                match self.advance() {
                    Some('"') => break TokenKind::Lit(LitKind::Str),
                    Some('\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => break TokenKind::Unknown,
                }
            },
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            _ => TokenKind::Unknown,
        };

        Token::new(kind, self.mk_span(start))
    }

    pub fn get_raw(&self, span: Span) -> &str {
        &self.src[span.start().into()..span.end().into()]
    }

    fn advance(&mut self) -> Option<char> {
        self.chars.next()
    }

    fn advance_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while self.peek().map_or(false, &mut predicate) {
            self.advance();
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    fn pos(&self) -> BytePos {
        let start = self.src.as_ptr() as usize;
        let current = self.chars.as_str().as_ptr() as usize;
        (current - start).try_into().unwrap()
    }

    fn mk_span(&self, start: BytePos) -> Span {
        Span::new(start, self.pos())
    }
}

impl Token {
    fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lex_nested_arithmetic() {
        let src = "2 + (40 * (12/3) - 9)";
        let mut lexer = Lexer::new(src);

        let mut pos = 0;
        let mut check = |kind: TokenKind, len: u32| {
            let span = Span::new(pos.into(), (pos + len).into());
            assert_eq!(lexer.next(), Token::new(kind, span));
            pos += len;
        };

        check(TokenKind::Lit(LitKind::Integer), 1);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::Plus, 1);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::LeftParen, 1);
        check(TokenKind::Lit(LitKind::Integer), 2);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::Star, 1);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::LeftParen, 1);
        check(TokenKind::Lit(LitKind::Integer), 2);
        check(TokenKind::Slash, 1);
        check(TokenKind::Lit(LitKind::Integer), 1);
        check(TokenKind::RightParen, 1);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::Minus, 1);
        check(TokenKind::Whitespace, 1);
        check(TokenKind::Lit(LitKind::Integer), 1);
        check(TokenKind::RightParen, 1);
    }
}

use super::span::{Span, TextSize};
use crate::escape::{UnescapeError, unescape};
use std::{hint::unreachable_unchecked, marker::PhantomData};
use thiserror::Error;

pub struct Lexer<'a> {
    /// A pointer to the start of the source text.
    start: *const u8,
    /// The current position in the source text.
    /// SAFETY: Must always point to a UTF-8 start byte, or be equal to `end`.
    current: *const u8,
    /// A pointer to the end of the source text.
    end: *const u8,
    _phantom: PhantomData<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    /// The token kind.
    pub kind: TokenKind,
    /// The raw source text.
    pub raw: &'a str,
    /// The start of the source text.
    src_start: *const u8,
}

impl<'a> Default for Token<'a> {
    fn default() -> Self {
        let kind = TokenKind::None;
        let raw = "";
        Self { kind, raw, src_start: raw.as_ptr() }
    }
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind, raw: &'a str) -> Self {
        let src_start = raw.as_ptr();
        Self { kind, raw, src_start }
    }

    pub fn ident(&self) -> Result<String, UnescapeError> {
        debug_assert!(self.kind == TokenKind::Identifier);

        if self.raw.as_bytes()[0] == b'\'' {
            unescape(&self.raw[1..self.raw.len() - 1])
        } else {
            Ok(self.raw.into())
        }
    }

    pub fn string(&self) -> Result<String, UnescapeError> {
        debug_assert!(self.kind == TokenKind::String);
        debug_assert!(self.raw.as_bytes()[0] == b'\"');

        unescape(&self.raw[1..self.raw.len() - 1])
    }

    pub fn span(&self) -> Span {
        make_span(self.raw, self.src_start)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum TokenKind {
    #[default]
    None,
    Comment,
    Eof,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Plus,
    Minus,
    Asterisk,
    Solidus,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    QuestionMark,
    DoubleQuestionMark,
    Arrow,
    Identifier,
    String,
    Number,
    Null,
    True,
    False,
    And,
    Or,
    Let,
    If,
    Else,
    Asc,
    Desc,
    Do,
    Func,
}

#[derive(Error, Debug)]
pub struct LexError {
    pub message: &'static str,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // FIXME: span
        write!(f, "Error at {:?}: {}", self.span, self.message)
    }
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            start: source.as_ptr(),
            current: source.as_ptr(),
            end: unsafe {
                // SAFETY: This gets the address of the first byte past the end of `source`,
                // which is guaranteed to be safe.
                source.as_ptr().byte_add(source.len())
            },
            _phantom: PhantomData,
        }
    }

    pub fn next(&mut self) -> Result<Token<'a>, LexError> {
        self.consume_whitespace();

        let start = self.current;
        let raw = |lexer: &Self| unsafe {
            // SAFETY: Reconstructs a `str` from raw pointers.
            // This is safe so long as `lexer.current` upholds its invariant of always pointing
            // to either the first byte of a UTF-8 byte sequence or to the end of the source text.
            // The offset is guaranteed to be non-negative (we only advance forward) and fit in
            // usize (it cannot exceed the original string length).
            let len = lexer
                .current
                .offset_from(start)
                .try_into()
                .unwrap_unchecked();
            let bytes = std::slice::from_raw_parts(start, len);
            std::str::from_utf8_unchecked(bytes)
        };
        let error =
            |lexer: &Self, message| Err(LexError { message, span: lexer.make_span(raw(lexer)) });

        let Some(char) = self.advance() else {
            let kind = TokenKind::Eof;
            return Ok(Token { kind, raw: raw(self), src_start: self.start });
        };

        let kind = match char {
            b'(' => TokenKind::LeftParen,
            b')' => TokenKind::RightParen,
            b'{' => TokenKind::LeftBrace,
            b'}' => TokenKind::RightBrace,
            b'[' => TokenKind::LeftBracket,
            b']' => TokenKind::RightBracket,
            b';' => TokenKind::Semicolon,
            b',' => TokenKind::Comma,
            b':' => TokenKind::Colon,
            b'.' => TokenKind::Dot,
            b'+' => TokenKind::Plus,
            b'-' => match self.matches(b'>') {
                true => TokenKind::Arrow,
                false => TokenKind::Minus,
            },
            b'*' => TokenKind::Asterisk,
            b'/' => match self.matches(b'/') {
                true => {
                    self.consume_line();
                    TokenKind::Comment
                }
                false => TokenKind::Solidus,
            },
            b'!' => match self.matches(b'=') {
                true => TokenKind::BangEqual,
                false => TokenKind::Bang,
            },
            b'=' => match self.matches(b'=') {
                true => TokenKind::EqualEqual,
                false => TokenKind::Equal,
            },
            b'<' => match self.matches(b'=') {
                true => TokenKind::LessEqual,
                false => TokenKind::Less,
            },
            b'>' => match self.matches(b'=') {
                true => TokenKind::GreaterEqual,
                false => TokenKind::Greater,
            },
            b'?' => match self.matches(b'?') {
                true => TokenKind::DoubleQuestionMark,
                false => TokenKind::QuestionMark,
            },
            b'\'' => loop {
                match self.advance() {
                    Some(b'\'') => break TokenKind::Identifier,
                    Some(b'\n') => return error(self, "Newline in identifier"),
                    Some(b'\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => return error(self, "Unterminated identifier"),
                }
            },
            b'"' => loop {
                match self.advance() {
                    Some(b'"') => break TokenKind::String,
                    Some(b'\n') => return error(self, "Newline in string"),
                    Some(b'\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => return error(self, "Unterminated string"),
                }
            },
            b'0'..=b'9' => {
                self.consume_while(|c| c.is_ascii_digit());
                if let (Some(b'.'), Some(b'0'..=b'9')) = (self.peek(), self.peek_next()) {
                    self.advance();
                    self.consume_while(|c| c.is_ascii_digit());
                }
                self.consume_while(|c| c.is_ascii_alphabetic());
                TokenKind::Number
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.consume_while(|c| c.is_ascii_alphanumeric() || c == b'_');
                match raw(self) {
                    "let" => TokenKind::Let,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "null" => TokenKind::Null,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "and" => TokenKind::And,
                    "or" => TokenKind::Or,
                    "asc" => TokenKind::Asc,
                    "desc" => TokenKind::Desc,
                    "do" => TokenKind::Do,
                    "fn" => TokenKind::Func,
                    _ => TokenKind::Identifier,
                }
            }
            _ => return error(self, "Unexpected character"),
        };

        Ok(Token { kind, raw: raw(self), src_start: self.start })
    }

    fn peek(&self) -> Option<u8> {
        unsafe {
            // SAFETY: The byte is only dereferenced after performing the bounds check.
            (self.current != self.end).then(|| *self.current)
        }
    }

    fn peek_next(&self) -> Option<u8> {
        unsafe {
            // SAFETY: The byte is only dereferenced after performing the bounds check.
            (self.end.offset_from(self.current) >= 2).then(|| *self.current.add(1))
        }
    }

    #[allow(unused)]
    fn peek_utf8(&self) -> Option<char> {
        self.as_str().chars().next()
    }

    #[allow(unused)]
    fn as_str(&self) -> &str {
        unsafe {
            // SAFETY: The invariants of `self.current` and `self.end` ensure this is safe.
            let len = self.end.offset_from(self.current) as usize;
            let slice = std::slice::from_raw_parts(self.current, len);
            std::str::from_utf8_unchecked(slice)
        }
    }

    fn advance(&mut self) -> Option<u8> {
        self.advance_if(|_| true)
    }

    fn advance_if(&mut self, pred: impl Fn(u8) -> bool) -> Option<u8> {
        match self.peek() {
            Some(byte) if pred(byte) => {
                unsafe {
                    // SAFETY: It is an invariant that `self.current` always points
                    // to a UTF-8 start byte, or the end of the input.
                    // It cannot be pointing to the end of the input because `self.peek`
                    // returned `Some`, and therefore calling `utf8_char_width` is safe.
                    self.current = self.current.add(utf8_char_width(byte));
                }
                Some(byte)
            }
            _ => None,
        }
    }

    fn matches(&mut self, expected: u8) -> bool {
        self.advance_if(|b| b == expected).is_some()
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(|c| matches!(c, b' ' | b'\r' | b'\n'));
    }

    fn consume_line(&mut self) {
        self.consume_while(|c| !matches!(c, b'\r' | b'\n'));
    }

    fn consume_while(&mut self, pred: impl Fn(u8) -> bool) {
        while self.advance_if(&pred).is_some() {}
    }

    fn make_span(&self, raw: &str) -> Span {
        make_span(raw, self.start)
    }
}

/// Computes the number of bytes in a UTF-8 character given the first byte.
///
/// SAFETY: `first_byte` must be a valid UTF-8 byte which is not a continuation byte.
const unsafe fn utf8_char_width(first_byte: u8) -> usize {
    match first_byte.leading_ones() {
        0 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        // SAFETY: We've exhausted all possibilities for a UTF-8 multibyte start byte.
        _ => unsafe { unreachable_unchecked() },
    }
}

fn make_span(raw: &str, src_start: *const u8) -> Span {
    let src_start = src_start as usize;
    let start = raw.as_ptr() as usize;
    let end = raw[raw.len()..].as_ptr() as usize;
    Span::new(
        TextSize::try_from(start - src_start).unwrap(),
        TextSize::try_from(end - src_start).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let mut lexer = Lexer::new("foo + 'my column'");
        assert!(matches!(
            lexer.next().unwrap(),
            Token { kind: TokenKind::Identifier, raw: "foo", .. }
        ));
        assert!(matches!(
            lexer.next().unwrap(),
            Token { kind: TokenKind::Plus, raw: "+", .. }
        ));
        assert!(matches!(
            lexer.next().unwrap(),
            Token { kind: TokenKind::Identifier, raw: "'my column'", .. }
        ));
    }

    #[test]
    fn unescape_string() {
        let mut lexer = Lexer::new(r#""that\'s a lie""#);
        let ident = lexer.next().unwrap();
        assert_eq!(ident.kind, TokenKind::String);
        assert_eq!(ident.raw, r#""that\'s a lie""#);
        assert_eq!(ident.string().unwrap(), "that's a lie");
    }

    #[test]
    fn unescape_identifier() {
        let mut lexer = Lexer::new(r#"'that\'s a lie'"#);
        let ident = lexer.next().unwrap();
        assert_eq!(ident.kind, TokenKind::Identifier);
        assert_eq!(ident.raw, r#"'that\'s a lie'"#);
        assert_eq!(ident.ident().unwrap(), "that's a lie");
    }

    #[test]
    fn lex_fn_definition() {
        let src = r#"
                fn unit_price(price: f32, qty: f32) -> f32 {
                    price / qty
                }
            "#;

        let mut lexer = Lexer::new(src);
        let mut next = || {
            let token = lexer.next().unwrap();
            (token.kind, token.raw)
        };
        assert_eq!(next(), (TokenKind::Func, "fn"));
        assert_eq!(next(), (TokenKind::Identifier, "unit_price"));
        assert_eq!(next(), (TokenKind::LeftParen, "("));
        assert_eq!(next(), (TokenKind::Identifier, "price"));
        assert_eq!(next(), (TokenKind::Colon, ":"));
        assert_eq!(next(), (TokenKind::Identifier, "f32"));
        assert_eq!(next(), (TokenKind::Comma, ","));
        assert_eq!(next(), (TokenKind::Identifier, "qty"));
        assert_eq!(next(), (TokenKind::Colon, ":"));
        assert_eq!(next(), (TokenKind::Identifier, "f32"));
        assert_eq!(next(), (TokenKind::RightParen, ")"));
        assert_eq!(next(), (TokenKind::Arrow, "->"));
        assert_eq!(next(), (TokenKind::Identifier, "f32"));
        assert_eq!(next(), (TokenKind::LeftBrace, "{"));
        assert_eq!(next(), (TokenKind::Identifier, "price"));
        assert_eq!(next(), (TokenKind::Solidus, "/"));
        assert_eq!(next(), (TokenKind::Identifier, "qty"));
        assert_eq!(next(), (TokenKind::RightBrace, "}"));
        assert_eq!(next(), (TokenKind::Eof, ""));
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Span {
    lo: u32,
    hi: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl Span {
    pub fn new(lo: u32, hi: u32) -> Self {
        Self { lo, hi }
    }

    pub fn dummy() -> Self {
        Self { lo: 0, hi: 0 }
    }

    pub fn lo(self) -> u32 {
        self.lo
    }

    pub fn hi(self) -> u32 {
        self.hi
    }
}

impl From<logos::Span> for Span {
    fn from(span: logos::Span) -> Self {
        Self {
            lo: span.start.try_into().expect("span too large"),
            hi: span.end.try_into().expect("span too large"),
        }
    }
}

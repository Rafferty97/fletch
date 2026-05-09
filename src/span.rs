pub type BytePos = line_index::TextSize;

pub type Span = line_index::TextRange;

#[derive(Clone, Debug)]
pub struct Spanned<T> {
    node: T,
    span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

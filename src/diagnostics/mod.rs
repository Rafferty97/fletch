use crate::span::Span;

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub labels: Vec<Label>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub primary: bool,
    pub message: Option<String>,
}

impl Diagnostic {
    pub fn error(msg: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            level: Level::Error,
            message: msg.into(),
            labels: vec![Label { span, primary: true, message: None }],
        }
    }
}

pub trait DiagnosticHandler {
    fn emit(&mut self, diag: Diagnostic);
}

pub struct DiagCtx<'a> {
    handler: &'a mut dyn DiagnosticHandler,
}

impl<'a> DiagCtx<'a> {
    pub fn new(handler: &'a mut dyn DiagnosticHandler) -> Self {
        Self { handler }
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        self.handler.emit(diag)
    }
}

#[derive(Default)]
pub struct Diagnostics {
    pub diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Default::default()
    }
}

impl DiagnosticHandler for Diagnostics {
    fn emit(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }
}

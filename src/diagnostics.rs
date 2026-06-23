use std::sync::{Arc, Mutex};

use crate::ast::span::Span;

pub trait DiagnosticReporter {
    fn report(&self, diagnostic: Diagnostic);

    fn report_err(&self, diagnostic: Diagnostic) -> ErrGuaranteed {
        assert!(diagnostic.is_error());
        self.report(diagnostic);
        ErrGuaranteed(())
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub primary: Label,
    pub secondary: Vec<Label>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(level: Level, message: impl Into<String>, span: Span) -> Self {
        let primary = Label { message: message.into(), span };
        Self { level, primary, secondary: vec![], notes: vec![] }
    }

    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self::new(Level::Error, message, span)
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self::new(Level::Error, message, span)
    }

    pub fn with_label(mut self, message: impl Into<String>, span: Span) -> Self {
        self.secondary.push(Label { message: message.into(), span });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        matches!(self.level, Level::Error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ErrGuaranteed(());

// useful for tests maybe
pub fn dummy_reporter() -> &'static impl DiagnosticReporter {
    struct Reporter;

    impl DiagnosticReporter for Reporter {
        fn report(&self, diagnostic: Diagnostic) {}
    }

    &Reporter
}

#[derive(Default, Clone, Debug)]
pub struct VecReporter(Arc<Mutex<Vec<Diagnostic>>>);

impl VecReporter {
    pub fn new() -> Self {
        Self(Arc::default())
    }

    pub fn num_errors(&self) -> usize {
        self.0.lock().unwrap().iter().filter(|d| d.is_error()).count()
    }

    pub fn into_errors(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.0.lock().unwrap())
    }

    pub fn assert_ok(&self) {
        let errors = self.into_errors();
        if !errors.is_empty() {
            panic!("unexpected errors: {errors:?}");
        }
    }
}

impl DiagnosticReporter for VecReporter {
    fn report(&self, diagnostic: Diagnostic) {
        self.0.lock().unwrap().push(diagnostic);
    }
}

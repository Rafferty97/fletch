use std::sync::{Arc, Mutex};

use crate::ast::span::Span;

pub trait DiagnosticReporter {
    fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed;
}

#[derive(Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub span: Span,
    pub secondary: Option<(String, Span)>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { level: Level::Error, message: message.into(), span, secondary: None }
    }

    pub fn with_secondary(self, message: impl Into<String>, span: Span) -> Self {
        let secondary = Some((message.into(), span));
        Self { secondary, ..self }
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
        fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed {
            ErrGuaranteed(())
        }
    }

    &Reporter
}

#[derive(Default, Clone, Debug)]
pub struct VecReporter(Arc<Mutex<Vec<Diagnostic>>>);

impl VecReporter {
    pub fn new() -> Self {
        Self(Arc::default())
    }

    pub fn has_errors(&self) -> bool {
        self.0.lock().unwrap().iter().any(|d| d.is_error())
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
    fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed {
        self.0.lock().unwrap().push(diagnostic);
        ErrGuaranteed(())
    }
}

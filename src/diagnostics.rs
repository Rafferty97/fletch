use std::sync::Mutex;

pub trait DiagnosticReporter {
    fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed;
}

#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
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

pub struct VecReporter(Mutex<Vec<Diagnostic>>);

impl VecReporter {
    pub fn new() -> Self {
        Self(Mutex::new(vec![]))
    }

    pub fn into_errors(self) -> Vec<Diagnostic> {
        self.0.into_inner().unwrap()
    }

    pub fn assert_ok(self) {
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

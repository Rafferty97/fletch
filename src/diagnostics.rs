pub trait DiagnosticReporter {
    fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed;
}

pub struct Diagnostic {
    // todo
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ErrGuaranteed(());

// useful for tests maybe
pub fn dummy_reporter() -> impl DiagnosticReporter {
    struct Reporter;

    impl DiagnosticReporter for Reporter {
        fn report(&self, diagnostic: Diagnostic) -> ErrGuaranteed {
            ErrGuaranteed(())
        }
    }

    Reporter
}

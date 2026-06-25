use codespan_reporting::diagnostic;

use crate::diagnostics::{Diagnostic, ErrGuaranteed};
use crate::parser::{Parser, SpannedToken};

pub type Result<T> = std::result::Result<T, ErrGuaranteed>;

impl<'a, 'sym> Parser<'a, 'sym> {
    /// Creates an unexpected token error from the provided token
    pub(super) fn unexpected_token(&self, token: SpannedToken) -> ErrGuaranteed {
        let diagnostic = Diagnostic::error(format!("unexpected {}", token.token), token.span);
        self.report_err(diagnostic)
    }

    /// Creates an unexpected token error from the current (unconsumed) token
    pub(super) fn unexpected_curr(&self) -> ErrGuaranteed {
        self.unexpected_token(self.current)
    }

    /// Creates an unexpected token error from the previously consumed token
    pub(super) fn unexpected_prev(&self) -> ErrGuaranteed {
        self.unexpected_token(self.previous)
    }

    /// Reports the diagnostic
    pub(super) fn report(&self, diagnostic: Diagnostic) {
        self.ctx.errors.report(diagnostic)
    }

    /// Reports the diagnostic, which must be an error, returning an `ErrGuaranteed`
    pub(super) fn report_err(&self, diagnostic: Diagnostic) -> ErrGuaranteed {
        self.ctx.errors.report_err(diagnostic)
    }
}

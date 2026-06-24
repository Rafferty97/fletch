#![cfg(test)]

use bumpalo::Bump;

use crate::diagnostics::{Diagnostic, Level, VecReporter};
use crate::interner::IndexedInterner;
use crate::name_resolution::NameResolution;
use crate::parser::{ParseCtx, Parser};
use crate::typecheck::TypeChecker;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::TyInterners;

#[test]
fn test_undefined_var() {
    let src = r#"
        fn main() {
            let x = y
        }"#;

    let errors = run_frontend(src);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].level, Level::Error);
    assert!(errors[0].primary.message.contains("cannot find"));
}

fn run_frontend(src: &str) -> Vec<Diagnostic> {
    // Create arena and interners
    let arena = Bump::new();
    let sym_interner = IndexedInterner::new();

    // Setup error reporting
    let errors = VecReporter::new();

    // Parse
    let ctx = ParseCtx::new(&arena, &sym_interner, &errors);
    let mut parser = Parser::new(ctx, src);
    let ast = parser.parse_program();
    let sym_table = &sym_interner.freeze();

    // Name resolution
    let mut name_resolution = NameResolution::new(sym_table, &errors);
    name_resolution.resolve_program(&ast);
    let name_tables = name_resolution.finish();

    // Typecheck
    let ty_interners = TyInterners::new(&arena);
    let ty_ctx = TyCtx::new(&arena, &ty_interners);
    let mut checker = TypeChecker::new(ty_ctx, &name_tables, sym_table, &errors);
    checker.check_func(&ast.main).unwrap();

    // Return diagnostics
    errors.into_errors()
}

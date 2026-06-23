use bumpalo::Bump;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use crate::ast::sexpr::{SExpr, SExprCtx};
use crate::ast::{ExprKind, Lit, StmtKind};
use crate::compile::compile_func;
use crate::diagnostics::{DiagnosticReporter, VecReporter};
use crate::interner::IndexedInterner;
use crate::name_resolution::{self, NameResolution};
use crate::parser::error::ParseError;
use crate::parser::{ParseCtx, Parser};
use crate::typecheck::TypeChecker;
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::{self, TyInterners};
use crate::vm::Vm;

#[derive(Default)]
pub struct FletchOpts {
    pub sexpr: bool,
    pub disassemble: bool,
}

pub fn run(filename: &str, src: &str, opts: FletchOpts) {
    // Create arena and interners
    let arena = Bump::new();
    let sym_interner = IndexedInterner::new();

    // Setup error reporting
    let mut files = SimpleFiles::new();
    let file_id = files.add(filename, src);
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();
    let errors = VecReporter::new();

    // Parse
    let ctx = ParseCtx::new(&arena, &sym_interner);
    let ast = match ctx.parse_program(src) {
        Ok(ast) => ast,
        Err(err) => {
            let diagnostic = Diagnostic::error().with_message(err.kind.to_string()).with_labels(vec![
                Label::primary(file_id, err.span).with_message(err.kind.to_string()),
            ]);
            term::emit_to_write_style(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
            return;
        }
    };
    let sym_table = &sym_interner.freeze();

    // Print s-expr
    if opts.sexpr {
        let mut output = String::new();
        let mut sexpr_ctx = SExprCtx { str: &mut output, sym_table };
        SExpr::write(&ast, &mut sexpr_ctx);
        println!("{output}\n");
    }

    // Name resolution
    let mut name_resolution = NameResolution::new(sym_table, &errors);
    name_resolution.resolve_program(&ast);
    let name_tables = name_resolution.finish();

    // Typecheck
    let ty_interners = TyInterners::new(&arena);
    let ty_ctx = TyCtx::new(&arena, &ty_interners);
    let mut checker = TypeChecker::new(ty_ctx, sym_table);
    match checker.check_func(&ast.main) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("type error: {}", err);
            return;
        }
    }

    // Report errors and bail if necessary
    let failed = errors.has_errors();
    let errors = errors.into_errors();

    let err_cnt = errors.len();
    for err in errors {
        let mut labels = vec![Label::primary(file_id, err.span).with_message(&err.message)];
        if let Some((msg, span)) = err.secondary {
            labels.push(Label::secondary(file_id, span).with_message(msg));
        }
        let diagnostic = Diagnostic::error().with_message(&err.message).with_labels(labels);
        term::emit_to_write_style(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
    }

    if failed {
        eprintln!("error: could not run `{}` due to {} errors", filename, err_cnt);
        return;
    }

    // Compile
    let chunk = match compile_func(&ast.main, sym_table) {
        Ok(func) => func,
        Err(err) => {
            eprintln!("compiler error: {err}");
            return;
        }
    };

    // Print chunk
    if opts.disassemble {
        println!("{}", chunk.disassemble());
    }

    // Execute
    let mut vm = Vm::new();
    vm.execute(&chunk);
}

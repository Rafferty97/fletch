use bumpalo::Bump;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

use crate::ast::sexpr::{SExpr, SExprCtx};
use crate::ast::{ExprKind, Lit, StmtKind};
use crate::interner::IndexedInterner;
use crate::parser::error::ParseError;
use crate::parser::{ParseCtx, Parser};

pub fn run(filename: &str, src: &str) {
    // Create arena and interners
    let arena = Bump::new();
    let sym_interner = IndexedInterner::new();

    // Setup error reporting
    let mut files = SimpleFiles::new();
    let file_id = files.add(filename, src);
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

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

    // Print s-expr
    let mut output = String::new();
    let mut sexpr_ctx = SExprCtx { str: &mut output, sym_interner: &sym_interner };
    SExpr::write(&ast, &mut sexpr_ctx);
    println!("{output}");
}

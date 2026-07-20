use bumpalo::Bump;
use codespan_reporting::diagnostic::Label;
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use cranelift::codegen::control::ControlPlane;
use cranelift::codegen::ir::Function;
use cranelift::codegen::settings::Configurable;
use cranelift::codegen::{Context, settings};
use serde::Serialize;
use target_lexicon::Triple;

use crate::ast::sexpr::to_sexpr;
use crate::ast::span::Span;
use crate::compile::{ProgramInput, compile_program};
use crate::diagnostics::{Diagnostic, Level, VecReporter};
use crate::interner::IndexedInterner;
use crate::name_resolution::NameResolution;
use crate::parser::{ParseCtx, Parser, SymTable};
use crate::typecheck::{Def, FuncDef, TypeChecker};
use crate::types::ty_ctx::TyCtx;
use crate::types::ty_interners::TyInterners;
use crate::vm::{OutputSink, Vm};

#[derive(Default)]
pub struct FletchOpts {
    pub sexpr: bool,
    pub disassemble: bool,
}

pub fn run(filename: &str, src: &str, opts: FletchOpts, output: &mut dyn OutputSink) {
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
    let ctx = ParseCtx::new(&arena, &sym_interner, &errors);
    let mut parser = Parser::new(ctx, src);
    let ast = parser.parse_program();
    let sym_table = &sym_interner.freeze();

    // Print s-expr
    if opts.sexpr {
        let mut buf = to_sexpr(&ast, sym_table);
        buf.push('\n');
        output.emit(&buf);
    }

    // Name resolution
    let mut name_resolution = NameResolution::new(sym_table, &errors);
    name_resolution.resolve_program(&ast);
    let name_tables = name_resolution.finish();

    // Typecheck
    let ty_interners = TyInterners::new(&arena);
    let ty_ctx = TyCtx::new(&arena, &ty_interners);
    let mut checker = TypeChecker::new(ty_ctx, &name_tables, sym_table, &errors);
    checker.check_program(&ast);
    let type_map = checker.finish();

    // Report errors and bail if necessary
    let num_errors = errors.num_errors();
    for err in errors.into_errors() {
        let primary = Label::primary(file_id, err.primary.span).with_message(&err.primary.message);
        let secondary = err
            .secondary
            .iter()
            .map(|l| Label::secondary(file_id, l.span).with_message(&l.message));
        let labels = std::iter::once(primary).chain(secondary).collect();
        let diagnostic = match err.level {
            Level::Error => codespan_reporting::diagnostic::Diagnostic::error(),
            Level::Warning => codespan_reporting::diagnostic::Diagnostic::warning(),
        };
        let diagnostic = diagnostic.with_message(&err.primary.message).with_labels(labels);
        term::emit_to_write_style(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
    }

    if num_errors > 0 {
        let s = if num_errors == 1 { "" } else { "s" };
        output.emit_err(&format!(
            "error: could not run `{}` due to {} error{}",
            filename, num_errors, s
        ));
        return;
    }

    // Compile
    let input = ProgramInput { ast: &ast, sym_table, uses: &name_tables.uses, type_map: &type_map };
    let module = compile_program(input);

    // Print chunks
    if opts.disassemble {
        todo!();
    }

    // Execute
    run_function(&module.funcs[module.main]);
}

fn run_function(func: &Function) {
    let mut builder = settings::builder();
    builder.set("opt_level", "speed");
    let flags = settings::Flags::new(builder);

    let isa = match cranelift::codegen::isa::lookup(Triple::host()) {
        Ok(builder) => builder.finish(flags).unwrap(),
        Err(err) => panic!("Error looking up target: {}", err),
    };

    let mut ctx = Context::for_function(func.clone());
    ctx.set_disasm(true);
    let code = ctx.compile(&*isa, &mut ControlPlane::default()).unwrap();
    println!("{}", code.vcode.as_ref().unwrap());
    let code = code.code_buffer();

    let mut buffer = memmap2::MmapOptions::new().len(code.len()).map_anon().unwrap();
    buffer.copy_from_slice(code);
    let buffer = buffer.make_exec().unwrap();

    let x = unsafe {
        let code_fn: unsafe extern "C" fn() -> i32 = std::mem::transmute(buffer.as_ptr());
        code_fn()
    };

    println!("out: {}", x);
}

#[derive(Serialize, Debug)]
pub struct CheckResult {
    diagnostics: Vec<Diagnostic>,
    types: Vec<(Span, String)>,
}

pub fn check(src: &str) -> CheckResult {
    // Create arena and interners
    let arena = Bump::new();
    let sym_interner = IndexedInterner::new();

    // Setup error reporting
    let mut files = SimpleFiles::new();
    files.add("<anon>", src);
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
    checker.check_program(&ast);
    let types = name_tables
        .idents
        .iter()
        .flat_map(|ident| {
            let def_id = name_tables.uses.get(&ident.id)?.ok()?;
            let ty = checker.def_map().get(&def_id)?;
            let name = sym_table.get_str(ident.sym);
            match ty {
                Def::Var(ty) => Some((ident.span, format!("let {} = {}", name, ty.display_ctx(&[], sym_table)))),
                Def::Func(func) => Some((ident.span, format_func(func, sym_table))),
            }
        })
        .collect();
    checker.finish();

    // Return info
    CheckResult { diagnostics: errors.into_errors(), types }
}

fn format_func(func: &FuncDef, sym_table: &SymTable<'_>) -> String {
    use std::fmt::Write;

    let mut buf = String::from("fn ");
    buf.push_str(sym_table.get_str(func.name));

    match &*func.ty_params {
        [] => {}
        [first, rest @ ..] => {
            buf.push('<');
            buf.push_str(sym_table.get_str(first.sym));
            for param in rest {
                buf.push_str(", ");
                buf.push_str(sym_table.get_str(param.sym));
            }
            buf.push('>');
        }
    }

    match &*func.params {
        [] => buf.push_str("()"),
        [first, rest @ ..] => {
            buf.push('(');
            write!(&mut buf, "{}", first.display_ctx(&func.ty_params, sym_table)).unwrap();
            for param in rest {
                buf.push_str(", ");
                write!(&mut buf, "{}", param.display_ctx(&func.ty_params, sym_table)).unwrap();
            }
            buf.push(')');
        }
    }

    if !func.ret.is_unit() {
        buf.push_str(" -> ");
        write!(&mut buf, "{}", func.ret.display_ctx(&func.ty_params, sym_table)).unwrap();
    }

    buf
}

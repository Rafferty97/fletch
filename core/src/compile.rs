use bumpalo::Bump;
use cranelift::codegen::ir::types::I32;
use cranelift::codegen::ir::{AbiParam, Function, InstBuilder, Signature, Type, UserFuncName};
use cranelift::codegen::isa::CallConv;
use cranelift::codegen::{settings, verify_function};
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext};
use fnv::FnvHashMap;
use itertools::Itertools;
use target_lexicon::Triple;
use thiserror::Error;

use crate::ast::{self, NodeId};
use crate::diagnostics::ErrGuaranteed;
use crate::name_resolution::DefId;
use crate::parser::SymTable;
use crate::types::Ty;

pub struct ProgramInput<'a> {
    pub ast: &'a ast::Program,
    pub sym_table: &'a SymTable<'a>,
    pub uses: &'a FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
    pub type_map: &'a FnvHashMap<NodeId, Ty<'a>>,
}

pub struct Module {
    pub main: usize,
    pub funcs: Vec<Function>,
}

pub fn compile_program(program: ProgramInput<'_>) -> Module {
    let mut ctx = FunctionBuilderContext::new();
    let mut funcs = vec![];

    for func in &program.ast.funcs {
        let func = compile_function(func, &mut ctx);
        funcs.push(func);
    }

    let main = program
        .ast
        .funcs
        .iter()
        .position(|f| program.sym_table.get_str(f.name.sym) == "main")
        .expect("no main function");

    Module { main, funcs }
}

fn compile_function(ast: &ast::Func, ctx: &mut FunctionBuilderContext) -> Function {
    let mut sig = Signature::new(CallConv::triple_default(&Triple::host()));
    sig.returns.push(AbiParam::new(I32));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);
    let mut b = FunctionBuilder::new(&mut func, ctx);

    let block = b.create_block();
    b.seal_block(block);

    b.switch_to_block(block);
    let a = b.ins().iconst(I32, 21);
    let ret = b.ins().imul_imm(a, 2);
    b.ins().return_(&[ret]);

    b.finalize();
    let flags = settings::Flags::new(settings::builder());
    verify_function(&func, &flags).unwrap();

    // println!("{}", func.display());
    func
}

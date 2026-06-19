use thiserror::Error;

use crate::ast::Func;
use crate::vm::chunk::{Chunk, ChunkBuilder};
use crate::vm::instr::{Instr, Reg};
use crate::vm::value::Value;

pub fn compile_func(ast: &Func) -> Result<Chunk> {
    let mut builder = ChunkBuilder::new();
    let imm = builder.constant(Value::Null);
    builder.ins(Instr::Const(Reg(0), imm));
    builder.ins(Instr::Print(Reg(0)));
    builder.ins(Instr::Return);
    Ok(builder.build())
}

#[derive(Error, Debug)]
#[error("compiler error")]
pub struct CompilerError {
    //
}

pub type Result<T, E = CompilerError> = std::result::Result<T, E>;

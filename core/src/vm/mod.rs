use crate::ast::Symbol;
use crate::interner::IndexTable;
use crate::parser::SymTable;
use crate::types::ty::FloatTy;
use crate::vm::instr::{EncodedInstr, Reg, Width};
use crate::vm::module::{FuncId, Module};
use crate::vm::value::{Int, Value};

use self::chunk::Chunk;
use self::instr::Instr;

pub mod chunk;
pub mod instr;
pub mod module;
pub mod value;

pub struct Vm<'a> {
    module: &'a Module,
    sym_table: &'a SymTable<'a>,
    frames: Vec<CallFrame<'a>>,
    stack: Vec<Value>,
    current: CallFrame<'a>,
    ret: Value,
}

pub trait OutputSink {
    fn emit(&mut self, text: &str);
    fn emit_err(&mut self, text: &str);
}

struct CallFrame<'a> {
    code: &'a [EncodedInstr],
    constants: &'a [Value],
    base_idx: usize,
    pc: usize,
    rd: Reg,
}

impl<'a> Vm<'a> {
    pub fn new(module: &'a Module, sym_table: &'a SymTable<'a>) -> Self {
        let current = CallFrame { code: &[], constants: &[], base_idx: 0, pc: 0, rd: Reg(0) };
        Self {
            module,
            sym_table,
            stack: vec![],
            frames: vec![],
            current,
            ret: Value::new_null(),
        }
    }

    pub fn execute(&mut self, output: &mut dyn OutputSink) {
        self.push_frame(self.module.main, 0, Reg(0));

        loop {
            // println!(
            //     "{}: {}",
            //     self.current.pc,
            //     Instr::decode(self.current.code[self.current.pc])
            // );
            match Instr::decode(self.current.code[self.current.pc]) {
                Instr::Return { r0 } => {
                    let value = self.read(r0).clone();
                    self.pop_frame(value);
                    if self.frames.is_empty() {
                        return;
                    }
                }
                Instr::LoadUnit { rd } => self.write(rd, Value::new_unit()),
                Instr::LoadNull { rd } => self.write(rd, Value::new_null()),
                Instr::LoadFalse { rd } => self.write(rd, Value::new_bool(false)),
                Instr::LoadTrue { rd } => self.write(rd, Value::new_bool(true)),
                Instr::LoadIntZero { rd } => self.write(rd, Value::new_int(Int::ZERO)),
                Instr::LoadF32Zero { rd } => self.write(rd, Value::new_f32(0.0)),
                Instr::LoadF64Zero { rd } => self.write(rd, Value::new_f64(0.0)),
                Instr::Load { rd, imm } => {
                    let value = self.current.constants[imm.0 as usize].clone();
                    self.write(rd, value);
                }
                Instr::Print { r0 } => {
                    let value = self.read(r0);
                    output.emit(&format!("{}\n", value.display_ctx(self.sym_table)));
                }
                Instr::Add { r0, r1, rd } => {
                    let lhs = self.read(r0).as_int();
                    let rhs = self.read(r1).as_int();
                    self.write(rd, Value::new_int(lhs + rhs));
                }
                Instr::Sub { r0, r1, rd } => {
                    let lhs = self.read(r0).as_int();
                    let rhs = self.read(r1).as_int();
                    self.write(rd, Value::new_int(lhs - rhs));
                }
                Instr::Mul { r0, r1, rd } => {
                    let lhs = self.read(r0).as_int();
                    let rhs = self.read(r1).as_int();
                    self.write(rd, Value::new_int(lhs * rhs));
                }
                Instr::Div { r0, r1, rd } => {
                    let lhs = self.read(r0).as_int();
                    let rhs = self.read(r1).as_int();
                    self.write(rd, Value::new_int(lhs / rhs));
                }
                Instr::Eq { r0, r1, rd } => {
                    let lhs = self.read(r0);
                    let rhs = self.read(r1);
                    self.write(rd, Value::new_bool(lhs == rhs));
                }
                Instr::Lt { r0, r1, rd } => {
                    let lhs = self.read(r0).as_int();
                    let rhs = self.read(r1).as_int();
                    self.write(rd, Value::new_bool(lhs < rhs));
                }
                Instr::Not { r0, rd } => {
                    let operand = self.read(r0).as_bool();
                    self.write(rd, Value::new_bool(!operand));
                }
                Instr::Neg { r0, rd } => {
                    let operand = self.read(r0).as_int();
                    self.write(rd, Value::new_int(-operand));
                }
                Instr::FNeg { w: FloatTy::Float32, r0, rd } => {
                    let operand = self.read(r0).as_f32();
                    self.write(rd, Value::new_f32(-operand));
                }
                Instr::FNeg { w: FloatTy::Float64, r0, rd } => {
                    let operand = self.read(r0).as_f64();
                    self.write(rd, Value::new_f64(-operand));
                }
                Instr::Move { r0, rd } => {
                    self.write(rd, self.read(r0).clone());
                }
                Instr::MakeArray { r0, rn, rd } => {
                    let elements = self.read_many(r0, rn);
                    self.write(rd, Value::new_array(elements.iter().cloned()));
                }
                Instr::Index { r0, r1, rd } => {
                    let expr = self.read(r0).as_array();
                    let index: usize = self.read(r1).as_int().try_into().unwrap(); // FIXME: unwrap
                    // FIXME: should throw error on out-of-bounds
                    self.write(rd, expr.get(index).cloned().unwrap_or(Value::new_null()));
                }
                Instr::MakeTuple { r0, rn, rd } => {
                    let elements = self.read_many(r0, rn);
                    self.write(rd, Value::new_tuple(elements.iter().cloned()));
                }
                Instr::Jump { addr } => {
                    self.current.pc = addr.0 as usize;
                    continue;
                }
                Instr::JumpIfTrue { r0, addr } => {
                    if self.read(r0).as_bool() {
                        self.current.pc = addr.0 as usize;
                        continue;
                    }
                }
                Instr::JumpIfFalse { r0, addr } => {
                    if !self.read(r0).as_bool() {
                        self.current.pc = addr.0 as usize;
                        continue;
                    }
                }
                Instr::FAdd { w: FloatTy::Float64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs + rhs));
                }
                Instr::FSub { w: FloatTy::Float64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs - rhs));
                }
                Instr::FMul { w: FloatTy::Float64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs * rhs));
                }
                Instr::FDiv { w: FloatTy::Float64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs / rhs));
                }
                Instr::FLt { w: FloatTy::Float64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs / rhs));
                }
                Instr::FAdd { .. }
                | Instr::FSub { .. }
                | Instr::FMul { .. }
                | Instr::FDiv { .. }
                | Instr::FLt { .. } => unreachable!(),
                Instr::Call { func, rd } => {
                    let func_id = self.read(func).as_func();
                    self.push_frame(func_id, func.0 as usize + 1, rd);
                    continue;
                }
            }
            self.current.pc += 1;
        }
    }

    fn push_frame(&mut self, func_id: FuncId, offset: usize, rd: Reg) {
        let func = &self.module.funcs[&func_id];

        let code = func.chunk.code();
        let constants = func.chunk.constants();
        let base_idx = self.current.base_idx + offset;
        let pc = 0;
        let frame = CallFrame { code, constants, base_idx, pc, rd };

        self.frames.push(std::mem::replace(&mut self.current, frame));
        let new_size = base_idx + func.chunk.stack_size();
        self.stack.resize(new_size.max(self.stack.len()), Value::new_null());
    }

    fn pop_frame(&mut self, ret: Value) {
        let rd = self.current.rd;
        self.current = self.frames.pop().expect("call stack underflow");
        self.write(rd, ret);
    }

    fn read(&self, reg: Reg) -> &Value {
        let base = self.current.base_idx;
        &self.stack[base + reg.0 as usize]
    }

    fn read_many(&self, start: Reg, end: Reg) -> &[Value] {
        let base = self.current.base_idx;
        &self.stack[(base + start.0 as usize)..(base + end.0 as usize)]
    }

    fn write(&mut self, reg: Reg, value: Value) {
        let base = self.current.base_idx;
        self.stack[base + reg.0 as usize] = value;
    }
}

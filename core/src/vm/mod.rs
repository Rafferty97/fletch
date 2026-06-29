use crate::vm::instr::{EncodedInstr, Reg, Width};
use crate::vm::module::{FuncId, Module};
use crate::vm::value::Value;

use self::chunk::Chunk;
use self::instr::Instr;

pub mod chunk;
pub mod instr;
pub mod module;
pub mod value;

pub struct Vm<'a> {
    module: &'a Module,
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
    pub fn new(module: &'a Module) -> Self {
        let current = CallFrame { code: &[], constants: &[], base_idx: 0, pc: 0, rd: Reg(0) };
        Self { module, stack: vec![], frames: vec![], current, ret: Value::new_null() }
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
                Instr::Load { rd, imm } => {
                    let value = self.current.constants[imm.0 as usize].clone();
                    self.write(rd, value);
                }
                Instr::Print { r0 } => {
                    let value = self.read(r0);
                    output.emit(&format!("{value}\n"));
                }
                Instr::Add { w, r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs + rhs, w));
                }
                Instr::Sub { w, r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs - rhs, w));
                }
                Instr::Mul { w, r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs * rhs, w));
                }
                Instr::UDiv { w, r0, r1, rd } => {
                    let lhs = self.read(r0).as_uint();
                    let rhs = self.read(r1).as_uint();
                    self.write(rd, Value::new_uint(lhs / rhs, w));
                }
                Instr::SDiv { w, r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs / rhs, w));
                }
                Instr::Eq { r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_bool(lhs == rhs));
                }
                Instr::ULt { r0, r1, rd } => {
                    let lhs = self.read(r0).as_uint();
                    let rhs = self.read(r1).as_uint();
                    self.write(rd, Value::new_bool(lhs < rhs));
                }
                Instr::SLt { r0, r1, rd } => {
                    let lhs = self.read(r0).as_sint();
                    let rhs = self.read(r1).as_sint();
                    self.write(rd, Value::new_bool(lhs < rhs));
                }
                Instr::Not { r0, rd } => {
                    let operand = self.read(r0).as_bool();
                    self.write(rd, Value::new_bool(!operand));
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
                    let index = self.read(r1).as_uint();
                    // FIXME: should throw error on out-of-bounds
                    self.write(rd, expr.get(index as usize).cloned().unwrap_or(Value::new_null()));
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
                Instr::FAdd { w: Width::_64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs + rhs));
                }
                Instr::FSub { w: Width::_64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs - rhs));
                }
                Instr::FMul { w: Width::_64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs * rhs));
                }
                Instr::FDiv { w: Width::_64, r0, r1, rd } => {
                    let lhs = self.read(r0).as_f64();
                    let rhs = self.read(r1).as_f64();
                    self.write(rd, Value::new_f64(lhs / rhs));
                }
                Instr::FLt { w: Width::_64, r0, r1, rd } => {
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
        self.stack.resize(base_idx + func.chunk.stack_size(), Value::new_null());
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

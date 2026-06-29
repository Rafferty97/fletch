use crate::vm::instr::{Reg, Width};
use crate::vm::value::Value;

use self::chunk::Chunk;
use self::instr::Instr;

pub mod chunk;
pub mod instr;
pub mod module;
pub mod value;

pub struct Vm {
    registers: Vec<Value>,
}

pub trait OutputSink {
    fn emit(&mut self, text: &str);
    fn emit_err(&mut self, text: &str);
}

impl Vm {
    pub fn new() -> Self {
        Self { registers: vec![] }
    }

    pub fn execute(&mut self, chunk: &Chunk, output: &mut dyn OutputSink) {
        self.registers.resize(chunk.stack_size(), Value::new_null());

        let code = chunk.code();
        let mut pc = 0;

        loop {
            // println!("{}: {}", pc, Instr::decode(code[pc]));
            match Instr::decode(code[pc]) {
                Instr::Return { .. } => return, // FIXME
                Instr::Load { rd, imm } => {
                    self.write(rd, chunk.get_const(imm).clone());
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
                    pc = addr.0 as usize;
                    continue;
                }
                Instr::JumpIfTrue { r0, addr } => {
                    if self.read(r0).as_bool() {
                        pc = addr.0 as usize;
                        continue;
                    }
                }
                Instr::JumpIfFalse { r0, addr } => {
                    if !self.read(r0).as_bool() {
                        pc = addr.0 as usize;
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
            }
            pc += 1;
        }
    }

    fn read(&self, reg: Reg) -> &Value {
        &self.registers[reg.0 as usize]
    }

    fn read_many(&self, start: Reg, end: Reg) -> &[Value] {
        &self.registers[start.0 as usize..end.0 as usize]
    }

    fn write(&mut self, reg: Reg, value: Value) {
        self.registers[reg.0 as usize] = value;
    }
}

use crate::vm::instr::Reg;
use crate::vm::value::Value;

use self::chunk::Chunk;
use self::instr::Instr;

pub mod chunk;
pub mod instr;
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
            match Instr::decode(code[pc]) {
                Instr::Return => return,
                Instr::Load(dst, imm) => {
                    self.write(dst, chunk.get_const(imm).clone());
                }
                Instr::Print(src) => {
                    let value = self.read(src);
                    output.emit(&format!("{value}\n"));
                }
                Instr::Add { r0, r1, rd } => {
                    let (lhs, ty) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs + rhs, ty));
                }
                Instr::Sub { r0, r1, rd } => {
                    let (lhs, ty) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs - rhs, ty));
                }
                Instr::Mul { r0, r1, rd } => {
                    let (lhs, ty) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs * rhs, ty));
                }
                Instr::UDiv { r0, r1, rd } => {
                    let (lhs, ty) = self.read(r0).as_uint();
                    let (rhs, _) = self.read(r1).as_uint();
                    self.write(rd, Value::new_uint(lhs / rhs, ty));
                }
                Instr::SDiv { r0, r1, rd } => {
                    let (lhs, ty) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
                    self.write(rd, Value::new_sint(lhs / rhs, ty));
                }
                Instr::Eq { r0, r1, rd } => {
                    let (lhs, _) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
                    self.write(rd, Value::new_bool(lhs == rhs));
                }
                Instr::ULt { r0, r1, rd } => {
                    let (lhs, _) = self.read(r0).as_uint();
                    let (rhs, _) = self.read(r1).as_uint();
                    self.write(rd, Value::new_bool(lhs < rhs));
                }
                Instr::SLt { r0, r1, rd } => {
                    let (lhs, _) = self.read(r0).as_sint();
                    let (rhs, _) = self.read(r1).as_sint();
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
                    let (index, _) = self.read(r1).as_uint();
                    // FIXME: should throw error on out-of-bounds
                    self.write(rd, expr.get(index as usize).cloned().unwrap_or(Value::new_null()));
                }
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

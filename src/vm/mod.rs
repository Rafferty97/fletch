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

impl Vm {
    pub fn new() -> Self {
        Self { registers: vec![] }
    }

    pub fn execute(&mut self, chunk: &Chunk) {
        self.registers.resize(chunk.stack_size(), Value::new_null());

        let code = chunk.code();
        let mut pc = 0;

        loop {
            match Instr::decode(code[pc]) {
                Instr::Return => return,
                Instr::Const(dst, imm) => {
                    self.write(dst, chunk.get_const(imm).clone());
                }
                Instr::PrintInt(src) => {
                    let value = self.read(src).as_int();
                    println!("{value}");
                }
                Instr::PrintStr(src) => {
                    let value = self.read(src).as_str();
                    println!("{value}");
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
                Instr::Move { r0, rd } => {
                    self.write(rd, self.read(r0).clone());
                }
            }
            pc += 1;
        }
    }

    fn read(&self, reg: Reg) -> &Value {
        &self.registers[reg.0 as usize]
    }

    fn write(&mut self, reg: Reg, value: Value) {
        self.registers[reg.0 as usize] = value;
    }
}

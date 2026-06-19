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
        Self { registers: vec![Value::Null; 16] }
    }

    pub fn execute(&mut self, chunk: &Chunk) {
        let code = chunk.code();
        let mut pc = 0;

        loop {
            match Instr::decode(code[pc]) {
                Instr::Return => return,
                Instr::Const(dst, imm) => {
                    self.write(dst, chunk.get_const(imm).clone());
                }
                Instr::Print(src) => {
                    let value = self.read(src);
                    println!("{value:?}");
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

use self::chunk::Chunk;
use self::instr::Instr;

mod chunk;
mod instr;
mod value;

pub struct Vm {}

impl Vm {
    pub fn execute(&mut self, chunk: &Chunk) {
        let code = chunk.code();
        let mut pc = 0;

        loop {
            match Instr::decode(code[pc]) {
                Instr::Return => return,
            }
            pc += 1;
        }
    }
}

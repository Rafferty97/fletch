use std::fmt::Write;
use std::sync::Arc;

use crate::vm::instr::Imm;

use super::instr::{EncodedInstr, Instr};
use super::value::Value;

#[derive(Clone, Debug)]
pub struct Chunk {
    code: Vec<EncodedInstr>,
    labels: Vec<(usize, Arc<str>)>,
    constants: Vec<Value>,
    stack_size: usize,
}

impl Chunk {
    pub fn code(&self) -> &[EncodedInstr] {
        &self.code
    }

    pub fn get_const(&self, imm: Imm) -> &Value {
        &self.constants[imm.0 as usize]
    }

    pub fn disassemble(&self) -> String {
        let mut out = String::new();

        write!(out, "[attrs]\n");
        write!(out, "stack_size = {}\n", self.stack_size);

        write!(out, "\n[code]\n");
        let mut labels = self.labels.iter();
        let mut next_label = labels.next();
        for (index, &instr) in self.code.iter().enumerate() {
            while let Some((_, label)) = next_label.filter(|&&(i, _)| i <= index) {
                write!(out, "{label}:\n");
                next_label = labels.next();
            }
            write!(out, "    {}\n", Instr::decode(instr));
        }

        write!(out, "\n[constants]\n");
        for value in &self.constants {
            write!(out, "    {:?}\n", value);
        }

        out
    }
}

#[derive(Default, Debug)]
pub struct ChunkBuilder {
    code: Vec<EncodedInstr>,
    constants: Vec<Value>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn ins(&mut self, instr: Instr) {
        self.code.push(instr.encode());
    }

    pub fn constant(&mut self, value: Value) -> Imm {
        match self.constants.iter().position(|v| *v == value) {
            Some(idx) => Imm(idx as u16),
            None => {
                let imm = Imm(self.constants.len().try_into().expect("too many constants"));
                self.constants.push(value);
                imm
            }
        }
    }

    pub fn build(self, stack_size: usize) -> Chunk {
        Chunk {
            code: self.code,
            labels: vec![(0, "start".into())],
            constants: self.constants,
            stack_size,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_minimal_chunk() {
        let chunk = Chunk {
            code: vec![Instr::Return.encode()],
            labels: vec![(0, "start".into())],
            constants: vec![],
            stack_size: 0,
        };
        let text = chunk.disassemble();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("[attrs]"));
        assert_eq!(lines.next(), Some("stack_size = 0"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("[code]"));
        assert_eq!(lines.next(), Some("start:"));
        assert_eq!(lines.next(), Some("    ret"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("[constants]"));
        assert_eq!(lines.next(), None);
    }
}

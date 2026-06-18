use std::fmt::Write;
use std::sync::Arc;

use super::instr::{EncodedInstr, Instr};

pub struct Chunk {
    code: Vec<EncodedInstr>,
    labels: Vec<(usize, Arc<str>)>,
}

impl Chunk {
    pub fn code(&self) -> &[EncodedInstr] {
        &self.code
    }

    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        let mut labels = self.labels.iter();
        let mut next_label = labels.next();
        for (index, &instr) in self.code.iter().enumerate() {
            while let Some((_, label)) = next_label.filter(|&&(i, _)| i <= index) {
                write!(out, ".{label}\n");
                next_label = labels.next();
            }
            write!(out, "    {}\n", Instr::decode(instr));
        }
        out
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_minimal_chunk() {
        let chunk = Chunk { code: vec![Instr::Return.encode()], labels: vec![(0, "start".into())] };
        let text = chunk.disassemble();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some(".start"));
        assert_eq!(lines.next(), Some("    ret"));
        assert_eq!(lines.next(), None);
    }
}

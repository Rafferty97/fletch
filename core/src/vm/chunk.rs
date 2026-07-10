use std::fmt::{Display, Write};
use std::ops::Add;
use std::sync::Arc;

use fnv::FnvHashMap;
use itertools::Itertools;

use crate::ast::Symbol;
use crate::vm::instr::{Addr, Imm, Reg};
use crate::vm::module::FuncId;

use super::instr::{EncodedInstr, Instr};
use super::value3::Value;

#[derive(Clone, Debug)]
pub struct Chunk {
    func_id: FuncId,
    code: Vec<EncodedInstr>,
    labels: Vec<(usize, Arc<str>)>,
    constants: Vec<Value>,
    stack_size: usize,
}

impl Chunk {
    pub fn stack_size(&self) -> usize {
        self.stack_size
    }

    pub fn code(&self) -> &[EncodedInstr] {
        &self.code
    }

    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    pub fn get_const(&self, imm: Imm) -> &Value {
        &self.constants[imm.0 as usize]
    }

    pub fn disassemble(&self) -> String {
        let mut out = String::new();

        write!(out, "[attrs]\n");
        write!(out, "func_id = {}\n", self.func_id.0);
        write!(out, "stack_size = {}\n", self.stack_size);

        write!(out, "\n[code]\n");
        let mut labels = self.labels.iter();
        let mut next_label = labels.next();
        for (index, &instr) in self.code.iter().enumerate() {
            while let Some((_, label)) = next_label.filter(|&&(i, _)| i <= index) {
                write!(out, "{label}:\n");
                next_label = labels.next();
            }
            let instr = Instr::decode(instr);
            if let Instr::Jump { addr } | Instr::JumpIfTrue { addr, .. } | Instr::JumpIfFalse { addr, .. } = instr {
                let addr = addr.0 as usize;
                let label = self.labels.binary_search_by_key(&addr, |(addr, _)| *addr);
                match label {
                    Ok(idx) => write!(out, "    {} <{}>\n", instr, &self.labels[idx].1),
                    Err(_) => write!(out, "    {} <unresolved label>\n", instr),
                };
            } else {
                write!(out, "    {}\n", instr);
            }
        }

        if !self.constants.is_empty() {
            write!(out, "\n[constants]\n");
            for value in &self.constants {
                write!(out, "    {:?}\n", value);
            }
        }

        out
    }
}

#[derive(Default, Debug)]
pub struct ChunkBuilder {
    code: Vec<EncodedInstr>,
    label_uses: Vec<(String, Addr)>,
    label_addrs: FnvHashMap<String, Addr>,
    constants: Vec<Value>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn ins(&mut self, instr: Instr) {
        self.code.push(instr.encode());
    }

    pub fn ins_jump(&mut self, label: impl Display) {
        let label = label.to_string().into();
        self.label_uses.push((label, self.curr_pos()));
        self.code.push(Instr::Jump { addr: Addr(0) }.encode());
    }

    pub fn ins_jump_if_true(&mut self, r0: Reg, label: impl Display) {
        let label = label.to_string().into();
        self.label_uses.push((label, self.curr_pos()));
        self.code.push(Instr::JumpIfTrue { r0, addr: Addr(0) }.encode());
    }

    pub fn ins_jump_if_false(&mut self, r0: Reg, label: impl Display) {
        let label = label.to_string().into();
        self.label_uses.push((label, self.curr_pos()));
        self.code.push(Instr::JumpIfFalse { r0, addr: Addr(0) }.encode());
    }

    pub fn ins_label(&mut self, label: impl Display) {
        let label = label.to_string().into();
        if self.label_addrs.insert(label, self.curr_pos()).is_some() {
            panic!("duplicate label");
        }
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

    pub fn build(mut self, func_id: FuncId, stack_size: usize) -> Chunk {
        self.backpatch();

        let mut labels = self
            .label_addrs
            .into_iter()
            .map(|(label, addr)| (addr.0 as usize, Arc::from(label)))
            .collect_vec();
        labels.sort_by_key(|(addr, _)| *addr);

        Chunk { func_id, code: self.code, labels, constants: self.constants, stack_size }
    }

    fn backpatch(&mut self) {
        for (label, use_addr) in &self.label_uses {
            let dst_addr = *self.label_addrs.get(label).expect("unresolved label");
            let instr = &mut self.code[use_addr.0 as usize];
            *instr = Instr::decode(*instr).patch_addr(dst_addr).encode();
        }
    }

    fn curr_pos(&self) -> Addr {
        Addr(self.code.len() as u16)
    }
}

#[cfg(test)]
mod test {
    use crate::vm::instr::Reg;

    use super::*;

    #[test]
    pub fn test_minimal_chunk() {
        let chunk = Chunk {
            func_id: FuncId(12),
            code: vec![Instr::Return { r0: Reg(0) }.encode()],
            labels: vec![(0, "start".into())],
            constants: vec![Value::new_null()],
            stack_size: 0,
        };
        let text = chunk.disassemble();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("[attrs]"));
        assert_eq!(lines.next(), Some("func_id = 12"));
        assert_eq!(lines.next(), Some("stack_size = 0"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("[code]"));
        assert_eq!(lines.next(), Some("start:"));
        assert_eq!(lines.next(), Some("    ret       r0"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("[constants]"));
        assert_eq!(lines.next(), Some("    Null"));
        assert_eq!(lines.next(), None);
    }
}

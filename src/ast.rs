use std::fmt::Display;

use crate::diagnostics::ErrGuaranteed;
use crate::interner::{Index, IndexedInterner};
use crate::span::Spanned;

pub type Stmt = Spanned<StmtKind>;
pub type Expr = Spanned<ExprKind>;

#[derive(Clone, Debug)]
pub struct Program {
    pub main: Func,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: Ident,
    pub body: Block,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Print(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Lit(Lit),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lit {
    Null,
    Bool(bool),
    Int(Symbol),
    Float(Symbol),
    Str(Symbol),
    Err(ErrGuaranteed),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
}

#[derive(Clone, Copy, Debug)]
pub struct Ident {
    pub sym: Symbol,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Symbol(u32);

impl Index for Symbol {
    fn from_usize(index: usize) -> Self {
        Self(index.try_into().expect("too many symbols"))
    }

    fn into_usize(self) -> usize {
        self.0 as usize
    }
}

pub trait SExpr {
    fn write(&self, ctx: &mut SExprCtx);
}

impl SExpr for Program {
    fn write(&self, ctx: &mut SExprCtx) {
        ctx.write_program(self)
    }
}

impl SExpr for Expr {
    fn write(&self, ctx: &mut SExprCtx) {
        ctx.write_expr(self)
    }
}

pub struct SExprCtx<'a, 'sym> {
    pub str: &'a mut String,
    pub sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
}

impl<'a, 'sym> SExprCtx<'a, 'sym> {
    fn write_program(&mut self, node: &Program) {
        self.write_func(&node.main);
    }

    fn write_func(&mut self, node: &Func) {
        self.str.push_str("(func ");
        self.str.push_str(self.sym_interner.get_str(node.name.sym));
        self.str.push(' ');
        self.write_block(&node.body);
        self.str.push(')');
    }

    fn write_block(&mut self, block: &Block) {
        self.str.push_str("(block");
        for stmt in &block.stmts {
            self.str.push(' ');
            self.write_stmt(stmt);
        }
        match &block.tail {
            Some(tail) => {
                self.str.push(' ');
                self.write_expr(tail);
            }
            None => self.str.push_str(" none)"),
        }
    }

    fn write_stmt(&mut self, stmt: &Stmt) {
        match &stmt.node {
            StmtKind::Print(expr) => {
                self.str.push_str("(print ");
                self.write_expr(expr);
                self.str.push(')');
            }
        }
    }

    fn write_expr(&mut self, node: &Expr) {
        match &node.node {
            ExprKind::Lit(lit) => self.write_lit(lit),
            ExprKind::Binary(op, lhs, rhs) => {
                self.str.push('(');
                self.write_binop(*op);
                self.str.push(' ');
                self.write_expr(lhs);
                self.str.push(' ');
                self.write_expr(rhs);
                self.str.push(')');
            }
        }
    }

    fn write_binop(&mut self, op: BinOp) {
        match op {
            BinOp::Add => self.str.push('+'),
        }
    }

    fn write_lit(&mut self, lit: &Lit) {
        match lit {
            Lit::Null => self.str.push_str("null"),
            Lit::Bool(false) => self.str.push_str("false"),
            Lit::Bool(true) => self.str.push_str("true"),
            Lit::Int(sym) => {
                self.str.push_str("(int ");
                self.str.push_str(self.sym_interner.get_str(*sym));
                self.str.push(')');
            }
            Lit::Float(sym) => {
                self.str.push_str("(float ");
                self.str.push_str(self.sym_interner.get_str(*sym));
                self.str.push(')');
            }
            Lit::Str(sym) => {
                self.str.push_str("(float ");
                let str = self.sym_interner.get_str(*sym);
                self.str.push_str(&crate::parser::escape::escape(str));
                self.str.push(')');
            }
            Lit::Err(_) => self.str.push_str("err"),
        }
    }
}

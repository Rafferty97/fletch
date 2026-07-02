use crate::interner::IndexTable;

use super::*;

pub trait SExpr {
    fn write(&self, ctx: &mut SExprCtx);
}

pub struct SExprCtx<'a> {
    pub str: &'a mut String,
    pub sym_table: &'a IndexTable<'a, Symbol, str>,
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

impl SExpr for Ty {
    fn write(&self, ctx: &mut SExprCtx) {
        ctx.write_ty(self)
    }
}

impl<'a> SExprCtx<'a> {
    fn write_program(&mut self, node: &Program) {
        for func in &node.funcs {
            self.write_func(&func);
            self.str.push('\n');
        }
    }

    fn write_func(&mut self, node: &Func) {
        self.str.push_str("(func ");
        self.write_sym(node.name.sym);
        self.str.push_str(" (params");
        for (name, ty) in &node.params {
            self.str.push_str(" (");
            self.write_sym(name.sym);
            self.str.push(' ');
            self.write_ty(ty);
            self.str.push(')');
        }
        self.str.push_str(") ");
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
            StmtKind::Expr(expr) => {
                self.write_expr(expr);
            }
            StmtKind::Let(name, ty, value, mutability) => {
                match mutability {
                    Mutability::Not => self.str.push_str("(let "),
                    Mutability::Mut => self.str.push_str("(var "),
                };
                match ty {
                    Some(ty) => self.write_ty(ty),
                    None => self.str.push('_'),
                }
                self.str.push(' ');
                self.write_sym(name.sym);
                self.str.push(' ');
                self.write_expr(value);
                self.str.push(')');
            }
            StmtKind::Assign(lhs, rhs) => {
                self.str.push_str("(= ");
                self.write_sym(lhs.sym);
                self.str.push(' ');
                self.write_expr(rhs);
                self.str.push(')');
            }
        }
    }

    fn write_expr(&mut self, node: &Expr) {
        match &node.node {
            ExprKind::Lit(lit) => self.write_lit(lit),
            ExprKind::Var(var) => {
                self.str.push_str("(var ");
                self.write_sym(var.sym);
                self.str.push(')');
            }
            ExprKind::Unary(op, rhs, _) => {
                self.str.push('(');
                self.write_unaryop(*op);
                self.str.push(' ');
                self.write_expr(rhs);
                self.str.push(')');
            }
            ExprKind::Binary(op, lhs, rhs, _) => {
                self.str.push('(');
                self.write_binop(*op);
                self.str.push(' ');
                self.write_expr(lhs);
                self.str.push(' ');
                self.write_expr(rhs);
                self.str.push(')');
            }
            ExprKind::Call(func, args, _) => {
                self.str.push_str("(call ");
                self.write_expr(func);
                for arg in args {
                    self.str.push(' ');
                    self.write_expr(arg);
                }
                self.str.push(')');
            }
            ExprKind::Grouped(expr) => self.write_expr(expr),
            ExprKind::Array(exprs) => {
                self.str.push_str("(array");
                for arg in exprs {
                    self.str.push(' ');
                    self.write_expr(arg);
                }
                self.str.push(')');
            }
            ExprKind::Index(expr, index) => {
                self.str.push_str("(index ");
                self.write_expr(expr);
                self.str.push(' ');
                self.write_expr(index);
                self.str.push(')');
            }
            ExprKind::If { cond, then, r#else } => {
                self.str.push_str("(if ");
                self.write_expr(cond);
                self.str.push(' ');
                self.write_expr(then);
                self.str.push(' ');
                self.write_opt_expr(r#else.as_deref());
                self.str.push(')');
            }
            ExprKind::Block(block) => {
                self.write_block(block);
            }
        }
    }

    fn write_opt_expr(&mut self, expr: Option<&Expr>) {
        match expr {
            Some(expr) => self.write_expr(expr),
            None => self.str.push_str("none"),
        }
    }

    fn write_unaryop(&mut self, op: UnaryOp) {
        use std::fmt::Write;
        write!(&mut self.str, "{op}");
    }

    fn write_binop(&mut self, op: BinOp) {
        use std::fmt::Write;
        write!(&mut self.str, "{op}");
    }

    fn write_lit(&mut self, lit: &Lit) {
        match lit {
            Lit::Null => self.str.push_str("null"),
            Lit::Bool(false) => self.str.push_str("false"),
            Lit::Bool(true) => self.str.push_str("true"),
            Lit::Int(sym) => {
                self.str.push_str("(int ");
                self.write_sym(*sym);
                self.str.push(')');
            }
            Lit::Float(sym) => {
                self.str.push_str("(float ");
                self.write_sym(*sym);
                self.str.push(')');
            }
            Lit::Str(sym) => {
                self.str.push_str("(str \"");
                let str = self.sym_table.get_str(*sym);
                self.str.push_str(&crate::parser::escape::escape(str));
                self.str.push_str("\")");
            }
            Lit::Err(_) => self.str.push_str("err"),
        }
    }

    fn write_ty(&mut self, node: &Ty) {
        match &node.node {
            TyKind::Infer => self.str.push('_'),
            TyKind::Var(var) => self.write_sym(var.sym),
            TyKind::Nullable(ty) => {
                self.str.push_str("(? ");
                self.write_ty(ty);
                self.str.push(')');
            }
            TyKind::Array(ty) => {
                self.str.push_str("(array ");
                self.write_ty(ty);
                self.str.push(')');
            }
            TyKind::Tuple(tys) => {
                self.str.push_str("(tuple");
                for ty in tys {
                    self.str.push(' ');
                    self.write_ty(ty);
                }
                self.str.push(')');
            }
        }
    }

    fn write_sym(&mut self, sym: Symbol) {
        self.str.push_str(self.sym_table.get_str(sym));
    }
}

use super::*;

pub trait SExpr {
    fn write(&self, ctx: &mut SExprCtx);
}

pub struct SExprCtx<'a, 'sym> {
    pub str: &'a mut String,
    pub sym_interner: &'a IndexedInterner<'sym, Symbol, str>,
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
            StmtKind::Expr(expr) => {
                self.write_expr(expr);
            }
            StmtKind::Let(name, value) => {
                self.str.push_str("(let ");
                self.write_sym(name.sym);
                self.str.push(' ');
                self.write_expr(value);
                self.str.push(')');
            }
            StmtKind::Assign(lhs, rhs) => {
                self.str.push_str("(= ");
                self.write_expr(lhs);
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
            ExprKind::Binary(op, lhs, rhs) => {
                self.str.push('(');
                self.write_binop(*op);
                self.str.push(' ');
                self.write_expr(lhs);
                self.str.push(' ');
                self.write_expr(rhs);
                self.str.push(')');
            }
            ExprKind::Call(func, args) => {
                self.str.push_str("(call ");
                self.write_expr(func);
                for arg in args {
                    self.str.push(' ');
                    self.write_expr(arg);
                }
                self.str.push(')');
            }
            ExprKind::Grouped(expr) => self.write_expr(expr),
        }
    }

    fn write_binop(&mut self, op: BinOp) {
        match op {
            BinOp::Add => self.str.push('+'),
            BinOp::Sub => self.str.push('-'),
        }
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
                let str = self.sym_interner.get_str(*sym);
                self.str.push_str(&crate::parser::escape::escape(str));
                self.str.push_str("\")");
            }
            Lit::Err(_) => self.str.push_str("err"),
        }
    }

    fn write_sym(&mut self, sym: Symbol) {
        self.str.push_str(self.sym_interner.get_str(sym));
    }
}

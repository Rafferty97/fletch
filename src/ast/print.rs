use std::fmt::Display;

use crate::arena::Ctx;
use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, Lit};

pub fn print_expr<'a, 'cx>(ctx: Ctx<'cx>, expr: &'a Expr) -> String {
    format!("{}", PrettyPrint(ctx, expr))
}

struct PrettyPrint<'a, 'cx, T>(Ctx<'cx>, &'a T);

impl<'a, 'cx, T> PrettyPrint<'a, 'cx, T> {
    fn with<U>(&self, value: &'a U) -> PrettyPrint<'a, 'cx, U> {
        PrettyPrint(self.0, value)
    }
}

impl Display for PrettyPrint<'_, '_, Expr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.1.kind {
            ExprKind::Lit(lit) => write!(f, "{}", self.with(lit)),
            ExprKind::Var(ident) => write!(f, "{}", self.0.get_str(ident.sym)),
            ExprKind::Binary(op, lhs, rhs) => {
                let lhs = self.with(&**lhs);
                let rhs = self.with(&**rhs);
                write!(f, "{} {} {}", lhs, op, rhs)
            }
            ExprKind::Paren(expr) => write!(f, "({})", self.with(&**expr)),
            ExprKind::Call(callee, args) => {
                write!(f, "{}(", self.with(&**callee))?;
                if let [first, rest @ ..] = &**args {
                    write!(f, "{}", self.with(first))?;
                    for arg in rest {
                        write!(f, ", {}", self.with(arg))?;
                    }
                }
                write!(f, ")")
            }
        }
    }
}

impl Display for PrettyPrint<'_, '_, Lit> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get_str(self.1.sym))
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self.node {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
        };
        write!(f, "{}", sym)
    }
}

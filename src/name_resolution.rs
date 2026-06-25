use fnv::FnvHashMap;

use crate::ast::ExprKind::Binary;
use crate::ast::span::Span;
use crate::ast::{Expr, ExprKind, Func, Ident, Mutability, NodeId, Program, StmtKind, Symbol};
use crate::diagnostics::{Diagnostic, DiagnosticReporter, ErrGuaranteed};
use crate::interner::IndexTable;
use crate::util::IdGen;

pub struct NameResolution<'a> {
    sym_table: &'a IndexTable<'a, Symbol, str>,
    defs: FnvHashMap<DefId, BindingInfo>,
    uses: FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
    scopes: Vec<FnvHashMap<Symbol, DefId>>,
    def_ids: IdGen<DefId>,
    errors: &'a dyn DiagnosticReporter,
}

pub struct NameTables {
    pub defs: FnvHashMap<DefId, BindingInfo>,
    pub uses: FnvHashMap<NodeId, Result<DefId, ErrGuaranteed>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DefId(u32);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BindingInfo {
    pub mutability: Mutability,
    pub span: Span,
}

impl<'a> NameResolution<'a> {
    pub fn new(sym_table: &'a IndexTable<'a, Symbol, str>, errors: &'a dyn DiagnosticReporter) -> Self {
        Self {
            sym_table,
            defs: FnvHashMap::default(),
            uses: FnvHashMap::default(),
            scopes: vec![],
            def_ids: IdGen::new(DefId),
            errors,
        }
    }

    pub fn finish(self) -> NameTables {
        NameTables { defs: self.defs, uses: self.uses }
    }

    pub fn resolve_program(&mut self, program: &Program) {
        self.resolve_func(&program.main);
    }

    pub fn resolve_func(&mut self, func: &Func) {
        self.push_scope();
        for stmt in &func.body.stmts {
            match &stmt.node {
                StmtKind::Expr(expr) => self.resolve_expr(expr),
                StmtKind::Let(name, expr, mutability) => {
                    self.resolve_expr(expr);
                    self.define_name(*name, *mutability);
                }
                StmtKind::Assign(name, rhs) => {
                    self.resolve_expr(rhs);
                    let Some(binding_info) = self.resolve_name(*name) else {
                        continue;
                    };
                    if binding_info.mutability == Mutability::Not {
                        let diag = Diagnostic::error("cannot assign to immutable binding", name.span)
                            .with_label("variable was declared here", binding_info.span);
                        self.errors.report(diag);
                    }
                }
            }
        }
        self.pop_scope();
    }

    pub fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.node {
            ExprKind::Lit(_) => {}
            ExprKind::Var(name) => {
                self.resolve_name(*name);
            }
            ExprKind::Binary(_, lhs, rhs) => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            ExprKind::Call(func, args) => {
                // FIXME
                // self.resolve_expr(func);
                args.iter().for_each(|arg| self.resolve_expr(arg));
            }
            ExprKind::Grouped(expr) => self.resolve_expr(expr),
            ExprKind::Array(exprs) => {
                exprs.iter().for_each(|arg| self.resolve_expr(arg));
            }
            ExprKind::Index(expr, index) => {
                self.resolve_expr(expr);
                self.resolve_expr(index);
            }
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(FnvHashMap::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_name(&mut self, ident: Ident, mutability: Mutability) -> DefId {
        let def_id = self.def_ids.next();
        let binding_info = BindingInfo { mutability, span: ident.span };
        self.defs.insert(def_id, binding_info);
        self.uses.insert(ident.id, Ok(def_id));
        self.scopes.last_mut().unwrap().insert(ident.sym, def_id);
        def_id
    }

    fn resolve_name(&mut self, ident: Ident) -> Option<&BindingInfo> {
        let def_id = self.find_name(ident.sym).ok_or_else(|| {
            let msg = format!("cannot find `{}` in this scope", self.sym_table.get_str(ident.sym));
            self.errors.report_err(Diagnostic::error(msg, ident.span))
        });
        self.uses.insert(ident.id, def_id);
        def_id.ok().map(|def_id| &self.defs[&def_id])
    }

    fn find_name(&self, name: Symbol) -> Option<DefId> {
        self.scopes.iter().rev().find_map(|scope| scope.get(&name)).copied()
    }
}

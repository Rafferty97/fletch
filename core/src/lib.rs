#![allow(unused)]

pub use driver::{FletchOpts, check, run};
pub use vm::OutputSink;

mod ast;
mod compile;
mod diagnostics;
mod driver;
mod interner;
mod name_resolution;
mod parser;
mod tests;
mod thin_rc;
mod typecheck;
mod types;
mod util;
mod vm;

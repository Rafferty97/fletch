#![allow(unused)]

pub use driver::{FletchOpts, run};

mod ast;
mod compile;
mod diagnostics;
mod driver;
mod interner;
mod name_resolution;
mod parser;
mod tests;
mod typecheck;
mod types;
mod util;
mod vm;

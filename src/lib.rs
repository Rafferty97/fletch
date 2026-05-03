#![allow(unused)]

mod ast;
mod error;
mod interpreter;
mod ir;
mod lower;
mod parser;
mod util;

#[cfg(test)]
mod test {
    use crate::interpreter::{Value, interpret_program};
    use crate::lower::lower_program;
    use crate::parser::Parser;

    #[test]
    fn basic_test() {
        let src = "2 + 3";

        let mut parser = Parser::new(src);
        let ast = parser.parse_program().unwrap();

        let ir = lower_program(&ast).unwrap();

        let result = interpret_program(ir);
        assert_eq!(result, Value::UInt64(5));
    }
}

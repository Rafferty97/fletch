use std::io::{self, Stdin, Stdout, Write};

use fletch::ReplIo;

struct StdIo {
    stdin: Stdin,
    stdout: Stdout,
}

impl StdIo {
    pub fn new() -> Self {
        Self { stdin: io::stdin(), stdout: io::stdout() }
    }
}

impl ReplIo for StdIo {
    fn read_line(&mut self, cont: bool, out: &mut String) {
        match cont {
            false => write!(self.stdout, "> ").unwrap(),
            true => write!(self.stdout, "| ").unwrap(),
        }
        self.stdout.flush().unwrap();
        self.stdin.read_line(out).unwrap();
    }

    fn write(&mut self) -> &mut impl Write {
        &mut self.stdout
    }
}

fn main() {
    fletch::run_repl(StdIo::new());
}

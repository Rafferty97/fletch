use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    write!(&mut out, "> ").unwrap();
    out.flush().unwrap();

    fletch::run_repl(|run| {
        for line in stdin.lock().lines() {
            let line = line.expect("Failed to read line");
            if line.trim() == ".exit" {
                break;
            }
            if line.trim().is_empty() {
                write!(&mut out, "> ").unwrap();
                out.flush().unwrap();
                continue;
            }
            run(line, &mut out);
            write!(&mut out, "\n> ").unwrap();
            out.flush().unwrap();
        }
    });
}

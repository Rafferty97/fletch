// fn main() {
//     fletch::run("2+(6-3)/4");
// }

use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    write!(&mut out, "> ").unwrap();
    out.flush().unwrap();

    for line in stdin.lock().lines() {
        let line = line.expect("Failed to read line");
        if line.is_empty() {
            break;
        }
        fletch::run(&line, &mut out);
        write!(&mut out, "\n> ").unwrap();
        out.flush().unwrap();
    }
}

use std::{fs, path::PathBuf};

use clap::Parser;
use tap::Pipe;

#[derive(Parser, Debug)]
struct Args {
    /// Input file
    filename: PathBuf,
    /// Print the parsed AST as an s-expr
    #[arg(long)]
    sexpr: bool,
    /// Print disassembly of each chunk
    #[arg(short, long)]
    disassemble: bool,
}

pub fn run() -> Result<(), String> {
    let args = Args::parse();

    let filename = args.filename.file_name().unwrap().to_string_lossy();

    let src = fs::read(&args.filename)
        .map_err(|err| format!("Cannot read '{}': {err:#}", &filename))?
        .pipe(String::from_utf8)
        .map_err(|_| format!("Cannot read '{}': Invalid UTF-8", &filename))?;

    let opts = eld::FletchOpts { sexpr: args.sexpr, disassemble: args.disassemble };

    eld::run(&filename, &src, opts);

    Ok(())
}

pub fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(-1);
    }
}

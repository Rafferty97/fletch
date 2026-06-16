use std::fs;

use clap::Parser;
use tap::Pipe;

#[derive(Parser, Debug)]
struct Args {
    filename: String,
}

pub fn run() -> Result<(), String> {
    let args = Args::parse();

    let src = fs::read(&args.filename)
        .map_err(|err| format!("Cannot read '{}': {err:#}", &args.filename))?
        .pipe(String::from_utf8)
        .map_err(|_| format!("Cannot read '{}': Invalid UTF-8", &args.filename))?;

    if let Err(err) = eld::run(&src) {
        Err(format!("{err:#}"))?
    }

    Ok(())
}

pub fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(-1);
    }
}

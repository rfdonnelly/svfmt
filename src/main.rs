use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

use svfmt::{self, format, parse};

use env_logger;
use snafu::ErrorCompat;

fn main() {
    env_logger::init();

    let filename = env::args().skip(1).next().unwrap();
    let filename = Path::new(&filename);
    let extension = filename.extension().and_then(OsStr::to_str).unwrap();

    let source = load_file(filename).unwrap();

    match transform(&extension, &source) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("An error occurred: {}", e);
            if let Some(backtrace) = ErrorCompat::backtrace(&e) {
                println!("{}", backtrace);
            }
        }
    }
}

fn transform(extension: &str, source: &str) -> svfmt::Result<()> {
    let language = match extension {
        "c" | "h" => unsafe { svfmt::tree_sitter_c() },
        _ => unsafe { svfmt::tree_sitter_verilog() },
    };

    let tree = parse(language, &source)?;
    svfmt::debug(&mut std::io::stdout(), &source, &tree)?;
    format(&mut std::io::stdout(), &source, &tree)
}

fn load_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}

use std::env;
use std::io;
use std::io::Read;
use std::path::Path;
use std::ffi::OsStr;
use std::fs::File;
use tree_sitter::Language;
use svfmt::{parse, format};

extern "C" { fn tree_sitter_c() -> Language; }
extern "C" { fn tree_sitter_verilog() -> Language; }

fn main() {
    let filename = env::args().skip(1).next().unwrap();
    let filename = Path::new(&filename);
    let extension = filename
        .extension()
        .and_then(OsStr::to_str)
        .unwrap();

    let language =
        match extension {
            "c" | "h" => unsafe { tree_sitter_c() },
            _ => unsafe {tree_sitter_verilog() },
        };

    let source = load_file(filename).unwrap();
    let tree = parse(language, &source);
    format(&mut std::io::stdout(), &source, tree);
}

fn load_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content)
}

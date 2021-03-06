use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

fn main() {
    build_language("verilog");
    build_language("c");

    generate_symbols_enum().unwrap();
}

struct Symbol {
    name: String,
    id: String,
}

fn in_file() -> PathBuf {
    ["..", "vendor", "tree-sitter-verilog", "src", "parser.c"]
        .iter()
        .collect()
}

fn out_file() -> PathBuf {
    let out_dir = env::var("OUT_DIR").unwrap();
    Path::new(&out_dir).join("symbols.rs")
}

fn generate_symbols_enum() -> io::Result<()> {
    let symbols = parse_symbols(in_file())?;

    write_symbols_enum(&symbols, out_file())
}

fn parse_symbols(path: PathBuf) -> io::Result<Vec<Symbol>> {
    let mut symbols = Vec::new();

    let f = File::open(path)?;
    let reader = BufReader::new(f);

    for line in reader.lines() {
        if let Ok(line) = line {
            add_symbol_if_starts_with(&line, "sym_", &mut symbols);
            add_symbol_if_starts_with(&line, "alias_sym_", &mut symbols);
        }
    }

    Ok(symbols)
}

fn add_symbol_if_starts_with(line: &str, starts_with: &str, symbols: &mut Vec<Symbol>) {
    // Ignore hidden symbols
    //
    // Hidden symbols begin with a '_'.
    let not_starts_with = format!("{}_", starts_with);

    let line = line.trim_start();

    if line.starts_with(starts_with) && !line.starts_with(&not_starts_with) {
        let tokens = line.split(|c| char::is_whitespace(c) || c == '=' || c == ',')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();
        let (name, id) = (tokens[0], tokens[1].to_string());
        let name = name
            .get(starts_with.len()..)
            .unwrap().split('_')
            .map(titlecase)
            .collect::<Vec<String>>()
            .join("");
        symbols.push(Symbol { name, id });
    }
}

fn write_symbols_enum(symbols: &[Symbol], path: PathBuf) -> io::Result<()> {
    let mut f = File::create(path)?;

    writeln!(f, "#[derive(Debug, PartialEq)]")?;
    writeln!(f, "enum Symbol {{")?;
    writeln!(f, "    Undefined,")?;
    for symbol in symbols {
        writeln!(f, "    {},", symbol.name)?;
    }
    writeln!(f, "}}")?;
    writeln!(f)?;
    writeln!(f, "impl From<u16> for Symbol {{")?;
    writeln!(f, "    fn from(id: u16) -> Self {{")?;
    writeln!(f, "        match id {{")?;
    for symbol in symbols {
        writeln!(f, "            {} => Symbol::{},", symbol.id, symbol.name)?;
    }
    writeln!(f, "            _ => Symbol::Undefined,")?;
    writeln!(f, "        }}")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;

    Ok(())
}

fn titlecase(s: &str) -> String {
    let mut result = s.to_string();
    result.get_mut(0..1).unwrap().make_ascii_uppercase();
    result
}

fn build_language(language: &str) {
    let package = format!("tree-sitter-{}", language);
    let source_directory: PathBuf = ["..", "vendor", &package, "src"].iter().collect();
    let source_file = source_directory.join("parser.c");

    println!("cargo:rerun-if-changed={}", source_file.to_string_lossy());

    cc::Build::new()
        .file(source_file)
        .include(source_directory)
        .compile(&package);
}

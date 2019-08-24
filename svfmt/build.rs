use std::path::PathBuf;

fn main() {
    build_language("verilog");
    build_language("c");
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

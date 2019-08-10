fn main() {
    build_language("verilog");
    build_language("c");
}

fn build_language(language: &str) {
    let package = format!("tree-sitter-{}", language);
    let source_directory = format!("{}/src", package);
    let source_file = format!("{}/parser.c", source_directory);

    println!("rerun-if-changed={}", source_file);

    cc::Build::new()
        .file(source_file)
        .include(source_directory)
        .compile(&package);
}

fn main() {
    build_language("verilog");
    build_language("c");
}

fn build_language(language: &str) {
    let package = format!("tree-sitter-{}", language);
    let package_path = format!("../vendor/{}", package);
    let source_directory = format!("{}/src", package_path);
    let source_file = format!("{}/parser.c", source_directory);

    println!("cargo:rerun-if-changed={}", source_file);

    cc::Build::new()
        .file(source_file)
        .include(source_directory)
        .compile(&package);
}

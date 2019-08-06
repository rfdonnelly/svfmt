fn main() {
    let mut build = cc::Build::new();

    build
        .file("tree-sitter-verilog/src/parser.c")
        .include("tree-sitter-verilog/src")
        .compile("tree-sitter-verilog");
}

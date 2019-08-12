# svfmt ![Stability: Experimental](http://badges.github.io/stability-badges/dist/experimental.svg)

A tool for formatting Verilog/SystemVerilog code.

NOTE: This tool is in the very early development phase.
It is not ready to format real code.

## How it Works

Verilog/SystemVerilog code is parsed using [Tree-sitter].
The resulting tree is walked and converted to a string representation.
The result is formatted Verilog/SystemVerilog code.

[Tree-sitter]: http://tree-sitter.github.io/tree-sitter

## Development Dependencies

* Rust
* Node.js

## Build

```sh
git clone --recursive git@github.com:rfdonnelly/svfmt.git
cd svfmt
(cd vendor/tree-sitter-verilog; npm install)
cargo test
cargo run -- fixtures/expressions.sv
```

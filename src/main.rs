use std::io;
use tree_sitter::{Language, Parser, Node};

extern "C" {
    fn tree_sitter_verilog() -> Language;
}

fn main() {
    let language = unsafe { tree_sitter_verilog() };
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    let source_code = "module mymodule(); endmodule";
    let tree = parser.parse(source_code, None).unwrap();

    let formatter = Formatter::new(source_code);
    formatter.write_debug_node(&mut std::io::stdout(), 0, tree.root_node());
    println!();
    formatter.write_node(&mut std::io::stdout(), tree.root_node());
}

struct Formatter<'a> {
    source: &'a [u8],
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
        }
    }

    fn write_debug_node<T>(&self, f: &mut T, indent: usize, node: Node<'a>)
    where
        T: io::Write,
    {
        writeln!(f, "{:indent$}{}:{}: {}", "", node.kind(), node.kind_id(), node.utf8_text(self.source).unwrap(), indent = indent).unwrap();

        for child in node.children() {
            self.write_debug_node(f, indent + 2, child);
        }
    }

    fn write_node<T>(&self, f:&mut T, node: Node<'a>)
    where
        T: io::Write,
    {
        match node.kind() {
            "source_file" => self.write_children(f, node),
            "module_declaration" => self.write_module(f, node),
            _ => {
                self.write_node_generic(f, node);
                self.write_children(f, node);
            }
        }

    }

    fn write_children<T>(&self, f:&mut T, node: Node<'a>)
    where
        T: io::Write,
    {
        for child in node.children() {
            self.write_node(f, child);
        }
    }

    fn write_module<T>(&self, f:&mut T, node: Node<'a>)
    where
        T: io::Write,
    {
        writeln!(f, "{} ", node.utf8_text(self.source).unwrap()).unwrap();
    }

    fn write_node_generic<T>(&self, f:&mut T, node: Node<'a>)
    where
        T: io::Write,
    {
        writeln!(f, "{} ", node.utf8_text(self.source).unwrap()).unwrap();
    }
}

use std::fmt;
use std::io;

use tree_sitter::{Language, Node, Parser, Tree, TreeCursor};

extern "C" {
    pub fn tree_sitter_c() -> Language;
}
extern "C" {
    pub fn tree_sitter_verilog() -> Language;
}

pub fn parse<'a>(language: Language, source: &'a str) -> Tree {
    let mut parser = Parser::new();
    parser.set_language(language).unwrap();
    parser.parse(&source, None).unwrap()
}

pub fn format<'a, T>(f: &mut T, source: &'a str, tree: &Tree) -> io::Result<()>
where
    T: io::Write,
{
    let formatter = Formatter::new(&source, tree);
    write!(f, "{}", formatter)
}

pub fn debug<'a, T>(f: &mut T, source: &'a str, tree: &Tree) -> io::Result<()>
where
    T: io::Write,
{
    writeln!(f, "{}", tree.root_node().to_sexp())?;
    writeln!(f)?;
    let formatter = Formatter::new(&source, tree);
    formatter.debug_walk(f, 0, &mut tree.walk())
}

struct Formatter<'a> {
    source: &'a [u8],
    tree: &'a Tree,
}

impl<'a> fmt::Display for Formatter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format_node(f, self.tree.root_node())
    }
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, tree: &'a Tree) -> Self {
        Self {
            source: source.as_bytes(),
            tree,
        }
    }

    fn debug_walk<T>(&self, f: &mut T, mut indent: usize, cursor: &mut TreeCursor<'a>) -> io::Result<()>
    where
        T: io::Write,
    {
        loop {
            let node = cursor.node();
            write!(f, "{:indent$}", "", indent = indent)?;
            if node.is_named() {
                write!(f, "{}", node.kind())?;
            } else {
                write!(f, "anonymous")?;
            }
            if let Some(field_name) = cursor.field_name() {
                write!(f, "({})", field_name)?;
            }
            if node.child_count() == 0 {
                write!(f, ": {}", self.text(node))?;
            }
            writeln!(f)?;

            if cursor.goto_first_child() {
                indent += 4;
            } else {
                if !cursor.goto_next_sibling() {
                    loop {
                        if !cursor.goto_parent() {
                            return Ok(());
                        }

                        indent -= 4;

                        if cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
        }
    }

    fn text(&self, node: Node<'a>) -> &'a str {
        node.utf8_text(self.source).unwrap()
    }

    fn format_with_newline<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        writeln!(f, "{}", self.text(node))
    }

    fn format_terminals<T>(&self, f: &mut T, node: Node<'a>, sep: &str, suffix: &str) -> fmt::Result
    where
        T: fmt::Write,
    {
        let nodes: Vec<&'a str> = Terminals::new(node).map(|node| self.text(node)).collect();

        write!(f, "{}{}", nodes.join(sep), suffix)
    }

    fn format_children<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        for child in node.children() {
            self.format_node(f, child)?;
        }

        Ok(())
    }

    fn format_node<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        match node.kind() {
            "function_declaration" => self.format_function_declaration(f, node),
            "expression" => self.format_expression(f, node),
            "jump_statement" => self.format_jump_statement(f, node),
            "integer_atom_type" => write!(f, "{} ", self.text(node)),
            "simple_identifier" => write!(f, "{}", self.text(node)),
            "list_of_arguments_parent" => self.format_list_of_arguments(f, node),
            _ => self.format_children(f, node),
        }
    }

    fn format_list_of_arguments<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        write!(f, "(")?;
        let children = node
            .children()
            .filter(|node| node.is_named())
            .identify_last();

        for (last, child) in children {
            self.format_node(f, child)?;

            if !last {
                write!(f, ", ")?;
            }
        }
        write!(f, ")")
    }

    fn format_expression<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        if node.child_count() == 3 {
            // Binary expression
            let left = node.child(0).unwrap();
            let operator = node.child(1).unwrap();
            let right = node.child(2).unwrap();

            self.format_node(f, left)?;
            write!(f, " {} ", self.text(operator))?;
            self.format_node(f, right)
        } else {
            assert_eq!(node.child_count(), 1);

            self.format_node(f, node.child(0).unwrap())
        }
    }

    fn format_jump_statement<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        let jump_type = node.child(0).unwrap();

        write!(f, "{}", self.text(jump_type))?;
        if node.child_count() == 3 {
            write!(f, " ")?;
            let expression = node.child(1).unwrap();
            self.format_expression(f, expression)?;
        }

        Ok(())
    }

    fn format_function_declaration<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        let indent = 0;

        assert_eq!(node.child_count(), 2);

        let keyword = node.child(0).unwrap();
        let body = node.child(1).unwrap();

        assert_eq!(keyword.kind(), "function");
        assert_eq!(body.kind(), "function_body_declaration");

        write!(f, "function ")?;

        for child in body.children() {
            match child.kind() {
                "function_data_type_or_implicit1" => self.format_terminals(f, child, " ", " ")?,
                "function_identifier" => self.format_terminals(f, child, " ", "")?,
                "tf_port_list" => self.format_tf_port_list(f, child)?,
                "function_statement_or_null" => {
                    self.format_function_statement_or_null(f, indent + 4, child)?;
                }
                "comment" => self.format_with_newline(f, child)?,
                _ => {}
            }
        }

        writeln!(f, "endfunction")?;
        writeln!(f)
    }

    fn write_spaces<T>(&self, f: &mut T, spaces: usize) -> fmt::Result
    where
        T: fmt::Write,
    {
        write!(f, "{:1$}", "", spaces)
    }

    fn format_tf_port_list<T>(&self, f: &mut T, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        writeln!(f, "(")?;

        let children = node
            .children()
            .filter(|child| child.is_named())
            .identify_last();

        for (last, child) in children {
            self.write_spaces(f, 4)?;
            self.format_node(f, child)?;

            if !last {
                writeln!(f, ",")?;
            }
        }

        writeln!(f, "\n);")
    }

    fn format_function_statement_or_null<T>(&self, f: &mut T, indent: usize, node: Node<'a>) -> fmt::Result
    where
        T: fmt::Write,
    {
        assert_eq!(node.child_count(), 1);

        self.write_spaces(f, indent)?;
        self.format_children(f, node)?;
        writeln!(f, ";")
    }
}

/// Iterator struct for iterating over all terminal nodes
struct Terminals<'a> {
    index: usize,
    terminals: Vec<Node<'a>>,
}

impl<'a> Terminals<'a> {
    fn new(node: Node<'a>) -> Self {
        let mut terminals = Vec::new();

        Self::collect_terminals(node, &mut terminals);

        Terminals {
            index: 0,
            terminals,
        }
    }

    fn collect_terminals(node: Node<'a>, terminals: &mut Vec<Node<'a>>) {
        for child in node.children() {
            if child.child_count() == 0 {
                terminals.push(child);
            }
            Self::collect_terminals(child, terminals);
        }
    }
}

impl<'a> Iterator for Terminals<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = if self.index == self.terminals.len() {
            None
        } else {
            Some(self.terminals[self.index])
        };

        self.index += 1;

        item
    }
}

// From: https://users.rust-lang.org/t/iterator-need-to-identify-the-last-element/8836
pub trait IdentifyLast: Iterator + Sized {
    fn identify_last(self) -> Iter<Self>;
}

impl<T> IdentifyLast for T
where
    T: Iterator,
{
    fn identify_last(mut self) -> Iter<Self> {
        let e = self.next();
        Iter {
            iter: self,
            buffer: e,
        }
    }
}

pub struct Iter<T>
where
    T: Iterator,
{
    iter: T,
    buffer: Option<T::Item>,
}

impl<T> Iterator for Iter<T>
where
    T: Iterator,
{
    type Item = (bool, T::Item);

    fn next(&mut self) -> Option<Self::Item> {
        match self.buffer.take() {
            None => None,
            Some(e) => match self.iter.next() {
                None => Some((true, e)),
                Some(f) => {
                    self.buffer = Some(f);
                    Some((false, e))
                }
            },
        }
    }
}

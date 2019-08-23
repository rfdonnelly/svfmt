use std::fmt;
use std::io;

use log::debug;
use snafu::{ensure, Backtrace, Snafu};
use tree_sitter::{Language, Node, Parser, Tree, TreeCursor};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not write output"))]
    IoError { source: io::Error },
    #[snafu(display("Could not set source language. {}", message))]
    LanguageError { message: String },
    #[snafu(display("Unexpected syntax tree.  Node does not contain expected child."))]
    TreeError { backtrace: Backtrace },
    #[snafu(display("Unexpected syntax tree.  Invalid node child count."))]
    InvalidCount { backtrace: Backtrace },
    #[snafu(display("Unexpected syntax tree.  Invalid node kind."))]
    InvalidKind { backtrace: Backtrace },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::IoError { source }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error::LanguageError { message }
    }
}

extern "C" {
    pub fn tree_sitter_c() -> Language;
}
extern "C" {
    pub fn tree_sitter_verilog() -> Language;
}

pub fn parse<'a>(language: Language, source: &'a str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser.set_language(language)?;
    Ok(parser.parse(&source, None).unwrap())
}

pub fn format<'a, T>(f: &mut T, source: &'a str, tree: &Tree) -> Result<()>
where
    T: io::Write,
{
    let length = source.len() + source.len() / 2;
    let mut b = Buffer::with_capacity(length);
    Formatter::new(&source).format_node(&mut b, tree.root_node())?;
    write!(f, "{}", b)?;
    Ok(())
}

pub fn debug<'a, T>(f: &mut T, source: &'a str, tree: &Tree) -> Result<()>
where
    T: io::Write,
{
    writeln!(f, "{}", tree.root_node().to_sexp())?;
    writeln!(f)?;
    Formatter::new(&source).debug_walk(f, 0, &mut tree.walk())
}

struct Buffer {
    content: String,
    /// The current length of the current line.
    ///
    /// As content is pushed into the buffer, the line_length is incremented for every character
    /// added.  If a newline character is seen, the line length is reset to 0.
    line_length: usize,
    /// The current indent level in number of spaces.
    indent: usize,
}

impl Buffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            content: String::with_capacity(capacity),
            line_length: 0,
            indent: 0,
        }
    }

    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push(c);
        }
    }

    fn push(&mut self, c: char) {
        if c == '\n' {
            self.line_length = 0;
        } else {
            self.line_length += 1;
        }

        self.content.push(c);
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.push(' ');
        }
    }

    fn increment_indent(&mut self) {
        self.indent += 4;
    }

    fn decrement_indent(&mut self) {
        self.indent -= 4;
    }
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
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

    fn debug_walk<T>(&self, f: &mut T, mut indent: usize, cursor: &mut TreeCursor<'a>) -> Result<()>
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

    fn format_terminals(&self, node: Node<'a>, sep: &str) -> String {
        Terminals::new(node)
            .map(|node| self.text(node))
            .collect::<Vec<&str>>()
            .join(sep)
    }

    fn format_children(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        for child in node.children() {
            self.format_node(buffer, child)?;
        }

        Ok(())
    }

    fn format_node(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        debug!("format_node() kind:{}", node.kind());
        match node.kind() {
            "function_declaration" => self.format_function_declaration(buffer, node)?,
            "expression" => self.format_expression(buffer, node)?,
            "jump_statement" => self.format_jump_statement(buffer, node)?,
            "integer_atom_type" => {
                buffer.push_str(self.text(node));
                buffer.push_str(" ");
            }
            "simple_identifier" => buffer.push_str(self.text(node)),
            "list_of_arguments_parent" => self.format_list_of_arguments(buffer, node)?,
            "primary_literal" => buffer.push_str(self.text(node)),
            _ => self.format_children(buffer, node)?,
        }

        Ok(())
    }

    fn format_list_of_arguments(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        buffer.push_str("(");
        let children = node
            .children()
            .filter(|node| node.is_named())
            .identify_last();

        for (last, child) in children {
            self.format_node(buffer, child)?;

            if !last {
                buffer.push_str(", ");
            }
        }
        buffer.push_str(")");
        Ok(())
    }

    fn format_expression(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        if node.child_count() == 3 {
            // Binary expression
            let left = node.child(0).unwrap();
            let operator = node.child(1).unwrap();
            let right = node.child(2).unwrap();

            self.format_node(buffer, left)?;
            buffer.push_str(" ");
            buffer.push_str(self.text(operator));
            buffer.push_str(" ");
            self.format_node(buffer, right)?;
        } else {
            ensure!(node.child_count() == 1, InvalidCount);

            self.format_node(buffer, node.child(0).unwrap())?;
        }

        Ok(())
    }

    fn format_jump_statement(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        let jump_type = node.child(0).unwrap();

        buffer.push_str(self.text(jump_type));
        if node.child_count() == 3 {
            buffer.push_str(" ");
            let expression = node.child(1).unwrap();
            self.format_expression(buffer, expression)?;
        }

        Ok(())
    }

    fn format_function_declaration(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        ensure!(node.child_count() == 2, InvalidCount);

        let keyword = node.child(0).unwrap();
        let body = node.child(1).unwrap();

        ensure!(keyword.kind() == "function", InvalidKind);
        ensure!(body.kind() == "function_body_declaration", InvalidKind);

        buffer.push_str("function ");

        for child in body.children() {
            debug!("format_function_declaration() child:{}", child.kind());
            match child.kind() {
                "function_data_type_or_implicit1" => {
                    buffer.push_str(&self.format_terminals(child, " "));
                    buffer.push_str(" ");
                }
                "function_identifier" => {
                    buffer.push_str(&self.format_terminals(child, " "));
                }
                "tf_port_list" => {
                    &self.format_tf_port_list(buffer, child)?;
                    buffer.push_str("\n");
                }
                "function_statement_or_null" => {
                    buffer.increment_indent();
                    self.format_function_statement_or_null(buffer, child)?;
                    buffer.decrement_indent();
                }
                "comment" => {
                    buffer.push_str(self.text(child));
                    buffer.push_str("\n");
                }
                _ => {}
            }
        }

        buffer.push_str("endfunction\n\n");
        Ok(())
    }

    fn to_line_buffer<F>(&self, f: F, node: Node<'a>) -> Result<String>
    where
        F: Fn(&Self, &mut Buffer, Node<'a>) -> Result<()>,
    {
        let mut b = Buffer::with_capacity(1024);
        f(self, &mut b, node)?;
        Ok(b.to_string())
    }

    fn format_tf_port_list(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        let children = node
            .children()
            .filter(|child| child.is_named())
            .map(|child| self.to_line_buffer(Self::format_node, child))
            .collect::<Result<Vec<_>>>()?;

        let single_line = format!("({});", children.join(", "));

        if buffer.line_length + single_line.len() <= 80 {
            buffer.push_str(&single_line);
        } else {
            buffer.push_str("(\n");
            buffer.increment_indent();
            for (last, child) in children.iter().identify_last() {
                buffer.push_indent();
                buffer.push_str(&child);
                if !last {
                    buffer.push_str(",");
                }
                buffer.push_str("\n");
            }
            buffer.decrement_indent();
            buffer.push_str(");");
        }

        Ok(())
    }

    fn format_function_statement_or_null(
        &self,
        buffer: &mut Buffer,
        node: Node<'a>,
    ) -> Result<()> {
        ensure!(node.child_count() == 1, InvalidCount);

        buffer.push_indent();
        self.format_children(buffer, node)?;
        buffer.push_str(";\n");
        Ok(())
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

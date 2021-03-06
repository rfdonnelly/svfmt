use std::fmt;
use std::io;

use log::debug;
use snafu::{ensure, Backtrace, Snafu};
use tree_sitter::{Language, Node, Parser, Tree, TreeCursor};

include!(concat!(env!("OUT_DIR"), "/symbols.rs"));

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
    /// Holds the current content of the buffer.
    ///
    /// Clients use `push_str()` and `push()` to add content to the buffer.  Clients obtain the
    /// content of the buffer via its Display implementation.
    content: String,

    /// The current length of the current line.
    ///
    /// As content is pushed into the buffer, the line_length is incremented for every character
    /// added.  If a newline character is seen, the line length is reset to 0.
    line_length: usize,

    /// The current indent level in number of spaces.
    indent: usize,

    /// Indicates whether a blank line needs to be inserted in current indent.
    ///
    /// This gets reset anytime indentation changes and anytime a blank line is automatically
    /// inserted.  It gets set by maybe_blank_line().  Clients should call maybe_blank_line()
    /// at the end of a block.  This allows a blank line to be inserted between blocks in a given
    /// scope but prevents lines from being inserted before the first block and after the last block.
    insert_blank_line: bool,
}

impl Buffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            content: String::with_capacity(capacity),
            line_length: 0,
            indent: 0,
            insert_blank_line: false,
        }
    }

    /// Adds a string to the buffer one character at a time
    fn push_str(&mut self, s: &str) {
        for c in s.chars() {
            self.push(c);
        }
    }

    /// Adds a character to the buffer
    ///
    /// Updates line_length.  Adds indentation for new non-blank lines.
    fn push(&mut self, c: char) {
        if c == '\n' {
            self.line_length = 0;
        } else {
            self.line_length += 1;
        }

        if c != '\n' && self.content.ends_with('\n') {
            if self.insert_blank_line {
                self.content.push('\n');
                self.insert_blank_line = false;
            }
            self.push_indent();
        }

        self.content.push(c);
    }

    /// Adds the current indentation level to the buffer
    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.content.push(' ');
        }
    }

    fn increment_indent(&mut self) {
        self.indent += 4;
        self.insert_blank_line = false;
    }

    fn decrement_indent(&mut self) {
        self.indent -= 4;
        self.insert_blank_line = false;
    }

    fn maybe_blank_line(&mut self) {
        self.insert_blank_line = true;
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
                write!(f, ": '{}'", self.text(node))?;
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
        match Symbol::from(node.kind_id()) {
            Symbol::FunctionDeclaration => self.format_function_declaration(buffer, node)?,
            Symbol::ClassDeclaration => self.format_class_declaration(buffer, node)?,
            Symbol::Expression => self.format_expression(buffer, node)?,
            Symbol::JumpStatement => self.format_jump_statement(buffer, node)?,
            Symbol::OperatorAssignment => self.format_operator_assignment(buffer, node)?,
            Symbol::IntegerAtomType => {
                buffer.push_str(self.text(node));
                buffer.push_str(" ");
            }
            Symbol::SimpleIdentifier => buffer.push_str(self.text(node)),
            Symbol::ListOfArgumentsParent => self.format_list_of_arguments(buffer, node)?,
            Symbol::PrimaryLiteral => buffer.push_str(self.text(node)),
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

    fn format_operator_assignment(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        ensure!(node.child_count() == 3, InvalidCount);

        let lvalue = node.child(0).unwrap();
        let assignment_operator = node.child(1).unwrap();
        let expression = node.child(2).unwrap();

        buffer.push_str(self.text(lvalue));
        buffer.push(' ');
        buffer.push_str(self.text(assignment_operator));
        buffer.push(' ');
        self.format_expression(buffer, expression)?;

        Ok(())
    }

    fn format_class_declaration(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        let mut class_item_seen = false;

        buffer.push_str("class ");
        for child in node.children() {
            debug!("format_class_declaration() child:{}", child.kind());
            match Symbol::from(child.kind_id()) {
                Symbol::ClassIdentifier => {
                    buffer.push_str(&self.format_terminals(child, " "));
                }
                Symbol::ClassItem => {
                    if !class_item_seen {
                        buffer.push_str(";\n");
                        buffer.increment_indent();
                        class_item_seen = true;
                    }

                    self.format_node(buffer, child)?;
                }
                _ => {}
            }
        }

        buffer.decrement_indent();
        buffer.push_str("endclass\n");
        buffer.maybe_blank_line();
        Ok(())
    }

    fn format_function_declaration(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        ensure!(node.child_count() == 2, InvalidCount);

        let keyword = node.child(0).unwrap();
        let body = node.child(1).unwrap();

        ensure!(keyword.kind() == "function", InvalidKind);
        ensure!(Symbol::from(body.kind_id()) == Symbol::FunctionBodyDeclaration, InvalidKind);

        buffer.push_str("function ");

        for child in body.children() {
            debug!("format_function_declaration() child:{}", child.kind());
            match Symbol::from(child.kind_id()) {
                Symbol::FunctionDataTypeOrImplicit1 => {
                    buffer.push_str(&self.format_terminals(child, " "));
                    buffer.push_str(" ");
                }
                Symbol::FunctionIdentifier => {
                    buffer.push_str(&self.format_terminals(child, " "));
                }
                Symbol::TfPortList => {
                    &self.format_tf_port_list(buffer, child)?;
                    buffer.push_str("\n");
                }
                Symbol::FunctionStatementOrNull => {
                    buffer.increment_indent();
                    self.format_function_statement_or_null(buffer, child)?;
                    buffer.decrement_indent();
                }
                Symbol::Comment => {
                    buffer.increment_indent();
                    if self.blank_lines_after_previous_function_item(child) > 0 {
                        buffer.push('\n');
                    }
                    buffer.push_str(self.text(child));
                    buffer.push_str("\n");
                    buffer.decrement_indent();
                }
                _ => {}
            }
        }

        buffer.push_str("endfunction\n");
        buffer.maybe_blank_line();
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

    fn format_function_statement_or_null(&self, buffer: &mut Buffer, node: Node<'a>) -> Result<()> {
        ensure!(node.child_count() == 1, InvalidCount);

        if self.blank_lines_after_previous_function_item(node) > 0 {
            buffer.push('\n');
        }

        self.format_children(buffer, node)?;
        buffer.push_str(";\n");
        Ok(())
    }

    fn blank_lines_after_previous_function_item(&self, node: Node<'a>) -> usize {
        let prev = node.prev_sibling();

        if let Some(prev) = prev {
            // Need to check the previous symbol type because the grammar mixes function items with
            // function head items.
            match Symbol::from(prev.kind_id()) {
                Symbol::FunctionStatementOrNull
                | Symbol::Comment => {
                    let difference = node.start_position().row - prev.end_position().row;

                    if difference == 0 {
                        0
                    } else {
                        difference - 1
                    }
                }
                _ => 0,
            }
        } else {
            0
        }
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

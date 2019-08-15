use std::fmt;
use svfmt::*;

pub fn transform(source: &str) -> String {
    let tree = parse(unsafe { tree_sitter_verilog() }, source).unwrap();
    let mut s = Vec::new();
    format(&mut s, source, &tree).unwrap();
    String::from_utf8_lossy(&s).to_string()
}

/// Wrapper around string slice that makes debug output `{:?}` to print string same way as `{}`.
/// Used in different `assert*!` macros in combination with `pretty_assertions` crate to make
/// test failures to show nice diffs.
#[derive(PartialEq, Eq)]
#[doc(hidden)]
pub struct PrettyString<'a>(pub &'a str);

/// Make diff to display string as multi-line string
impl<'a> fmt::Debug for PrettyString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        pretty_assertions::assert_eq!(PrettyString($left), PrettyString($right));
    };
}

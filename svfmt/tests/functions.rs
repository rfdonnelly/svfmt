#[macro_use]
mod common;

mod functions {
    use indoc::indoc;
    use crate::common::*;

    #[test]
    fn functions() {
        let input = indoc!(
            "
            function int f(int long_name_a, int long_name_b, int long_name_c, int long_name_d);
            endfunction"
        );
        let expected = indoc!(
            "
            function int f(
                int long_name_a,
                int long_name_b,
                int long_name_c,
                int long_name_d
            );
            endfunction\n\n\n"
        );

        assert_eq!(&transform(input), expected);
    }
}

#[macro_use]
mod common;

mod functions {
    use indoc::indoc;
    use crate::common::*;

    #[test]
    fn wrap_at_81() {
        let input = indoc!(
            "
            function int wrap_at_81(int long_parameter_name_a, int long_parameter_name_b___);
            endfunction"
        );
        let expected = indoc!(
            "
            function int wrap_at_81(
                int long_parameter_name_a,
                int long_parameter_name_b___
            );
            endfunction\n\n\n"
        );

        assert_eq!(&transform(input), expected);
    }

    #[test]
    fn dont_wrap_at_80() {
        let input = indoc!(
            "
            function int dont_wrap_at_80(int parameter_a, int parameter_b, int parameter_c);
            endfunction"
        );
        let expected = indoc!(
            "
            function int dont_wrap_at_80(int parameter_a, int parameter_b, int parameter_c);
            endfunction\n\n\n"
        );

        assert_eq!(&transform(input), expected);
    }
}

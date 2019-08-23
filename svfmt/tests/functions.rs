#[macro_use]
mod common;

mod functions {
    use crate::common::*;
    use indoc::indoc;

    #[test]
    fn wrap_at_81() {
        init();

        let input = indoc!(
            "
            function int wrap_at_81(int long_parameter_name_a, int long_parameter_name_b___);
            endfunction
            "
        );
        let expected = indoc!(
            "
            function int wrap_at_81(
                int long_parameter_name_a,
                int long_parameter_name_b___
            );
            endfunction
            \n\n"
        );

        assert_eq!(&transform(input), expected);
    }

    #[test]
    fn dont_wrap_at_80() {
        init();

        let input = indoc!(
            "
            function int dont_wrap_at_80(int parameter_a, int parameter_b, int parameter_c);
            endfunction
            "
        );
        let expected = indoc!(
            "
            function int dont_wrap_at_80(int parameter_a, int parameter_b, int parameter_c);
            endfunction
            \n\n"
        );

        assert_eq!(&transform(input), expected);
    }
}

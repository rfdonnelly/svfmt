#[macro_use]
mod common;

mod expressions {
    use indoc::indoc;

    use crate::common::*;

    #[test]
    fn binary() {
        let input = indoc!(
            "
            function int  f ( int a , int b ) ;
                return(a+b* 2);
            endfunction"
        );
        let expected = indoc!(
            "
            function int f(int a, int b);
                return a + b * 2;
            endfunction\n\n\n"
        ); // FIXME remove trailing blank lines

        assert_eq!(&transform(input).unwrap(), expected);
    }

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

        assert_eq!(&transform(input).unwrap(), expected);
    }
}

mod common;

mod expressions {
    use indoc::indoc;

    use crate::common::*;

    #[test]
    fn binary() {
        let input = indoc!(
            "
            function f ( int a , int b ) ;
                return(a+b);
            endfunction"
        );
        let expected = indoc!(
            "
            function f(
                int a,
                int b
            );
                return a + b;
            endfunction\n\n\n"
        ); // FIXME remove trailing blank lines

        assert_eq!(&transform(input), expected);
    }
}

#[macro_use]
mod common;

mod expressions {
    use crate::common::*;
    use indoc::indoc;

    #[test]
    fn binary() {
        init();

        let input = indoc!(
            "
            function int  f ( int a , int b ) ;
                return(a+b* 2);
            endfunction
            "
        );
        let expected = indoc!(
            "
            function int f(int a, int b);
                return a + b * 2;
            endfunction
            "
        );

        assert_eq!(&transform(input), expected);
    }
}

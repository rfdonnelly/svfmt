#[macro_use]
mod common;

mod assignment {
    use crate::common::*;
    use indoc::indoc;

    #[test]
    fn operator_assignment() {
        init();

        let input = indoc!(
            "
            function int f(a);
                a  *=  2 ;
                a <<= 3;
            endfunction
            "
        );
        let expected = indoc!(
            "
            function int f(a);
                a *= 2;
                a <<= 3;
            endfunction
            "
        );

        assert_eq!(&transform(input), expected);
    }
}

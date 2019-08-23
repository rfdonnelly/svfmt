#[macro_use]
mod common;

mod classes {
    use crate::common::*;
    use indoc::indoc;

    #[test]
    fn basic() {
        init();

        let input = indoc!(
            "
            class myclass;

            function int f(int a);

            return a;

            endfunction

            endclass
            "
        );
        let expected = indoc!(
            "
            class myclass;
                function int f(int a);
                    return a;
                endfunction
            endclass
            "
        );

        assert_eq!(&transform(input), expected);
    }
}

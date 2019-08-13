# This script takes care of testing your crate

set -ex

# This is the "test phase", tweak it as you see fit
main() {
    (cd vendor/tree-sitter-verilog && npm install)

    local APP="cross"
    local ARGS="--target $TARGET"

    case $TARGET in
        # FIXME Build on host until https://github.com/rfdonnelly/svfmt/issues/1 is resolved
        x86_64-unknown-linux-gnu)
            APP=cargo
            ARGS=
            ;;
        *)
            APP="cross"
            ARGS="--target $TARGET"
            ;;
    esac

    $APP build $ARGS
    $APP build $ARGS --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    $APP test $ARGS --all
    $APP test $ARGS --all --release

    # $APP run $ARGS
    # $APP run $ARGS --release
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi

#!/bin/sh
# Optional Linux integration test; independent of the regular build and CTest.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
formatter=${1:-"$repo/target/release/hspfmt"}
case "$formatter" in
    /*) ;;
    *) formatter="$(pwd)/$formatter" ;;
esac
chsp=$(command -v "${CHSP:-chsp}")
hsp3cl=$(command -v "${HSP3CL:-hsp3cl}")
cc=$(command -v "${CC:-cc}")
: "${HSP_COMMON:?Set HSP_COMMON to the OpenHSP common directory}"
: "${CHSP_INCLUDE:?Set CHSP_INCLUDE to the OpenHSP root containing src/hsp3/hsp3struct.h}"
hsp_common=$(CDPATH= cd -- "$HSP_COMMON" && pwd)
chsp_include=$(CDPATH= cd -- "$CHSP_INCLUDE" && pwd)
case "$chsp" in
    /*) ;;
    *) chsp="$(pwd)/$chsp" ;;
esac
case "$hsp3cl" in
    /*) ;;
    *) hsp3cl="$(pwd)/$hsp3cl" ;;
esac
case "$cc" in
    /*) ;;
    *) cc="$(pwd)/$cc" ;;
esac
test_dir=$(mktemp -d)
trap 'echo "hspfmt cHSP integration artifacts: $test_dir" >&2' EXIT
printf '8\n3.00\n5\n' > "$test_dir/expected.out"

for target in plugin c; do
    mkdir "$test_dir/$target"
    sed "s/target=plugin/target=$target/" "$repo/test/chsp.chsp" > "$test_dir/$target/original.chsp"
    for mode in original default compact tabs preserve combined; do
        case "$mode" in
            original) set -- --roundtrip ;;
            default) set -- ;;
            compact) set -- --comma-spacing=compact --operator-spacing=compact ;;
            tabs) set -- --tabs --base-indent=1 ;;
            preserve) set -- --indent=preserve ;;
            combined) set -- --comment-style=c --block-comments=lines --condition-parens=add \
                --repeat-parens=add --operator-style=c --increment-style=c \
                --blank-lines-before-module=2 --blank-lines-before-defcfunc=1 --blank-lines-before-deffunc=1 ;;
        esac
        mode_dir="$test_dir/$target/$mode"
        mkdir "$mode_dir"
        "$formatter" --no-config "$@" "$test_dir/$target/original.chsp" > "$mode_dir/native.chsp"
        "$formatter" --no-config "$@" "$mode_dir/native.chsp" > "$mode_dir/again.chsp"
        cmp "$mode_dir/native.chsp" "$mode_dir/again.chsp"
        (
            cd "$mode_dir"
            "$chsp" -d -i -u --chsp-compile=none "--compath=$hsp_common/" native.chsp > compile.log
            case "$target" in
                plugin) set -- -DHSPLINUX -DHSP64 ;;
                c) set -- ;;
            esac
            "$cc" -std=c11 "-I$chsp_include" "$@" -shared -fPIC hspfmt_native.c -o hspfmt_native.so -lm
            LD_LIBRARY_PATH="$mode_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" "$hsp3cl" native.ax > runtime.log
            tr -d '\r' < runtime.log | sed '/^gpiod initalize failed\.$/d' > actual.out
            cmp "$test_dir/expected.out" actual.out
        )
        echo "cHSP $target $mode passed"
    done
done

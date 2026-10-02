#!/bin/sh
# Run from any directory. Requires a built hspfmt, GNU coreutils and OpenHSP.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
formatter=${1:-"$repo/build/hspfmt"}
case "$formatter" in
    /*) ;;
    *) formatter="$(pwd)/$formatter" ;;
esac
hspcmp=$(command -v "${HSPCMP:-hspcmp}")
hsp3cl=$(command -v "${HSP3CL:-hsp3cl}")
: "${HSP_COMMON:?Set HSP_COMMON to the OpenHSP common directory}"
hsp_common=$(CDPATH= cd -- "$HSP_COMMON" && pwd)
case "$hspcmp" in
    /*) ;;
    *) hspcmp="$(pwd)/$hspcmp" ;;
esac
case "$hsp3cl" in
    /*) ;;
    *) hsp3cl="$(pwd)/$hsp3cl" ;;
esac
test_dir=$(mktemp -d)
# Retain generated artifacts for inspection, including on failure.
trap 'echo "hspfmt integration artifacts: $test_dir" >&2' EXIT

"$formatter" --roundtrip "$repo/test/behavior.hsp" > "$test_dir/roundtrip.hsp"
cmp "$repo/test/behavior.hsp" "$test_dir/roundtrip.hsp"

modes='default compact short parens-add parens-remove comments-semicolon comments-c comments-block labels declarations operator-hsp operator-c increment-hsp increment-c operators-compact operators-hsp combined'
for mode in $modes; do
    case "$mode" in
        default) set -- ;;
        compact) set -- --compact-operators ;;
        short) set -- --short-if --hsp-prefixes ;;
        parens-add) set -- --condition-parens=add --repeat-parens=add ;;
        parens-remove) set -- --condition-parens=remove --repeat-parens=remove ;;
        comments-semicolon) set -- --comment-style=semicolon --block-comments=lines ;;
        comments-c) set -- --comment-style=c --block-comments=lines ;;
        comments-block) set -- --block-comments=block ;;
        labels) set -- --indent-labels ;;
        declarations) set -- --blank-lines-before-module=2 --blank-lines-before-deffunc=1 --blank-lines-before-defcfunc=0 ;;
        operator-hsp) set -- --operator-style=hsp ;;
        operator-c) set -- --operator-style=c ;;
        increment-hsp) set -- --increment-style=hsp ;;
        increment-c) set -- --increment-style=c ;;
        operators-compact) set -- --operator-style=hsp --increment-style=c --compact-operators ;;
        operators-hsp) set -- --operator-style=hsp --increment-style=hsp ;;
        combined) set -- --indent-labels --comment-style=semicolon --block-comments=lines --condition-parens=add --repeat-parens=add --short-if --operator-style=c --increment-style=c --blank-lines-before-module=2 --blank-lines-before-deffunc=1 --blank-lines-before-defcfunc=1 ;;
    esac
    "$formatter" "$@" "$repo/test/behavior.hsp" > "$test_dir/$mode.hsp"
    "$formatter" "$@" --check "$test_dir/$mode.hsp"
    "$formatter" "$@" "$test_dir/$mode.hsp" > "$test_dir/again.hsp"
    cmp "$test_dir/$mode.hsp" "$test_dir/again.hsp"
done

# In-place mode writes the same bytes as stdout mode and preserves permissions.
cp "$test_dir/roundtrip.hsp" "$test_dir/write.hsp"
chmod 640 "$test_dir/write.hsp"
"$formatter" --write "$test_dir/write.hsp" > "$test_dir/write.stdout"
test ! -s "$test_dir/write.stdout"
cmp "$test_dir/default.hsp" "$test_dir/write.hsp"
test "$(stat -c %a "$test_dir/write.hsp")" = 640
touch -t 200001010000 "$test_dir/write.hsp"
before=$(stat -c '%i:%Y' "$test_dir/write.hsp")
"$formatter" -w "$test_dir/write.hsp"
test "$(stat -c '%i:%Y' "$test_dir/write.hsp")" = "$before"
cp "$test_dir/roundtrip.hsp" "$test_dir/short-write.hsp"
"$formatter" -w --short-if --hsp-prefixes "$test_dir/short-write.hsp"
cmp "$test_dir/short.hsp" "$test_dir/short-write.hsp"

expect_error() {
    if "$formatter" "$@" > "$test_dir/error.stdout" 2> "$test_dir/error.stderr"; then
        echo 'expected exit code 2' >&2
        exit 1
    else
        test "$?" -eq 2
        test ! -s "$test_dir/error.stdout"
    fi
}
expect_error --write
expect_error --write -
expect_error --write --check "$test_dir/roundtrip.hsp"
expect_error --write --roundtrip "$test_dir/roundtrip.hsp"
expect_error --comment-style=unknown "$test_dir/roundtrip.hsp"
expect_error --comment-style=basic "$test_dir/roundtrip.hsp"
expect_error --operator-style=unknown "$test_dir/roundtrip.hsp"
expect_error --increment-style=unknown "$test_dir/roundtrip.hsp"
expect_error --block-comments=unknown "$test_dir/roundtrip.hsp"
expect_error --condition-parens=unknown "$test_dir/roundtrip.hsp"
expect_error --repeat-parens=unknown "$test_dir/roundtrip.hsp"
expect_error --blank-lines-before-module=17 "$test_dir/roundtrip.hsp"
expect_error --blank-lines-before-deffunc=-1 "$test_dir/roundtrip.hsp"
expect_error --blank-lines-before-defcfunc=invalid "$test_dir/roundtrip.hsp"
cp "$repo/test/invalid.hsp" "$test_dir/invalid.hsp"
expect_error --write "$test_dir/invalid.hsp"
cmp "$repo/test/invalid.hsp" "$test_dir/invalid.hsp"
ln -s "$test_dir/roundtrip.hsp" "$test_dir/symlink.hsp"
expect_error --write "$test_dir/symlink.hsp"
test -L "$test_dir/symlink.hsp"
ln "$test_dir/roundtrip.hsp" "$test_dir/hardlink.hsp"
expect_error --write "$test_dir/hardlink.hsp"
cmp "$repo/test/behavior.hsp" "$test_dir/roundtrip.hsp"
test -z "$(find "$test_dir" -name '.hspfmt-*' -print)"

if "$formatter" --check "$repo/test/behavior.hsp"; then
    echo 'expected --check to report unformatted input' >&2
    exit 1
else
    test "$?" -eq 1
fi
if "$formatter" --unknown > "$test_dir/error.stdout" 2> "$test_dir/error.stderr"; then
    exit 1
else
    test "$?" -eq 2
    test ! -s "$test_dir/error.stdout"
fi

cd "$test_dir"
for mode in roundtrip $modes; do
    "$hspcmp" -i -u "--compath=$hsp_common/" "-o$test_dir/$mode.ax" "$test_dir/$mode.hsp" > "$test_dir/$mode.compile"
    test -s "$test_dir/$mode.ax"
    "$hsp3cl" "$test_dir/$mode.ax" > "$test_dir/$mode.out"
    tail -n 1 "$test_dir/$mode.out" | grep -Fx 'hspfmt integration ok'
    cmp "$test_dir/roundtrip.out" "$test_dir/$mode.out"
done
echo 'CLI checks and all compiler/runtime comparisons passed'
# The caller can inspect the complete generated sources, bytecode, and outputs.

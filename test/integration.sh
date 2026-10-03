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

# Ambiguous spacing is preserved and reported on stderr, including in check/write.
printf 'foo*bar\n' > "$test_dir/ambiguous.hsp"
printf '%s:1: warning: ambiguous label or multiplication; preserving whitespace\nfoo*bar\n' \
    "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.expected-stderr"
"$formatter" "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
cmp "$test_dir/ambiguous.hsp" "$test_dir/ambiguous.out"
cmp "$test_dir/ambiguous.expected-stderr" "$test_dir/ambiguous.stderr"
"$formatter" --check "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
test ! -s "$test_dir/ambiguous.out"
cmp "$test_dir/ambiguous.expected-stderr" "$test_dir/ambiguous.stderr"
touch -t 200001010000 "$test_dir/ambiguous.hsp"
before=$(stat -c '%i:%Y' "$test_dir/ambiguous.hsp")
"$formatter" --write "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
test ! -s "$test_dir/ambiguous.out"
test "$(stat -c '%i:%Y' "$test_dir/ambiguous.hsp")" = "$before"
cmp "$test_dir/ambiguous.expected-stderr" "$test_dir/ambiguous.stderr"
"$formatter" - < "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
printf '<stdin>:1: warning: ambiguous label or multiplication; preserving whitespace\nfoo*bar\n' \
    > "$test_dir/ambiguous.expected-stdin-stderr"
cmp "$test_dir/ambiguous.hsp" "$test_dir/ambiguous.out"
cmp "$test_dir/ambiguous.expected-stdin-stderr" "$test_dir/ambiguous.stderr"
"$formatter" --roundtrip "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
cmp "$test_dir/ambiguous.hsp" "$test_dir/ambiguous.out"
test ! -s "$test_dir/ambiguous.stderr"
"$formatter" --compact-operators "$test_dir/ambiguous.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
cmp "$test_dir/ambiguous.hsp" "$test_dir/ambiguous.out"
test ! -s "$test_dir/ambiguous.stderr"
printf 'foo * bar\n' > "$test_dir/ambiguous-spaces.hsp"
"$formatter" --compact-operators "$test_dir/ambiguous-spaces.hsp" > "$test_dir/ambiguous.out" 2> "$test_dir/ambiguous.stderr"
cmp "$test_dir/ambiguous-spaces.hsp" "$test_dir/ambiguous.out"
printf '%s:1: warning: ambiguous label or multiplication; preserving whitespace\nfoo * bar\n' \
    "$test_dir/ambiguous-spaces.hsp" > "$test_dir/ambiguous.expected-stderr"
cmp "$test_dir/ambiguous.expected-stderr" "$test_dir/ambiguous.stderr"

modes='default compact sample-config compact-config structured-config spacing-compact short short-parens-remove parens-add parens-remove comments-semicolon comments-c comments-block labels declarations operator-hsp operator-c increment-hsp increment-c operators-compact operators-hsp combined'
for mode in $modes; do
    case "$mode" in
        default) set -- ;;
        compact) set -- --compact-operators ;;
        sample-config) set -- "--config=$repo/.hspfmt.example" ;;
        compact-config) set -- "--config=$repo/presets/compact.hspfmt" ;;
        structured-config) set -- "--config=$repo/presets/structured.hspfmt" ;;
        spacing-compact) set -- --operator-spacing=compact --comma-spacing=compact --colon-spacing=compact --comment-spacing=compact ;;
        short) set -- --short-if --hsp-prefixes ;;
        short-parens-remove) set -- --short-if --condition-parens=remove --repeat-parens=remove ;;
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
    "$formatter" "$@" "$repo/test/behavior.hsp" > "$test_dir/$mode.hsp" 2> "$test_dir/$mode.stderr"
    "$formatter" "$@" --check "$test_dir/$mode.hsp" 2> "$test_dir/$mode.check-stderr"
    "$formatter" "$@" "$test_dir/$mode.hsp" > "$test_dir/again.hsp" 2> "$test_dir/$mode.again-stderr"
    cmp "$test_dir/$mode.hsp" "$test_dir/again.hsp"
done

# Keep Japanese source bytes intact in both encodings, with the matching input mode.
for encoding in utf8 cp932; do
    "$formatter" --no-config "--encoding=$encoding" --roundtrip "$repo/test/japanese-$encoding.hsp" \
        > "$test_dir/japanese-$encoding-roundtrip.hsp"
    cmp "$repo/test/japanese-$encoding.hsp" "$test_dir/japanese-$encoding-roundtrip.hsp"
    for mode in default compact operators labels sample-config compact-config structured-config; do
        case "$mode" in
            default) set -- --no-config ;;
            compact) set -- --no-config --compact-operators ;;
            sample-config) set -- "--config=$repo/.hspfmt.example" ;;
            compact-config) set -- "--config=$repo/presets/compact.hspfmt" ;;
            structured-config) set -- "--config=$repo/presets/structured.hspfmt" ;;
            operators) set -- --no-config --operator-style=c --increment-style=c ;;
            labels) set -- --no-config --indent-labels ;;
        esac
        "$formatter" "--encoding=$encoding" "$@" "$repo/test/japanese-$encoding.hsp" \
            > "$test_dir/japanese-$encoding-$mode.hsp" 2> "$test_dir/japanese-$encoding-$mode.stderr"
        "$formatter" "--encoding=$encoding" "$@" "$test_dir/japanese-$encoding-$mode.hsp" \
            > "$test_dir/japanese-again.hsp" 2> "$test_dir/japanese-again.stderr"
        cmp "$test_dir/japanese-$encoding-$mode.hsp" "$test_dir/japanese-again.hsp"
    done
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
expect_error "$test_dir/roundtrip.hsp" "$test_dir/write.hsp"
expect_error --write - "$test_dir/roundtrip.hsp"
expect_error --stdin-filepath=custom.hsp "$test_dir/roundtrip.hsp"
expect_error --config=nonexistent.cfg "$test_dir/roundtrip.hsp"

# Test --stdin-filepath with stdin
printf 'foo*bar\n' | "$formatter" --stdin-filepath=virtual/path.hsp - > "$test_dir/stdin-path.out" 2> "$test_dir/stdin-path.stderr"
printf 'virtual/path.hsp:1: warning: ambiguous label or multiplication; preserving whitespace\nfoo*bar\n' > "$test_dir/stdin-path.expected-stderr"
cmp "$test_dir/stdin-path.expected-stderr" "$test_dir/stdin-path.stderr"

# Test multiple files with --check and --write
cp "$test_dir/roundtrip.hsp" "$test_dir/multi1.hsp"
cp "$test_dir/roundtrip.hsp" "$test_dir/multi2.hsp"
if "$formatter" --check "$test_dir/multi1.hsp" "$test_dir/multi2.hsp"; then
    echo 'expected --check to report unformatted multi-files' >&2
    exit 1
else
    test "$?" -eq 1
fi
"$formatter" --write "$test_dir/multi1.hsp" "$test_dir/multi2.hsp"
"$formatter" --check "$test_dir/multi1.hsp" "$test_dir/multi2.hsp"
cmp "$test_dir/default.hsp" "$test_dir/multi1.hsp"
cmp "$test_dir/default.hsp" "$test_dir/multi2.hsp"

# Test config file loading
printf 'indent=2\ntabs\n' > "$test_dir/test.cfg"
printf 'repeat\nx=1\nloop\n' | "$formatter" --config="$test_dir/test.cfg" - > "$test_dir/config.out"
printf 'repeat\n\tx = 1\nloop\n' > "$test_dir/config.expected"
cmp "$test_dir/config.expected" "$test_dir/config.out"
printf 'repeat\nx=1\nloop\n' | "$formatter" --config="$test_dir/test.cfg" --no-config - > "$test_dir/no-config.out"
printf 'repeat\n    x = 1\nloop\n' > "$test_dir/no-config.expected"
cmp "$test_dir/no-config.expected" "$test_dir/no-config.out"

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

for encoding in utf8 cp932; do
    case "$encoding" in
        utf8) set -- -i ;;
        cp932) set -- ;;
    esac
    for mode in roundtrip default compact operators labels sample-config compact-config structured-config; do
        "$hspcmp" "$@" -u "--compath=$hsp_common/" "-o$test_dir/japanese-$encoding-$mode.ax" \
            "$test_dir/japanese-$encoding-$mode.hsp" > "$test_dir/japanese-$encoding-$mode.compile"
        test -s "$test_dir/japanese-$encoding-$mode.ax"
        "$hsp3cl" "$test_dir/japanese-$encoding-$mode.ax" > "$test_dir/japanese-$encoding-$mode.out"
        tail -n 1 "$test_dir/japanese-$encoding-$mode.out" | grep -Fx 'japanese identifiers ok'
        cmp "$test_dir/japanese-$encoding-roundtrip.out" "$test_dir/japanese-$encoding-$mode.out"
    done
done
cmp "$test_dir/japanese-utf8-roundtrip.out" "$test_dir/japanese-cp932-roundtrip.out"

# Preserve the local Linux compiler behavior; normalization makes U+3000 whitespace.
printf 'mes 1\nend\n' > "$test_dir/full-space-baseline.hsp"
"$hspcmp" -i -u "--compath=$hsp_common/" "-o$test_dir/full-space-baseline.ax" \
    "$test_dir/full-space-baseline.hsp" > "$test_dir/full-space-baseline.compile"
"$hsp3cl" "$test_dir/full-space-baseline.ax" > "$test_dir/full-space-baseline.out"
for encoding in utf8 cp932; do
    case "$encoding" in
        utf8) set -- -i; full_space=$(printf '\343\200\200') ;;
        cp932) set -- ; full_space=$(printf '\201\100') ;;
    esac
    for placement in indent separator; do
        case "$placement" in
            indent) printf '%smes 1\nend\n' "$full_space" ;;
            separator) printf 'mes%s1\nend\n' "$full_space" ;;
        esac > "$test_dir/full-space-$encoding-$placement-original.hsp"
        "$formatter" --no-config "--encoding=$encoding" --full-width-spaces=preserve "$test_dir/full-space-$encoding-$placement-original.hsp" \
            > "$test_dir/full-space-$encoding-$placement-formatted.hsp"
        cmp "$test_dir/full-space-$encoding-$placement-original.hsp" "$test_dir/full-space-$encoding-$placement-formatted.hsp"
        for mode in original formatted; do
            if "$hspcmp" "$@" -u "--compath=$hsp_common/" "-o$test_dir/full-space-$encoding-$placement-$mode.ax" \
                    "$test_dir/full-space-$encoding-$placement-$mode.hsp" \
                    > "$test_dir/full-space-$encoding-$placement-$mode.compile"; then
                echo 'expected full-width space used as whitespace to be rejected' >&2
                exit 1
            fi
        done
        "$formatter" --no-config "--encoding=$encoding" --full-width-spaces=normalize \
            "$test_dir/full-space-$encoding-$placement-original.hsp" \
            > "$test_dir/full-space-$encoding-$placement-normalized.hsp"
        cmp "$test_dir/full-space-baseline.hsp" "$test_dir/full-space-$encoding-$placement-normalized.hsp"
        "$formatter" --no-config "--encoding=$encoding" --full-width-spaces=normalize \
            "$test_dir/full-space-$encoding-$placement-normalized.hsp" > "$test_dir/full-space-again.hsp"
        cmp "$test_dir/full-space-$encoding-$placement-normalized.hsp" "$test_dir/full-space-again.hsp"
        "$hspcmp" "$@" -u "--compath=$hsp_common/" "-o$test_dir/full-space-$encoding-$placement-normalized.ax" \
            "$test_dir/full-space-$encoding-$placement-normalized.hsp" \
            > "$test_dir/full-space-$encoding-$placement-normalized.compile"
        "$hsp3cl" "$test_dir/full-space-$encoding-$placement-normalized.ax" \
            > "$test_dir/full-space-$encoding-$placement-normalized.out"
        cmp "$test_dir/full-space-baseline.out" "$test_dir/full-space-$encoding-$placement-normalized.out"
    done
done
echo 'CLI checks and all compiler/runtime comparisons passed'
# The caller can inspect the complete generated sources, bytecode, and outputs.

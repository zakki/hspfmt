#!/bin/sh
# Run from any directory. Requires a built hspfmt, hspcmp and hsp3cl.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
formatter=${1:-"$repo/build/hspfmt/hspfmt"}
test_dir=$(mktemp -d)
# Retain generated artifacts for inspection, including on failure.
trap 'echo "hspfmt integration artifacts: $test_dir" >&2' EXIT

"$formatter" --roundtrip "$repo/test/test_hspfmt/behavior.hsp" > "$test_dir/roundtrip.hsp"
cmp "$repo/test/test_hspfmt/behavior.hsp" "$test_dir/roundtrip.hsp"

for mode in default compact short; do
    case "$mode" in
        default) set -- ;;
        compact) set -- --compact-operators ;;
        short) set -- --short-if --hsp-prefixes ;;
    esac
    "$formatter" "$@" "$repo/test/test_hspfmt/behavior.hsp" > "$test_dir/$mode.hsp"
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
cp "$repo/test/test_hspfmt/invalid.hsp" "$test_dir/invalid.hsp"
expect_error --write "$test_dir/invalid.hsp"
cmp "$repo/test/test_hspfmt/invalid.hsp" "$test_dir/invalid.hsp"
ln -s "$test_dir/roundtrip.hsp" "$test_dir/symlink.hsp"
expect_error --write "$test_dir/symlink.hsp"
test -L "$test_dir/symlink.hsp"
ln "$test_dir/roundtrip.hsp" "$test_dir/hardlink.hsp"
expect_error --write "$test_dir/hardlink.hsp"
cmp "$repo/test/test_hspfmt/behavior.hsp" "$test_dir/roundtrip.hsp"
test -z "$(find "$test_dir" -name '.hspfmt-*' -print)"

if "$formatter" --check "$repo/test/test_hspfmt/behavior.hsp"; then
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
for mode in roundtrip default compact short; do
    "$repo/hspcmp" -i -u "--compath=$repo/common/" "-o$test_dir/$mode.ax" "$test_dir/$mode.hsp" > "$test_dir/$mode.compile"
    test -s "$test_dir/$mode.ax"
    "$repo/hsp3cl" "$test_dir/$mode.ax" > "$test_dir/$mode.out"
    tail -n 1 "$test_dir/$mode.out" | grep -Fx 'hspfmt integration ok'
    cmp "$test_dir/roundtrip.out" "$test_dir/$mode.out"
done
echo 'CLI checks and four compiler/runtime comparisons passed'
# The caller can inspect the complete generated sources, bytecode, and outputs.

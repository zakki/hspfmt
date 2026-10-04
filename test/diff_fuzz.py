#!/usr/bin/env python3
"""Differential fuzzer comparing two hspfmt executables.

The baseline is usually a build of an earlier revision, e.g. the last C++
implementation (commit 28d57e4) or a previous Rust release. Stdout, stderr,
and exit status must match for random inputs and options.
"""
import os
import random
import string
import subprocess
import sys

def generate_random_source():
    # Mix of HSP tokens, keywords, symbols, whitespace, UTF-8, CP932 bytes, comments, and garbage
    keywords = [
        "mes", "if", "else", "repeat", "loop", "while", "wend", "for", "next",
        "switch", "case", "default", "swbreak", "swend",
        "#module", "#global", "#deffunc", "#defcfunc",
        "#chsp_module", "#chsp_module_end", "#chsp_func", "#chsp_cfunc", "#chsp_end",
        "#ifdef", "#ifndef", "#else", "#endif", "#define",
        "return", "goto", "gosub", "break", "continue",
    ]
    symbols = ["{", "}", "(", ")", ",", ":", "=", "+", "-", "*", "/", "\\", "==", "!=", "<=", ">=", "<<", ">>", "&&", "||", "/*", "*/", ";", "//"]
    numbers = ["0", "1", "42", "0x1a", "0b101", "$1a", "%101"]
    strings = ['"hello"', '"world\\n"', '{"multi\nline"}', '""']
    japanese_cp932 = [b"\x93\xfa\x96\x7b\x8c\xea", b"\x82\xa0\x82\xa2\x82\xa4", b"\x81\x40"] # 日本語, あいう, 全角空白
    japanese_utf8 = ["日本語".encode("utf-8"), "あいう".encode("utf-8"), "\u3000".encode("utf-8")]

    mode = random.choice(["structured", "tokens", "raw_bytes"])

    if mode == "raw_bytes":
        length = random.randint(0, 200)
        return bytes(random.randint(0, 255) for _ in range(length))

    parts = []
    num_tokens = random.randint(1, 30)
    for _ in range(num_tokens):
        r = random.random()
        if r < 0.3:
            parts.append(random.choice(keywords).encode("ascii"))
        elif r < 0.5:
            parts.append(random.choice(symbols).encode("ascii"))
        elif r < 0.65:
            parts.append(random.choice(numbers).encode("ascii"))
        elif r < 0.8:
            parts.append(random.choice(strings).encode("ascii"))
        elif r < 0.9:
            parts.append(random.choice(japanese_cp932 if random.random() < 0.5 else japanese_utf8))
        else:
            # random ascii word
            word = "".join(random.choices(string.ascii_letters + "_", k=random.randint(1, 8)))
            parts.append(word.encode("ascii"))

        # separator
        sep = random.choice([b" ", b"  ", b"\t", b":", b"\n", b"\r\n", b""])
        parts.append(sep)

    return b"".join(parts)

def generate_random_options():
    opts = []
    if random.random() < 0.2:
        opts.append("--tabs")
    if random.random() < 0.2:
        opts.append("--compact-operators")
    if random.random() < 0.3:
        opts.append(random.choice(["--operator-spacing=preserve", "--operator-spacing=space", "--operator-spacing=compact"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--comma-spacing=preserve", "--comma-spacing=space", "--comma-spacing=compact"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--colon-spacing=preserve", "--colon-spacing=space", "--colon-spacing=compact"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--comment-spacing=preserve", "--comment-spacing=space", "--comment-spacing=compact"]))
    if random.random() < 0.2:
        opts.append("--hsp-prefixes")
    if random.random() < 0.2:
        opts.append("--short-if")
    if random.random() < 0.3:
        opts.append(random.choice(["--full-width-spaces=preserve", "--full-width-spaces=normalize"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--operator-style=preserve", "--operator-style=hsp", "--operator-style=c"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--increment-style=preserve", "--increment-style=hsp", "--increment-style=c"]))
    if random.random() < 0.2:
        opts.append(random.choice(["--indent-labels", "--no-indent-labels"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--comment-style=preserve", "--comment-style=semicolon", "--comment-style=c"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--block-comments=preserve", "--block-comments=lines", "--block-comments=block"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--condition-parens=preserve", "--condition-parens=add", "--condition-parens=remove"]))
    if random.random() < 0.3:
        opts.append(random.choice(["--repeat-parens=preserve", "--repeat-parens=add", "--repeat-parens=remove"]))
    if random.random() < 0.3:
        opts.append(f"--indent={random.choice(['preserve', '0', '2', '4', '8'])}")
    if random.random() < 0.3:
        opts.append(f"--base-indent={random.randint(0, 4)}")
    if random.random() < 0.3:
        opts.append(f"--loop-indent={random.randint(0, 4)}")
    if random.random() < 0.3:
        opts.append(random.choice(["--encoding=auto", "--encoding=utf8", "--encoding=cp932"]))
    if random.random() < 0.1:
        opts.append("--roundtrip")
    return opts

def run_fuzz(baseline_bin, candidate_bin, iterations=2000):
    print(f"Running {iterations} differential fuzzing iterations between baseline and candidate...")
    for i in range(1, iterations + 1):
        source = generate_random_source()
        options = generate_random_options()

        # Run baseline
        p_base = subprocess.run(
            [baseline_bin, "--no-config"] + options,
            input=source,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        # Run candidate
        p_cand = subprocess.run(
            [candidate_bin, "--no-config"] + options,
            input=source,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        # Check returncode
        if p_base.returncode != p_cand.returncode:
            print(f"FAIL at iteration {i}: exit code mismatch!")
            print(f"Options: {options}")
            print(f"Baseline code: {p_base.returncode}, candidate code: {p_cand.returncode}")
            print(f"Baseline stderr:\n{p_base.stderr.decode('utf-8', errors='replace')}")
            print(f"Candidate stderr:\n{p_cand.stderr.decode('utf-8', errors='replace')}")
            return False

        # If success, check stdout
        if p_base.returncode == 0:
            if p_base.stdout != p_cand.stdout:
                print(f"FAIL at iteration {i}: stdout mismatch on success!")
                print(f"Options: {options}")
                print(f"Baseline stdout ({len(p_base.stdout)} bytes): {p_base.stdout!r}")
                print(f"Candidate stdout ({len(p_cand.stdout)} bytes): {p_cand.stdout!r}")
                return False

        # Check stderr
        if p_base.stderr != p_cand.stderr:
            print(f"FAIL at iteration {i}: stderr mismatch!")
            print(f"Options: {options}")
            print(f"Baseline stderr:\n{p_base.stderr.decode('utf-8', errors='replace')}")
            print(f"Candidate stderr:\n{p_cand.stderr.decode('utf-8', errors='replace')}")
            return False

        if i % 500 == 0:
            print(f"  {i}/{iterations} iterations passed...")

    print(f"All {iterations} differential fuzzing iterations PASSED!")
    return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: diff_fuzz.py BASELINE [CANDIDATE [ITERATIONS]]", file=sys.stderr)
        sys.exit(2)
    baseline_bin = sys.argv[1]
    candidate_bin = sys.argv[2] if len(sys.argv) > 2 else "target/release/hspfmt"
    iterations = int(sys.argv[3]) if len(sys.argv) > 3 else 3000
    if not run_fuzz(baseline_bin, candidate_bin, iterations):
        sys.exit(1)

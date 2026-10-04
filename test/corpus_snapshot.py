#!/usr/bin/env python3
"""Corpus snapshot tool for hspfmt.

Records or verifies SHA-256 hashes of formatted corpus sources
across multiple presets and option configurations.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

DEFAULT_PATTERNS = [
    "tmp/sample/**/*.*",
    "tmp/chsp/**/*.*",
    "hsp3-ginger/**/*.*",
]
VALID_EXTS = {".hsp", ".as", ".hsp3", ".chsp"}

PROFILES = {
    "default": ["--no-config"],
    "compact": ["--config=presets/compact.hspfmt"],
    "structured": ["--config=presets/structured.hspfmt"],
    "tabs": ["--no-config", "--tabs", "--base-indent=1"],
    "c_style": [
        "--no-config",
        "--comment-style=c",
        "--operator-style=c",
        "--increment-style=c",
        "--condition-parens=add",
    ],
    "roundtrip": ["--no-config", "--roundtrip"],
}


def find_corpus_files(repo_root, custom_paths=None):
    files = []
    if custom_paths:
        for p in custom_paths:
            path = Path(p)
            if path.is_file() and path.suffix.lower() in VALID_EXTS:
                files.append(path)
            elif path.is_dir():
                for sub in path.glob("**/*.*"):
                    if sub.is_file() and sub.suffix.lower() in VALID_EXTS:
                        files.append(sub)
    else:
        for pattern in DEFAULT_PATTERNS:
            for f in repo_root.glob(pattern):
                if f.is_file() and f.suffix.lower() in VALID_EXTS:
                    files.append(f)
    return sorted(set(files))


def hash_output(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run_formatter(formatter, args, file_path, cwd):
    cmd = [str(formatter)] + args + [str(file_path)]
    proc = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return proc.returncode, proc.stdout, proc.stderr


def record_snapshot(formatter, repo_root, files, output_path):
    print(f"Recording corpus snapshot for {len(files)} files...")
    snapshot = {
        "version": 1,
        "profiles": list(PROFILES.keys()),
        "files": {},
    }

    for idx, f in enumerate(files, 1):
        rel_path = f.relative_to(repo_root).as_posix()
        snapshot["files"][rel_path] = {}

        for profile_name, args in PROFILES.items():
            code, stdout, stderr = run_formatter(formatter, args, f.resolve(), repo_root)
            if code == 0:
                h = hash_output(stdout)
                snapshot["files"][rel_path][profile_name] = {"status": 0, "hash": h}
            else:
                snapshot["files"][rel_path][profile_name] = {"status": code}

        if idx % 50 == 0 or idx == len(files):
            print(f"Processed {idx}/{len(files)} files")

    with open(output_path, "w", encoding="utf-8") as out:
        json.dump(snapshot, out, indent=2, sort_keys=True)
    print(f"Snapshot written to {output_path}")


def verify_snapshot(formatter, repo_root, snapshot_path):
    print(f"Verifying corpus against snapshot: {snapshot_path}")
    with open(snapshot_path, "r", encoding="utf-8") as f:
        snapshot = json.load(f)

    mismatches = []
    total_checks = 0
    passed_checks = 0

    for rel_path, profiles in snapshot.get("files", {}).items():
        file_path = repo_root / rel_path
        if not file_path.is_file():
            print(f"Warning: corpus file not found locally: {rel_path} (skipping)")
            continue

        for profile_name, expected in profiles.items():
            if profile_name not in PROFILES:
                continue
            total_checks += 1
            args = PROFILES[profile_name]
            code, stdout, stderr = run_formatter(formatter, args, file_path.resolve(), repo_root)

            expected_status = expected.get("status", 0)
            if code != expected_status:
                mismatches.append(
                    f"{rel_path} [{profile_name}]: expected exit {expected_status}, got {code}"
                )
                continue

            if code == 0:
                h = hash_output(stdout)
                expected_hash = expected.get("hash")
                if h != expected_hash:
                    mismatches.append(
                        f"{rel_path} [{profile_name}]: hash mismatch (expected {expected_hash}, got {h})"
                    )
                    continue

            passed_checks += 1

    print(f"\nVerification summary: {passed_checks}/{total_checks} checks passed.")
    if mismatches:
        print(f"\nFound {len(mismatches)} mismatches:")
        for m in mismatches[:20]:
            print(f"  FAIL: {m}")
        if len(mismatches) > 20:
            print(f"  ... and {len(mismatches) - 20} more.")
        return False
    return True


def main():
    parser = argparse.ArgumentParser(description="Corpus snapshot tool for hspfmt")
    default_formatter = "target/release/hspfmt" if os.path.exists("target/release/hspfmt") else "target/debug/hspfmt"
    parser.add_argument("--formatter", default=default_formatter, help="Path to hspfmt binary")
    parser.add_argument("--snapshot", default="test/corpus_snapshot.json", help="Path to snapshot JSON file")
    parser.add_argument("--record", action="store_true", help="Record snapshot instead of verifying")
    parser.add_argument("corpus_paths", nargs="*", help="Optional corpus paths")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    formatter = Path(args.formatter)
    if not formatter.is_absolute():
        formatter = repo_root / formatter
    if not formatter.is_file():
        print(f"Error: formatter not found: {formatter}", file=sys.stderr)
        sys.exit(2)

    snapshot_path = Path(args.snapshot)
    if not snapshot_path.is_absolute():
        snapshot_path = repo_root / snapshot_path

    if args.record:
        files = find_corpus_files(repo_root, args.corpus_paths)
        if not files:
            print("No corpus files found to record.", file=sys.stderr)
            sys.exit(1)
        record_snapshot(formatter, repo_root, files, snapshot_path)
    else:
        if not snapshot_path.is_file():
            print(f"Error: snapshot file not found: {snapshot_path}", file=sys.stderr)
            sys.exit(2)
        ok = verify_snapshot(formatter, repo_root, snapshot_path)
        if not ok:
            sys.exit(1)


if __name__ == "__main__":
    main()

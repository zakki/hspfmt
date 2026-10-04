#!/usr/bin/env python3
"""Language-neutral golden test runner for hspfmt.

Runs golden cases located under test/cases/ (or specified directories)
against a given hspfmt executable.
"""

import argparse
import os
import subprocess
import sys
from pathlib import Path


def parse_args():
    parser = argparse.ArgumentParser(description="Run golden test cases for hspfmt.")
    parser.add_argument(
        "--formatter",
        default="target/release/hspfmt" if os.path.exists("target/release/hspfmt") else "target/debug/hspfmt",
        help="Path to the hspfmt executable (default: target/release/hspfmt or target/debug/hspfmt)",
    )
    parser.add_argument(
        "test_dirs",
        nargs="*",
        default=["test/cases"],
        help="Directories containing test cases (default: test/cases)",
    )
    parser.add_argument(
        "-k", "--filter",
        help="Only run tests whose path contains this substring",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Verbose output",
    )
    return parser.parse_args()


def find_cases(root_dirs, filter_str=None):
    cases = []
    for root in root_dirs:
        root_path = Path(root)
        if not root_path.exists():
            continue
        # If the directory itself contains input.hsp, it's a test case.
        if (root_path / "input.hsp").is_file():
            cases.append(root_path)
            continue
        # Otherwise, search recursively
        for input_file in sorted(root_path.glob("**/input.hsp")):
            case_dir = input_file.parent
            if filter_str and filter_str not in str(case_dir):
                continue
            cases.append(case_dir)
    return sorted(cases)


def run_case(formatter, case_dir, verbose=False):
    input_file = case_dir / "input.hsp"
    with open(input_file, "rb") as f:
        input_data = f.read()

    # Read args if present
    args_file = case_dir / "args"
    cli_args = []
    has_config_option = False
    use_stdin = (case_dir / "stdin").is_file()

    if args_file.is_file():
        with open(args_file, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                cli_args.append(line)
                if line == "--no-config" or line.startswith("--config"):
                    has_config_option = True
                if line == "-":
                    use_stdin = True

    # If config file exists in the directory, use it unless already specified
    if (case_dir / "config").is_file() and not has_config_option:
        cli_args.insert(0, "--config=config")
        has_config_option = True

    if not has_config_option:
        cli_args.insert(0, "--no-config")

    # Command line to run
    cmd = [str(formatter)] + cli_args
    if not use_stdin:
        cmd.append("input.hsp")

    # Determine expected status
    status_file = case_dir / "status"
    expected_err_file = case_dir / "expected.err"
    expected_out_file = case_dir / "expected.hsp"

    if status_file.is_file():
        expected_status = int(status_file.read_text().strip())
    elif expected_err_file.is_file() and not expected_out_file.is_file():
        expected_status = 2
    else:
        expected_status = 0

    expected_out = None
    if expected_out_file.is_file():
        with open(expected_out_file, "rb") as f:
            expected_out = f.read()

    expected_err = None
    if expected_err_file.is_file():
        with open(expected_err_file, "rb") as f:
            expected_err = f.read()

    # Run the process inside case_dir so relative paths (e.g. input.hsp, config) resolve
    try:
        proc = subprocess.run(
            cmd,
            cwd=case_dir,
            input=input_data if use_stdin else None,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except Exception as e:
        return False, f"Failed to execute {cmd}: {e}"

    # Check status code
    if proc.returncode != expected_status:
        return False, (
            f"Expected exit code {expected_status}, got {proc.returncode}.\n"
            f"Command: {' '.join(cmd)}\n"
            f"stdout:\n{proc.stdout.decode('utf-8', errors='replace')}\n"
            f"stderr:\n{proc.stderr.decode('utf-8', errors='replace')}"
        )

    # Check stdout
    if expected_out is not None:
        if proc.stdout != expected_out:
            return False, (
                f"stdout mismatch for case {case_dir}.\n"
                f"Expected ({len(expected_out)} bytes):\n"
                f"{expected_out.decode('utf-8', errors='replace')}\n"
                f"Actual ({len(proc.stdout)} bytes):\n"
                f"{proc.stdout.decode('utf-8', errors='replace')}"
            )

    # Check stderr
    if expected_err is not None:
        # Compare normalized newlines for stderr to tolerate Windows/POSIX CRLF difference in diagnostics
        actual_err_norm = proc.stderr.replace(b"\r\n", b"\n")
        expected_err_norm = expected_err.replace(b"\r\n", b"\n")
        if actual_err_norm != expected_err_norm:
            return False, (
                f"stderr mismatch for case {case_dir}.\n"
                f"Expected:\n{expected_err_norm.decode('utf-8', errors='replace')}\n"
                f"Actual:\n{actual_err_norm.decode('utf-8', errors='replace')}"
            )
    elif proc.stderr and expected_status == 0:
        # If expected_err was not provided but status is 0, stderr should normally be empty
        # unless it's an expected warning (in which case expected.err should have been provided).
        return False, (
            f"Unexpected stderr output for case {case_dir}:\n"
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )

    # Idempotency check: formatting formatted output must be identical
    if (
        expected_status == 0
        and expected_out is not None
        and not (case_dir / "no_idempotent").is_file()
    ):
        # Format the actual output again via stdin
        idemp_cmd = [str(formatter)] + cli_args + ["-"]
        try:
            idemp_proc = subprocess.run(
                idemp_cmd,
                cwd=case_dir,
                input=proc.stdout,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        except Exception as e:
            return False, f"Failed to execute idempotency check {idemp_cmd}: {e}"

        if idemp_proc.returncode != 0:
            return False, (
                f"Idempotency check failed with exit code {idemp_proc.returncode}.\n"
                f"stderr:\n{idemp_proc.stderr.decode('utf-8', errors='replace')}"
            )

        if idemp_proc.stdout != proc.stdout:
            return False, (
                f"Idempotency failed for case {case_dir}.\n"
                f"Re-formatted output differs from first formatting.\n"
                f"First formatting ({len(proc.stdout)} bytes):\n"
                f"{proc.stdout.decode('utf-8', errors='replace')}\n"
                f"Second formatting ({len(idemp_proc.stdout)} bytes):\n"
                f"{idemp_proc.stdout.decode('utf-8', errors='replace')}"
            )

    return True, None


def run_behavioral_tests(formatter, verbose=False):
    """Run CLI behavioral tests: atomic write, mtime preservation, dev/full, etc."""
    import tempfile
    import time

    failures = []

    with tempfile.TemporaryDirectory() as temp_dir:
        td = Path(temp_dir)

        # 1. --write formats every input before replacing any (atomic write)
        unformatted = td / "unformatted.hsp"
        unformatted.write_text("x=1\n")
        invalid = td / "invalid.hsp"
        invalid.write_text("repeat\n")

        proc = subprocess.run(
            [str(formatter), "--no-config", "--write", "unformatted.hsp", "invalid.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 2:
            failures.append(f"Atomic write: expected exit 2, got {proc.returncode}")
        if unformatted.read_text() != "x=1\n":
            failures.append("Atomic write: rejected input was modified!")

        # 2. --write does not write when content is unchanged (preserves mtime)
        clean = td / "clean.hsp"
        clean.write_text("x = 1\n")
        past_time = time.time() - 3600
        os.utime(clean, (past_time, past_time))
        mtime_before = clean.stat().st_mtime

        proc = subprocess.run(
            [str(formatter), "--no-config", "--write", "clean.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 0:
            failures.append(f"Clean write: expected exit 0, got {proc.returncode}")
        mtime_after = clean.stat().st_mtime
        if mtime_before != mtime_after:
            failures.append("Clean write: file was written even though content was unchanged!")

        # 3. --config with directory does not modify file on write
        config_dir = td / "config-dir"
        config_dir.mkdir()
        proc = subprocess.run(
            [str(formatter), "--config=config-dir", "--write", "unformatted.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 2:
            failures.append(f"Config dir: expected exit 2, got {proc.returncode}")
        if unformatted.read_text() != "x=1\n":
            failures.append("Config dir error changed input file!")

        # 4. /dev/full failure on Unix
        if os.path.exists("/dev/full"):
            proc = subprocess.run(
                f'"{formatter}" --no-config clean.hsp > /dev/full',
                shell=True,
                cwd=td,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if proc.returncode != 2:
                failures.append(f"/dev/full: expected exit 2, got {proc.returncode}")
            if b"output write failed" not in proc.stderr:
                failures.append(f"/dev/full: expected 'output write failed', got {proc.stderr}")
            proc = subprocess.run(
                f'"{formatter}" --help > /dev/full',
                shell=True,
                cwd=td,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if proc.returncode != 2 or proc.stderr != b"hspfmt: output write failed\n":
                failures.append(
                    f"--help to /dev/full: expected exit 2 and output write failure, "
                    f"got {proc.returncode}: {proc.stderr}"
                )

        # 5. Successful --write modifies file correctly
        to_modify = td / "to_modify.hsp"
        to_modify.write_text("a=1:b=2\n")
        proc = subprocess.run(
            [str(formatter), "--no-config", "--write", "to_modify.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 0:
            failures.append(f"Write modify: expected exit 0, got {proc.returncode}")
        if to_modify.read_text() != "a = 1 : b = 2\n":
            failures.append(f"Write modify: unexpected content {to_modify.read_text()}")

        # 6. Multiple files without --write or --check
        f1 = td / "f1.hsp"
        f2 = td / "f2.hsp"
        f1.write_text("x=1\n")
        f2.write_text("y=1\n")
        proc = subprocess.run(
            [str(formatter), "--no-config", "f1.hsp", "f2.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 2 or b"multiple files are only supported" not in proc.stderr:
            failures.append("Multiple files check failed")

        # 7. --roundtrip cannot be combined with multiple files
        proc = subprocess.run(
            [str(formatter), "--no-config", "--roundtrip", "--check", "f1.hsp", "f2.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 2 or b"--roundtrip cannot be combined with multiple files" not in proc.stderr:
            failures.append("--roundtrip multiple files check failed")

        # 8. --write rejects symbolic link
        target_file = td / "target.hsp"
        target_file.write_bytes(b"x=1\n")
        symlink_file = td / "symlink.hsp"
        try:
            symlink_file.symlink_to(target_file)
            proc = subprocess.run(
                [str(formatter), "--no-config", "--write", "symlink.hsp"],
                cwd=td,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if proc.returncode != 2 or b"requires a regular file without symbolic or hard links" not in proc.stderr:
                failures.append("Symlink write check failed")
        except OSError:
            pass  # Ignore on platforms where symlinks require privileges

        # 9. Reject hard links on both Unix and Windows, before any file is replaced.
        hardlink_file = td / "hardlink.hsp"
        os.link(target_file, hardlink_file)
        proc = subprocess.run(
            [str(formatter), "--no-config", "--write", "unformatted.hsp", "hardlink.hsp"],
            cwd=td,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if proc.returncode != 2 or b"requires a regular file without symbolic or hard links" not in proc.stderr:
            failures.append("Hardlink write check failed")
        if target_file.read_bytes() != b"x=1\n" or hardlink_file.read_bytes() != b"x=1\n":
            failures.append("Hardlink write changed input contents")
        if not os.path.samefile(target_file, hardlink_file):
            failures.append("Hardlink write broke the link relationship")
        if unformatted.read_text() != "x=1\n":
            failures.append("Hardlink rejection modified an earlier input")

        # 10. Paths that cannot be decoded as UTF-8 must survive CLI parsing on Unix.
        if os.name == "posix":
            raw_path = os.fsencode(td) + b"/\x82\xa0.hsp"
            with open(raw_path, "wb") as f:
                f.write(b"repeat\na=1\nloop\n")
            raw_config = os.fsencode(td) + b"/\x82\xa0.hspfmt"
            with open(raw_config, "wb") as f:
                f.write("; ソースの設定\nindent=2\n".encode("cp932"))
            expected = b"repeat\n  a = 1\nloop\n"
            for extra_args in [[], ["--"]]:
                proc = subprocess.run(
                    [str(formatter), b"--config=" + raw_config] + extra_args + [raw_path],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                if proc.returncode != 0 or proc.stdout != expected:
                    failures.append("Non-UTF8 input/config path formatting failed")
            proc = subprocess.run(
                [str(formatter), b"--config=" + raw_config, "--write", raw_path],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            with open(raw_path, "rb") as f:
                if proc.returncode != 0 or f.read() != expected:
                    failures.append("Non-UTF8 path write failed")
            proc = subprocess.run(
                [str(formatter), "--no-config", b"--stdin-filepath=" + raw_path],
                input=b"repeat\n",
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if proc.returncode != 2 or b"unterminated block, expected loop" not in proc.stderr:
                failures.append("Non-UTF8 stdin diagnostic path failed")

        # 11. --write must preserve Unix permissions and a file-specific Windows DACL.
        permission_file = td / "permissions.hsp"
        permission_file.write_bytes(b"x=1\n")
        if os.name == "posix":
            import stat

            permission_file.chmod(0o640)
            proc = subprocess.run(
                [str(formatter), "--no-config", "--write", str(permission_file)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if proc.returncode != 0 or stat.S_IMODE(permission_file.stat().st_mode) != 0o640:
                failures.append("Write did not preserve Unix permissions")
        elif os.name == "nt":
            acl_env = dict(os.environ, HSPFMT_TEST_ACL_PATH=str(permission_file))
            get_acl = """
$ErrorActionPreference = 'Stop'
(Get-Acl -LiteralPath $env:HSPFMT_TEST_ACL_PATH).GetSecurityDescriptorSddlForm(
    [System.Security.AccessControl.AccessControlSections]::Access)
"""
            set_acl = """
$ErrorActionPreference = 'Stop'
$acl = Get-Acl -LiteralPath $env:HSPFMT_TEST_ACL_PATH
$acl.SetAccessRuleProtection($true, $false)
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
$rule = [System.Security.AccessControl.FileSystemAccessRule]::new($user, 'FullControl', 'Allow')
$acl.SetAccessRule($rule)
Set-Acl -LiteralPath $env:HSPFMT_TEST_ACL_PATH -AclObject $acl
"""
            before = subprocess.run(
                ["powershell", "-NoProfile", "-Command", set_acl + get_acl],
                env=acl_env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if before.returncode != 0:
                failures.append(f"Windows ACL test setup failed: {before.stderr!r}")
            else:
                proc = subprocess.run(
                    [str(formatter), "--no-config", "--write", str(permission_file)],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                after = subprocess.run(
                    ["powershell", "-NoProfile", "-Command", get_acl],
                    env=acl_env,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                )
                if proc.returncode != 0 or permission_file.read_bytes() != b"x = 1\n":
                    failures.append(f"Windows ACL write failed: {proc.stderr!r}")
                if after.returncode != 0 or before.stdout != after.stdout:
                    failures.append("Write did not preserve the Windows DACL")

    return failures


def main():
    args = parse_args()
    formatter = Path(args.formatter).resolve()
    if not formatter.is_file():
        print(f"Error: formatter not found at {formatter}", file=sys.stderr)
        sys.exit(2)

    cases = find_cases(args.test_dirs, args.filter)
    if not cases:
        print("No test cases found.", file=sys.stderr)
        sys.exit(0)

    print(f"Found {len(cases)} test cases.")

    passed = 0
    failed = 0
    failures = []

    for case_dir in cases:
        ok, msg = run_case(formatter, case_dir, args.verbose)
        if ok:
            passed += 1
            if args.verbose:
                print(f"PASS: {case_dir}")
        else:
            failed += 1
            print(f"FAIL: {case_dir}")
            failures.append((str(case_dir), msg))

    # Run behavioral tests
    behavioral_failures = run_behavioral_tests(formatter, args.verbose)
    if behavioral_failures:
        for bf in behavioral_failures:
            failed += 1
            failures.append(("Behavioral test", bf))
    else:
        passed += 1
        print("PASS: CLI behavioral tests (atomic write, unchanged mtime, dev/full)")

    total = len(cases) + 1
    print(f"\nSummary: {passed} passed, {failed} failed out of {total} test items.")

    if failures:
        print("\n--- Failure Details ---")
        for case_dir, msg in failures:
            print(f"\n[{case_dir}]:")
            print(msg)
        sys.exit(1)

    sys.exit(0)


if __name__ == "__main__":
    main()

use std::process::Command;

#[test]
fn test_golden_cases() {
    let formatter = env!("CARGO_BIN_EXE_hspfmt");
    let status = Command::new("python3")
        .args(["test/run_tests.py", "--formatter", formatter])
        .status()
        .expect("failed to run python3 test/run_tests.py");
    assert!(status.success(), "golden test runner failed");
}

#[test]
fn test_corpus_invariants() {
    let corpus_bin = env!("CARGO_BIN_EXE_hspfmt_corpus");
    let status = Command::new(corpus_bin)
        .arg("test")
        .status()
        .expect("failed to run hspfmt_corpus");
    assert!(status.success(), "corpus invariance check failed");
}

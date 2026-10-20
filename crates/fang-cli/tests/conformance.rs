//! End-to-end conformance: every tests/conformance/*.fang runs through the
//! full pipeline and its stdout must match the .expected file exactly.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn conformance_suite() {
    let dir = repo_root().join("tests/conformance");
    let fang = env!("CARGO_BIN_EXE_fang");
    let mut run = 0;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "fang") {
            let expected_path = path.with_extension("expected");
            let expected = std::fs::read_to_string(&expected_path)
                .unwrap_or_else(|_| panic!("missing .expected for {:?}", path));
            let out = Command::new(fang)
                .arg("run")
                .arg(&path)
                .current_dir(repo_root())
                .output()
                .expect("run fang");
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            assert!(
                stderr.is_empty(),
                "{} failed to compile: {}",
                path.display(),
                stderr
            );
            assert!(
                out.status.success(),
                "{} exited with {:?}",
                path.display(),
                out.status.code()
            );
            if stdout != expected {
                failures.push(format!(
                    "{}:\n--- expected ---\n{}\n--- actual ---\n{}",
                    path.display(),
                    expected,
                    stdout
                ));
            }
            run += 1;
        }
    }
    assert!(
        run >= 8,
        "expected at least 8 conformance programs, ran {}",
        run
    );
    assert!(
        failures.is_empty(),
        "{} failing programs:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

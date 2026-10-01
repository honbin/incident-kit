//! Helpers for the toolbox's cross-tool integration tests. The tests live under
//! `tests/`; this lib only provides the plumbing to locate and run the real
//! binaries, shared by `tests/window_burst.rs` and `tests/incident_flow.rs`.
//!
//! Run via a whole-workspace `cargo test`, which builds the tool binaries. They
//! are located relative to this test executable (see `bin`), so any target
//! directory works; `bin` asserts if a binary is missing (e.g. after
//! `cargo test -p integration-tests` alone, which builds no other crate).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// Absolute path to a built tool binary.
///
/// Derived from this test executable's own location (`<target>/<profile>/deps/…`)
/// rather than a guessed path, so it follows `--target-dir` / `CARGO_TARGET_DIR`
/// and any profile directory.
pub fn bin(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("test executable path");
    dir.pop(); // drop the test binary's file name -> .../deps
    if dir.ends_with("deps") {
        dir.pop(); // .../deps -> .../<profile>
    }
    let path = dir.join(name);
    assert!(
        path.exists(),
        "{name} not built at {path:?}; run the whole-workspace `cargo test`"
    );
    path
}

/// Absolute path to a file under `tests/fixtures/`.
pub fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Feed `input` to tool `name` with `args` and capture its output.
pub fn run(name: &str, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(bin(name))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {name}: {e}"));
    // ignore BrokenPipe: the child may exit before reading stdin
    let _ = child.stdin.take().unwrap().write_all(input);
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {name}: {e}"))
}

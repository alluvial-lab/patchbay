//! Test-support: scope all `tempfile` usage to a cleanable workspace-local
//! root at binary load, regardless of how the test binary is invoked — direct
//! `cargo test`, CI, a git worktree, or the `scripts/test-rust` wrapper.
//!
//! This closes the leaked-`.tmp*`-fills-`/tmp` (ENOSPC) failure mode. The
//! `test-tempfile-hygiene` wrapper is opt-in (it scopes only when tests run
//! through it); linking this crate into a test binary runs the `#[ctor]` at
//! load — before any test — locating the workspace `target/` from the test
//! binary's path, ensuring `target/test-tmp` exists, and setting `TMPDIR` so
//! every `tempfile::NamedTempFile::new()` / `tempdir()` lands there instead of
//! the shared system `/tmp`.
//!
//! Linkage: add `patchbay-test-support` as a `[dev-dependencies]` entry and
//! reference it from each test binary — `extern crate patchbay_test_support;`
//! at the top of each integration test, or `#[cfg(test)] extern crate
//! patchbay_test_support;` at a library crate root for unit/inline tests.

#[ctor::ctor]
fn scope_test_tmpdir() {
    // Idempotent across re-entrant / multiple linked instances.
    if std::env::var_os("PATCHBAY_TEST_TMPDIR_SCOPED").is_some() {
        return;
    }
    std::env::set_var("PATCHBAY_TEST_TMPDIR_SCOPED", "1");

    // The test binary lives at `<workspace>/target/<profile>/deps/<bin>`;
    // walk up to the `target` dir. Works regardless of `CARGO_TARGET_DIR`
    // overrides (e.g. isolated worktree builds pointed elsewhere).
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(target_dir) = exe
        .ancestors()
        .find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
    else {
        return;
    };
    let test_tmp = target_dir.join("test-tmp");
    let _ = std::fs::create_dir_all(&test_tmp);
    std::env::set_var("TMPDIR", &test_tmp);
}

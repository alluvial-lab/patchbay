fn main() {
    // Ensure the workspace test-temp root exists so the [env] TMPDIR=target/test-tmp
    // (set in .cargo/config.toml) always resolves to an existing dir for every test
    // binary — wired or not, new or old. Prevents the leaked-.tmp*-fills-/tmp ENOSPC.
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        if let Some(target) = std::path::Path::new(&out_dir)
            .ancestors()
            .find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
        {
            let _ = std::fs::create_dir_all(target.join("test-tmp"));
        }
    }
}

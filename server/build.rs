use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    env::set_var("PROTOC", protoc);

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .extern_path(".patchbay", "::patchbay_contracts::patchbay")
        .compile_protos(
            &[
                "../contracts/proto/patchbay/admin.proto",
                "../contracts/proto/patchbay/control.proto",
                "../contracts/proto/patchbay/adapter_control.proto",
            ],
            &["../contracts/proto"],
        )?;

    println!("cargo:rerun-if-changed=../contracts/proto/patchbay/admin.proto");
    println!("cargo:rerun-if-changed=../contracts/proto/patchbay/control.proto");
    println!("cargo:rerun-if-changed=../contracts/proto/patchbay/adapter_control.proto");
    println!("cargo:rerun-if-changed=../contracts/proto/patchbay");

    // Ensure the workspace test-temp root exists so `.cargo/config.toml`
    // `[env] TMPDIR = target/test-tmp` always resolves to an existing dir for
    // every test binary (wired or not). Prevents the leaked-`.tmp*`-fills-`/tmp`
    // (ENOSPC) failure mode. Merged into the codegen build.rs (do not replace it).
    if let Ok(out_dir) = env::var("OUT_DIR") {
        if let Some(target) = std::path::Path::new(&out_dir)
            .ancestors()
            .find(|a| a.file_name().and_then(|n| n.to_str()) == Some("target"))
        {
            let _ = std::fs::create_dir_all(target.join("test-tmp"));
        }
    }

    Ok(())
}

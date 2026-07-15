use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Match the repo's protoc resolution: use the vendored protoc binary so the
    // build does not depend on a system protoc.
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    env::set_var("PROTOC", protoc);

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .out_dir(&out_dir)
        .build_server(true)
        .build_client(false)
        .compile_protos(&["../proto/spike.proto"], &["../proto"])?;

    // Re-run when the proto changes.
    println!("cargo:rerun-if-changed=../proto/spike.proto");
    Ok(())
}

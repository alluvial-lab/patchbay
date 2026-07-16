use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../proto/patchbay");
    println!("cargo:rerun-if-changed=../proto/buf.yaml");

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    env::set_var("PROTOC", protoc);

    let out_dir = PathBuf::from("src/gen/patchbay");
    fs::create_dir_all(&out_dir)?;

    let protos = [
        "../proto/patchbay/common.proto",
        "../proto/patchbay/operations.proto",
        "../proto/patchbay/observations.proto",
        "../proto/patchbay/elicitations.proto",
        "../proto/patchbay/sessions.proto",
        "../proto/patchbay/authority.proto",
        "../proto/patchbay/adapter.proto",
        "../proto/patchbay/control.proto",
        "../proto/patchbay/adapter_control.proto",
    ];

    let mut config = prost_build::Config::new();
    config.enum_attribute(
        ".patchbay.ObservationRequest.observation",
        "#[allow(clippy::large_enum_variant)]",
    );
    config
        .out_dir(out_dir)
        .compile_protos(&protos, &["../proto"])?;

    Ok(())
}

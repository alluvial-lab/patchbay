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
                "../contracts/proto/patchbay/control.proto",
                "../contracts/proto/patchbay/adapter_control.proto",
            ],
            &["../contracts/proto"],
        )?;

    println!("cargo:rerun-if-changed=../contracts/proto/patchbay/control.proto");
    println!("cargo:rerun-if-changed=../contracts/proto/patchbay/adapter_control.proto");
    println!("cargo:rerun-if-changed=../contracts/proto/patchbay");
    Ok(())
}

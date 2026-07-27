//! Single-generator enforcement: the committed bindings in `src/gen/` are
//! produced by `buf generate` (protoc-gen-prost, the generator of record —
//! see contracts/README.md). This build script intentionally does NOT
//! regenerate them: prost-build and protoc-gen-prost output diverge
//! cosmetically, and a build-time writer flipped the committed file between
//! two generators' byte layouts, defeating the CI drift check. Edit protos →
//! run `buf generate` from `contracts/` → commit. CI's check:drift enforces.
fn main() {
    println!("cargo:rerun-if-changed=../proto/patchbay");
    println!("cargo:rerun-if-changed=../proto/buf.yaml");
    println!("cargo:warning=patchbay-contracts: bindings come from `buf generate`; do not edit src/gen by hand");
}

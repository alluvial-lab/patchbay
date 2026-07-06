//! Generated Patchbay protocol contracts.
//!
//! The types in this crate are generated from `contracts/proto/patchbay/*.proto`.
//! Do not edit the generated files by hand; run `buf generate` from `contracts/`.

pub mod patchbay {
    include!("gen/patchbay/patchbay.rs");
}

pub use patchbay::*;

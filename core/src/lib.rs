//! Patchbay coordination core.
//!
//! The core owns the durable event log, operation acceptance, authority,
//! session registry, and snapshots. It is the single authoritative writer
//! to the durable log; control surfaces and adapters are clients.
//!
//! This crate is the v0.1.0 implementation of the coordination core defined
//! in `docs/ARCHITECTURE.md` § "v0.1.0 component slice".

#[cfg(test)]
extern crate patchbay_test_support;

pub mod acceptance;
pub mod audit;
pub mod adapter;
pub mod authority;
pub mod contract_validation;
pub mod diagnostics;
pub mod resource;
pub mod session;
pub mod security;
pub mod storage;
pub mod target;
pub mod time;

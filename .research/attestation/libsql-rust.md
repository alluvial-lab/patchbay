---
source_handle: libsql-rust
fetched: 2026-07-07
source_url: https://docs.rs/libsql/latest/libsql/
provenance: source-direct
---

# Attestation: libSQL Rust API docs

## Summary

The libSQL Rust API docs describe libSQL as an embeddable SQL database engine based on SQLite, a Rust wrapper around the SQLite C API, and a crate with local, remote, and replicated variants exposed through an async API. The fetched docs.rs latest page identified the documented crate as `libsql-0.9.30`.

## Key passages

1. From "libSQL API for Rust":

> libSQL is an embeddable SQL database engine based on SQLite.

2. From the same section:

> This Rust API is a batteries-included wrapper around the SQLite C API to support transparent replication while retaining compatibility with the SQLite ecosystem, such as the SQL dialect and extensions.

3. From `Builder` docs in the item list:

> A builder for Database. This struct can be used to build all variants of Database.

4. From `Connection` docs in the item list:

> A connection to some libsql database, this can be a remote one or a local one.

5. From the "Getting Started" example:

> let db = Builder::new_local(":memory:").build().await.unwrap();

6. From the remote example:

> let db = Builder::new_remote("libsql://my-remote-db.com".to_string(), "my-auth-token".to_string()).build().await.unwrap();

7. From the feature-flags section:

> core this includes the core C code that backs both the basic local database usage and embedded replica features.

8. From the feature-flags section:

> replication this feature flag includes the core feature flag and adds replication.

9. From the fetched docs.rs page metadata:

> libsql-0.9.30

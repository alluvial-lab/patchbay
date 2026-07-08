---
source_handle: rusqlite
fetched: 2026-07-07
source_url: https://docs.rs/rusqlite/latest/rusqlite/
provenance: source-direct
---

# Attestation: rusqlite Rust SQLite wrapper docs

## Summary

The rusqlite docs describe the crate as an ergonomic Rust wrapper for SQLite. Its API exposes connections, transactions, rollback-by-default transaction behavior, WAL hooks, busy handlers, and SQLite open flags. The fetched docs.rs latest page identified the documented crate as `rusqlite-0.40.1`.

## Key passages

1. From the crate description:

> Rusqlite is an ergonomic wrapper for using SQLite from Rust.

2. From `Connection`:

> A connection to a SQLite database.

3. From `Transaction`:

> Represents a transaction on a database connection.

4. From `Transaction` note:

> Transactions will roll back by default. Use commit method to explicitly commit the transaction, or use set_drop_behavior to change what happens when the transaction is dropped.

5. From `Transaction::new`:

> Begin a new transaction. Cannot be nested; see savepoint for nested transactions.

6. From `Connection::wal_hook`:

> Register a callback that is invoked each time data is committed to a database in wal mode.

7. From `Connection::busy_timeout`:

> Set a busy handler that sleeps for a specified amount of time when a table is locked.

8. From `OpenFlags`:

> The default open flags are SQLITE_OPEN_READ_WRITE SQLITE_OPEN_CREATE SQLITE_OPEN_URI SQLITE_OPEN_NO_MUTEX.

9. From `OpenFlags::SQLITE_OPEN_NO_MUTEX`:

> This is used by default, as proper Send / Sync usage (in particular, the fact that Connection does not implement Sync) ensures thread-safety without the need to perform locking around all calls.

10. From the fetched docs.rs page metadata:

> rusqlite-0.40.1

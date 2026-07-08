---
source_handle: sqlx-sqlite
fetched: 2026-07-07
source_url: https://docs.rs/sqlx/latest/sqlx/sqlite/
provenance: source-direct
---

# Attestation: SQLx SQLite driver docs

## Summary

The SQLx SQLite module docs describe the SQLite database driver, its `libsqlite3-sys` dependency behavior, default static linking, connection/options types, journal-mode and synchronous enums that defer meaning to SQLite documentation, and the documented crate version shown by docs.rs latest.

## Key passages

1. From the module description:

> SQLite database driver.

2. From "Note: libsqlite3-sys Version":

> This driver uses the libsqlite3-sys crate which links the native library for SQLite 3.

3. From the same section:

> As of SQLx 0.9.0, the version of libsqlite3-sys is now a range instead of any specific version.

4. From "Static Linking (Default)":

> The sqlite feature enables the bundled feature of libsqlite3-sys, which builds SQLite 3 from included source code and statically links it into the final binary.

5. From the same section:

> This version of SQLite is generally much newer than system-installed versions of SQLite (especially for LTS Linux distributions), and can be updated with a cargo update, so this is the recommended option for ease of use and keeping up-to-date.

6. From listed structs:

> SqliteConnectOptions — Options and flags which can be used to configure a SQLite connection.

7. From listed enums:

> SqliteJournalMode — Refer to SQLite documentation for the meaning of the database journaling mode.

8. From listed enums:

> SqliteSynchronous — Refer to SQLite documentation for the meaning of various synchronous settings.

9. From the fetched docs.rs page metadata:

> sqlx-0.9.0

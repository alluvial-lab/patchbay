---
source_handle: sqlite-wal
fetched: 2026-07-07
source_url: https://www.sqlite.org/wal.html
provenance: source-direct
---

# Attestation: SQLite write-ahead log documentation

## Summary

SQLite's WAL documentation describes WAL as an alternative journaling mode that appends changes to a separate WAL file, treats a commit as a special record appended to the WAL, supports concurrent readers with a single writer, and ties durability to synchronous/checkpoint behavior.

## Key passages

1. From "Overview":

> Beginning with version 3.7.0 (2010-07-21), a new "Write-Ahead Log" option (hereafter referred to as "WAL") is available.

2. From "How WAL Works":

> The WAL approach inverts this. The original content is preserved in the database file and the changes are appended into a separate WAL file. A COMMIT occurs when a special record indicating a commit is appended to the WAL.

3. From "Concurrency":

> When a read operation begins on a WAL-mode database, it first remembers the location of the last valid commit record in the WAL. Call this point the "end mark".

4. From "Concurrency":

> Writers merely append new content to the end of the WAL file. Because writers do nothing that would interfere with the actions of readers, writers and readers can run at the same time. However, since there is only one WAL file, there can only be one writer at a time.

5. From "Performance Considerations":

> Write transactions are very fast since they only involve writing the content once (versus twice for rollback-journal transactions) and because the writes are all sequential.

6. From "Performance Considerations":

> Writers sync the WAL on every transaction commit if PRAGMA synchronous is set to FULL but omit this sync if PRAGMA synchronous is set to NORMAL.

7. From "Performance Considerations":

> Checkpointing does require sync operations in order to avoid the possibility of database corruption following a power loss or hard reboot. The WAL must be synced to persistent storage prior to moving content from the WAL into the database and the database file must be synced prior to resetting the WAL.

8. From "Performance Considerations":

> The downside to this configuration is that transactions are no longer durable and might rollback following a power failure or hard reset.

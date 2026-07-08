---
source_handle: sled
fetched: 2026-07-07
source_url: https://docs.rs/sled/latest/sled/
provenance: source-direct
---

# Attestation: sled embedded database docs

## Summary

The sled docs describe sled as an embedded database with a BTreeMap-like API, thread-safe atomic operations, serializable transactions, watch subscriptions, recovery detection, and explicit flush semantics for crash recovery. The fetched docs.rs latest page identified the documented crate as `sled-0.34.7`.

## Key passages

1. From the crate description:

> sled is a high-performance embedded database with an API that is similar to a BTreeMap<[u8], [u8]>, but with several additional capabilities for assisting creators of stateful systems.

2. From the crate description:

> It is fully thread-safe, and all operations are atomic.

3. From the crate description:

> ACID transactions involving reads and writes to multiple items are supported with the Tree::transaction method.

4. From the crate description:

> Users may also subscribe to updates on individual Trees by using the Tree::watch_prefix method, which returns a blocking Iterator over updates to keys that begin with the provided prefix.

5. From `open`:

> You can use the Db::was_recovered method to determine if your database was recovered from a previous instance.

6. From `Db::was_recovered`:

> Note that database state is only guaranteed to be present up to the last call to flush! Otherwise state is synced to disk periodically if the sync_every_ms configuration option is set to Some(number_of_ms_between_syncs) or if the IO buffer gets filled to capacity before being rotated.

7. From `Db::generate_id`:

> Generate a monotonic ID. Not guaranteed to be contiguous.

8. From `Tree::flush` / `Db::flush`:

> Synchronously flushes all dirty IO buffers and calls fsync. If this succeeds, it is guaranteed that all previous writes will be recovered if the system crashes.

9. From `Tree::flush_async` / `Db::flush_async`:

> Asynchronously flushes all dirty IO buffers and calls fsync. If this succeeds, it is guaranteed that all previous writes will be recovered if the system crashes.

10. From `Tree::transaction`:

> Perform a multi-key serializable transaction.

11. From the fetched docs.rs page metadata:

> sled-0.34.7

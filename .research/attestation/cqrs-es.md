---
source_handle: cqrs-es
fetched: 2026-07-07
source_url: https://docs.rs/cqrs-es/latest/cqrs_es/
provenance: source-direct
---

# Attestation: cqrs-es event-sourcing framework docs

## Summary

The cqrs-es docs describe a CQRS/event-sourcing framework, event sourcing as using generated events as application state source of truth, supported backing stores, an in-memory local-testing store, and an `EventEnvelope` uniqueness model based on aggregate type, aggregate id, and per-aggregate sequence. The fetched docs.rs latest page identified the documented crate as `cqrs-es-0.5.0`.

## Key passages

1. From the crate description:

> A lightweight, opinionated CQRS and event sourcing framework targeting serverless architectures.

2. From the crate description:

> Command Query Responsibility Segregation (CQRS) is a pattern in Domain Driven Design that uses separate write and read models for application objects and interconnects them with events.

3. From the crate description:

> Event sourcing uses the generated events as the source of truth for the state of the application.

4. From the crate description:

> Three backing data stores are supported: PostgreSQL - postgres-es; MySQL - mysql-es; DynamoDb - dynamo-es.

5. From the crate description:

> Other data stores supported elsewhere: SQLite - sqlite-es.

6. From `mem_store`:

> An in-memory event store suitable for local testing.

7. From `EventEnvelope`:

> EventEnvelope is a data structure that encapsulates an event with its pertinent information. All of the associated data will be transported and persisted together and will be available for queries.

8. From `EventEnvelope`:

> Within any system an event must be unique based on the compound key composed of its: aggregate_type aggregate_id sequence.

9. From `EventEnvelope.sequence`:

> The sequence number for an aggregate instance.

10. From the fetched docs.rs page metadata:

> cqrs-es-0.5.0

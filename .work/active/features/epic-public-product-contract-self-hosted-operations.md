---
id: epic-public-product-contract-self-hosted-operations
kind: feature
stage: drafting
tags: [foundation, security]
parent: epic-public-product-contract
depends_on: [epic-public-product-contract-public-compatibility]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Reliable self-hosted operations

## Brief

Deliver the supported operational path that makes `v1.0.0` a reliable self-hosted product rather than source code plus installation hints. An independent operator must be able to install and secure the reference deployment, enroll and revoke operator and adapter access, migrate configuration and persisted state, upgrade and roll back within the declared policy, back up and restore, inspect health and diagnostics, and recover from crashes through tested procedures.

The feature owns one tested reference deployment path and the operational evidence for that path. It must preserve deployment neutrality by keeping domain semantics, persistence behavior, and authority rules behind their existing ports and generated contracts; it does not promise HA, federation, multiple storage backends, zero-downtime upgrades, or every packaging topology. It consumes the public compatibility feature's configuration, migration, CLI, and persisted-data promises rather than inventing parallel contracts.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: operational consumer of `epic-public-product-contract-public-compatibility`; its crash/recovery evidence is later consumed by executable release assurance.
- Full delivery may remain blocked until the coordination core, storage backend, packaging, and control surfaces exist; metadata or prose alone cannot satisfy this feature.

## Foundation references

- `docs/SPEC.md` — v1 supported deployment floor
- `docs/ARCHITECTURE.md` — deployment plane, persistence topology, crash recovery
- `docs/SECURITY.md` — deployment posture, enrollment, revocation, TLS, audit
- `docs/UX.md` — diagnostic CLI projections
- `docs/PROTOCOL.md` — persistence and recovery

---
id: epic-public-product-contract-public-compatibility
kind: feature
stage: drafting
tags: [foundation, protocol]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Public compatibility contract

## Brief

Turn the already-ratified version horizon into an enforceable public compatibility contract. An operator, script author, adapter author, or downstream integrator must be able to tell which surfaces are stable at `v1.0.0`, which remain private, how `0.x` breaking changes are announced and migrated, and what SemVer compatibility means for the adapter protocol, designated operator API, persisted-data migrations, documented configuration, and script-facing CLI.

This feature operationalizes the commitments already present in the foundation docs rather than rewriting the version vocabulary. It must reuse the existing Protobuf/Buf generated-contract machinery and identify the checks, compatibility categories, release notes, migration ceremonies, and public/private inventory needed to keep those promises honest. It does not deliver installation, backup/restore, or runtime operations; those belong to the sibling self-hosted-operations feature. It should coordinate with `.work/backlog/idea-proto-prose-registry-consistency-check.md` rather than silently duplicating that drift concern. It should also coordinate with `.work/backlog/idea-public-client-api-vs-split-deployment.md`: that parked finding distinguishes the public-client-API / auth-topology seam (multi-client on one host) from the split-deployment / network-reachable-core seam (multi-host, reserved). The public/private inventory and any "multi-host" language in the compatibility contract must name which seam it means, and must not lean on one for the other's outcome.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: contract foundation — self-hosting operations, adapter portability, and executable release assurance depend on its designated public boundaries.
- Inherits the epic decision that this is a full v1-readiness program, while leaving every child `release_binding` late-bound.

## Foundation references

- `docs/SPEC.md` — Versioned product horizon; v1 public compatibility contract
- `docs/ARCHITECTURE.md` — Versioned deployment horizon; boundary rules
- `docs/PROTOCOL.md` — canonical protocol registries and extension seams
- `docs/VERIFICATION.md` — artifact authority order and release-assurance vocabulary
- `contracts/README.md` — existing generated-contract and drift-check machinery

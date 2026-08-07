---
id: epic-token-commune-observer-adapter-foundation-credential-diagnostics
kind: story
stage: implementing
tags: [adapter, security, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-contract-foundation]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# Load the gateway credential and enforce diagnostic redaction

## Design checkpoint

Implement `GatewayCredential` from the regular 0600 file named by
`PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE`. Reject symlinks, non-regular files,
empty/multiline values, and group/world permission bits; use a read/stat
consistency check. The key is read once, applied only as an Authorization bearer
header, registered as an exact diagnostic secret, never stringified, and never
placed in config, resource payloads, Observations, audit, or forwarded
diagnostics. The ordinary environment does not accept the raw gateway key.

Port the Pi adapter's durable-diagnostic architecture into token-commune-owned
modules: bounded rotating 0600 JSONL, sink non-interference, exact/pattern
redaction, bounded queue/drop record, idempotent close, and bounded non-retrying
core forwarding. Use the one
`TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES` registry specified in the feature;
local/forwarded structures have no arbitrary message/body/header/path field.

## Acceptance evidence

- Permission, symlink, file-kind, empty, multiline, and race-oriented credential
  tests fail closed with fixed non-secret errors.
- Successful loading applies exactly `Authorization: Bearer <key>` and no
  `x-api-key`/query credential; disposal releases owned references.
- JSONL and forwarder tests inject the key, bearer header, attachment evidence,
  and file path into every accepted diagnostic string field and prove none is
  serialized or forwarded.
- Diagnostics open/write/rotate/flush/forward failures cannot alter credential,
  attach, delivery, or disposal results; no forwarding retry or recursive
  failure report occurs.
- The manifest derives all forwarded codes from the same registry used by the
  forwarder.

## Ordering constraint

Depends on the config/manifest registry. The gateway client consumes the opaque
credential interface and must not gain another key-reading path.

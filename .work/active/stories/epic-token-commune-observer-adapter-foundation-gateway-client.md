---
id: epic-token-commune-observer-adapter-foundation-gateway-client
kind: story
stage: implementing
tags: [adapter, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-credential-diagnostics]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# Implement the consumer-owned token-commune gateway client

## Design checkpoint

Implement `TokenCommuneGatewayClient` and its HTTP adapter exactly as designed in
the feature body. It owns typed, runtime-validated GET methods for:

- `/commune/status`;
- `/commune/pool`;
- `/commune/me`;
- `/commune/events` (annotated latest-50/no-cursor);
- `/commune/fingerprint` (safe Anthropic/Codex summaries only);
- `/v1/models`.

The port returns Patchbay-owned immutable DTOs and imports no token-commune
filesystem module. Every method rejects redirects, bounds the response body,
parses from `unknown`, preserves nullable multi-window capacity/draw data, and
uses the opaque credential to apply bearer auth. Error objects contain only a
fixed category, endpoint, and optional status; they retain no response body,
headers, URL credentials, or key.

Do not join `/status` contribution ids to `/pool` rows, invent stable ids,
normalize missing capacity to zero, close provider ids into a copied enum, or
rewrite model aliases.

## Acceptance evidence

- Fake-fetch interface tests prove exact method/path/Accept/auth behavior,
  redirect rejection, body-size bound, abort propagation, and all error
  categories.
- Runtime decoder fixtures cover all nullable CapacityReading/DrawReport fields,
  duplicate same-provider rows, health discriminants, event variants,
  fingerprints, and model metadata.
- Malformed/missing fields, invalid discriminants/timestamps/fractions,
  non-finite numbers, and invalid arrays fail before DTO return.
- Model ids including `gpt-5.5`, `gpt-5.3-codex-spark`,
  `claude-sonnet-4-5`, `token-commune/glm-5`, and
  `token-commune/kimi-for-coding` remain byte-for-byte unchanged; no `gpt-5.6`
  alias is created.
- Serialized errors and diagnostics contain no gateway key or response body.

## Ordering constraint

Depends on credential/redaction so HTTP has one authorization path. It produces
the stable port later mapping/polling consume, but starts no polling loop here.

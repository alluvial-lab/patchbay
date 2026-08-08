---
id: epic-token-commune-observer-cockpit-panel-cli-projection
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-verdict-synthesis]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
---

# CLI resource query and inspect projections

## Checkpoint

Add `resource-query` and `resource-inspect` over canonical resource/security snapshots, using the shared token-commune compositor for text-table and redacted JSON output. Extract the existing canonical resource identity parser rather than duplicating its percent-encoding grammar.

## Primary files

- `cli/src/commands/resources.ts`
- `cli/src/commands/token-commune-projection.ts`
- `cli/src/commands/diagnostics.ts`
- `cli/src/main.ts`
- `cli/src/output.ts`
- `cli/tests/resource-projection.test.ts`
- `cli/tests/output-diagnostics.test.ts`

## Acceptance evidence

- RESOURCE snapshot view/domain/discriminator/LSN framing and security snapshot domain are validated before projection.
- Query prints provider/draw/credentials/5h/verdict/freshness/models tables; inspect prints canonical wrapper before the same summary and derivation note.
- Exact identity parsing rejects partial, duplicate, mixed, empty, unknown, and malformed percent-encoded fields.
- Local query-grant filtering fails closed; core authorization remains authoritative.
- JSON uses safe timestamps/decimal strings/null and exposes no raw envelope, contribution/member identity, or credentials.
- The same canonical fixture yields the same signals/verdict as web because both invoke `@patchbay/operator-domain`.

## Ordering

Depends on verdict synthesis and may proceed beside cockpit integration under the same feature owner. Final honesty evidence depends on this checkpoint and the panel component.

## Implementation notes

- Added `resource-query` and `resource-inspect` over validated RESOURCE plus security snapshots. Both locally grant-filter exact pool/draw resources and invoke the shared `@patchbay/operator-domain` decoder/compositor; no persistence, raw envelopes, member labels, contribution keys, or credential material is exposed.
- Human output uses the required provider/draw/credentials/5h/verdict/freshness/models table. Inspect prints canonical identity/revision/completeness/freshness/time first; JSON uses decimal strings, RFC 3339/null, safe summaries, and the visible Patchbay derivation rule.
- Exported and reused diagnostics' strict percent-encoded canonical resource parser instead of introducing a second grammar. Empty authorized query results are explicit successful output.
- Verification: full `cli` build/test passed 42/42, including snapshot framing, exact grant filtering, wrong-adapter join isolation, safe text/JSON, inspect ordering, and malformed identity rejection.

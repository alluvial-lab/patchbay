---
id: epic-token-commune-observer-cockpit-panel-cli-projection
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-verdict-synthesis]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
---

# CLI resource query and inspect projections

## Checkpoint

Add `resource-query` and `resource-inspect` over the core-authorized canonical resource snapshot, using the shared token-commune compositor for text-table and redacted JSON output. The CLI must not require the authority-domain security inventory: `LoadSnapshot(RESOURCE)` filters records through the verified issuer's live `query` grants, including exact-resource grants. Extract the existing canonical resource identity parser rather than duplicating its percent-encoding grammar.

## Primary files

- `cli/src/commands/resources.ts`
- `cli/src/commands/token-commune-projection.ts`
- `cli/src/commands/diagnostics.ts`
- `cli/src/main.ts`
- `cli/src/output.ts`
- `cli/tests/resource-projection.test.ts`
- `cli/tests/output-diagnostics.test.ts`

## Acceptance evidence

- RESOURCE snapshot view/domain/discriminator/LSN framing is validated before projection; the real core filters unauthorized resources and view metadata before encoding.
- Query prints provider/draw/credentials/5h/verdict/freshness/models tables; inspect prints canonical wrapper before the same summary and derivation note.
- Exact identity parsing rejects partial, duplicate, mixed, empty, unknown, and malformed percent-encoded fields.
- Core query-grant filtering fails closed per resource; the CLI does not load the authority-domain security inventory or duplicate authority decisions.
- JSON uses safe timestamps/decimal strings/null and exposes no raw envelope, contribution/member identity, or credentials.
- The same canonical fixture yields the same signals/verdict as web because both invoke `@patchbay/operator-domain`.

## Ordering

Depends on verdict synthesis and may proceed beside cockpit integration under the same feature owner. Final honesty evidence depends on this checkpoint and the panel component.

## Implementation notes

- Added `resource-query` and `resource-inspect` over validated RESOURCE snapshots. Phase-8 pass-2 removed the security-snapshot dependency and moved exact pool/draw filtering to the real core's canonical grant checker; no persistence, raw envelopes, member labels, contribution keys, or credential material is exposed.
- Human output uses the required provider/draw/credentials/5h/verdict/freshness/models table. Inspect prints canonical identity/revision/completeness/freshness/time first; JSON uses decimal strings, RFC 3339/null, safe summaries, and the visible Patchbay derivation rule.
- Exported and reused diagnostics' strict percent-encoded canonical resource parser instead of introducing a second grammar. Empty authorized query results are explicit successful output.
- Original verification: full `cli` build/test passed 42/42, including snapshot framing, fake-client exact grant filtering, wrong-adapter join isolation, safe text/JSON, inspect ordering, and malformed identity rejection. Phase-8 pass-2 replaces the fake authority-boundary evidence with `cli/tests/real-core-resource-projection.mjs`: a real server denies `LoadSecuritySnapshot` yet serves the exact pool projection and withholds an ungranted draw resource.

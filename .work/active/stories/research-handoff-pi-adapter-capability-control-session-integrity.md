---
id: research-handoff-pi-adapter-capability-control-session-integrity
kind: story
stage: review
tags: [adapter, protocol, security, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Pi control handshake, session materialization, and tree integrity

## Checkpoint

Resolve review BLOCKERs 9–10 before the process supervisor can claim continuation. Supply a real challenged control-extension handshake for initialized cwd and current Pi session identity. Distinguish a declared-but-in-memory session from a materialized valid JSONL. Validate the entire persisted session tree because current Pi parsing skips malformed lines, overwrites duplicate ids in its index, and presents orphans as roots.

This checkpoint produces adapter-local proof/evidence. It does not register or promote a successor and never sends raw cwd/session paths to core diagnostics.

## Design

**Files**
- New `pi-adapter/extensions/patchbay-control.ts` — bounded handshake and reload-marker commands/custom-entry family.
- New `pi-adapter/src/control_handshake.ts` — command discovery, challenge generation, marker recovery, generic-RPC cross-check, and canonical cwd verification.
- New `pi-adapter/src/session_file.ts` — safe open, materialization classification, strict raw parser/tree validator, raw-vs-RPC equality, seal/prefix verification, and redacted error vocabulary.
- `contracts/proto/patchbay/pi_adapter.proto` — generated Pi control marker/profile and integrity-failure vocabulary where boundary carriage is needed.
- Focused control/session fixtures and property/mutation tests.

```ts
export interface PiControlHandshake {
  readonly challenge: string;
  readonly launchNonce: string;
  readonly extensionEpoch: string;
  readonly cwd: string;
  readonly sessionId: string;
  readonly sessionFile: string;
  readonly markerEntryId: string;
}

export interface MaterializedSessionSeal {
  readonly canonicalPath: string;
  readonly sessionRootId: string;
  readonly sessionId: string;
  readonly device: bigint;
  readonly inode: bigint;
  readonly size: bigint;
  readonly contentDigest: string;
  readonly treeDigest: string;
  readonly orderedEntryIds: readonly string[];
  readonly leafId: string;
}

export type PiSessionMaterialization =
  | { readonly kind: "memory_only"; readonly sessionId: string; readonly declaredPath: string }
  | { readonly kind: "materialized"; readonly seal: MaterializedSessionSeal }
  | { readonly kind: "invalid"; readonly failure: PiSessionIntegrityFailure };
```

Handshake protocol:

1. verify `patchbay-control-handshake` appears as the expected extension command/source;
2. submit `/patchbay-control-handshake <bounded-base64url-challenge>` using RPC `prompt`;
3. ignore prompt success as proof and fetch entries until the exact marker or bound expires;
4. require exact challenge + pre-journaled launch nonce + current extension epoch;
5. canonicalize and compare marker cwd to configured project cwd;
6. compare marker path/id to both `get_state` and `get_session_stats`;
7. retain raw values only in the 0600 local journal/store and return redacted failures elsewhere.

Strict session validation requires:

- `open`/`lstat`/`fstat` through a configured allowed root without symlink following; one regular file and stable physical identity during the read;
- non-empty bytes ending in LF; every non-empty line parses as an object;
- exactly one first header, supported current version, exact expected session id, bounded/canonical header fields;
- one closed supported entry family with every required base/type-specific field;
- non-empty unique entry ids; each non-null parent references an earlier entry; no self-parent, cycle, orphan, or second root;
- validity of label/compaction/branch and other entry references where present;
- exact deep equality between parsed entries and `get_entries()` plus exact leaf equality after the control marker;
- post-launch preservation of the sealed ordered entry prefix and tree edges; only validated startup/control entries may extend it.

`sessionFile` being non-empty, `SessionManager.isPersisted()`, a custom entry in memory, or Pi successfully loading the file proves none of the above.

## Acceptance evidence

- [x] A child launched in the wrong cwd cannot pass by returning the expected path/id through generic RPC.
- [x] Prompt `success:true`, a swallowed extension-handler error, wrong command source, stale challenge/launch nonce/epoch, or old-process marker never passes.
- [x] A fresh session with no assistant message classifies `memory_only` even when RPC reports a session path and in-memory custom entries.
- [x] The first persisted assistant response can transition the same verified Pi session to `materialized` only after full validation; no hidden prompt or invented flush is used.
- [x] Malformed interior JSON, duplicate ids, broken/forward/self parents, multiple roots, bad secondary references, unsupported version/type, truncation, symlink/root escape, inode swap, and raw-vs-RPC mismatch classify invalid.
- [x] `resumed` input proof requires a materialized pre-stop seal and valid post-launch sealed-prefix extension; header/inode/framing alone cannot pass.
- [x] Cwd/path values are absent from core/audit/diagnostic scan fixtures.
- [x] Mutations using `get_state` as cwd proof, treating `sessionFile` as existence, skipping malformed lines, overwriting duplicate ids, or promoting orphans fail.

## Ordering constraint

Consumes the shared logical/external identity and exact continuation payload shapes. The claim-aware supervisor consumes this proof; this story performs no launch/promotion itself.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; selected by the delegating autopilot for the security- and persistence-sensitive BLOCKER 9/10 unit.
- Review weight: `thorough` (caller override); implementation is left at `stage: review` for the independent convergence review requested by the delegating goal.
- Dispatch rationale: direct-read implementation. This worker is already a delegated child and did not attempt forbidden nested orchestration.
- Files changed: added `pi-adapter/extensions/patchbay-control.ts`, `pi-adapter/src/control_handshake.ts`, `pi-adapter/src/session_file.ts`, focused extension/handshake/session tests, and the prebuilt `tests/fixtures/session-valid.jsonl`; added `contracts/proto/patchbay/pi_adapter.proto`, regenerated committed Rust/TypeScript contracts with `cd contracts/ts && npm run gen`, exported the generated TypeScript module, and included extension sources in the adapter TypeScript build.
- Handshake mechanism: command discovery requires one exact `extension` source at the canonical adapter-owned entrypoint; each bounded random challenge must produce one current-leaf `patchbay.control.handshake.v1` custom entry with the pre-journaled launch nonce, initialized canonical `ctx.cwd`, current session id/path, and extension-instance epoch. Prompt success is ignored as proof. Marker id/path are cross-checked with both generic RPCs, cwd is compared only from the marker to the canonical configured project cwd, stale required/previous epochs fail, every RPC call is time-bounded, and all failures use generated redacted vocabulary without embedding local values.
- Extension mechanism: the handshake and bounded reload request/completion custom-entry family use generated marker shapes. Reload arguments are exact bounded base64url JSON over generated resource enums; malformed/stale/duplicate requests fail before marker/reload effect, and a new extension instance emits the matching completion on `session_start(reason=reload)`. The future reload controller still owns idle/materialized admission and success composition.
- Materialization/integrity mechanism: absent or zero-byte regular files are `memory_only`; existing candidates are opened with `O_NOFOLLOW`, checked through `/proc/self/fd` against a canonical allowed root, verified as one stable regular device/inode/size/mtime/ctime identity, and read with a size bound. The strict parser requires final LF, fatal UTF-8, every non-empty JSON object, one first current-v3 exact-id header, the closed current Pi entry family with required shapes, unique bounded ids, earlier-only parent/reference/tool-call links, and one root. Raw entries must deep-equal `get_entries()` and the RPC leaf must name the validated tree.
- Seal/resume mechanism: seals bind canonical path, root/session identity, device/inode, size, byte digest, tree-edge digest, ordered ids, and sealed leaf. Pre-launch verification requires exact equality. Post-launch verification requires the original byte/tree/id prefix, same physical/header identity, a bounded linear startup/control suffix from the sealed leaf, exact raw/RPC equality, and the challenged handshake marker as both last appended entry and current leaf. Cwd is never used as seal authority; the challenged handshake remains its proof.
- Tests added: 16 focused offline tests cover command/source/cwd/session correlation, extension request/completion behavior, deferred-JSONL `memory_only → materialized`, full framing/schema/tree/reference checks, symlink/root escape and deterministic inode-swap races, raw/RPC/leaf divergence, exact seal preservation, sealed-prefix continuation, and redacted failures. No ambient credentials, invented flush, or model call is used.
- Mutation evidence: all required mutants were killed and restored with `git restore`: wrong-cwd acceptance; stale challenge; stale launch nonce; stale extension epoch; marker-only acceptance; prompt-success-only acceptance; missing-file-as-materialized; malformed-line skipping; duplicate-id overwrite; orphan/forward/self-parent acceptance; symlink-following safe-open removal; allowed-root containment removal; and raw-vs-RPC comparison removal.
- Full verification: PASS — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; PASS — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`; PASS — `cd operator-domain && npm run build && npm test`; PASS — `cd pi-adapter && npm test` (54 tests).
- Simplification: one strict session-file module owns path opening, framing, schema/tree/reference validation, RPC equality, sealing, and prefix verification; one generated Pi vocabulary owns marker/profile/failure wire values. No alternate permissive parser, synthetic flush, or path-bearing diagnostic DTO was added.
- Discrepancies from design: the stable-open proof is intentionally POSIX/Linux (`O_NOFOLLOW`, device/inode, `/proc/self/fd`) because the committed seal shape and current deployment require those primitives; non-POSIX process/session fencing remains a reserved seam. Initial launch accepts a fresh well-formed extension epoch correlated by challenge + launch nonce; reload can additionally require a different prior epoch because Pi exposes no independent generic-RPC epoch oracle.
- Adjacent issues parked: none.

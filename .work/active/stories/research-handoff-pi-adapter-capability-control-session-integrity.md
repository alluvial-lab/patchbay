---
id: research-handoff-pi-adapter-capability-control-session-integrity
kind: story
stage: implementing
tags: [adapter, protocol, security, verification]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
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

- [ ] A child launched in the wrong cwd cannot pass by returning the expected path/id through generic RPC.
- [ ] Prompt `success:true`, a swallowed extension-handler error, wrong command source, stale challenge/launch nonce/epoch, or old-process marker never passes.
- [ ] A fresh session with no assistant message classifies `memory_only` even when RPC reports a session path and in-memory custom entries.
- [ ] The first persisted assistant response can transition the same verified Pi session to `materialized` only after full validation; no hidden prompt or invented flush is used.
- [ ] Malformed interior JSON, duplicate ids, broken/forward/self parents, multiple roots, bad secondary references, unsupported version/type, truncation, symlink/root escape, inode swap, and raw-vs-RPC mismatch classify invalid.
- [ ] `resumed` input proof requires a materialized pre-stop seal and valid post-launch sealed-prefix extension; header/inode/framing alone cannot pass.
- [ ] Cwd/path values are absent from core/audit/diagnostic scan fixtures.
- [ ] Mutations using `get_state` as cwd proof, treating `sessionFile` as existence, skipping malformed lines, overwriting duplicate ids, or promoting orphans fail.

## Ordering constraint

Consumes the shared logical/external identity and exact continuation payload shapes. The claim-aware supervisor consumes this proof; this story performs no launch/promotion itself.

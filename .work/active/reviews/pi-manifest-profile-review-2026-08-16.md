---
id: pi-manifest-profile-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-manifest-profile
created: 2026-08-16
updated: 2026-08-16
---

# Thorough review — opaque Pi runtime profile and conservative declaration gate

## Verdict

**MATERIAL** — return `research-handoff-pi-adapter-capability-manifest-profile` to `implementing`.

Commit `c084aa2` keeps the core profile envelope opaque, confines the new Pi vocabulary to the generated Pi contract, and leaves spawn, managed target shapes, session replacement, continuation proof, cursor support, generation fencing, and reconciliation inactive. The implementation itself passed direct inspection and the complete clean-tree suite. One material verification gap remains: the test claiming semantic-sentinel rejection never supplies a semantically decodable invalid profile, and removing the entire Pi semantic validator leaves both profile tests green.

Review mode: independent fresh-context story review, effective weight `thorough`, one rigorous pass, implementation range `ed5fbfa..c084aa2`.

## Findings

### Material

1. **The adapter-owned semantic-decoder oracle is vacuous** (`pi-adapter/src/core_client.ts:542,592-650`; `pi-adapter/tests/core_client.test.ts:126-139`). The production decoder does perform exact generated V1 checks, but the test named “semantic enum sentinels fail” exercises only a missing envelope, invalid protobuf wire bytes (`[0]`), and an empty payload. None reaches `validatePiRuntimeProfile`. A reviewer mutation deleting `validatePiRuntimeProfile(profile)` passed both focused profile tests unchanged, so the suite would not detect loss of every scalar enum, nested-message, exact-set, duplicate, and `UNSPECIFIED` check. That makes the story's claimed semantic-decoder evidence non-mutation-sensitive. **Concrete fix:** construct semantically decodable `PiRuntimeProfile` payload mutants with a representative scalar set to `UNSPECIFIED`, one required nested message absent, one repeated set containing a duplicate/unknown-or-sentinel member, and one required expected set member omitted; assert `decodePiRuntimeProfile` rejects each. Retain at least one mutation witness proving that bypassing the `validatePiRuntimeProfile` call fails the focused suite.

### Nits

None.

## Checklist disposition

- **Opaqueness — PASS.** `core/src/adapter/capability.rs:355-383` validates only profile presence framing, the inclusive 64 KiB maximum, bounded schema reference, and known non-sentinel content type. The deliberately invalid Pi payload passes both Attach and Replay in the generic core test. `core/src/diagnostics/mod.rs:789-801` projects only presence, schema reference, and content type; raw bytes are structurally absent. The 64 KiB static-registration judgment and schema-ref-carried version identity are recorded in the story and rolling protocol/glossary text.
- **Conservative declaration gate — PASS.** `pi-adapter/src/core_client.ts:475-491` has no managed target shape or `SPAWN`, sets session replacement false, keeps continuation/cursor/fence false and reconciliation `NONE`, and accepts no conformance evidence. The generated profile is carriage only; lifecycle conformance remains the positive activation owner.
- **Vocabulary placement — PASS.** Pi transport, event, materialization, cursor, control proof, cwd/project-trust/resource-root, reload, process-replacement, target-spec, and reconfigure values live in `contracts/proto/patchbay/pi_adapter.proto`. Generic adapter/diagnostics contracts contain only the opaque envelope and safe summary. Reusing the already-landed `PiReloadableResourceKind` avoids a duplicate resource registry.
- **Assurance consumption — PASS.** The Pi constructor uses the generated `AdapterAssuranceManifestV1` branch directly. Existing evidence-backed dedup/action values remain explicit; every uncertain lifecycle dimension is false/none.
- **Generated contracts and docs — PASS.** Rust and TypeScript generated drift is clean, and the protocol/architecture/Pi/glossary assertions match the implementation.
- **Mutation sensitivity — MATERIAL.** Opaqueness, vocabulary leakage, and premature activation mutants were killed. The fresh semantic-validator bypass survived, establishing finding 1.

## Mutation matrix

Every mutant was applied independently on the main tree, run through a focused oracle, and reverted with `git restore`. The tracked tree was clean after each restoration and before the full suite.

| Mutation / probe | Focused oracle | Result |
|---|---|---|
| Add core schema-specific `PiRuntimeProfile::decode` and reject malformed Pi bytes | `cargo test -p patchbay-core --test adapter_capability opaque_adapter_profile_validates_only_generic_bounded_framing` | **KILLED** — the deliberately invalid Pi payload stopped passing the generic opaque boundary; exit 101. |
| Leak a Pi cwd-shaped value into generic `supportedTargetSpecShapes` | focused `Pi manifest declares one complete conservative assurance V1` | **KILLED** — expected empty managed-shape list differed. |
| Prematurely activate `sessionReplacementSupport=true` | focused `Pi manifest declares one complete conservative assurance V1` | **KILLED** — expected conservative false differed. |
| Fresh probe: remove `validatePiRuntimeProfile(profile)` entirely | both focused Pi profile tests | **SURVIVED** — 2/2 passed, proving no current test reaches the semantic-invalid path. |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 registered mutation witnesses; generation drift and model traceability passed.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 28/28.
4. `cd pi-adapter && npm test`: **PASS** — 62/62, including the real-core loop.
5. `cd web-cockpit && npm test`: **PASS** — 144/144.
6. `cd cli && npm test`: **PASS** — 53/53 plus the real-core resource projection.
7. `cd token-commune-adapter && npm test`: **PASS** — 63/63, including both real-core flows.

`git diff --check` passed. The tree was clean before review mutations, after every restoration, and before writing this review. Initial root filesystem availability was 54 GiB; no temporary worktree was created.

## Recommendation

**Return to `implementing`.** Add semantically valid-but-invalid-profile regression cases that kill complete semantic-validator bypass, rerun the focused survivor and all seven clean-tree verification commands, then submit the story for the next `thorough` convergence pass.

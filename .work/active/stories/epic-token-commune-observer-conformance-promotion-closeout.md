---
id: epic-token-commune-observer-conformance-promotion-closeout
kind: story
stage: done
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Promote exact evidence and close through the verification deep lane

## Checkpoint

Promote the seven token-commune vectors only after phase-1 completeness,
real-core E2E, and phase-2 adversarial evidence all execute and every declared
mutation is exactly reported killed. Regenerate the existing conformance
traceability and implementation-evidence prose from the shared profile; record
paths/property ids/mutation ids without hand-maintained totals.

Run the project `[verification]` deep lane for every child and the integrated
feature at effective weight `thorough`: completeness convergence first,
adversarial convergence second. Reviewers attack vector field consumption,
reference-oracle independence, runner/count drift, key sink coverage, lost
terminalization, stale rendering, and surviving mutations. Findings are
proposals; the receiver verifies and dispositions each one.

## Primary files

- all seven `contracts/vectors/token-commune-*.json` files
- `contracts/scripts/check-vectors.mjs`
- `docs/VERIFICATION.md`
- all package runners/oracles/E2E tests from prior checkpoints

## Acceptance evidence

- The exact seven property/vector pairs have production scenario execution and
  exact mutation-kill reports; missing/unexpected evidence fails closed.
- Full adapter/core/server/operator-domain/web/contracts verification, clippy,
  generated drift/presentation/model checks, and `git diff --check` pass without
  skip, retry masking, weakened expectation, or hard-coded success.
- Assurance language is limited to promoted vector + implementation-checked;
  no model-checked, checked-normative, cross-adapter portability, or
  release-verified claim is made.
- Completeness and adversarial review phases each converge to no
  receiver-confirmed material current-cycle blocker before the feature advances.

## Ordering constraint

Final checkpoint. Depends on all completeness, E2E, security, failure, and
presentation evidence.

## Implementation notes

- Promoted the exact profile together after every registered package scenario and declared mutation witness executed. Generated conformance/model traceability classifies the token-commune properties as stated-normative implementation evidence only.
- Final corpus, promotion, implementation-check, proto-reference, and mutation totals are checker-derived rather than retained as prose assertions.
- Fail-closed guard checks transiently removed the profile vector, renamed its property and scenario, and removed a mutation declaration. Every variant failed before runner execution and left `docs/VERIFICATION.md` byte-identical.
- Four explicit self-mutation checks flipped independent expected outcomes for PARTIAL completeness, current-generation acceptance, gateway-key absence, and stale presentation. Each vector/property check failed, traceability remained byte-identical, and the original artifact was restored.
- Integrated verification: `check:vectors`, `check:drift`, `check:presentation`, and `check:models` passed; `cargo test --workspace` passed 345 listed Rust tests including doctests; clippy passed with warnings denied; token-commune adapter passed 60/60; operator-domain passed 9/9; web cockpit passed 114/114; `git diff --check` passed.
- Real-core result: local HTTP gateway → 0600 key loader → adapter → Rust core/SQLite → PARTIAL snapshot → overlap repair → missed poll → disconnect/stale → generation-2 reconnect/latest-50 gap/listed recovery → stale-generation/cross-owner rejection → redaction scans and real cockpit projection all passed.
- Assurance remains bounded: promoted vector + implementation-checked, not model-checked, checked-normative, cross-adapter portability proof, or release-verified.
- Review boundary: effective weight is `thorough` from the explicit caller. Per caller instruction this implementation worker stops after the feature advances to `review`; the separate deep-lane reviewer owns completeness→adversarial convergence.

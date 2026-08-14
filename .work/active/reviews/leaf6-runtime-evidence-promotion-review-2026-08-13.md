---
id: leaf6-runtime-evidence-promotion-review-2026-08-13
kind: story
stage: done
tags: [review, design, spawn]
parent: research-handoff-spawn-runtime-evidence-promotion-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-13
updated: 2026-08-13
---

# Deep review — runtime-evidence-promotion-contract (Leaf 6), 2026-08-13

Independent fresh-context `openai-codex/gpt-5.6-sol` (xhigh) review of commit `eee06b2` (`stage: review`). **Verdict: BLOCKER (6).** The contract types, the dedicated atomic SQLite promotion append, and the focused tests are sound in isolation, but **Leaf 6 is not wired into the production paths** — the old raw-Observation / `SessionRegistered` / `SessionGenerationBumped` routes still exist and bypass the new machinery. Physical transaction atomicity is confirmed sound; the gaps are integration + boundary-exclusivity + semantic-validity, not re-architecture. The next pass must integrate, not redesign.

## BLOCKERs

### 1. `ClaimedSuccessor` is not authenticated against the current durable attachment; has no production call site
`core/src/session/runtime_evidence.rs:95-126,140-153`. `source_matches` compares adapter id/generation only; never validates `attachment_event_id` against `AdapterRegistry`/durable prefix. The `Current` branch can return `Current` even when `source_matches` is false. The classifier has NO production call site — managed reports still enter `session::ingest_session_report` via `server/src/adapter_service.rs:943`, which directly emits `SessionRegistered`/`SessionGenerationBumped` (`core/src/session/ingest.rs:105-188`). Test at `core/tests/runtime_evidence_promotion.rs:137-143,308-336` uses LSN 1 (the SpawnClaim event) as the attachment id and still gets `ClaimedSuccessor`.
**Fix:** validate adapter id + adapter generation + exact attachment event id against the current authenticated `AdapterRegistry`/durable prefix, for BOTH `Current` and `ClaimedSuccessor`. Route managed `spawn_origin` reports exclusively through classifier → `SpawnSuccessorEvidenceStaged`, never ordinary registration/generation-bump.

### 2. The forbidden raw stale-Observation replay shape REMAINS (BLOCKER 5 not yet resolved)
`core/src/acceptance/observation.rs:84-89,141-161`. Terminal stale candidates are still encoded `STORED_EVENT_KIND_OBSERVATION` and passed to `append_decision`; production storage places the raw source BEFORE its audit, so normal replay sees and dispatches the Observation first. A regression test ASSERTS this legacy shape (`core/tests/acceptance_observation.rs:563-581`).
**Fix:** route EVERY runtime-targeted stale/unknown/mismatched admitted family through `QuarantinedRuntimeEvidence`; remove raw stale Observation persistence; flip the legacy-asserting test; add hot-fold + replay tests proving no normal projection consumes the nested candidate.

### 3. The quarantine boundary admits malformed/untyped payloads
`core/src/storage/audited.rs:420-427`, `core/src/storage/rusqlite.rs:1189-1207`, `core/src/session/runtime_evidence.rs:256-308`. `AuditedStorage::append_audited` accepts the quarantine kind without decoding/validating; a probe committed `kind=QuarantinedRuntimeEvidence, payload=[0xff]` → Ok. Even the typed validator checks only candidate presence/disposition/reason/syntactic-attachment, not reason↔disposition↔candidate↔target consistency or a real durable attachment.
**Fix:** add a dedicated typed quarantine append (or centrally validate/decode this special kind on every storage path); reject it from generic append/batch/dedup/audited routes; require the canonical stale-audit framing and validate candidate + classification context + reason + durable attachment together.

### 4. Promotion can mint descendant authority for the wrong subject
`core/src/authority/registry.rs:312-402`, `core/src/session/runtime_evidence.rs:418-449`. Promotion validates the parent grant against the accepted sender but, after extracting the descendant, checks only the promoted target — never requires descendant subject actor = spawner/accepted actor, descendant endpoint/class = spawning endpoint, or that the accepted spawning Grant actually permits `spawn` against the accepted adapter target. `validate_spawn_promotion_envelope` checks only audit/domain/provenance equality. → authority-laundering: a promotion can embed a canonical descendant Grant for ANOTHER actor and install it.
**Fix:** bind descendant subject actor + endpoint/class + target + canonical allowed-kind set + deterministic grant id + timestamps + both provenance Grants to the EXACT accepted Operation; revalidate parent grant kind/scope + continuation authority before insertion; add one-dimension-at-a-time authority-laundering mutation tests.

### 5. Promotion storage accepts unreplayable promotions + has a generic audited bypass
`core/src/storage/rusqlite.rs:1391-1521,2100-2112`. The storage test's promotion fixture (descendant id `"descendant"`, only `OperationKind::Instruct`) is NOT replayable by the authority fold (which requires deterministic id `desc:authority-main:spawn-a` + all 8 canonical session kinds) — so the "complete promotion" test isn't a real promotion. Separately, `RusqliteStorage::append_audited` does NOT apply `reject_generic_unaudited_special`; a probe committed `SpawnPromotionCommitted::default()` through the generic method, bypassing promotion stamping + grant-identity reservation.
**Fix:** reject promotion from EVERY generic backend route (append, append_dedup, append_audited, batch) — only the dedicated append path; validate the promotion is replayable by all 4 projections BEFORE committing; replace the invalid test fixture with a genuinely authority-valid promotion.

### 6. The ordered authority→session→claim→command fold is not on a real replay path
`core/src/session/runtime_evidence.rs:51-78`, `server/src/state.rs:608-634`. `fold_spawn_promotion_ordered` has NO call sites beyond its export. Server aggregate replay folds authority → targets/session → command but has NO `SpawnClaimRegistry` — so the exact active/poisoned claim history is NOT part of the aggregate promotion decision. The focused test (`core/tests/runtime_evidence_promotion.rs:406-455`) publishes a live session by folding ONLY `SessionRegistry`; no authority or claim fold runs (this is the OPPOSITE of mutation requirement (a), not evidence for it).
**Fix:** make ONE aggregate promotion fold — INCLUDING `SpawnClaimRegistry` — the only hot/replay publication path; validate all staged clones before acquiring publication; install all views atomically; add a real server recovery/catch-up test; kill fold-omission and fold-reordering mutations.

## Test-strength gaps (each needs a killing test)
| Required mutation | Status |
|---|---|
| (a) Publish N+1 without installed descendant authority | **NOT killed** — session-only test currently demonstrates it succeeds |
| (b) Dispatch nested quarantine candidate normally | Partial (CommandIndex only); no full Observation/session/Elicitation/transcript/ack/authority/diagnostics hot+replay matrix; real raw-Observation path remains |
| (c) Split promotion source/audit transactions / crash after source | **NOT covered** — no failure injection between inserts |
| (d) Reorder/omit authority/session/claim/command folds | **NOT covered** — helper unused, server aggregate lacks claims |
| (e) Admit wrong attachment/adapter-generation/claim/prior/deployment/generation | Only wrong Operation id tested; fixture uses a non-attachment event id |
| (f) Promote through anything except exact `SpawnPromotionCommitted` | Legacy `SpawnClaimEvent` covered; generic audited storage promotion NOT covered and currently succeeds |
Also missing: a fully-valid integrated promotion fixture; continuation promotion with both Grants + promotion-time revocation/expiry; N→N+1 tombstoning; rollback of grant-identity reservation; quarantine malformed-wire rejection.

## Storage atomicity assessment
The dedicated SQLite method (`append_spawn_promotion_audited`, `rusqlite.rs:1444-1521`) is a REAL one-transaction implementation: one `rusqlite::Transaction`, stamps both ids + descendant audit id, inserts source, reserves grant identity, inserts audit, single `commit`. **No two-append fallback.** The design's riskiest-assumption concern (can the storage abstraction do it atomically?) is satisfied. The unsoundness is at the BOUNDARY (generic routes bypass the dedicated method — BLOCKER 5) and SEMANTICS (accepted promotions aren't authority-replayable — BLOCKER 5; descendant not bound to spawner — BLOCKER 4), not the transaction itself.

## Fix scope for the next pass
This is **integration + boundary-exclusivity + semantic-validity** work, NOT redesign:
- Wire the classifier + quarantine + ordered-promotion-fold into the REAL production ingress/observation/authority/server-aggregate paths (BLOCKERs 1, 2, 6).
- Enforce promotion/quarantine boundary exclusivity on ALL storage routes + validate payloads (BLOCKERs 3, 5).
- Bind descendant authority to the exact accepted Operation (BLOCKER 4).
- Replace the isolated test fixtures with a fully-valid integrated promotion + add the missing mutation tests (test-strength gaps a–f).
The resolved designs in `.work/active/features/research-handoff-spawn.md` (§2, §4, §5, Leaf 6) already specify all of this; the worker implemented the types but not the wiring.

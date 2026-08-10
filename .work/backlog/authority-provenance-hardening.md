---
id: authority-provenance-hardening
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-09
---

# Authority provenance hardening

> **Superseded 2026-08-09 (split, option A).** This consolidation grab-bag was split into standalone focused features for separate review boundaries (per the 2026-08-09 adversarial review). The absorbed-findings detail + currency below is retained as the analysis record; the actionable work now lives in:
> - `authority-descendant-grant-completion` (durable-acceptance-metadata + live-composition — PARTIAL/OPEN)
> - `authority-writer-correctness` (ingest-pre-append-conflict-check — OPEN)
> - `replay-integrity-prefix-discipline` (replay-gap-detection — OPEN, cross-projection)
> - `elicitation-responder-validation` (elicitation-responder-authority — OPEN)
> - `authority-grant-selection-determinism` (grant-selection-determinism — PARTIAL)
>
> Already-DONE findings (`failed-authorization-audit`, `payload-actor-in-descendant-issuance`): retain no-regression coverage; fold into the relevant child's acceptance criteria.

## Brief
Consolidate the authority follow-ups absorbed from the parked backlog so acceptance provenance, authorization decisions, replay, and live composition are explicit and durable. Absorbed findings:

- **`backlog-authority-durable-acceptance-metadata`** — server-attested acceptance metadata (verified actor/endpoint/authority-domain/authorizing `GrantId`) surviving replay, to populate descendant-grant provenance + audit linkage. *Src:* authority review #2(C)+#3(2). *Currency (2026-08-09 review):* **PARTIAL** — acceptance now overwrites sender with the verified issuer + durably stores the grant (`core/src/acceptance/pipeline.rs:316-322`), replay reconstructs the grant-bearing command (`index.rs:139-175`), spawn-tail consumes it (`spawn_tail.rs:148-152`); but `audit_id` is still `None` (`spawn_tail.rs:292-295`). *Direction:* add the spawn-completion audit producer, carry its `EventId` into `DescendantGrant.audit_id`; couples with `live-composition`. *Disposition:* **split** — retain audit production/linkage with live descendant issuance.
- **`backlog-authority-failed-authorization-audit`** — distinct durable security-audit record for denied authorizations (not just a submission rejection). *Src:* authority review R4. *Currency:* **DONE** — production `Submit` routes rejections through `audit_submission_rejection` (`server/src/service.rs:287-290`), maps denials to `AuthorizationFailed`, durable audit sink installed (`service.rs:203-210`). *Disposition:* **drop** as impl scope; keep a no-regression test that every production denial appends a distinct audit record without creating an Operation event (the review warned a "consolidate into shared acceptance primitive" must not bypass `service.rs:289`).
- **`backlog-authority-grant-selection-determinism`** — overlapping matching grants need a stable selection rule so the returned `grant_id` is replay-stable. *Src:* deep review Phase 1+2. *Currency:* **PARTIAL** — candidates now sorted by `grant_id` before selection (`core/src/authority/check.rs:47-58`); no overlapping-grants before/after-replay regression. *Direction:* pick the rule (most-specific-scope-first / sort-by-`grant_id` / reject-ambiguity) + add the regression. *Disposition:* **split** into the authorization-decision seam.
- **`backlog-authority-ingest-pre-append-conflict-check`** — authority ingest appends *before* the conflict check (which runs in `observe`), so a conflicting re-ingest poisons the log and an identical retry appends a second event (no durable writer idempotency). *Src:* deep review Phase 1+2. *Currency:* **OPEN** — `current_grant` only chooses audit kind (`ingest.rs:39-48`); append-before-observe (`ingest.rs:179-187`); descendant grants share the non-dedup path (`ingest.rs:75-78`); can poison exactly as described. *Direction:* pre-append check-and-append (identical→existing id, different→reject, absent→append); needs a storage-level atomic or serialized writer; the existing "warm-after-write" test does NOT retry the writer (false confidence). *Disposition:* **split** into authority-writer correctness; resolve with `live-composition`. *(Highest durability hazard here.)*
- **`backlog-authority-live-composition`** — no live consumer loop / composition root feeds committed events to `SpawnDescendantTail` and durably writes descendant grants (reactor is a pure fold, no production writer). *Src:* authority review #3(E). *Currency:* **OPEN** — module doc still says it "does not write grants or own a live consumer loop" (`spawn_tail.rs:1-5`); no production consumer found. *Direction:* startup rebuild → bootstrap → cursor catch-up → continuous committed-event delivery; wire `Issuance`→`ingest_descendant_grant`; couple with the ingress features. *Disposition:* **split** into descendant-issuance/live-composition.
- **`backlog-authority-payload-actor-in-descendant-issuance`** — descendant-grant subject must derive from verified acceptance identity, not self-asserted `Operation.sender` (a valid spawn grant + `sender=B` payload would issue the descendant grant to B). *Src:* deep review Phase 1+2 (top-flagged). *Currency:* **DONE** — durable Operation sender replaced with the verified issuer before append (`pipeline.rs:316-322`); spoofed-sender regression verifies (`core/tests/acceptance_pipeline.rs:784-818`). *Disposition:* **drop**; retain regression coverage.
- **`backlog-authority-replay-gap-detection`** — replay checks `event_lsn <= previous_lsn`, not `== +1`, and `Unspecified` is silently ignored; a gapped sequence could resurrect a revoked grant. *Src:* deep review Phase 2. *Currency:* **OPEN** — `replay.rs:32-39` uses `<=`; `StoredEventKind::Unspecified` silently ignored (`registry.rs:59-67`); contradicts the gap-free contract (`PROTOCOL.md:444-448`). *Direction:* strict gap-free or a documented storage guarantee; reject `Unspecified` as `CorruptLog`; cross-cutting (sessions/acceptance replay share the `<=` check). *Disposition:* **split** into cross-projection replay-integrity, not authority-only.
- **`backlog-elicitation-responder-authority`** — response Operations accepted only when verified issuer = `Elicitation.expected_responder_actor`; neither GrantCheck nor acceptance currently receives the Elicitation. *Src:* authority review #2(G)+#3(R6). *Currency:* **OPEN** — projection retains `expected_responder_actor` (`elicitation.rs:50-56`) but the `ActiveElicitation` port omits it (`ports.rs:110-119`), so `validate_response_payload` can't compare (`pipeline.rs:247-263`). *Direction:* add an Elicitation lookup port to response-Operation acceptance, deny-by-default on mismatch. *Disposition:* **split** into acceptance/Elicitation responder validation. *(Highest-risk silent-check-drop: a "shared grant primitive" must not absorb this — keep it a distinct fail-fast acceptance check.)*

*Currency verified 2026-08-09 (cross-model adversarial review; evidence file:line above). 2 of 8 already DONE (`failed-authorization-audit`, `payload-actor`). Per the review this feature should **split into ~5 seams** (descendant-grant completion; authority-writer atomicity; shared replay-integrity; Elicitation responder validation; grant-selection regression) rather than be implemented as one grab-bag.*

## Simplification opportunity
Consolidate overlapping authority, acceptance, and replay checks into shared boundary and writer primitives; avoid preserving separate authority-only mechanisms where the core acceptance/storage seams can enforce the same guarantees.

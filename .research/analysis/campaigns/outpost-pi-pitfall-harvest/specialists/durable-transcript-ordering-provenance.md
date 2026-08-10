---
source_handle: durable-transcript-ordering-provenance
campaign: outpost-pi-pitfall-harvest
facet: durable-transcript-ordering-provenance
provenance: agent-synthesis
updated: 2026-08-09
---

# Durable transcript ownership, ordering, and provenance

## Scope and evidence

This brief mines Outpost-Pi's local git history and source paths, centered on the
ordering feature, its review, the systematic provenance sweep, the durability
spike, and the resulting durable-transcript epic. The cited material is design
and implementation evidence from commits, not an assertion that the later
architecture is shipped.

## Findings for Patchbay

### 1. A projection rebuilt from another subsystem's durable records creates a divergence class

#### Disconfirming analysis

The ordering fix itself was not dismissed: the review found the projection
render-sort sound, and the implementation deliberately kept lifecycle reduction
in arrival order while sorting only rendered authoritative bubbles by canonical
server time [transcript-ordering-review-70900a5]{1} [transcript-ordering-implementation-455dce8]{1}.
The failure therefore was not simply “sorting is wrong.”

#### Claim

Outpost-Pi identifies the deeper failure as a transcript that is process-local
and rebuilt after restart from SDK messages. Live hook events and SDK-persisted
messages became competing sources; restart silently selected the SDK-derived
version [transcript-durable-epic-f2ed387]{1}. This is a direct `{extends}`
convergence with Patchbay's durable-event-log thesis: the component that owns
transcript semantics must own the durable transcript facts, rather than
reconstructing them from an adjacent LLM-context record.

**Patchbay seam:** make the durable event log the sole ordering authority. A
snapshot/checkpoint is a derived acceleration artifact, never a second event
source. Rehydration should consume validated log records and derive the snapshot,
not re-infer canonical events from adapter-specific messages. `{inferred}`

### 2. Timestamp provenance is not event ordering

#### Disconfirming analysis

Outpost-Pi's app initially tried a store-level `(ts, seq)` sort. The review record
says this mixed phone-receipt-time deltas with server-time terminal events,
resurrected `streaming`, and broke convergence tests; that attempt was reverted
[transcript-ordering-review-70900a5]{1}. A later projection-level sort avoided
that lifecycle regression [transcript-ordering-implementation-455dce8]{1}.

#### Claim

A wall-clock timestamp can describe when an event happened, but it is not a
safe total-order key when clocks, arrival paths, and reducer semantics differ.
Outpost-Pi's surviving design keeps arrival order for lifecycle reduction and
uses canonical server `ts` only for rendering [transcript-ordering-implementation-455dce8]{1}.
For Patchbay, the stronger seam is an append-assigned monotonic LSN (or equivalent
log position) as the canonical order key; `occurred_at`/producer time remains
provenance metadata. `{extends}` `{inferred}`

**Pitfall:** do not let a snapshot's local insertion order, adapter receipt time,
or a peer-provided timestamp silently become the durable order. Define the
comparison tuple and ownership at the log boundary, including tie behavior and
late/duplicate handling.

### 3. First-writer-wins deduplication is not durable ownership

#### Disconfirming analysis

First-writer-wins is useful as a process-local duplicate guard, and Outpost-Pi
used deterministic event IDs to prevent duplicate rows. The problem surfaced
only when different hooks supplied different timestamps for the same logical
event: the later append was discarded locally, while restart backfill could
recreate another timestamp [transcript-provenance-sweep-306dd7f]{1}.

#### Claim

Deduplication answers “is this identity already present?” It does not answer
“which producer owns the canonical timestamp, payload, order position, or
provenance?” Outpost-Pi found tool-result divergence across restart and a second
app-origin user-confirmation case beyond the initial review findings
[transcript-provenance-sweep-306dd7f]{1}. Patchbay should require each logical
operation/event kind to declare an authority and reconciliation rule, rather than
letting whichever hook arrives first establish durable meaning. `{extends}`

**Seam decision:** an event identity must map to one durable record/owner. Late
hooks may enrich or acknowledge that record under an explicit rule; they must not
silently replace its order key. `{inferred}`

### 4. Hook order and multiplicity defeat “retrofit the timestamp later” designs

#### Disconfirming analysis

The SDK permits a `message_end` replacement, so a superficial design could try
to rewrite the assistant message timestamp there. The durability spike traced the
actual order: assistant `message_end` occurs before `tool_execution_start`; one
assistant message can contain multiple tool calls with distinct execution times
[transcript-durability-spike-876d350]{1}.

#### Claim

A single SDK message timestamp cannot represent multiple per-tool execution
events, and a later hook cannot retroactively repair an already-persisted
single-valued message without changing semantics. Outpost-Pi's spike therefore
selected durable custom entries for hook-owned canonical events, alongside SDK
messages [transcript-durability-spike-876d350]{1}. Patchbay should model
multi-event operations explicitly: tool request, execution, result, approval, and
agent output need separate event identities and order positions, even when an
adapter also emits a composite message. `{extends}`

### 5. The audit method is itself a reusable safety seam

#### Disconfirming analysis

The first review found four residual gaps after the dominant path was fixed:
tool live/history divergence, buffered narration fallback, an omitted `agent_done`
timestamp, and error diagnostics without wire provenance [transcript-ordering-review-70900a5]{1}.
The subsequent sweep found three additional cases plus a mesh scope contradiction
[transcript-provenance-sweep-306dd7f]{1}.

#### Claim

Incremental patching was insufficient; the sweep changed the method to
“enumerate first”: every server-to-app broadcast, every authoritative event
constructor, and every schema message capable of producing a rendered bubble
was recorded with timestamp source, wire presence, fallback behavior, and
live-versus-history equality [transcript-provenance-sweep-306dd7f]{1}. This is a
Patchbay verification seam: maintain a producer/consumer provenance matrix for
each event kind, and require a real producer-connected test plus restart/replay
coverage before declaring the ordering invariant closed. `{extends}`

**Gap:** the sweep itself shows why “all events” must include paths that appear
secondary: initial and deduplicated echoes, fallback narration, diagnostics, and
mesh notifications. `{inferred}`

### 6. Authority boundaries must be explicit across adapter, client, and transport

#### Disconfirming analysis

Outpost-Pi considered making mesh tool cards non-authoritative because they were
initially treated as a transport-like path. The enumeration showed that the app
persisted them as rendered tool events, so excluding them would contradict actual
behavior [transcript-provenance-sweep-306dd7f]{1}.

#### Claim

The recorded decision makes app-facing mesh cards authoritative, assigns the
extension sole timestamp authority, makes the app consumer-only, and treats the
relay as opaque transport [transcript-ownership-decision-c3d5bdc]{1}. Patchbay
should make the same boundaries explicit in its protocol: adapters may produce
facts, control surfaces may project them, and transport may deliver them, but
transport receipt time must not acquire transcript authority. `{extends}`

**Seam decision:** every event kind should state whether it is durable authority,
non-authoritative UI state, or transport/control metadata. A “hidden” or
“future-filterable” event should remain in the authoritative log if the product
chooses completeness; otherwise it must be excluded before entering the
canonical projection. `{inferred}`

### 7. Durable custom entries make feasibility concrete, but reconciliation is the hard part

#### Disconfirming analysis

The spike did not claim that adding a custom-entry API automatically solves
consistency. It explicitly calls the two durable sources—SDK messages and
extension entries—the design-bearing reconciliation problem [transcript-durable-epic-f2ed387]{1}.

#### Claim

The feasibility result is specific: custom entries can be appended to session
JSONL, recovered through compaction-aware context entries, ignored by LLM
context, and preferred during transcript backfill [transcript-durability-spike-876d350]{1}.
This suggests a seam for versioned adapter-owned event envelopes and checkpoint
derivation, but not a license to keep two ungoverned authorities.
The log's codec, schema version, validation, duplicate identity, compaction,
truncation, and migration rules must be normative and tested against reopened
storage. `{extends}` `{inferred}`

**Test seam:** use a real durable store, close/reopen it, replay a multi-event
turn (including multiple tool calls), and assert live projection, recovered log,
and checkpoint projection share the same event identities and order keys. The
Outpost-Pi spike explicitly called for this kind of file-backed integration test
[transcript-durability-spike-876d350]{1}.

## Contradictions

| Question | Source positions | Patchbay consequence |
|---|---|---|
| Who owns `ts`? | The initial seed considered SDK `message_end` as owner; the operator decision selected execution/delivery hooks, with `message_end` reusing that value [transcript-durable-epic-f2ed387]{1} [transcript-ownership-decision-c3d5bdc]{1}. | Do not encode ownership as an incidental hook order. Register owner per event kind and make the durable record authoritative. `{contested}` `{inferred}` |
| Are mesh cards authoritative? | The sweep exposed the path as app-facing and authoritative in current behavior; the decision explicitly chose authoritative rather than UI-only [transcript-provenance-sweep-306dd7f]{1} [transcript-ownership-decision-c3d5bdc]{1}. | Patchbay must classify adapter-originated notifications before they enter the log; future filtering is a projection concern only after authority is chosen. `{contested}` |
| Is optional wire `ts` enough? | Tool `ts` was added as an optional compatibility field and made live tool frames match the locally computed history timestamp [transcript-wire-0a048de]{1}. The review/sweep still found missing producers and restart divergence [transcript-ordering-review-70900a5]{1} [transcript-provenance-sweep-306dd7f]{1}. | Optional timestamp fields help migration but cannot establish durable ordering or ownership. LSN/order authority must be independent of optional producer time. `{inferred}` |

## Patchbay seam decisions and gaps

1. **Committed seam:** durable log record owns canonical order; snapshots/checkpoints
   are derived. `{inferred}`
2. **Committed seam:** separate `lsn`/log position from producer `occurred_at`;
   never sort the durable log by wall-clock time alone. `{inferred}`
3. **Committed seam:** per-event-kind authority/reconciliation registry, including
   duplicate, late-hook, multi-tool, compaction, and reopen behavior. `{inferred}`
4. **Verification gap:** generated conformance vectors should cover live delivery,
   duplicate delivery, reconnect replay, process restart, compaction, and snapshot
   rebuild for the same event sequence. The source evidence supports these risks,
   but does not define Patchbay's vector format. `{inferred}`
5. **Verification gap:** explicitly classify diagnostics, optimistic UI, deltas,
   mesh notifications, and transport/control frames as authoritative or excluded;
   do not infer the classification from the current renderer. `{inferred}`
6. **Adoption caution:** Outpost-Pi's ordering feature shipped the dominant fix
   while a review-requested residual invariant remained open, then spun closure
   into a new design feature [transcript-ordering-review-70900a5]{1} [transcript-provenance-sweep-306dd7f]{1}.
   Patchbay should distinguish “observed reorder fixed” from “global provenance
   invariant proven.” `{extends}`

## Disconfirming analysis

The evidence does not establish that every proposed durable-transcript change in
Outpost-Pi reached production: the durable epic and custom-entry architecture are
recorded as a drafting/design arc, while the cited spike establishes feasibility,
not completed implementation [transcript-durable-epic-f2ed387]{1} [transcript-durability-spike-876d350]{1}.
The findings here therefore support pitfalls and seams, not a claim that
Outpost-Pi is a completed reference implementation.

## Revisit if

- Patchbay adds a second adapter that emits equivalent events; reassess the
  per-event authority registry and cross-adapter identity rules.
- Patchbay permits distributed/federated writers; reassess LSN allocation,
  authority domains, and whether a single-writer log remains valid.
- Patchbay introduces snapshot-only recovery or compaction; verify that recovery
  cannot re-derive order from adapter messages or wall-clock timestamps.
- Outpost-Pi lands the custom-entry implementation and restart tests; refresh this
  brief against those source commits, especially reconciliation and migration
  behavior.

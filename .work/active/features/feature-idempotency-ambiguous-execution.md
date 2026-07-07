---
id: feature-idempotency-ambiguous-execution
kind: feature
stage: review
tags: [protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-session-identity-adapter-contract]
created: 2026-06-28
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Refine idempotency and ambiguous execution semantics

Patchbay can deduplicate accepted commands at the coordination boundary, but adapters may not guarantee exactly-once external execution. The docs need to distinguish safe retry from maybe-executed ambiguity.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes genuine semantic design choices: a new `maybe_executed` / ambiguous execution state (or equivalent), idempotency-key scope and lifetime rules, and payload-equivalence rules. These are protocol semantic decisions with real alternatives, not prose consolidation. The prose-author black-box test should have caught this originally.

## Scope

- Command id vs idempotency key.
- Idempotency key scope, payload equivalence, and lifetime.
- Acceptance deduplication vs adapter/target execution deduplication.
- Adapter crash-after-execute-before-ack scenario.
- `maybe_executed` / ambiguous state or equivalent.
- UI language for safe retry, unsafe retry, and intentional duplicate.

## Acceptance criteria

- `docs/PROTOCOL.md` no longer overclaims end-to-end idempotency.
- `docs/UX.md` explains retry affordances using precise execution state.
- `docs/VERIFICATION.md` scopes the formal guarantee to Patchbay acceptance unless adapter capability declares stronger semantics.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions (2026-07-07, operator-confirmed)

- **Q1 — how is `maybe_executed` ambiguity represented?** → **(b) new failure-vocabulary term, not a new CommandState.** Add `execution_outcome_unknown` to the failure vocabulary's execution layer, mapping to `CommandState = failed` as the command effect but carrying the ambiguity signal. Rationale: the terminal-race section already rules out "a distinct durable conflict state" for late candidates; adding a CommandState would force model/proto/vector changes for a signal that is really a failure-layer distinction. The adapter's `idempotency_strength` capability (`none` / `at-Patchbay-boundary` / `end-to-end`) already carries the safety information; UX combines it with the vocabulary term to show safe-vs-unsafe retry. The protocol does NOT add `maybe_executed` as a CommandState.
- **Q2 — idempotency-key dedup scope.** → **(b) per-target.** A key dedups only against existing commands to the same target. Rationale: a retry is always a retry of the same command to the same target; scoping dedup to target prevents cross-target collision while staying broad enough not to fragment. Per-(target,kind) over-constrains; global-within-domain (the current model's `appliedKeys` shape) risks a key reused across different targets wrongly returning the wrong command. Cross-target key reuse in v0 is treated as a new command (no dedup), since a genuine retry always targets the same session/resource.
- **Q3 — payload mismatch on key reuse.** → **(a) reject as `validation_failed`.** A retry arriving with the same id+key but a non-identical payload is rejected at submission. Rationale: fail-fast — a payload mismatch on key reuse signals a client bug and must not silently succeed or silently ignore the new payload. Defining fuzzy "equivalence" (option c) would force v0 to pin a payload-equivalence semantics it does not need; byte-identical-or-reject is simplest and safest. A retry must carry the same payload as the original.
- **Q4 — key retention lifetime.** → **(b) at least until terminal; post-terminal retention implementation-defined.** The protocol pins the minimum: an idempotency key stays dedup-eligible at least until its command reaches a terminal `CommandState`, so `RetryAfterTerminalReturnsExisting` holds for any retry before terminal. Retention beyond terminal (a window for late retries) is an implementation/config concern, not a protocol constant. Rationale: satisfies the checked model and the operator-facing "retry returns existing" guarantee without over-specifying a retention number or mandating unbounded memory.

## Architectural choice

**Represent execution ambiguity as a failure-vocabulary term scoped to the execution layer, scope idempotency-key dedup per-target, reject payload-mismatched retries at submission, and pin key retention to at-least-until-terminal.**

Chosen over:
1. **New `maybe_executed` CommandState (Q1=a)** — rejected: forces model/proto/vector changes; the terminal-race section already rules out a distinct durable conflict state; the ambiguity is a failure-layer distinction, not a lifecycle state. The adapter capability manifest already carries the safety info the operator needs.
2. **Global idempotency-key dedup (Q2=a)** — rejected: a key reused across different targets would wrongly dedup against an unrelated command; genuine retries always target the same session/resource, so per-target is the correct scope.
3. **Per-(target, OperationKind) dedup (Q2=c)** — rejected: over-constrains — an operator retrying `instruct` to the same target with the same key expects dedup regardless of kind-boundary edge cases; the narrowing buys little and fragments the dedup set.
4. **Dedup-and-ignore-new-payload on mismatch (Q3=b)** — rejected: silently ignoring a mismatched payload hides client bugs; fail-fast (reject) surfaces them.
5. **Payload equivalence semantics (Q3=c)** — rejected: forces v0 to define fuzzy equivalence it does not need; byte-identical-or-reject is simplest.
6. **Infinite key retention (Q4=a)** — rejected: unbounded memory with no benefit; the operator-facing guarantee only needs retention through terminal.

This is consistent with the existing checked model (`BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`), the terminal-race section's no-distinct-conflict-state rule, and the capability-manifest `idempotency_strength` field. No new CommandState, no new checked property, no proto enum change.

## Implementation Units

This is a docs-only foundation feature. Four tightly-coupled edits across three docs; single-stride inline implementation (no child stories — the chunks all touch the same idempotency/retry vocabulary and are not independently testable). The checked model (`command_lifecycle.qnt`) needs no change: its `appliedKeys` models the dedup handle abstractly; the per-target scoping is a protocol-level refinement of what `appliedKeys` represents, not a model-level change.

### Unit 1: `docs/PROTOCOL.md` — fix the idempotency overclaim + add the ambiguity vocabulary

**File**: `docs/PROTOCOL.md`

**Edit A — § Idempotency and retry (rewrite, ~line 371).** Current text overclaims end-to-end idempotency. Replace with boundary-scoped language:

> ## Idempotency and retry
>
> Commands are deduplicated at the Patchbay acceptance boundary. Retrying the same command id and idempotency key returns the existing command record and does not create a new accepted record at the boundary. This is a boundary guarantee, not an end-to-end execution guarantee: an adapter that does not track idempotency internally may execute the same logical Operation more than once on retry, and that adapter-side behavior is governed by the adapter's declared `idempotency_strength` capability, not by Patchbay's boundary dedup.
>
> **Idempotency-key dedup scope.** A key dedups only against existing commands to the same target. A retry is always a retry of the same command to the same target; a key reused across different targets does not dedup and is treated as a new command. (The checked `command_lifecycle.qnt` models the dedup handle abstractly as `appliedKeys`; per-target scoping is the protocol-level refinement of what that set represents.)
>
> **Payload equivalence.** A retry must carry the same payload as the original. A submission arriving with an idempotency key already applied to a command to the same target, but with a non-identical payload, is rejected at submission with `validation_failed` before acceptance. An intentional duplicate action uses a new command id and a new idempotency key.
>
> **Key retention.** An idempotency key stays dedup-eligible at least until its command reaches a terminal `CommandState`, so a retry before terminal returns the existing record. Retention beyond terminal is an implementation-defined window for late retries, not a protocol constant.
>
> Adapters that cannot guarantee idempotent external execution must report that limitation as a capability constraint (`idempotency_strength`). Patchbay still deduplicates at the coordination boundary and exposes the adapter limitation to control surfaces.

**Edit B — § Failure and outcome vocabulary (~line 348, add one row).** Add `execution_outcome_unknown` to the execution layer:

> | `execution_outcome_unknown` | execution | The target may have begun or completed execution, but Patchbay cannot determine the outcome (e.g. adapter crash after execute-before-ack, transport loss after delivery). The command transitions to `failed`; the ambiguity is surfaced to control surfaces so retry safety can be evaluated against the adapter's `idempotency_strength`. | `failed` (with ambiguity signal) |

Place it adjacent to `execution_failed` in the execution layer block.

**Acceptance Criteria**:
- [ ] § Idempotency and retry no longer claims end-to-end idempotency; it is explicitly boundary-scoped.
- [ ] Per-target dedup scope, payload-equivalence (reject on mismatch), and at-least-until-terminal retention are all stated.
- [ ] `execution_outcome_unknown` is in the failure vocabulary, execution layer, mapping to `failed`.
- [ ] No new CommandState is added.

### Unit 2: `docs/UX.md` — retry affordances using precise execution state

**File**: `docs/UX.md`

**Edit — § Failure vocabulary mapping (~line 22) and the retry anti-pattern (~line 119).** The existing "Show what is safe to retry" bullet becomes precise: retry safety is derived from `execution_outcome_unknown` (the ambiguity signal) combined with the adapter's `idempotency_strength` capability. State the matrix:

> - **Retry safety is derived, not assumed.** When a command is `failed` with `execution_outcome_unknown`, the surface shows retry safety derived from the adapter's declared `idempotency_strength`: `end-to-end` → safe to retry (the adapter dedups externally); `at-Patchbay-boundary` → retry may double-execute (Patchbay dedups its record but the adapter may not); `none` → retry will double-execute if the original executed. A plain `failed` (without `execution_outcome_unknown`) means execution did not reach the "maybe executed" window and is safe to retry by default. The surface must never present a retry as unconditionally safe without these signals.

And strengthen the anti-pattern at ~line 119:

> - Retrying commands without showing idempotency behavior or retry safety (per the `idempotency_strength` + `execution_outcome_unknown` matrix).

**Acceptance Criteria**:
- [ ] UX explains retry affordances using `execution_outcome_unknown` + `idempotency_strength`, not a blanket "safe to retry."
- [ ] The three retry-safety tiers (end-to-end / at-boundary / none) are presented.
- [ ] A plain `failed` without the ambiguity signal is distinguished from `failed` with `execution_outcome_unknown`.

### Unit 3: `docs/VERIFICATION.md` — scope the formal guarantee to Patchbay acceptance

**File**: `docs/VERIFICATION.md`

**Edit — § Idempotent retry (~line 184) and the spawn-idempotency note (~line 326).** Make the scope of the checked guarantee explicit:

> ### Idempotent retry
>
> The checked idempotency properties (`BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`) are guarantees about deduplication at the Patchbay acceptance boundary, not end-to-end execution guarantees. They hold per-target: a key dedups against commands to the same target. They do not claim that an adapter executes a given Operation exactly once on retry; adapter-side execution idempotency is governed by the adapter's declared `idempotency_strength` capability and is not a formal property until a future adapter contract model is scoped (see the spawn-idempotency note below).
>
> The `execution_outcome_unknown` failure term is a presentation/audit signal, not a checked property: it surfaces ambiguity to control surfaces so retry safety can be evaluated, but the protocol does not formally model adapter-side execution determinism.

At ~line 326 (the spawn-idempotency note), align wording: it already says adapter-side duplicate external process prevention is "not claimed as a formal property until a future adapter contract model is scoped" — confirm this is consistent and cross-reference the new boundary-scope statement above.

**Acceptance Criteria**:
- [ ] VERIFICATION scopes the formal guarantee to Patchbay acceptance (per-target boundary dedup), explicitly not end-to-end.
- [ ] `execution_outcome_unknown` is marked as a presentation signal, not a checked property.
- [ ] No new checked property or model area is added.

### Unit 4: extension-seams registry row

**File**: `docs/PROTOCOL.md` (Extension seams registry, ~line 588+)

The registry currently has a row for reserved enum values including `adapter-utility-exec`'s "full lifecycle/idempotency modeling deferred." Confirm whether a new row is warranted for "end-to-end adapter execution idempotency" as a reserved seam. It is already implied by the `idempotency_strength` capability (`end-to-end` is a manifest value adapters may declare). Add an explicit row if the cross-cutting index would otherwise omit it:

> | adapter execution idempotency | end-to-end adapter-side execution idempotency (no double-execute on retry) | R | PROTOCOL `idempotency_strength` capability; VERIFICATION spawn-idempotency note |

**Acceptance Criteria**:
- [ ] The registry either carries an explicit row for end-to-end adapter execution idempotency or confirms the `idempotency_strength` capability row already covers it (avoid silent omission of a reserved seam).

## Implementation Order

Single-stride inline. Order: Unit 1 (PROTOCOL overclaim fix + vocabulary) → Unit 3 (VERIFICATION scope, depends on Unit 1's boundary-scoped language) → Unit 2 (UX matrix, depends on Unit 1's vocabulary term) → Unit 4 (registry row, independent). No child stories.

## Testing

No code tests — docs-only foundation feature. Verification by:
- Grepping PROTOCOL for any remaining end-to-end idempotency overclaim (e.g. "does not apply the command twice" without boundary qualifier — should be zero after Unit 1).
- Confirming `execution_outcome_unknown` appears in the failure vocabulary table.
- Confirming UX presents the three retry-safety tiers.
- Confirming VERIFICATION scopes the checked guarantee to the boundary, per-target.
- Walking each acceptance criterion against the edited docs.
- Confirming the checked model needs no change (`appliedKeys` abstraction is unchanged; per-target is a protocol refinement).

## Risks

- **Risk: per-target scoping narrows the checked model's `appliedKeys` abstraction.** Mitigation: the model treats `appliedKeys` as an abstract dedup handle; per-target is a protocol-level refinement of what the set contains, not a change to the model's actions or invariants. If a future adapter contract model needs to check per-target scoping formally, it can; the seed model's `BoundaryDedup` invariant (no key applied twice) holds regardless of the set's granularity.
- **Risk: `execution_outcome_unknown` is confused with a checked property.** Mitigation: Unit 3 explicitly marks it as a presentation/audit signal, not a checked property; the surface matrix in Unit 2 derives retry safety from it + the capability, not from a formal guarantee.
- **Risk: payload-reject is too strict for clients that legitimately vary metadata.** Mitigation: payloads that vary only in non-semantic metadata should be canonicalized before submission (a client concern); the protocol's contract is byte-identical-or-reject on the payload it receives. If real-world pressure shows this is too strict, a follow-on feature can define equivalence — but v0 ships strict and fail-fast per the design principle.
- **Risk: at-least-until-terminal retention leaves late-retry-after-terminal behavior unspecified.** Mitigation: this is intentional — post-terminal retention is implementation-defined; the protocol pins only the minimum (`RetryAfterTerminalReturnsExisting` before terminal). A late retry after terminal and after the implementation's retention window is treated as a new command, which is the safe default.

## Implementation notes

- **Files changed:**
  - `docs/PROTOCOL.md` — (a) § Idempotency and retry: rewrote to boundary-scoped language ("boundary guarantee, not an end-to-end execution guarantee"); added per-target dedup scope, payload-equivalence (reject on mismatch with `validation_failed`), and at-least-until-terminal key retention statements. (b) § Failure and outcome vocabulary: added `execution_outcome_unknown` row in the execution layer, adjacent to `execution_failed`, mapping to `failed` with an ambiguity signal. (c) Extension seams registry: added an explicit row for end-to-end adapter execution idempotency as a reserved seam (declared via `idempotency_strength`, not a formal property).
  - `docs/UX.md` — § Failure vocabulary mapping: expanded "Show what is safe to retry" into the derived retry-safety matrix (`execution_outcome_unknown` + `idempotency_strength` → end-to-end safe / at-boundary may-double-execute / none will-double-execute; plain `failed` = safe by default). Strengthened the retry anti-pattern.
  - `docs/VERIFICATION.md` — § Idempotent retry: added the boundary-vs-end-to-end scope statement (checked properties are per-target boundary dedup, not end-to-end execution guarantees; `execution_outcome_unknown` is a presentation signal, not a checked property). Cross-referenced the spawn-idempotency note to the new scope statement.
- **Tests added:** none (docs-only foundation feature). Verification by grep: zero remaining end-to-end overclaim; `execution_outcome_unknown` in vocab; UX three-tier matrix present; VERIFICATION boundary-scoped; no `maybe_executed` CommandState added; registry row present; per-target scope stated. The checked `command_lifecycle.qnt` model needs no change (`appliedKeys` abstraction unchanged; per-target is a protocol refinement).
- **Discrepancies from design:** none.
- **Adjacent issues parked:** none.
- **Acceptance criteria walk:**
  - PROTOCOL no longer overclaims end-to-end idempotency → met (rewritten to boundary-scoped; overclaim phrases removed).
  - UX explains retry affordances using precise execution state → met (derived retry-safety matrix using `execution_outcome_unknown` + `idempotency_strength`).
  - VERIFICATION scopes the formal guarantee to Patchbay acceptance unless adapter capability declares stronger → met (boundary scope statement + presentation-signal classification; spawn note cross-referenced).

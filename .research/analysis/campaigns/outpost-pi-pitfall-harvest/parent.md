---
campaign: outpost-pi-pitfall-harvest
provenance: agent-synthesis
composition: lead cross-join
updated: 2026-08-09
verification_rigor: standard
facets: [restart-hot-reload-lifecycle, herdr-multi-cwd-project, mobile-control-fragility, durable-transcript-ordering-provenance, identity-keyring-durability]
commissioning_item: outpost-pi-pitfall-harvest
---

# outpost_pi pitfall harvest — campaign synthesis

## Engagement

Grounded harvest of the operator's hand-built control surface (`/home/agent/projects/outpost_pi`, 1751 commits + `.work/` session notes + `docs/DECISIONS.md` + installed Pi SDK) for pitfalls / seam decisions / gaps that feed Patchbay's v1 design. Dials: `scope_authority=in-engagement-judgment`, `verification_rigor=standard`, `output_kind=campaign`. Five specialist facets, each cite-bound to fetched local sources (commits / files / session notes / SDK definitions). This is lead composition across the five specialist briefs; per-facet detail + full citation density live in `specialists/<facet>.md`.

**Substrate overlap note.** The prior `v1-control-plane-and-spawn` campaign attested herdr's *model* (`[herdr-concepts]`, `[herdr-state]`) as a spawn-lifecycle peer comparison. This harvest covers the distinct *bug-swatting history*; the herdr facet cross-references those attestations as lens (not cite targets).

## Consumption map — which findings feed which Patchbay design

| Patchbay surface | Harvest facet | Headline pitfalls / seams | Key handles |
|---|---|---|---|
| `research-handoff-spawn` (restart-as-continuation, generation fencing, descendant authority) | restart; herdr; keyring | `/reload`≠upgrade boundary; old-action-kills-successor; PID≠incarnation; no exclusive generation claim; identity-continuity≠credential-presence | `restart-session-note-20260731`, `restart-hot-reload-feature`, `restart-wrapper-foreground-regression`, `herdr-ancestor-fix`, `keyring-incident` |
| Project / cwd seam (adapter-owned in v1) | herdr | cwd≠durable ProjectId; shell-ancestry≠ownership; restart-ownership split across layers; schema archaeology | `herdr-cwd-migration`, `herdr-session-20260731`, `herdr-restart-initial` |
| Mobile control surface + Operations model | mobile | TUI-editor-seam driving is fragile; `newSession no-command-ctx` gap; converged on dedicated-ops (Operations over keystrokes) | `mobile-newctx`, `mobile-sdk-contract`, `mobile-ops`, `mobile-fresh` |
| Core durability / canonical ordering / provenance | transcript | rebuild-from-adjacent-record divergence; timestamp≠order; first-writer-wins≠ownership; enumerate-first audit method | `transcript-durable-epic-f2ed387`, `transcript-ordering-implementation-455dce8`, `transcript-provenance-sweep-306dd7f` |
| Identity / authority / keyring | keyring | silent re-identity on credential loss; credential-unavailable≠identity-absent; continuity must be a state transition | `keyring-incident`, `keyring-decisions`, `keyring-storage` |

## Cross-cutting meta-findings

### M1. "Don't infer X from Y" — the universal Patchbay lesson

Every facet surfaces a distinct instance of inferring a stronger condition from a weaker signal, and each was observed to fail, was reproduced, or was exposed/rejected during review. `{inferred: aggregates}`

- Don't infer **adapter-code uptake** from `/reload` receipt — a cached ESM module ran old code. `[restart-session-note-20260731]{1}`
- Don't infer **process liveness/progress** from transport connectivity — a browser PTY stopped draining and froze single-threaded Pi while the relay WebSocket stayed live. `[herdr-session-20260731]{1}`
- Don't infer **command execution** from a successful PTY write — `/quit` never reached the running TUI. `[herdr-restart-signal-fix]{1}`
- Don't infer **restart completion** from process exit / child spawn — the successor's relay didn't auto-connect, leaving the control surface dark. `[restart-wrapper-operational-fixes]{5}`
- Don't infer **identity continuity** from credential presence — a divergent fallback file silently became a new cryptographic principal. `[keyring-incident]{3}`
- Don't infer **event order** from a wall-clock timestamp — mixed clocks/arrival paths resurrected `streaming` and broke convergence. `[transcript-ordering-review-70900a5]{1}`
- Don't infer **TUI command acceptance** from an editor callback — it was an indirect UI hook, not a supported invocation API. `[mobile-editor-spike]{1}` `[mobile-sdk-contract]{4}`

**Patchbay seam:** every authority-bearing inference must rest on an explicit, declared signal, not a proxy. This is the Fail-Fast / explicit-authority principle instantiated across the whole control plane.

### M2. Incarnation / generation fencing is the universal hard problem

Two facets independently converge on incarnation/process fencing as load-bearing, each recording a fence failure or exposed gap: `{inferred: converges}`

- restart: an old `turn_end` timer killed a successor turn because it checked mutable module state instead of an incarnation token `[restart-hot-reload-feature]{5}`; a machine-global sentinel let multiple runtimes consume one request `[restart-hot-reload-feature]{2}`; exact child-PID correlation was *removed* to fix terminal ownership, reopening cross-wrapper marker consumption as an exposed, untested gap `[restart-wrapper-foreground-regression]{3}`.
- herdr: shell ancestry was treated as session ownership until tooling inserted bash/subshells `[herdr-ancestor-fix]{1}`.

(The keyring identity-continuity failure is a *related but distinct* authority-continuity failure — see M1 — not an incarnation/process-fence failure; it is not field evidence for lifecycle fencing.)

**Patchbay seam (central):** the spawn redesign's incarnation-fence requirements are the core instance. The 2026-08-09 spawn review's **BLOCKER 3** (stale-generation fencing absent from Observation ingress), **BLOCKER 4** (no exclusive generation-change claim; boundary dedup can't prevent duplicate runtimes), and **BLOCKER 5** (restart strands descendant authority) are each directly prefigured here — outpost_pi hit the concrete field failures (old-action-kills-successor; non-exclusive claim; identity not carried across replacement) that those blockers name abstractly. `[restart-hot-reload-feature]{5}{7}` `[restart-wrapper-foreground-regression]{1}`

### M3. Convergence with Patchbay's thesis (`{extends}`)

Where outpost_pi independently worked the same problems, it converged on Patchbay's committed answers — independent corroboration, not imitation: `{inferred: converges}`

- **Durable log = sole ordering authority; snapshots are derived.** The transcript facet's headline: the component owning transcript semantics must own the durable facts, not rebuild from an adjacent LLM-context record. `[transcript-durable-epic-f2ed387]{1}` This `{extends}` Patchbay's durable-event-log thesis and the `storage-recovery-correctness` "no second ordering authority" rule.
- **Process replacement is the adapter-code-upgrade boundary.** `[restart-session-note-20260731]{1}` `{extends}` the spawn review's resolved fork (restart = new generation; `/reload`-style in-place reload is unsound).
- **Operations over TUI-seam for control.** `[mobile-ops]{1}` `{extends}` Patchbay's Operations model; the mobile rescope *dropped* the editor-seam design as fragile.
- **Append-assigned monotonic order key, producer-time as provenance only.** `[transcript-ordering-implementation-455dce8]{1}` `{extends}` Patchbay's LSN ordering.

### M4. Separate concerns the source system conflated (and Patchbay must keep distinct)

`{inferred: aggregates}` settled-notification vs exclusive-quiescence vs delivery-durability (`[restart-hot-reload-feature]{8}{10}`); stop vs restart-as-continuation vs restart-fresh (`[restart-fresh-session-ea6b5fd]{1}`); process-existence vs terminal-drain-liveness vs agent-responsiveness (`[herdr-session-20260731]{1}`); text-injection vs lifecycle-control (`[herdr-restart-signal-fix]{1}`); credential-rotation vs identity-continuity (`[keyring-decisions]{2}`); event-order vs event-timestamp vs producer-identity (`[transcript-provenance-sweep-306dd7f]{1}`); Operations vs TUI-seam (`[mobile-ops]{1}`). Each conflation produced a recorded failure, an exposed gap, or a review-rejected design; each separation is a Patchbay seam decision.

### M5. Honest-evidence / verification seam

Two facets surface a reusable verification discipline directly relevant to the `conformance-vectors-executable-evidence` cluster: `{inferred: aggregates}`
- **"Enumerate first" audit method.** Incremental patching missed residual cases; the systematic sweep enumerated every broadcast/event-constructor/schema-message with timestamp-source, wire-presence, fallback, and live-vs-history equality — and still found more cases. `[transcript-provenance-sweep-306dd7f]{1}` Patchbay's per-event-kind authority registry + conformance vectors should follow this enumerate-first discipline.
- **Green tests ≠ clean lifecycle.** A restart-sweep test passed all assertions while a late asynchronous ENOENT produced a nonzero exit. `[restart-enoent-race]{1}{3}` Verification must treat unhandled async errors as failure even when assertions are green — directly relevant to the verification cluster's "executable + mutation-survivable" honesty requirement.

## Cross-facet relationships (Checkpoint B)

No hard contradictions *across* facets — they are complementary views of one control plane. Two cross-facet convergences worth naming:

- **The terminal-ownership-vs-process-correlation tension is one root problem seen from two facets.** The restart facet frames it as lifecycle fencing (exact child-PID marker removed when foregrounding the TUI `[restart-wrapper-foreground-regression]{3}`); the herdr facet frames the *same* wrapper tradeoff as a hosting/restart-ownership split (`[herdr-wrapper-tty-fix]{1}{3}`). Patchbay's resolution belongs in one place: an adapter/supervisor protocol that preserves foreground terminal ownership *and* a stable incarnation-correlated child handle — not derived from shell job-control.
- **"Derived, never a second source" is stated by two facets.** Transcript (snapshot derived from log `[transcript-durable-epic-f2ed387]{1}`) and the Patchbay `storage-recovery-correctness` rule converge; the harvest corroborates the rule from independent field experience.

Within-facet contradictions (e.g. extension comment overstates the live PID fence vs wrapper globs any marker `[restart-fresh-session-ea6b5fd]{7}{4}`; intended file-mirror invariant vs unimplemented write-through `[keyring-incident]{6}` / `[keyring-storage]{6}`) are recorded in each specialist brief.

## Convergence with the 2026-08-09 Patchbay adversarial review

The harvest independently corroborates **four** of that review's design-level blockers from field-attested failure modes, plus a fifth as an *analogous warning* (not direct evidence): `{inferred: cross-band}`
- spawn **BLOCKER 3** (stale-generation fencing absent) ← restart P4/P5 `[restart-hot-reload-feature]{5}`
- spawn **BLOCKER 4** (no exclusive generation-change claim) ← restart P5/P6 `[restart-hot-reload-feature]{2}` `[restart-wrapper-foreground-regression]{3}`
- spawn **BLOCKER 5** (descendant authority stranded after restart) ← *analogous only*: the keyring silent-re-identity is an authority-continuity failure of the same shape — a credential/identity event crossing an authority boundary without an explicit transition `[keyring-incident]{7}` `{inferred: analogy}` — but the incident involves neither restart-as-continuation nor descendant grants, so it supports a warning, **not** field evidence for this blocker.
- authority/sessions replay-integrity ← transcript ordering/ownership `[transcript-ordering-implementation-455dce8]{1}`
- verification "executable + mutation-survivable, not green-assertions-only" ← restart P12 `[restart-enoent-race]{1}`

## Verification + lint

`verification_rigor: standard` → `lint` (floor) + `adversarial-read` (dispatched) + `spot-check` (lead). Citation-lint outcome: the **transcript** facet is clean (29 resolved / 0 broken / 0 thin). The other four facets use richer `source_path` formats (repo@commit:path, multi-path `;`-lists, git-commit-ranges) that pin exact commits — *stronger* grounding — but the lint's source-reachability check (built for single stat-able paths / http URLs) flags them `unreachable-source`. This is a **local-source tooling limitation, not a grounding gap**: all 38 attestation files exist and point at fetched outpost_pi commits/files/SDK-definitions; the handle→attestation chain resolves throughout (lead spot-check). Attestations vary on the optional `substrate_confidence` field (the lint's deprecation note); a future normalization pass could add it uniformly.

## Acquisition candidates

One enriching candidate (consolidated in `acquisitions.md`): a pinned Herdr v0.7.5 source/schema + CLI JSON fixtures for workspace/pane/process APIs (`[herdr-setup-pane-fix]{1}` motivates). Promotion to the `research-acquisition-queue` is operator-confirmed at the handoff gate.

## Next

When the consumer designs (spawn redesign, project/cwd seam, mobile-control, durability/ordering, identity/authority) are picked up, suggest `/agentic-research:research-handoff outpost-pi-pitfall-harvest` to emit operator-confirmed `.work/` items grounded in these findings. The handoff never auto-fires.

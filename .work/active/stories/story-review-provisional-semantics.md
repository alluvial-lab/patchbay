---
id: story-review-provisional-semantics
kind: story
stage: drafting
tags: [protocol, security, verification, foundation]
parent: epic-foundation-hardening
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Review four provisional semantic choices in committed v0 docs

A retrospective pass over the done prose features flagged several semantic decisions that were made inside the prose lane without a design pass. Three were promoted to full design features (`feature-design-terminal-commit-race`, `feature-design-grant-shape`, and the retagged `feature-session-identity-adapter-contract`). Four weaker candidates were not promoted but deserve a deliberate review pass: are these acceptable as committed, or do they need their own design work?

## Candidates to review

### 1. Session state axis decomposition (the specific 5×3 state sets)

`docs/PROTOCOL.md` splits session presentation into `SessionConnectivityState` (live/stale/offline/unknown/failed = 5) × `SessionActivityState` (idle/working/unknown = 3). The *axis split* was directed by the epic; the *specific state sets* were not designed against alternatives.

Question: are these the right five connectivity states and three activity states? Alternatives to consider: merging `failed` into `offline` or `unknown`; splitting `unknown` into connectivity-unknown vs activity-unknown differently; whether `working` needs sub-states.

Location: `docs/PROTOCOL.md` session state axes.

### 2. Enrollment posture (first-run bootstrap)

`docs/SECURITY.md` commits: CLI/local-console bootstrap, one-time expiring setup secret, password/passphrase authenticator for v0, browser/CLI/adapter enrollment rules. From the research brief, adopted without an alternatives pass.

Question: is "one-time expiring bootstrap secret" the right first-run UX, or should it be an interactive CLI setup wizard, a config-file-only bootstrap, or something else? This affects operator ergonomics more than safety.

Location: `docs/SECURITY.md` enrollment and authentication.

### 3. The five revocation actions (especially "security lockdown")

`docs/SECURITY.md` commits five v0 revocation actions: current-session, all-sessions, endpoint/device, adapter/session grant, security lockdown. The specific set of five was invented during the security prose feature.

Question: is "security lockdown" as a named v0 posture the right granularity, or should it be simpler ("revoke all + require fresh login") or richer? Are the five actions the right five, or is one redundant / missing?

Location: `docs/SECURITY.md` revocation model.

### 4. LSN as gap-free, per authority domain

`docs/PROTOCOL.md` commits: monotonic, gap-free log sequence number per authority domain. Gap-free is a strong commitment — simplest model, but constrains future sharding/federation.

Question: for single-writer v0 this is almost certainly right, but does the "gap-free" and "per authority domain" choice paint the future-federation seam into a corner? Should v0 reserve a logical-clock abstraction (hybrid logical clocks, per-stream sequences) instead?

Location: `docs/PROTOCOL.md` revisions and cursors.

## Procedure

For each candidate: read the committed doc, consider the alternatives, decide one of:

- **Accept as committed** — the choice is right for v0; no change.
- **Accept with note** — the choice is right but add an explicit "alternative considered" note to the doc.
- **Promote to design** — the choice needs real design work; file a feature and route through `feature-design`.

This is a review story, not a design feature: the expected outcome is a decision per candidate, not a design pass. If a candidate is promoted to design, file the feature and stop reviewing that one here.

## Acceptance criteria

- Each of the four candidates has a recorded decision (accept / accept-with-note / promote-to-design).
- Any "accept-with-note" notes are added to the relevant docs.
- Any "promote-to-design" candidates are filed as features with a cross-reference back here.

---
id: story-review-provisional-semantics
kind: story
stage: done
tags: [protocol, security, verification, foundation]
parent: epic-foundation-hardening
depends_on: []
created: 2026-06-28
updated: 2026-06-29
gate_origin: null
release_binding: v0.1.0
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

## Review decisions (2026-06-29)

All four candidates were reviewed against alternatives. None promoted to design. Decisions:

### Candidate 1 — Session state axis decomposition (5×3): ACCEPT

The axis split (connectivity × activity) was an explicit epic decision and is sound. The specific state sets are minimal and well-motivated:

- `live`/`stale`/`offline`/`unknown` are the classic reachability states.
- `failed` earns its place distinct from `offline`: `offline` = authoritatively determined unavailable; `failed` = explicit error reported. Merging them loses "I know it's down" vs "I got an error trying," which matters for operator UX and for the failure vocabulary (`stale_event`, `execution_failed`).
- Activity `idle`/`working`/`unknown` is minimal. Sub-states of `working` belong in adapter capability/metadata, not core protocol.

No alternative improves on it. The provisional marker is removed.

### Candidate 2 — Enrollment posture: ACCEPT

The safety commitment (no unauthenticated network setup; CLI/local-console first-run) is correct and research-grounded. The one-time expiring setup secret is the bridge from CLI bootstrap → first browser enrollment — a coherent, common pattern. The alternatives (interactive CLI wizard, config-file-only) are implementer-time UX choices that don't change the protocol/security model. The story's own framing says this affects operator ergonomics more than safety.

**Coupling made explicit (load-bearing):** the enrollment channel (local CLI/console/SSH/trusted device) must be distinct from routine web login. This distinction is what makes security lockdown (candidate 3) meaningful — lockdown exit requires re-establishing bootstrap trust via the bootstrap channel, not routine web re-authentication. If a future deployment ever makes bootstrap trust == routine web login (same factor, same remote channel), lockdown would provide no protection. The channel distinction is a load-bearing dependency, not incidental.

The provisional marker is removed.

### Candidate 3 — Five revocation actions (incl. security lockdown): ACCEPT-WITH-NOTE

Actions #1 (current session), #2 (all sessions), #3 (endpoint/device), and #4 (adapter/session grant) target distinct, non-redundant scopes. "Security lockdown" (#5) is not redundant with #2: #2 affects only browser sessions, while lockdown rejects new commands at the core boundary across all channels (browser, CLI, adapter), marks runtime sessions stale, and requires fresh authentication.

**The note (load-bearing):** lockdown exit = re-establish the bootstrap trust level **via the bootstrap channel** (local CLI/console/SSH/trusted device — whatever the operator configured at setup, per candidate 2), not routine web re-authentication. This self-scales with the operator's configured security posture. Restart does not clear lockdown: the posture is durable (an audited, persisted event), so crash recovery replays the log and lockdown remains in effect; the only exit is the channel the operator originally configured as their trust boundary. This coupling depends on the enrollment channel being distinct from routine web login (candidate 2); if that distinction is lost, lockdown provides no protection.

The provisional marker is removed; the note is added to `docs/SECURITY.md`.

### Candidate 4 — Gap-free per-authority-domain LSN: ACCEPT-WITH-NOTE

For single-writer v0 this is correct and simplest — it directly supports terminal-commit-wins (lowest LSN) and snapshot reconciliation (consistent prefix). "Per authority domain" is itself the federation seam: each domain has its own gap-free LSN, and cross-domain coordination would be a separate layer on top, not a replacement of the per-domain counters. So the choice does not force a rewrite when federation arrives.

**The note (forward-compatibility hygiene):** event/cursor/revision identity is the **`(authority_domain_id, LSN)` tuple**, not a bare LSN. V0 has one domain, so in practice every key carries the same domain id — but the *shape* of the durable key includes the domain demarcator. When federation arrives, you add a cross-domain coordination layer; you don't migrate existing events because they were always domain-scoped. Hybrid logical clocks (HLC) / logical-clock abstraction was considered for cross-domain federation and deferred as premature — per-domain key shape is the federation seam, not a blocker to it.

The provisional marker is removed; the note is added to `docs/PROTOCOL.md`.

## Outcome

No candidates promoted to design. All four provisional markers removed. Two notes added (lockdown exit in SECURITY.md; domain-tuple key in PROTOCOL.md). The enrollment/lockdown coupling is made explicit in both the story and SECURITY.md.

## Review (2026-06-29)

**Verdict**: Approve — story verified by implementation

This is a review story (fast lane). The work was a decision pass over four provisional semantic candidates; the decisions are recorded above and applied to the docs (markers removed, two notes added). Self-verification via `rg` confirmed all four provisional markers are removed, the domain-tuple-key note is in `docs/PROTOCOL.md`, and the lockdown-exit note is in `docs/SECURITY.md`. No code/build applies (foundation-doc only). No blockers.

**Notes**: The review surfaced two load-bearing insights beyond a simple accept/reject: (1) lockdown exit must require the bootstrap channel, not routine web re-auth, and this couples candidates #2 and #3 (enrollment channel distinction is what makes lockdown meaningful); (2) the LSN key must carry the authority-domain demarcator from day one as federation hygiene. Both were folded into accept-with-note decisions rather than promoting to design, because the principles resolve the forks without new design work. No candidates promoted to design; no follow-up items filed.

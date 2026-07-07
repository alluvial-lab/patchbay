---
id: feature-ux-v0-acceptance
kind: feature
stage: review
tags: [ux, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Feature: Define v0 web cockpit UX acceptance criteria

The docs name Claude-app-style continuity as a quality bar, but the first web cockpit needs actionable acceptance criteria for screens, states, and failure handling.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes real UX design choices: required v0 screens and navigation, session detail / message timeline behavior, and composer requirements. These are UX architecture decisions (what screens, what navigation pattern, what timeline model), not just writing criteria. The `feature-design` lane can invoke the ux-ui-design skills (`screens`, `flows`) for the design pass. The prose-author black-box test should have caught this originally.

## Scope

- Required v0 screens and navigation.
- Session list fields and badges.
- Session detail / message timeline behavior.
- Composer requirements.
- Command delivery timeline and failure states.
- Reconnect/stale/offline banners.
- Multi-device continuity expectations.
- Empty/error/loading states.

## Acceptance criteria

- `docs/UX.md` separates session liveness states from command delivery states.
- `docs/UX.md` defines v0 required screens and visible fields.
- UX text references canonical protocol states rather than maintaining a divergent state list.
- The web cockpit can be designed without guessing what must be visible before sending a command.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions (feature-design, 2026-07-06)

Resolved interactively after a reframing of the deliverable. The original brief assumed a single v0 web cockpit to specify; the operator reframed Patchbay's UX as a **surface-neutral conformance floor** (the minimum any conformant control surface must implement), with the v0 web cockpit as the first conformant *instance*, and with operator-customizable skins/layouts ("Codex-style vs Claude-style vs CLI") as a reserved seam above the floor. This is symmetric to adapter-neutrality: just as Pi-specific capabilities are adapter-declared features, not core protocol primitives, surface-specific presentation is a surface-declared feature, not a core UX primitive.

- **Q1 — Deliverable shape → surface-neutral conformance floor + v0-web instance.** The primary deliverable is the conformance floor (registry-derived obligations any control surface must meet), not a pinned visual design. The v0 web cockpit is framed as the first conformant instance whose acceptance criteria are also specified. A surface is conformant when it meets the floor; skins/layouts above the floor are surface-declared.
- **Q2 — Surface-neutrality as a named seam.** The design explicitly names surface-neutrality as a reserved architectural seam, symmetric to adapter-neutrality. The floor permits operator-customizable skins/layouts above it; it does not mandate a single visual language. This ratifies what the docs already imply (the "control surface" registry concept, "Expo must not fork protocol semantics," the mobile-only/Pi-specific anti-patterns) without having named it.
- **Q3 — `docs/UX.md` restructure.** UX.md is restructured into (a) a surface-neutral conformance floor, then (b) the v0 web cockpit as the first conformant instance. The current doc is web-cockpit-flavored; the restructure makes the floor the frame and the instance the v0 reality. One UX doc, not a new file.
- **Q4 — Mockups deferred visibly.** The v0 web cockpit mockup pass is deferred to a named follow-on, recorded as a reserved seam here, not silently skipped. A mockup pins one conformant instance, not the conformance contract; mocking inside this feature would silently privilege one visual instance and work against surface-neutrality.
- **Q5 (operator-raised) — Shared presentation-component layer as a named seam.** The operator asked whether a UX component layer should be defined for different surfaces to consume. Web-stack answer: a three-layer split (design tokens → shared component/presentation library → surface). `docs/ARCHITECTURE.md:152` already names the "presentation model" as part of the shared TS operator domain (protocol client, delivery/reconnect state machines, presentation model); line 85 lists "stale/live/working/offline presentation model" as an operator-domain responsibility. This feature **names and refines that seam** as the **shared presentation-component layer** — the layer that binds canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`, etc.), making skins possible. Stated obligations: bind canonical states (not invent divergent ones); be skin-able via tokens; be composable by any conformant surface. Implementation is **deferred** — naming the seam, not building it, is this feature's job (symmetric to how `feature-pi-parity-checklist` named supervisord-control `spawn` as a reserved seam without building it). Bootstrapping the component layer or tokens here would be the same scope-creep trap as running the full `ux-ui-design` pipeline inside a criteria feature.
- **Q6 (operator-raised) — Floor is behavioral + state-binding, not visual.** The conformance floor obligates surfaces to present canonical states honestly and to compose from the shared state-binding component layer (when it exists), but does NOT mandate design tokens / visual design — those are surface-declared. Operator-customizable skins are a reserved seam above the floor.

## Architectural choice

A **surface-neutral UX conformance floor** consumed by surface instances, with the **shared presentation-component layer** named as the architectural seam that makes the floor enforceable and skins possible. The floor is registry-derived (it references `CommandState`, `SessionConnectivityState`/`SessionActivityState`, `ElicitationState`, the failure vocabulary, and the snapshot/reconnect rules from `docs/PROTOCOL.md`); it does not re-declare them. The v0 web cockpit is specified as the first conformant instance.

Approaches considered:

1. **Surface-neutral floor + named presentation-component seam + v0 instance (chosen).** The floor is the frame; the component layer is named as the seam that enforces it; the v0 web cockpit is the first instance. Optimizes for surface-neutrality (symmetric to adapter-neutrality), single-source-of-truth (the floor references canonical registries), and non-foreclosure (skins are a reserved seam). Sacrifices immediate visual alignment — mitigated by deferring the mockup pass visibly to a follow-on.
2. **Pinned v0 web cockpit design (rejected).** Specify one visual design for the v0 web cockpit. Contradicts surface-neutrality: it silently privileges one instance and forecloses the operator-customizable-skins direction the operator raised. Also risks scope creep (bootstrapping a design system inside a criteria feature).
3. **Floor only, no named component seam (rejected).** Define the floor but leave the presentation layer implicit. Without a named component layer, "conformant floor" is unenforceable: each surface would re-bind protocol states to presentation independently, and the floor becomes a prose checklist with no structural enforcement. The operator's "how would a web client stack handle this" question is precisely the signal that the seam needs to be named, not left implicit.
4. **Bootstrap the component layer + tokens here (rejected).** Build the design-system pipeline (palette → components → motion) inside this feature. Scope creep that delays the acceptance criteria `feature-pi-parity-checklist` §8 and the epic depend on; the design system belongs at epic/surface-design tier, not inside a criteria feature.

## Implementation Units

### Unit 1: `docs/UX.md` — restructure to surface-neutral floor + v0 instance

**File**: `docs/UX.md` (existing, full restructure)

**Structure**:

1. **Purpose and scope** — UX.md defines the surface-neutral UX conformance floor and the v0 web cockpit as its first conformant instance. State surface-neutrality as a principle (symmetric to adapter-neutrality): surface-specific presentation is a surface-declared feature, not a core UX primitive. State that the floor is registry-derived (references `docs/PROTOCOL.md` canonical registries; does not re-declare them). Note the relationship to `docs/ARCHITECTURE.md` (the operator domain + presentation model) and that a surface is conformant when it meets the floor.
2. **Surface-neutral conformance floor** — the obligations any conformant control surface must meet. Each obligation references the canonical registry in `docs/PROTOCOL.md` by section/anchor rather than re-listing the registry members (re-listing would reintroduce the duplicate-enum drift `feature-command-state-ssot` was created to kill — `docs/UX.md` references rather than redefines protocol state machines). UI labels (e.g. "Live idle", "Stale working") are clearly marked as non-authoritative presentation labels over the protocol axes, not a restatement of the registry:
   - **State presentation honesty.** Present every canonical member of `CommandState`, `SessionConnectivityState`, `SessionActivityState`, and `ElicitationState` **as defined in `docs/PROTOCOL.md`** (Command lifecycle state; Session state axes; ElicitationState lifecycle) without inventing divergent states. Stale/unknown must not be styled as live. Session display composes connectivity × activity; a stale/unknown connectivity value dominates presentation.
   - **Liveness vs delivery separation.** Separate session liveness (connectivity × activity) from command delivery (`CommandState`). Accepted ≠ completed; delivered ≠ completed. (Satisfies acceptance criterion.)
   - **Identity-before-intent.** Show stable target identity (adapter, deployment, runtime session id, session generation) before the operator can submit an Operation. Human-readable labels (project/cwd/name) are metadata, not identity — they must not override verified target identity.
   - **Authority/grant visibility.** Answer the canonical operator question "Who is allowed to control this session or resource?" (`docs/VISION.md`). Action availability is derived from grants + adapter capabilities, but UI availability is never authority (`docs/PROTOCOL.md` Authority grants: "Control surfaces may hide unavailable actions, but UI availability is never authority"). Surfaces must distinguish denial (`authorization_denied`) from unsupported (`unsupported_command`) from revoked, and show operator-visible grant/audit context where needed (current-session/endpoint/adapter revocation, lockdown). Revocation prevents future authority; already-accepted commands follow their attached policy.
   - **Operation affordance coverage.** Every committed v0 `OperationKind` (`spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management` per `docs/PROTOCOL.md` OperationKind registry) is either actionable through an appropriate surface flow or visibly presented as unavailable/unsupported with a canonical reason. Reserved kinds (`agent-send`, `adapter-utility-exec`) are not presented as committed v0 actions. (The composer need not surface every kind — e.g. `spawn`/`attach` may be entry-point actions rather than composer actions — but the surface as a whole must cover them.)
   - **Failure vocabulary mapping.** Map failure text to the protocol failure/outcome vocabulary so timeout, denial, rejection, expiration, cancellation, supersession, and execution failure remain distinct. Show what is safe to retry.
   - **Reconnect reconciliation.** On reconnect, submit cursor and reconcile against snapshots + newer events. An older snapshot is never rendered as live. Reconnect does not rely on wall-clock freshness alone.
   - **Elicitation presentation.** Surface pending Elicitations (approvals, questions) as attention-required state. V0 binds to the operator actor; any authenticated endpoint may answer; first valid answer clears everywhere. The responding endpoint is captured in the response Operation audit. Tighter binding is reserved.
   - **Observation/subscription-stream honesty.** Present Observations from subscription streams but never as authoritative alone; snapshots and core records reconcile. Streams are delivery optimizations.
   - **Terminal-race explanation.** Command timelines can explain terminal races (e.g. Completed-before-cancellation-arrived, Cancelled-before-completion, Expired-before-adapter-completion — examples, not protocol states) without adding protocol states.
   - **No optimistic-state authority.** Optimistic UI state is never authority for command submission, grant status, or session liveness.
3. **Shared presentation-component layer (named seam)** — name the seam (refining `docs/ARCHITECTURE.md:152` "presentation model"). State its obligations: bind canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`, etc.); be skin-able via design tokens; be composable by any conformant surface. Implementation **deferred** — named here as the **future structural enforcement mechanism** that makes the floor machine-checkable and skins possible; bootstrapping it is follow-on surface-design work. Until it is implemented, conformance is enforced by these UX acceptance criteria, protocol references, and later tests/vectors — not by the component layer. Note that `ux-ui-design`'s `components` skill is the mockup-time analog of this layer. (See Risks: the first real web cockpit must not proceed without either the component layer or an explicit conformance-test substitute.)
4. **v0 web cockpit — first conformant instance** — the v0-specific acceptance, framed as the first surface that must satisfy the floor:
   - **Required v0 screens.** Session list; session detail (message timeline + command delivery timeline); composer; Elicitation/attention surface. (Navigation pattern is an instance decision, deferred to the mockup follow-on; the floor requires the screens exist, not their layout.)
   - **Session list visible fields.** machine/deployment, adapter, project/working context when available, session label, model/runtime metadata when available, protocol-derived connectivity/activity status, last authoritative update time.
   - **Session detail / message timeline behavior.** Render Observations (assistant messages, tool calls/results, lifecycle facts) with source authentication and correlation context; render command delivery states distinctly from message content.
   - **Composer requirements.** Submit Operations; the composer surfaces the in-session OperationKinds (`instruct` with prompt payload, `cancel`/`interrupt`, `approval-response`/`elicitation-response`, `query`, `reconfigure`, `session-management`) while `spawn`/`attach` are surfaced as entry-point actions elsewhere (per the Operation affordance coverage obligation). Show local submission state + durable `CommandState`. Show idempotency behavior on retry.
   - **Reconnect/stale/offline banners.** Visible connectivity-state banners; stale view marked until a newer authoritative snapshot/live stream confirms.
   - **Multi-device continuity.** A command sent from phone is visible from laptop; a session inspected from desktop reflects accepted commands and authoritative replies from other surfaces.
   - **Empty/error/loading states.** Explicit states for no sessions, no messages, target-not-found, adapter-unavailable, and failure cases — all using the failure vocabulary.
   - **Mobile-first responsive.** Readable session list on phone; clear target identity before sending; composer ergonomics; low-friction reconnect; minimal reliance on continuous foreground connection; fast switching among sessions.
   - **CLI.** Setup, administration, debugging, scripted access — not a second independent product surface with divergent semantics.
5. **Reserved seams** — operator-customizable skins/layouts ("Codex-style vs Claude-style vs CLI"); design tokens / visual language; the shared presentation-component layer implementation; native/mobile/Expo affordances; push notifications; multi-surface presence-leak prevention.
6. **Rejected directions** — Pi-specific concepts mandatory in the core UI model; mobile-only assumptions in the shared operator domain; treating optimistic UI state as authoritative; hiding accepted/delivered/completed distinctions; a pinned single visual design as the floor; collapsing failure outcomes into a generic "failed".
7. **Anti-patterns** — carry forward the existing anti-patterns list (treating optimistic UI as authoritative; hiding delivery distinctions; stale-working-as-live; labels without identity context; retry without idempotency display; mobile-only assumptions; Pi-specific core UI concepts) and add: inventing divergent state names; rendering a stale snapshot as live; binding Elicitation responses to a specific endpoint rather than the operator actor.

**Implementation Notes**:
- Consume, do not duplicate: every state name and registry value must reference `docs/PROTOCOL.md` as authoritative (by section/anchor), not re-list the members — `feature-command-state-ssot` requires `docs/UX.md` to reference rather than redefine protocol state machines. UI labels ("Live idle", "Stale working", terminal-race examples) are non-authoritative presentation labels, clearly marked as such. If a value diverges, `docs/PROTOCOL.md` is correct.
- Restructure, not append: the current UX.md is web-cockpit-flavored; the restructure makes the floor the frame and the v0 instance a section. **Preserve the existing UX benchmark/quality-bar section** ("UX benchmark" — Remote Pi compatibility floor, Claude-app-style quality bar) by relocating it into the v0-instance section, and preserve the mobile-first content likewise.
- Surface-neutrality is named here but ratifies what `docs/ARCHITECTURE.md` and `docs/VISION.md` already imply; note the cross-reference.
- The presentation-component seam refines `docs/ARCHITECTURE.md:152` "presentation model"; do not contradict it — name the refinement.

**Acceptance Criteria**:
- [ ] UX.md separates session liveness states from command delivery states (floor obligation + v0 instance).
- [ ] UX.md defines v0 required screens and visible fields (v0 instance section).
- [ ] UX text references canonical protocol states rather than maintaining a divergent state list (every state name traces to `docs/PROTOCOL.md`).
- [ ] The web cockpit can be designed without guessing what must be visible before sending a command (identity-before-intent + composer requirements).
- [ ] Surface-neutrality named as a principle; the presentation-component layer named as a seam with deferred implementation.
- [ ] Mockup pass deferred visibly to a reserved seam, not silently skipped.
- [ ] No canonical registry value re-declared; all reference `docs/PROTOCOL.md`.

### Unit 2: Cross-reference in `docs/ARCHITECTURE.md`

**File**: `docs/ARCHITECTURE.md` (existing, "Human control surface plane" / operator-domain section)

**Change**: a one-sentence refinement note at the presentation-model mention (line ~85 / ~152) pointing to UX.md's surface-neutral floor + the named presentation-component seam. No other ARCHITECTURE.md content changed.

**Acceptance Criteria**:
- [ ] Cross-reference present; the high-level positioning stays in ARCHITECTURE.md; the floor + seam detail lives in UX.md.

## Implementation Order

1. Restructure `docs/UX.md` (Unit 1).
2. Add the cross-reference in `docs/ARCHITECTURE.md` (Unit 2).

Single inline implement stride — no child stories. The deliverable is one cohesive doc restructure with cross-section cohesion (surface-neutrality must be consistent across the floor, the seam, and the v0 instance); splitting would add overhead, not parallelism. No code, no build, no coordination → inline, not the orchestrator.

## Testing

No implementation code; verification is by document consistency:

- confirm every state name in UX.md traces to `docs/PROTOCOL.md` (CommandState, SessionConnectivityState, SessionActivityState, ElicitationState, failure vocabulary);
- confirm session liveness (connectivity × activity) is separated from command delivery (CommandState);
- confirm identity-before-intent is stated and project/cwd/name are framed as metadata;
- confirm the failure-vocabulary mapping preserves timeout/denial/rejection/expiration/cancellation/superseded/execution-failure as distinct;
- confirm reconnect reconciliation states cursor + snapshot + never-render-stale-as-live;
- confirm surface-neutrality is named and the presentation-component seam is named with deferred implementation;
- confirm the mockup pass is a visibly deferred reserved seam;
- confirm no canonical registry value is re-declared (only referenced);
- confirm the v0 instance section covers required screens, visible fields, composer, banners, multi-device, empty/error/loading, mobile-first, CLI.

## Risks

- **Floor over-specification forecloses skins.** If the floor mandates visual/interaction detail, it contradicts surface-neutrality. Mitigation: the floor is behavioral + state-binding only; visual design is explicitly surface-declared.
- **Presentation-component seam left too vague to enforce.** If the seam's obligations aren't concrete, "conformant floor" stays unenforceable. Mitigation: state the obligations (bind canonical states; skin-able via tokens; composable by any surface) even though implementation is deferred.
- **Conformance unenforceable until the component layer exists (review finding I3).** The floor's structural enforcement depends on a deferred component layer; until it is implemented, conformance is enforced only by UX acceptance criteria + protocol references. The first real web cockpit must not proceed without either the component layer or an explicit conformance-test substitute (e.g. a UX conformance vector/checklist that gates the web cockpit's stage:review). Mitigation: state this as a required follow-up (see Reserved follow-up); a future UX conformance vector is the test substitute if the component layer is not yet built.
- **v0 instance drifts from the floor.** The v0 web cockpit section could accidentally specify something the floor doesn't require (or contradict it). Mitigation: frame every v0-instance item as "satisfies floor obligation X" where applicable.
- **UX.md restructure churn.** A full restructure risks losing content (benchmark, mobile-first, anti-patterns). Mitigation: preserve and relocate, don't delete — the implementation notes explicitly preserve the UX benchmark/quality-bar section.
- **Surface-neutrality is a new named principle; downstream features may not honor it yet.** Mitigation: the cross-reference in ARCHITECTURE.md and the reserved-seams list make it discoverable; the central `feature-extension-seams-non-foreclosure` sweep will consolidate it.
- **Surface-neutrality is a new named principle; downstream features may not honor it yet.** Mitigation: the cross-reference in ARCHITECTURE.md and the reserved-seams list make it discoverable; the central `feature-extension-seams-non-foreclosure` sweep will consolidate it.

## Reserved follow-up (not this feature)

- **v0 web cockpit mockup pass — v0 surface-design follow-up, required before web cockpit implementation.** Design the first conformant instance (session list, session detail/timeline, composer, Elicitation surface, navigation pattern) via `ux-ui-design:screens`/`flows` + the design-system pipeline (palette → components → motion → screens). This is v0 work (the web cockpit is v0 per `docs/SPEC.md`), but it is **not this feature**: this feature defines the conformance contract; the mockup pass designs the first instance that satisfies it. It must land before the web cockpit reaches `stage: review` — either the shared presentation-component layer is built first, or a UX conformance vector/checklist gates the web cockpit (see Risks). The navigation-pattern decision (scope item) is tracked here and resolved in that follow-on.
- **Shared presentation-component layer implementation** — bootstrap the component library that binds canonical states to skin-able primitives. Follow-on; the seam is named here, the build is deferred. This is the structural enforcement mechanism for the floor.
- **UX conformance vector/checklist** — the test substitute for the component layer if it is not yet built; gates the web cockpit's `stage:review`. Reserved.
- **Operator-customizable skins/layouts** — the "Codex-style vs Claude-style vs CLI" direction. Reserved seam above the floor.

## Review (advisory design review, 2026-07-06)

**Verdict**: (pending cross-model review)

**Reviewer**: fresh-context cross-model deep review on `openai-codex/gpt-5.5` (high thinking), dispatched by the umans orchestrator after the surface-neutrality + presentation-component-seam reframing (a meaningful shift from the original brief, warranting a fresh-context adversarial pass before implementation). Advisory pass on the design body; no implementation existed yet; no substrate side-effects, no stage change.

(Review record to be appended when the reviewer returns.)

## Review (advisory design review, 2026-07-06)

**Verdict**: Block → fixed → proceeding to implement

**Reviewer**: fresh-context cross-model deep review on `openai-codex/gpt-5.5` (high thinking), dispatched by the umans orchestrator after the surface-neutrality + presentation-component-seam reframing (a meaningful shift from the original brief). Advisory pass on the design body; no implementation existed yet; no substrate side-effects, no stage change.

**Blocker** (fixed in-stride):
- **Design re-declared canonical state registries while saying not to.** Unit 1 told the implementer to list concrete `CommandState`/`SessionConnectivityState`/`SessionActivityState`/`ElicitationState` members in UX.md, while the same design said "No canonical registry value re-declared" and `feature-command-state-ssot.md:48` requires `docs/UX.md` to reference rather than redefine protocol state machines (the SSOT feature was created to kill exactly this duplicate-enum drift). Fixed: the floor obligations now reference `docs/PROTOCOL.md` by section/anchor ("present every canonical member of CommandState as defined in PROTOCOL") rather than re-listing members; UI labels ("Live idle", terminal-race examples) are explicitly marked non-authoritative presentation labels over the axes.

**Important findings** (all fixed in-stride):
- **I1 — conformance floor lacked authority/grant visibility.** Added a floor obligation: answer "Who is allowed to control this session or resource?" (`docs/VISION.md:22`); action availability derived from grants + adapter capabilities but UI availability is never authority (`docs/PROTOCOL.md:494`); distinguish denial (`authorization_denied`) from unsupported (`unsupported_command`) from revoked; show operator-visible grant/audit context.
- **I2 — Operation affordance coverage incomplete.** Added a floor obligation: every committed v0 `OperationKind` is either actionable through an appropriate surface flow or visibly unavailable with a canonical reason; reserved kinds not presented as committed. Composer note clarified: `spawn`/`attach` are entry-point actions, not composer actions.
- **I3 — presentation-component seam overstated enforceability while deferring the enforcer.** Rephrased: the named layer is the *future structural enforcement mechanism*; until implemented, conformance is enforced by UX acceptance criteria + protocol references + later tests/vectors. Added a Risk + follow-up: the first real web cockpit must not proceed without either the component layer or an explicit conformance-test substitute (UX conformance vector).
- **I4 — mockup pass misclassified as "not v0".** The web cockpit is v0 (`docs/SPEC.md`); the mockup pass is "not this feature," not "not v0." Reclassified as "v0 surface-design follow-up, required before web cockpit implementation"; navigation-pattern decision tracked there.

**Nits** (both applied):
- **N1** — "Observation/substream honesty" → "Observation/subscription-stream honesty".
- **N2** — Unit 1 structure now explicitly preserves the current UX benchmark/quality-bar section (relocated, not deleted), per the implementation notes.

**Notes**: Reviewer confirmed the surface-neutrality reframing is directionally sound and symmetric to adapter-neutrality, and the presentation-component seam aligns with `docs/ARCHITECTURE.md:85`/`:152`. The overall reframing survived; the SSOT blocker was the load-bearing fix. No implementation existed to review.

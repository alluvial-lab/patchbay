---
id: feature-v0-web-cockpit
kind: feature
stage: drafting
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-web-server, feature-v0-presentation-component-layer, feature-v0-elicitation-response-contract, feature-v0-approval-response-contract]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-20
---

# Feature: Responsive web cockpit

## Brief

Build the responsive web cockpit — the operator's primary control surface and the v0.1.0 product center. This is the "better than terminal" phone experience: session list with liveness/delivery badges, composer for sending prompts and instructions, command delivery timeline with failure states, and reconnect/stale/offline banners. The quality benchmark is Claude-app-style remote control continuity.

The cockpit runs the shared TypeScript operator domain in the browser (protocol client, delivery/reconnect state machines, presentation model) as a client of the web server. It must meet the surface-neutral UX conformance floor defined in `docs/UX.md`: present every canonical protocol state honestly, separate session liveness from command delivery, show stable target identity before allowing submission, and distinguish denial from unsupported from revoked.

The cockpit is the first conformant instance of the conformance floor. Mockups are deferred to a named follow-on per the UX design decision (mocking inside the criteria feature would silently privilege one visual instance and work against surface-neutrality); this feature's `feature-design` pass picks up the mockup work here, inheriting the design-system pipeline (`palette` → `components`) from `feature-v0-presentation-component-layer`.

## Mockups

- Design system: `.mockups/design-system/` (see `feature-v0-presentation-component-layer`)
- Shell screen: `.mockups/screens/feature-v0-web-cockpit/`
  - **Selected: option-2 (Identity-forward)** — locked 2026-07-16. Generous session rows (label + project dominant, identity + status as metadata), sidebar header actions for spawn/attach, filter search.
  - **v0.1.0 scope: sessions shell only.** Operator decision (2026-07-17): pare down to just the sessions shell for v0.1.0 to hone it in. The Attention destination is **deferred from v0.1.0** — elicitations surface inline in the session detail (cards + mobile bottom sheet) and via the `needs-you` badge on session rows, so they remain discoverable and answerable without a dedicated cross-session inbox. The cross-session Attention destination is the natural promotion when monitoring many sessions; its mock (`attention/attention.html`) is preserved on disk as a designed-but-deferred artifact, not wired into the nav.
  - **Responsive IA (committed A / reserved B):** desktop is two-pane (list + live session-detail side-by-side); mobile is drill-in (list is the home, tap a session → full-screen detail with back button). B (drill-in) is both the reserved seam AND the natural mobile mode — promotion to desktop drill-in is additive (container change), not a rebuild.
  - Detail-pane header hidden on desktop (redundant with the active sidebar row); kept on mobile (drill-in needs back button + which-session context).
  - option-2.html is self-contained (inlined tokens+components) and interactive (mobile drill-in works via tap/back).
  - **Session detail (folded into the shell's right pane):** chat-aligned timeline (operator right / agent left, capped 560px left-side content width), markdown rendering in agent bubbles (the mobile-readability differentiator), delivery state as a compact badge below each message (tap to expand full state history + LSNs as debug detail), binary approval = direct buttons (no option-list), multi-option question = select-one radio + free-text option + answer-and clarification, grouped multi-question card (N independent single-answer Elicitations as one visual card — v0.1.0-compatible; multi-answer contract is a reserved seam). Mobile: bottom-sheet for elicitations (clones the tapped card's real content), fixed composer, page scroll. Teaser previews on mobile: clamped prompt + 'Tap to answer' affordance; multi-question shows header + first question + count hint.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: the end of the phone-usable critical path (core → protocol-seam → web-server → web-cockpit). This is the layer that makes the operator's phone piloting real.

## Foundation references

- `docs/UX.md` — surface-neutral conformance floor, v0 web cockpit instance, required screens and fields, delivery-state separation, reconnect/stale/offline banners
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain, presentation model
- `docs/PROTOCOL.md` — CommandState, SessionConnectivityState, SessionActivityState, ElicitationState, failure vocabulary
- `docs/SPEC.md` — v0.1.0 performance posture (qualitative responsiveness floor: "feels responsive under normal single-operator use")
- `feature-ux-v0-acceptance` (done) — the UX conformance floor design this feature implements

## Architectural choice

A browser-side TypeScript cockpit (`patchbay-web-cockpit`) that runs the shared operator domain as a client of the `patchbay-web-server` thin translator. The browser holds the presentation model and the delivery/reconnect state machines; the web server only terminates HTTP + auth/CSRF and proxies Connect-Web calls to the core's gRPC `ControlService`. This realizes the committed v0.1.0 topology (Q5a — thin translator, operator domain browser-only) and the reserved-seam posture (server-side operator-domain promotion stays reserved).

The cockpit consumes the locked presentation-component layer (`feature-v0-presentation-component-layer`: `tokens.css` + `components.css` primitives) for all state-binding — it does not re-bind protocol states to presentation. The mockup-locked shell (`option-2.html`) is the visual reference; the implementation translates it into real components driven by live protocol state.

## Design decisions (feature-design, 2026-07-18)

Resolved interactively during the mockup pass; pinned here so implementation does not re-litigate them.

- **Q1 — Two-pane desktop / drill-in mobile (committed A / reserved B).** Desktop is two-pane (list + live detail side-by-side); mobile is drill-in (list home, tap → full-screen detail + back). B (drill-in) is both the reserved seam and the natural mobile mode; desktop drill-in promotion is additive.
- **Q2 — Delivery state as a compact badge below the message.** Not a separate timeline strip and not above the message. The badge shows current `CommandState` compactly; tap expands the full state history + LSNs as a debug detail (LSNs hidden by default — not conversational noise). Terminal-race explanations render as UI labels, not protocol states.
- **Q3 — Chat alignment (operator right / agent left).** Conventional chat-app affordance; position carries most of the speaker signal, the `who` label is secondary. Conversation column capped at 860px centered; left-side content capped at 560px for a clean right edge.
- **Q4 — Composer is text-first + contextual actions.** Default input is a prompt textarea (instruct). Attach button for files/images (the `file_attachment` reserved-contract surface). Cancel/Interrupt appear inline near a running command; Approve/Deny and question-answer surface inline as elicitation cards where the agent opened them. No composer-level OperationKind selector — actions appear where relevant.

### Response-contract-shape decisions (surfaced during elicitation mock review)

These three touch `response_contract` validation semantics and were grounded against `docs/PROTOCOL.md` before deciding.

- **EC1 — Free-text option within a `question` contract: v0.1.0 committed.** A `select-one`/`select-many` question may append a free-text option ("or type your own answer"). The response Operation carries the free-text string instead of a selected option id. This is a `free-text` ui_hint within the committed `question` contract_kind — no contract-kind promotion. The control shape matches the `ui_hint` (radio for select-one including the free-text alternative, checkbox for select-many).
- **EC2 — "Answer-and" composed response (structured selection + free-text clarification): v0.1.0 committed.** A question response may carry a selected option *plus* an appended free-text clarification in one Operation (the "And..." field). This is a response-payload shape on the `question` contract, not a new contract_kind. The clarification is supplementary context; the structured selection remains the primary answer.
- **EC3 — Grouped multi-question (N independent single-answer Elicitations as one visual card): v0.1.0 committed as the grouping; the multi-answer contract is reserved.** Claude's nested multi-question maps to N independent Elicitations opened as a batch, rendered as one visual card, each independently single-answer and independently terminal. This keeps every Elicitation single-answer (committed v0.1.0). A true multi-answer contract (one Elicitation carrying multiple questions) is a reserved seam ("multi-answer accumulation", PROTOCOL:312) — promotion is a clean reserved-seam reversal, not a quiet gap.
- **EC4 — Attention destination deferred from v0.1.0.** Elicitations surface inline in the session detail + via the `needs-you` badge. The cross-session Attention destination is deferred; its mock is preserved. Promotion is additive when monitoring many sessions.

## Implementation Units

### Unit 1: Operator-domain core — protocol client + state machines

**File**: `web-cockpit/src/domain/protocol-client.ts`, `web-cockpit/src/domain/reconcile.ts`

The browser-side operator domain. A typed Connect-Web client to `ControlService` (Submit / Subscribe / LoadSnapshot) over the web-server's Connect-Web bridge. Holds the cursor-based reconnect state machine: subscribe with last-known cursor, fold incoming `SubscribeEvent`s into the presentation model, reconcile against `LoadSnapshot` on reconnect gaps. Reconnect submits the cursor + reconciles against snapshots/core records — never optimistic UI state (the snapshot-correctness rule).

```typescript
// web-cockpit/src/domain/protocol-client.ts
import { createClient, type Transport } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";
import { ControlService } from "@patchbay/contracts";

export function createProtocolClient(): { client, transport } {
  const transport = createConnectTransport({ baseUrl: "/" }); // same-origin via web-server bridge
  const client = createClient(ControlService, transport);
  return { client, transport };
}
```

```typescript
// web-cockpit/src/domain/reconcile.ts — cursor-based reconnect
export class Reconciler {
  private cursor: bigint = 0n; // last applied LSN
  constructor(private client: ControlServiceClient) {}

  // Subscribe with cursor; on reconnect, LoadSnapshot then re-Subscribe.
  async *subscribe(domainId: AuthorityDomainId): AsyncIterable<SubscribeEvent> {
    while (true) {
      try {
        for await (const ev of this.client.subscribe({ authorityDomainId: domainId, cursor: this.cursor })) {
          this.cursor = ev.eventId!.lsn; // advance on fold
          yield ev;
        }
      } catch (e) {
        // stream broke — reconcile against snapshot before re-subscribing
        await this.reconcile(domainId);
      }
    }
  }
  private async reconcile(domainId): Promise<void> { /* LoadSnapshot at-or-before cursor; fold; mark unreconciled stale */ }
}
```

**Implementation Notes**:
- Cursor is the last *folded* LSN, advanced only after the presentation model applies the event. Reconnect resumes from there.
- A gap (missed events) is detected when the stream resumes at an LSN > cursor+1; reconcile via `LoadSnapshot` rather than synthesizing state.
- Unreconciled state is marked stale/unknown per the degraded-behavior rules — never rendered as live.

**Acceptance Criteria**:
- [ ] Subscribe folds events into the presentation model; cursor advances on fold
- [ ] Reconnect after a stream break re-subscribes from the last cursor without losing applied state
- [ ] A snapshot gap is reconciled via LoadSnapshot; unreconciled axes render stale/unknown
- [ ] Optimistic UI state is never authority for the cursor or the presentation model

### Unit 2: Presentation model — the session/command/elicitation projections

**File**: `web-cockpit/src/domain/model.ts`

The in-browser presentation model: a fold over `StoredEventPayload` events that produces the view state (sessions with connectivity×activity axes, commands with CommandState, pending elicitations). This is the browser-side analog of the core's `SessionRegistry` / `CommandIndex` — a pure projection, never authoritative. Binds to the presentation-component layer's primitives for rendering.

```typescript
// web-cockpit/src/domain/model.ts
export interface SessionView {
  identity: SessionIdentity; // adapter/scope/runtime/gen — identity-before-intent
  label: { project?: string; cwd?: string; name?: string };
  connectivity: SessionConnectivityState;
  activity: SessionActivityState;
  activityDetail?: string; // Observation-composed (Option C) — thinking/executing/waiting
  needsYou: boolean; // waiting for command OR pending elicitation
  lastUpdate: Date;
}
export interface CommandView { id: CommandId; state: CommandState; lsn: Lsn; race?: string; }
export interface ElicitationView { id: ElicitationId; kind: "approval"|"question"; state: ElicitationState; contract: ResponseContract; prompt: string; options?: Option[]; }
export interface PresentationModel {
  sessions: Map<SessionIdentity, SessionView>;
  commands: Map<CommandId, CommandView>;
  elicitations: Map<ElicitationId, ElicitationView>;
}
export function fold(model: PresentationModel, ev: StoredEventPayload): PresentationModel { /* pure fold */ }
```

**Implementation Notes**:
- The fold mirrors the core's projection semantics (registry observe + command-index observe) but is read-only — it never writes back. Reconnect reconciliation replaces the model from a snapshot.
- `activityDetail` (Option C) is composed from the Observation stream (`tool_call`, `tool_execution_start/end`, `message_update`, `agent_end`, `turn_start/end`) — an ephemeral presentation hint, not a durable state. The durable `activity` stays `working`/`idle`.
- `needsYou` is derived: a session is needs-you if its last command is terminal-and-awaiting-input OR it has a pending elicitation.

**Acceptance Criteria**:
- [ ] fold is a pure function over (model, event) → model
- [ ] stale/unknown connectivity never renders as live (dominance rule enforced in the view binding)
- [ ] activityDetail composes from Observations but does not mutate durable activity state
- [ ] Reconnect replaces the model from a snapshot; the old model is never rendered as live during reconciliation

### Unit 3: Markdown rendering (the mobile-readability differentiator)

**File**: `web-cockpit/src/ui/markdown.ts`

Renders agent Observation payloads (markdown) into the message timeline with excellent mobile readability — the v0.1.0 hard requirement. Headings, paragraphs, lists, tables, blockquotes, inline code, fenced code blocks with sane horizontal scroll (not layout-breaking). This is where the differentiator lives.

**Implementation Notes**:
- Use a small, safe markdown renderer (e.g. `marked` + `DOMPurify` for sanitization, or a streaming-friendly parser). The payload is source-authenticated but still untrusted at the render boundary — sanitize.
- Code blocks: `overflow-x: auto` on `<pre>`, never `overflow: hidden` (which breaks long lines). The mock's `pre` treatment is the reference.
- Tables: horizontal scroll wrapper on narrow viewports; never let a wide table break the chat column.
- Typography uses the locked Plex Sans body face (from tokens.css); code uses Plex Mono.

**Acceptance Criteria**:
- [ ] Markdown renders headings, lists, code blocks, tables, blockquotes, inline code on a 360px viewport without horizontal page-scroll
- [ ] Code blocks scroll internally, not the page
- [ ] Rendered output is sanitized (no unescaped HTML injection)
- [ ] Long content does not break the chat column width

### Unit 4: Elicitation handling (the three shapes + mobile sheet)

**File**: `web-cockpit/src/ui/elicitation.ts`

Implements the three elicitation shapes (EC1–EC3) and the mobile bottom-sheet. Binary approval = direct buttons; multi-option question = radio/checkbox + free-text option (EC1) + answer-and clarification (EC2); grouped multi-question = N independent single-answer Elicitations as one card (EC3).

**Implementation Notes**:
- The response Operation is built from the selected option (or free-text) + optional clarification, correlated to the `ElicitationId`. First-answer-wins is enforced core-side; the UI disables the controls once the elicitation terminalizes (answered/declined/expired) and shows the terminal state.
- The mobile bottom sheet clones the tapped card's content (per the locked mock behavior) and force-shows the options/actions that the inline-teaser CSS hides.
- Control shape matches `ui_hint`: radio for select-one (including the free-text alternative), checkbox for select-many. Never mix in one elicitation.

**Acceptance Criteria**:
- [ ] Binary approval submits Deny/Approve directly (no select-then-submit)
- [ ] Question with free-text option submits either a selected option id or a free-text string
- [ ] Answer-and submits a selected option + appended clarification in one Operation
- [ ] Grouped multi-question renders N questions as one card; each answers independently
- [ ] Once terminal, the elicitation controls disable and show the terminal state

### Unit 5: Shell + session list + responsive layout

**File**: `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/session-list.ts`, `web-cockpit/src/ui/session-detail.ts`

The responsive shell: desktop two-pane (list + live detail), mobile drill-in (list home, tap → full-screen detail + back). Session rows show identity-before-intent (identity tuple primary, labels metadata), connectivity×activity badges (separate, per the split), and the needs-you state. The detail pane carries the message timeline + delivery badges + composer.

**Implementation Notes**:
- Uses the presentation-component layer primitives (`.session-row`, `.session-status`, `.connectivity-indicator`, `.activity-indicator`, `.composer`, `.elicitation-card`, etc.) from `components.css` — no inline protocol-state rebinding.
- The drill-in (mobile) is a container swap, not a separate screen — the session-detail content component is identical in both modes (the reserved B seam).
- Composer: textarea + attach + send; contextual actions (Cancel/Interrupt) appear near running commands.

**Acceptance Criteria**:
- [ ] Desktop: list + detail side-by-side; selecting a session fills the detail pane
- [ ] Mobile: list is home; tap drills into full-screen detail; back returns to list
- [ ] Session rows show identity tuple + connectivity/activity (separate channels) + needs-you state
- [ ] All state-binding uses the presentation-component layer primitives

## Implementation Order

1. Unit 1 (protocol client + reconcile) — the foundation; nothing runs without it
2. Unit 2 (presentation model) — the fold the UI renders
3. Unit 3 (markdown rendering) — the differentiator; can parallelize with 4 once 2 lands
4. Unit 4 (elicitation handling) — can parallelize with 3
5. Unit 5 (shell + UI) — composes 2/3/4 into the locked shell

## Testing

- **Interface tests**: the fold (Unit 2) is a pure function — property-test it against event sequences (generation monotonicity, stale-never-live, reconnect reconciliation). The protocol client (Unit 1) — reconnect/resume behavior against a fake transport.
- **Regression tests**: markdown rendering on a 360px viewport (the differentiator); elicitation submission shapes (EC1 free-text, EC2 answer-and, EC3 grouped).
- **Unit tests**: elicitation control-shape matching ui_hint (radio vs checkbox); needs-you derivation.
- **Test removal**: none anticipated — greenfield.

## Risks

- **Markdown renderer choice** — must be small + safe + streaming-friendly. A heavy parser bloats the bundle; an unsafe one is an XSS vector despite source authentication. Spike the choice in Unit 3.
- **Reconnect correctness** — the snapshot-correctness rule is load-bearing. If the reconciler ever renders an unreconciled snapshot as live, that's a conformance-floor violation. Property-test the reconcile path.
- **Elicitation payload shapes (EC1–EC3)** — these are new response-contract shapes not yet in the proto. Implementation must extend the `elicitation-response` Operation payload schema; coordinate with the contracts crate. If the proto extension is non-trivial, surface as a blocker (semantic 50/50 per the harness rule).

## Simplification

- Dropped the standalone detail mock — folded into the shell (one coherent product mock, no competing paths).
- Deferred the Attention destination — elicitations surface inline + via needs-you badge; the cross-session inbox is a future promotion.
- No composer-level OperationKind selector — actions surface contextually where relevant, keeping the composer simple.

## Implementation discovery (2026-07-19)

Implementation returned to `drafting` before code was written because Unit 4's binary approval requirement cannot be represented by the current generated contract and core semantics.

- The shipped question-response work is usable: `QuestionContract`, `ResponseOption`, and `ElicitationResponsePayload` can represent select-one/free-text plus optional clarification.
- No `ApprovalResponsePayload` exists in `contracts/proto/patchbay/`, the generated TypeScript bindings, or the generated Rust bindings. The completed dependency's design body refers to an “existing” `ApprovalResponsePayload`, but repository-wide search finds that name only in that prose.
- `core/src/acceptance/elicitation_response.rs` accepts any `APPROVAL_RESPONSE` matched to an approval contract without decoding a decision payload. Therefore Approve and Deny cannot be built as distinct, boundary-valid Operations.
- `core/src/acceptance/elicitation.rs` maps every completed response Operation to `ElicitationState::Answered`; its own comment says mapping approval denial to `Declined` is deferred. Sending an ad-hoc text/JSON decision from the cockpit would be unvalidated and would not produce the required denial lifecycle semantics.
- `pi-adapter/src/delivery.ts` currently reports both approval-response and elicitation-response as `unsupported_command`, so an ad-hoc browser-only payload convention would not gain an authoritative consumer at the adapter boundary.

This is a protocol/safety design gap, not a mechanical TypeScript choice. Resolving it requires a contract decision for the binary approval decision (and corresponding core validation/terminal mapping and adapter delivery), or an explicit scope change that removes Deny from v0.1.0. Those options produce materially different protocol behavior, so the implementation worker did not choose between them silently.

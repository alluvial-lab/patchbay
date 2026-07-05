## Session bank — 2026-07-05 (Operation/Observation/Elicitation frame adversarial review)

**Read alongside `2026-07-04-frame-adversarial-review.md` and
`2026-07-04-action-inventory-reframe.md` before continuing
`feature-operator-presence-and-action-inventory`.**

### How this was produced

Second fresh-context adversarial review dispatched on `openai-codex/gpt-5.5` (thinking
high), 67 tool uses / 211.9k tokens. This pass **read the research corpus** (the
synthesis, verification-checklist, all 6 specialist briefs, the 4 cross-corpus
pointer-attestations, the SNC landscape brief, the deployed piloting guide, and selected
per-harness attestations) — closing the scoping gap from the first review. Briefed to
adversarially review the Operation/Observation/Elicitation frame (Frame D) against the
surveyed evidence, with 5 mechanization proposals (P1–P5) carried as labeled targets to
attack rather than ratify.

### Bottom-line verdict: SURVIVES WITH AMENDMENTS

Operation/Observation/Elicitation is a sound frame for the consuming feature's design
pass **if** three amendments land. As stated, P4 and P5 are too narrow, P1 is
under-justified, and the frame's biggest unaddressed gap is presence/subscription/
attention routing.

### The three load-bearing amendments

**A1 — The frame must be actor-neutral, not operator-centric.** As stated, "Elicitation
= a question/request *to the operator*" forecloses the future directions the operator
wanted preserved (agent→agent mesh, op→op, adapter→core service requests). Evidence this
matters *today*, not just for the future:
- Antigravity **triggers** are non-operator-originated drive actions (`automated_trigger`)
- Codex has **server→client service requests** (auth-token refresh, attestation, current-time read) that aren't "questions to the operator"
- remote_pi mesh `agent_send`/`agent_request` is agent→agent

Fix: Operation = "authorized control-plane request **by any actor to any actor/core/adapter
target**"; Elicitation = "pending response solicitation **from one actor to another**."
Same primitives, generalized language. The non-foreclosure seam depends on this.

**A2 — Replace `response-shape` with `response_contract` (P4 refuted as stated).** The
proposed `select-one`/`select-many`/`free-text` smuggles in a UI-widget assumption ("agent
questions are multiple-choice-ish"). Surveyed surfaces that don't fit: approval-with-
policy-amendments, modified tool input, secrets, custom function results, auth-token
refresh. Fix: a `response_contract` registry of contract kinds (approval, question,
secret, freeform, function_result, file/attachment, structured_schema, service_request)
with UI hints as an optional sub-field. Genuinely non-foreclosing.

**A3 — `ElicitationState` lifecycle needs more terminals (P5 partial).** Proposed
`opened→pending→answered|expired|cancelled` is incomplete. Missing terminals from real
harnesses:
- `declined`/`rejected` by operator (OpenCode `question.rejected`, approval denials)
- `withdrawn` by opener (distinct from operator cancel)
- `superseded` (newer question replaces older)
- `stale`/`orphaned` (target session gone)
- Invalid-answer handling (response Operation rejected vs. Elicitation terminalized)

### Verification posture (decisive axis) — frame preserves it IF docs are honest

- `OperationState` can't claim checked model authority until either `CommandState` is
  renamed or a documented refinement mapping is written. Safe path: reuse `CommandState`
  registry by equivalence, update names/models/contracts together.
- **`ElicitationState` is a new model obligation.** If v0 treats agent-originated
  questions/approvals as product behavior, the lifecycle + correlation + timeout/
  cancellation needs a promoted model and conformance vectors. Real new verification work.
- `TypedCorrelation` must be extended: response Operation → Elicitation isn't covered by
  `reply_correlation.qnt` today.
- `authority.qnt` must be promoted before grant semantics broaden (confirmed from first
  review).

Question-type-layered authority preserved **only if** prose may introduce the vocabulary,
but invariants are not stated as checked until models are promoted.

### Mechanization-proposal verdicts

- **P1 (fifth id space): PARTIAL.** Not proven necessary — Elicitation could be a typed
  `ReplyId` subkind instead. Either works; must choose and model it. `TypedCorrelation`
  needs extending regardless.
- **P2 (uniform lifecycle for reads): PARTIAL / MOSTLY SOUND.** `running` is unnatural
  for many reads (OpenCode `status`, Codex `thread/list/read`, Cursor list/read, Pi
  `session_sync`, Aider `/tokens`/`/settings`). Fix: one registry, per-kind transition
  subsets — reads may go `accepted→completed` without `running`.
- **P3 (response is Operation with TypedCorrelation): PARTIAL.** Not every answer is
  authority-bearing like a drive (Claude `AskUserQuestion` is informational, not a grant).
  "No answer" isn't an Operation; it's an Elicitation terminal. Need
  `OperationKind = elicitation_response` with appropriate authority semantics + extended
  typed references.
- **P4 (response-shape): REFUTED AS STATED → `response_contract` (A2).**
- **P5 (ElicitationState): PARTIAL → needs more terminals (A3).**

### The frame-specific blind spot (not addressed by P1–P5)

**Presence / subscription / attention routing.** Attach is distinct from operate because
it establishes connection + reconciles state; remote_pi presence is pull-based
`list_peers`; Claude Remote Control has push notifications for "action required";
Cursor/OpenCode expose SSE streams. The frame can carry presence facts as Observations,
but it doesn't *name* endpoint-availability vs. actor-presence vs. subscription vs.
attention-required state. Reviewer suggests Presence/Subscription as a protocol section
or registry (not necessarily a fifth primitive).

### Per-harness coverage proof (the frame fits all 7)

| Harness | O/O/E fit | Notes |
|---|---|---|
| Pi | fits | Operations: drive, approve, cancel, sync, session mgmt, reconfigure. Elicitation: tool approval gate. No distinct freeform agent question in surveyed wire types. |
| Claude Code | fits | Elicitation: tool permission + `AskUserQuestion`. Important: `AskUserQuestion` answer is informational, not a grant. |
| Codex | fits **only if Elicitation is actor-neutral** | Server→client service requests (currentTime/read, auth refresh, attestation, MCP elicitation, dynamic tool calls) are not simply "agent asks operator a question." |
| Cursor | fits | Cloud Agents specifically don't ask approval (dedicated machines). |
| OpenCode | fits | Elicitation: permission + question gates. `serve`+attach = spawn/attach prior art (via SNC). |
| Aider | fits | No explicit spawn/retire operator action; no strong structured external elicitation surface. |
| Antigravity | fits **only if Operation sender is not always human** | Triggers are non-operator-originated drive actions. |

### Non-foreclosure proof (the seam holds, with A1)

- Agent→operator uncodified ask: `Elicitation(sender=agent, recipient=operator, response_contract=...)` + `Operation(kind=elicitation_response, correlates_to=elicitation_id)`.
- Agent→agent mesh: `Operation(kind=agent_send, sender=agent, recipient=agent)` + `Observation(kind=delivery_ack)` + later correlated reply. `list_peers` = query Operation returning presence snapshot/Observation. **No new primitive needed.**
- Agent→agent task spawn (Antigravity subagents): Operation from agent/harness to agent/harness, Observations for subagent lifecycle.
- Op→op future: informational human-to-human = Operation (if routing/audit-authorized) delivering an Observation to recipient; or opens an Elicitation if response required.
- Non-request broadcast/stream: source-authenticated Observations; no request/response assumption.

Non-foreclosure failure risk: proposal text repeatedly said Elicitation is a
"question/request to the operator." That forecloses agent→agent, adapter→core, and
service-request variants unless amended to "pending response solicitation from one actor
to another" (A1).

### Minimal amendments carried forward to the design pass

1. Define primitives precisely (actor-neutral):
   - **Operation** — authorized control-plane request by any actor to any actor/core/adapter target. May be side-effecting, read-only, or response-submitting.
   - **Observation** — source-authenticated fact/event/output/status emission; no grant to act, but source identity/correlation required.
   - **Elicitation** — durable pending response solicitation from one actor/system component to another, with response contract, timeout/cancellation/withdrawal, correlation, and terminal state.
   - **Payload** — adapter-specific content or schema-bound body inside the above.
2. Add **Presence/Subscription** as a protocol section or registry (not necessarily a primitive).
3. Add `OperationKind` registry: spawn/attach/drive/cancel/interrupt/query/approval-response/elicitation-response/reconfigure/session-management.
4. `response_contract` registry (A2) — not `response-shape`.
5. `ElicitationState` with full terminal set (A3): `opened/pending → answered | declined | expired | cancelled | withdrawn | superseded | stale`, with first-durable-terminal-commit finality.
6. Promote/extend models before claiming product semantics:
   - `CommandState` → `OperationState` refinement/rename (with documented equivalence);
   - new `ElicitationState` model;
   - extended `TypedCorrelation` (Operation → Elicitation);
   - promoted `authority.qnt`.

### Open questions still gating the design pass

- Elicitation ids: new id space, or typed `ReplyId` subkind? Both work; docs must choose and model it.
- Is spawn v0 behavior or a reserved seam? (extension-pressure decision, flagged in feature body.)
- Should service requests like Codex `currentTime/read`, auth refresh, attestation be Elicitations or a separate `ServiceRequest` Operation kind? (Reviewer: generalized Elicitation can cover them, but the name may mislead.)
- `.proto` boundaries for response contracts — no generated contract exists yet.
- Multi-operator policy is out of v0, but Elicitation must not bake in single-responder assumptions.

### Relationship to the prior reframe (Frame B) and the operator's clarification

The operator clarified B's intent: forward-flexibility / non-foreclosure, not a normative
model layer. Frame D (Operation/Observation/Elicitation) achieves B's intent more
honestly than B did: extensibility lives in purpose-built primitives' internal structure
(actor-neutral fields, `response_contract` registry, `OperationKind` registry, reserved
future shapes) rather than in a universalizing `Message+force` layer that forces
everything into a force taxonomy and creates the verification-inversion risk. D preserves
what the operator wanted from B without B's costs.

---

## Full review (fresh-context, openai-codex/gpt-5.5, thinking high)

### 1. Grounding summary

Load-bearing facts verified:

- Current protocol is still **Command/Message/Reply** framed:
  - Four id spaces: command/message are client-generated, reply is adapter/core-generated, event is core LSN (`docs/PROTOCOL.md:41-45`).
  - `Command` is "operator intent that may cause external action" and requires target, grant, kind, payload validation, expiration/cancellation semantics (`docs/PROTOCOL.md:55-63`).
  - `Message` is informational and may ask for reply but does not grant authority (`docs/PROTOCOL.md:49-51`).
  - `Reply` correlates to prior command/message in the same authority/session context (`docs/PROTOCOL.md:65-67`).
- `CommandState` is the checked lifecycle registry for accepted commands (`docs/PROTOCOL.md:73-109`); terminal finality and first-durable-terminal-wins are model-backed in `command_lifecycle.qnt` (`specs/seed/command_lifecycle.qnt:19-21`, `:53-70`, `:123-253`).
- `TypedCorrelation` is checked, but only for replies to known command/message ids, not for response-to-elicitation (`docs/VERIFICATION.md:34`; `specs/seed/reply_correlation.qnt:18-25`, `:143-162`).
- Authority safety is v0-required in prose (`docs/VERIFICATION.md:30`, `:148-158`), but `authority.qnt` is explicitly draft/stated-normative, with `GrantAuthorityIsCommandKinds` not promoted (`specs/seed/authority.qnt:1-9`, `:312-388`; `docs/VERIFICATION.md:299-300`).
- Artifact authority is question-type-layered: promoted formal models own invariants; prose owns product vocabulary; `.proto` owns wire shape when it exists (`docs/VERIFICATION.md:5-17`).
- SPEC's initial v0 command-kind bullet force-mixes drive, cancel/interrupt, status/snapshot refresh, and correlated replies/events (`docs/SPEC.md:30`).
- The research synthesis now uses the five-primitives evidence spine **spawn / attach / operate / receive / payload** (`parent.md:16`, `:32`, `:36-148`), and explicitly separates:
  - no generic operator-originated no-grant `Message`;
  - real agent-originated question/elicitation surfaces in Claude, Codex, OpenCode, and Antigravity (`parent.md:180-188`).
- SNC prior art establishes spawn vs pilot and deployed `claude remote-control --spawn worktree --capacity 8` systemd usage (`remote-agent-operation-landscape.md:23-45`, `:155-163`; `remote-agent-piloting.md:178-206`, `:281-287`).

### 2. Coverage proof

Operation / Observation / Elicitation can cover the surveyed action surface, but only if "Operation" means **authorized control-plane request by any actor**, not only "operator command," and "Elicitation" means **pending response solicitation between actors**, not only a UI multiple-choice question to the operator.

| Harness | Surveyed actions | O/O/E classification | Fit / issue |
|---|---|---|---|
| Pi | `user_message`, `approve_tool`, `cancel`, `session_sync`, `session_new`, `session_compact`, `model_set`, `thinking_set`, `list_models`, `ping`; outbound `message_*`, `tool_call`, `tool_result`, compaction, model/thinking events (`[pi-extension]`; also `.research/attestation/pi-extension.md:27-44`) | Operations: drive, approve, cancel, sync/query, session mgmt, reconfigure. Observations: streamed messages/events/tool results. Elicitation: `tool_call` approval gate. | Fits. Pi has no distinct freeform agent question in surveyed wire types; tool approval is gate-style Elicitation. |
| Claude Code | prompt/SDK query/Remote Control messages; interrupt/stop; tool approvals; `AskUserQuestion`; session resume/fork/compact/clear; remote-control spawn; message/hook/task streams (`[claude-code-user-input]{1,4,10,11}`, `[claude-code-remote-control]{3,5,11}`; specialist `claude-code.md:39-45`, `:139-153`) | Operations: drive, interrupt, stop, spawn, session mgmt, permission-mode/model changes. Observations: assistant/tool/hook/result/task streams. Elicitation: tool permission and `AskUserQuestion`. | Fits. Important: `AskUserQuestion` answer is informational, not a grant (`claude-code.md:43`, `:121`, `:139`). |
| Codex | `thread/start/resume/fork/list/read/archive/delete/compact`, `turn/start/steer/interrupt`, `command/exec`, `process/spawn`; notifications; approvals; `item/tool/requestUserInput`, MCP elicitation, dynamic tool calls, auth-token refresh, attestation, current-time read (`[codex-appserver-readme]{5,6,8,9,11}`, `[codex-appserver-protocol]{2-9}`, `[codex-appserver-types]{5,7,8,10}`; specialist `codex.md:16-34`, `:67-71`) | Operations: thread/turn lifecycle, steer, interrupt, process/command utility, queries. Observations: thread/turn/item notifications and deltas. Elicitations: approvals, tool user input, MCP elicitation, dynamic tool call / function-like requests. | Fits only if Elicitation is actor-neutral. Codex has server→client service requests that are not simply "agent asks operator a question." |
| Cursor | local prompt/queued/immediate messages; Run Mode approvals; checkpoint restore; Cloud Agent create/run/list/read/stream/cancel/archive/delete; Cloud SSE (`[cursor-cloud-agents-api]{4,6,8,11-13}`, `[cursor-run-modes]{7}`; specialist `cursor.md:24-40`, `:48-63`, `:69-77`) | Operations: prompt/run create, immediate redirect, cancel, checkpoint restore, cloud spawn/archive/delete, read/list. Observations: CLI/Cloud SSE output, status, tool_call events. Elicitations: local tool approvals where Run Modes ask. | Fits. Cloud Agents specifically do not ask approval because they run in dedicated machines (`cursor.md:18`, `:63`, `:81`), so no Elicitation there. |
| OpenCode | HTTP `prompt`, `promptAsync`, `shell`, `loop`, `remove`, `removeMessage`, `status`, `summary.diff`, `revert/unrevert`, `compact`, `share/unshare`; SSE events; `permission.v2.*`; `question.*`; OpenCode `serve` + attach from SNC (`[opencode-session-handler]`, `[opencode-schema-events]`, `[snc-rao-ae-opencode-cli]`; specialist `opencode.md:14-29`, `:39-63`, `:75-80`) | Operations: drive, cancel/remove, session/git actions, query, share, attach to server. Observations: session/permission/question events. Elicitations: permission and question gates. | Fits. Note parent corrected OpenCode `serve` as spawn/attach prior art via SNC (`[snc-rao-ae-opencode-cli]`). |
| Aider | chat input, slash commands, shell passthrough, model/mode/session/file/git commands, Ctrl-C interrupt, local output streams (`[aider-commands]`, `[aider-base-coder]`; specialist `aider.md:22-31`, `:44-51`, `:58`) | Operations: drive, slash-command/session/git/model/file actions, interrupt. Observations: local output streams/analytics-ish events. Elicitation: only if Patchbay wraps local confirmations; no strong structured external elicitation surface in surveyed core. | Fits. Aider has no explicit spawn/retire operator action (`aider.md:58`). |
| Antigravity | SDK Agent context starts sidecar; `chat/send/cancel`; managed remote interactions; `ASK_QUESTION`; tool confirmations; custom function `requires_action`; triggers; subagents; history/status streams (`[antigravity-sdk-repo]`, `[antigravity-managed-agent]`; specialist `antigravity.md:35-43`, `:58-79`, `:89-100`) | Operations: spawn sidecar/session, drive, cancel, managed interaction, config, trigger-originated send, subagent config/start. Observations: chunks, steps, hooks, tool results. Elicitations: questions, tool confirmations, custom function results. | Fits if Operation sender is not always human: triggers are non-operator-originated drive actions (`parent.md:203`; `antigravity.md:37`, `:72`). |

Misses/misclassifications in the frame as stated:

1. **Codex service requests** are broader than "question to operator." `currentTime/read`, auth refresh, attestation, dynamic tool call, and MCP elicitation are server/client request-response surfaces (`[codex-appserver-protocol]{7}`), not necessarily human UI questions.
2. **Approval responses are not all answers.** Claude, Codex, OpenCode, Cursor, and Antigravity have allow/deny/allow-once/always/modified-input/policy-amendment shapes (`[claude-code-user-input]{7,8}`, `[codex-appserver-types]{7}`, `[opencode-schema-events]`).
3. **Automated/non-human origins exist.** Antigravity triggers send into the agent as `automated_trigger` (`[antigravity-sdk-repo]`; `parent.md:203`), so sender/recipient actor-neutrality is mandatory.

### 3. Non-foreclosure proof

The seam mostly holds if the primitives stay actor-neutral.

Concrete tests:

- **Agent→operator uncodified ask:** model as `Elicitation(sender=agent, recipient=operator/control-surface, response_contract=...)`, followed by `Operation(kind=elicitation_response, correlates_to=elicitation_id)`. This covers Claude `AskUserQuestion`, OpenCode `question.asked`, Codex user-input requests, and future freeform/attachment/drawn-region requests.
- **Agent→agent mesh:** remote-pi mesh has `agent_send` with ACK and asynchronous replies via `re`, and deprecated blocking `agent_request` (`/home/agent/projects/remote_pi/PROTOCOL.md:84-96`; `pi-extension/skills/agent-network/SKILL.md:129-160`, `:275-282`). This can be represented without a new primitive:
  - `Operation(kind=agent_send, sender=agent, recipient=agent, payload=...)`
  - `Observation(kind=delivery_ack/status, ...)`
  - later `Operation` or `Observation` carrying reply with typed correlation.
  - `list_peers` is a query Operation returning a presence snapshot/Observation (`pi-extension/skills/agent-network/SKILL.md:52-72`).
- **Agent→agent task spawn:** Antigravity subagent starts (`START_SUBAGENT`) fit as Operation from agent/harness to agent/harness target, with Observations for subagent lifecycle (`antigravity.md:96`).
- **Op→op future interaction:** an informational human-to-human message can still be an Operation if Patchbay must authorize routing/audit it. The delivered content is an Observation to the recipient. If it requires response, it opens an Elicitation.
- **Non-request broadcast/stream:** source-authenticated status/output/presence updates are Observations; no request/response assumption needed.

Non-foreclosure failure risk: the proposal text repeatedly says Elicitation is a "question/request to the operator." That forecloses agent→agent, adapter→core, and service-request variants unless amended to "pending response solicitation from one actor to another."

### 4. Verification-posture analysis

Current checked models bind to current concepts, not to the proposed frame:

- `CommandState` lifecycle is checked in `command_lifecycle.qnt`, with terminal states and non-terminal states hard-coded (`specs/seed/command_lifecycle.qnt:19-21`).
- `TypedCorrelation` is checked only for `ReplyId -> CommandId | MessageId`, not `OperationResponse -> ElicitationId` (`specs/seed/reply_correlation.qnt:18-25`, `:57-67`, `:143-162`).
- Session generation and stale-event inertness are checked separately (`specs/seed/session_generation.qnt:112-190`).
- Browser CSRF boundary is checked for state-changing requests before command acceptance (`specs/seed/csrf_browser.qnt:143-187`).
- Authority safety is load-bearing but draft/stated-normative (`docs/VERIFICATION.md:30`, `:158`, `:299-300`; `specs/seed/authority.qnt:1-9`, `:354-367`).

Implications for Operation / Observation / Elicitation:

1. **OperationState cannot claim checked model authority until `CommandState` is explicitly renamed/refactored or a refinement mapping is written.** The safe path is: `OperationState` initially reuses the `CommandState` registry and model by documented equivalence, then update names/contracts/models together.
2. **ElicitationState is a new model obligation.** If v0 treats agent-originated question/approval semantics as product behavior, the lifecycle, correlation, timeout, cancellation/withdrawal, and first-answer rules need a promoted model and conformance vectors.
3. **Correlation model must be extended.** A response Operation correlating to Elicitation is outside current `TypedCorrelation`.
4. **Authority model must be promoted before grant semantics are broadened.** Operation grant kind, actor-neutral senders, and adapter/service requests touch `GrantAuthorityIsCommandKinds`, `NoCommandWithoutGrant`, and `CompoundIssuer`, all draft today.
5. **Question-type-layered authority is preserved only if docs are honest:** prose may introduce Operation/Observation/Elicitation vocabulary, but invariants must not be stated as checked until models are promoted (`docs/VERIFICATION.md:5-17`, `:25-43`).

### 5. Mechanization-proposal verdicts

**P1 — Fifth id space for Elicitation — PARTIAL.**
Sound parts: existing protocol's client-generated command/message rule does not conflict with adapter/core-assigned elicitation ids, because replies are already adapter/core-generated (`docs/PROTOCOL.md:41-45`). Separate IDs help prevent a response Operation from masquerading as the pending Elicitation.
Attack: a fifth id space is not proven necessary. Current `ReplyId` is already adapter/core-generated and separate; an Elicitation could be modeled as a reply/request variant with a `reply_id` plus pending-state record. But current `reply_correlation.qnt` only allows replies to prior command/message ids and rejects reply/event as correlation types (`specs/seed/reply_correlation.qnt:25`, `:57-67`). It does not already cover Elicitation as response target.
Required amendment: choose one — (1) add `ElicitationId` as a new id space and extend typed correlation; or (2) reuse `ReplyId` with `reply_kind=elicitation` and define `ElicitationState(reply_id)`. In either case, update `TypedCorrelation` to include `Operation -> Elicitation` response references.

**P2 — Uniform OperationState lifecycle for reads and side-effecting operations — PARTIAL / MOSTLY SOUND WITH AMENDMENT.**
Sound parts: SPEC already includes status/snapshot refresh in initial command kinds (`docs/SPEC.md:30`). Status/snapshot reads are authority-relevant: snapshots are authoritative reconciliation inputs, stale cached state must not be rendered live, and rejected/nonconformant snapshots degrade state (`docs/PROTOCOL.md:365-368`). Keeping one lifecycle preserves the existing checked lifecycle posture better than creating an unmodeled read lifecycle.
Attack: "Running" is unnatural for many reads. OpenCode `status`, Codex `thread/list/read`, Cursor list/read, Pi `session_sync`, and Aider `/tokens`/`/settings` are often immediate query/observe surfaces (`parent.md:114-128`). Uniform lifecycle must not imply every read passes through `running`.
Required amendment: one registry, per-kind transition subsets. Reads may go `accepted -> completed` or `accepted -> delivered -> completed` without `running`. If polling log bloat matters, optimize projection/storage later, but do not split the normative state model unless the model is promoted.

**P3 — Elicitation response is an Operation with TypedCorrelation to Elicitation — PARTIAL.**
Sound parts: routing an operator answer through core needs identity, authorization, target, validation, audit, idempotency, and stale-target checks. That is Operation-shaped. Decline/reject answers are real in OpenCode (`question.rejected`) and approval-denial surfaces (`[opencode-schema-events]`, `[codex-appserver-types]{7}`, `[claude-code-user-input]{7,8}`).
Attack: not every answer is authority-bearing "in the same sense as drive." Claude `AskUserQuestion` explicitly supplies information rather than granting tool authority (`claude-code.md:43`, `:121`, `:139`). "No answer" is not an Operation; it is an Elicitation terminal outcome (`expired`) or still pending. Current `TypedCorrelation` does not cover Operation-to-Elicitation references (`specs/seed/reply_correlation.qnt:57-67`, `:143-162`).
Required amendment: define `OperationKind = elicitation_response` with grant/authority semantics appropriate to response submission, not necessarily external execution. Add terminal answer variants: answered, declined/rejected, expired, withdrawn/cancelled, stale/orphaned. Extend typed references to include `correlates_to: ElicitationId`.

**P4 — Elicitation response-shape as extensible field — PARTIAL LEANING REFUTED AS STATED.**
Sound parts: the initial shape list matches some surveyed question surfaces: Claude `AskUserQuestion` has questions/options/multi-select/freeform (`[claude-code-user-input]{10,11}`), OpenCode question answers are arrays of selected labels (`[opencode-schema-events]`), Antigravity `ASK_QUESTION` uses multiple-choice specs (`[antigravity-sdk-repo]`), Codex tool input questions carry options/answers/timeout (`[codex-appserver-types]{8}`).
Attack: it smuggles a UI-widget assumption: "agent questions are multiple-choice-ish." Surveyed surfaces include: approval-with-policy amendments / allow once / always / reject (`[codex-appserver-types]{7}`, `[opencode-schema-events]`); modified tool input and approve-and-remember (`[claude-code-user-input]{7,8}`); secrets (`ToolRequestUserInputQuestion.is_secret`) (`[codex-appserver-types]{8}`); custom function results and `requires_action` continuations (`[antigravity-managed-agent]`); auth-token refresh, attestation generation, current-time read (`[codex-appserver-protocol]{7}`).
Required amendment: replace `response-shape` with **response_contract**: `contract_kind` registry (approval, question, secret, freeform, function_result, file/attachment, structured_schema, service_request, etc.); optional UI hints (select-one, select-many, free-text, upload, draw); adapter-specific payload schema under generated contracts or schema refs.

**P5 — ElicitationState lifecycle `opened→pending→answered|expired|cancelled` — PARTIAL.**
Sound parts: a separate lifecycle is justified: Elicitations are not Operations because their pending unit is an externally opened response slot, not a command being delivered/executed.
Attack: missing terminal distinctions: `declined`/`rejected` by operator (OpenCode `question.rejected`; approvals deny/reject); `withdrawn` by opener/agent/harness, distinct from operator/core cancellation; `superseded` when a newer question replaces an older one; `stale`/`orphaned` when the target session/turn/generation is gone. Missing response validation behavior: invalid answer should reject the response Operation while the Elicitation remains pending or moves to `failed_validation`, depending on policy. Multiple operators/future responders need first-valid-answer-wins or multi-answer policy. Agent-side "blocked waiting" should probably be an Observation/session activity fact, not necessarily ElicitationState.
Required amendment: minimal lifecycle `opened/pending -> answered | declined | expired | cancelled | withdrawn | superseded | stale`, terminal finality with first durable terminal commit, invalid response is a response Operation rejection unless policy terminalizes the Elicitation.

### 6. Frame-specific blind spot

The frame does not explicitly model **presence / subscription / attention routing**, which is central to the consuming feature.

Evidence: Attach is distinct from operate because it establishes connection/authentication and reconciles state (`parent.md:52-64`). Remote-pi mesh presence is pull-based `list_peers`; peer join/leave does not wake the turn (`pi-extension/skills/agent-network/SKILL.md:52-72`). Claude Remote Control has push notifications for actions required/questions (`[claude-code-remote-control]{10}`). Cursor/Cloud and OpenCode expose streams/SSE scoped to runs/sessions (`[cursor-cloud-agents-api]{8}`, `[opencode-schema-events]`).

Operation/Observation can carry presence facts, but the frame does not name: endpoint availability vs actor presence; subscription to observations; attention-required state; which operator/control surface is the expected responder for an Elicitation; stale presence reconciliation. That blind spot is not solved by P1–P5.

### 7. Alternatives or amendments

No strictly better different frame among A/B/C as described. Operation / Observation / Elicitation is better than command-only A because it gives agent-originated pending requests a first-class home; better than B if it avoids unchecked "Message force" model authority; better than C if Operation is authority-bearing, not merely side-effecting.

Minimal amendments:

1. Rename/define precisely:
   - **Operation**: authorized control-plane request by any actor to any actor/core/adapter target. May be side-effecting, read-only, or response-submitting.
   - **Observation**: source-authenticated fact/event/output/status emission; no grant to act, but source identity/correlation required.
   - **Elicitation**: durable pending response solicitation from one actor/system component to another, with response contract, timeout/cancellation/withdrawal, correlation, and terminal state.
   - **Payload**: adapter-specific content or schema-bound body inside the above.
2. Add **Presence/Subscription** as a protocol section or registry, not necessarily a primitive.
3. Add `OperationKind` registry with spawn/attach/drive/cancel/interrupt/query/approval-response/elicitation-response/reconfigure/session-management.
4. Promote/extend models before claiming product semantics:
   - `CommandState` → `OperationState` refinement/rename;
   - new `ElicitationState`;
   - extended `TypedCorrelation`;
   - promoted `authority.qnt`.

### 8. Open questions

- Should Elicitation ids be a new id space or a typed `ReplyId` subkind? Both can work; the docs must choose and model it.
- Is spawn v0 behavior or only a reserved seam? The feature flags this as an extension-pressure decision (`feature-operator-presence-and-action-inventory.md:86-88`).
- Should service requests like Codex `currentTime/read`, auth refresh, and attestation be Elicitations or a separate "ServiceRequest" Operation kind? Generalized Elicitation can cover them, but the name may mislead.
- What are the exact `.proto` boundaries for response contracts? Current docs have no generated contract yet.
- Multi-operator policy is out of v0, but Elicitation should not bake in single-responder assumptions.

### 9. Bottom-line verdict

**Survives with amendments.** Operation / Observation / Elicitation is a sound frame for the design pass if it is made actor-neutral, if Elicitation is generalized beyond multiple-choice operator questions, and if the verification docs are updated honestly: OperationState can reuse/rename the checked CommandState model only with an explicit refinement, while ElicitationState and Operation-to-Elicitation correlation are new model obligations. As stated, P4 and P5 are too narrow, P1 is under-justified, and the frame's biggest unaddressed gap is presence/subscription/attention routing.

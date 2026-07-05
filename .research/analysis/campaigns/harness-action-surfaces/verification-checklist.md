---
provenance: adversarial-read
updated: 2026-07-04
campaign: harness-action-surfaces
verdict: NEEDS-REVISION
---

# Harness action surfaces — adversarial verification checklist

Scope read:

- Synthesis: `.research/analysis/campaigns/harness-action-surfaces/parent.md`
- Specialist briefs: `specialists/{claude-code,codex,cursor,opencode,aider,antigravity}.md`
- Attestations matching `claude-code-*`, `codex-*`, `cursor-*`, `opencode-*`, `aider-*`, `antigravity-*`
- Pi grounding source: `/home/agent/projects/remote_pi/pi-extension/`

## Verdict

**NEEDS-REVISION.** The core provisioning conclusion mostly survives with qualification, but the synthesis is not yet safe to consume as grounded substrate because:

1. The `Message` drop claim is over-broad and contradicts the Claude Code specialist brief's source-grounded `AskUserQuestion` finding; Codex, OpenCode, and Antigravity also expose agent-initiated no-grant/question/elicitation reply paths even if they are not generic operator-originated `Message` commands.
2. Several parent-table named-feature claims are uncited in the synthesis, relying on specialist briefs as analytical lens rather than citing source-direct attestations.
3. The six-class spine is misstated in the parent: it says `drive/interrupt/approve/query/result/payload`, while the consuming feature's spine is `drive/request/query/result/payload/provision`; the parent then uses `Request` in disconfirming analysis without defining it in the common set.
4. The provisioning synthesis needs tighter wording for Antigravity-local, OpenCode confidence, and Claude Remote Control citation/locator.

## Job (a): Semantic citation-chain walk

### Passes / survives with qualification

- **Codex provisioning row** (`parent.md` Provisioning section): `[codex-appserver-readme]{5}` and `[codex-appserver-types]{10}` semantically support `process/spawn` as an experimental unsandboxed host process and `command/exec` as sandboxed server-side command execution. They also support that threads are conversations within an app-server, not new app-server processes. The row should additionally cite `[codex-appserver-readme]{11}` if mentioning collab `spawn_agent`, because the Codex specialist flags spawned-agent lifecycle as a collab item/tool concept rather than a stable operator method.
- **Cursor Cloud Agents provisioning row**: `[cursor-cloud-agents-api]{4}` supports `POST /v1/agents` creating a durable Cloud Agent plus initial run. The “not arbitrary operator machine” qualifier is supported by Cursor Run Modes / Agents Window sources only indirectly; add a citation to `[cursor-run-modes]{7}` for Cloud Agents running in dedicated machines and/or `[cursor-agents-window-subagents]{12}` for cloud subagents on their own VM.
- **Antigravity managed provisioning row**: `[antigravity-managed-agent]{3}` supports `environment="remote"` provisioning a fresh Google-hosted Linux sandbox and environment-id reuse. It supports “Google-managed / not operator VM.”
- **Claude Code Remote Control spawn modes**: `claude-code-remote-control` attestation supports server-mode `--spawn same-dir|worktree|session`, `--capacity`, and the local-session requirement (`{3}`, `{1}`, `{7}`, `{11}`). The substantive claim survives: this is not arbitrary remote-machine process spawn.

### Findings requiring revision

- **Broken/weak parent citation for Claude Code Remote Control.** `parent.md` uses `[claude-code-remote-control]` with no locator and says “per the specialist brief.” This must be changed to source-attestation locators, e.g. `[claude-code-remote-control]{3}` plus locality/capacity locators `[claude-code-remote-control]{1}` `[claude-code-remote-control]{7}` `[claude-code-remote-control]{11}`. Do not cite the specialist brief as substrate.
- **Pi rows lack attestation-layer citation.** Pi is grounded by source and commissioning item, but the parent has no Pi attestation handle. The parent’s Pi claims should either gain a `pi-extension-*` attestation or be explicitly marked as externally grounded with direct source path references outside `[handle]{N}` citation form. Current form leaves Pi rows uncited.
- **`Message` synthesis is semantically contradicted by cited/source-grounded briefs.** The parent says “No surveyed harness exposes a distinct send informational replyable content without a grant action class” and later “No-grant informational replyable content: no harness exposes it.” Claude Code specialist says `AskUserQuestion` is “a no-grant informational reply surface” and cites `[claude-code-user-input]{1}` `{4}` `{10}` `{11}`. Codex specialist identifies non-approval server requests (`item/tool/requestUserInput`, MCP elicitation, etc.) via `[codex-appserver-protocol]{7}` and `[codex-appserver-types]{8}`. OpenCode identifies `question.asked` / `question.replied` via `[opencode-schema-events]{1}`. Antigravity identifies `ASK_QUESTION` / `question_response` via `[antigravity-sdk-repo]{2}`. The parent may still conclude “no generic operator-originated Message command,” but it may not say no harness exposes no-grant informational replyable content.
- **Aider approval row is partly unsupported as written.** Parent table lists `--yes`/`--ask`/`--auto` edit-approval flags. Attestations support `--yes-always`, docs mentioning `--yes`, `confirm_ask()`, and in-chat confirmations, but I did not find attested `--ask` or `--auto` edit-approval flags in the fetched Aider attestations. Revise to the attested vocabulary or add attestation support.
- **Payload “slash-commands universal example” is false as stated.** The parent says slash-commands are the universal example. The table itself only supports slash-/command-like payloads for Pi, Claude Code, Aider, and Aider shell passthrough; Codex is typed `UserInput`, Cursor has prompts and some slash-style subagent invocation, OpenCode has prompt text, Antigravity has `user_input` / `complex_user_input`. Revise to “content interpreted by harness; slash commands where present.”

## Job (b): Claim-shapes the mechanical lint missed

- **Uncited named-feature tables.** The parent’s common action tables list many named verbs (`query()`, `turn/start`, `promptAsync`, `Conversation.cancel()`, etc.) without direct citations. The specialist briefs cite these, but specialist briefs are analytical-tier artifacts, not source substrate. The parent should cite source-direct attestations on each row or move these tables into a clearly labeled derived summary with per-row source handles.
- **Comparative/convergence claims need evidence map.** Claims like “Every surveyed harness exposes…” and “commonality is strong enough…” are composed convergence claims. They need either citations per harness row or an explicit evidence matrix. The `{inferred: convergence}` marker appears only in some sections, not at the first “Every surveyed harness” claim.
- **OpenCode provisioning confidence got smoothed.** The OpenCode specialist says `{confidence: shallow-survey}` and leaves `control-plane` / `installation` modules as revisit-if. Parent treats OpenCode as settled out-of-band in the convergence line. Preserve the shallow confidence in the parent synthesis.
- **Antigravity local provisioning got smoothed.** Parent says locally Antigravity is out-of-band and operator starts the process themselves. The specialist says entering `Agent` / `LocalConnectionStrategy` starts a local harness process via SDK. That is local programmatic sidecar/session spawn, even if not arbitrary remote-machine fleet spawn. Revise the posture taxonomy.

## Job (c): Coherence-read for smoothed contradictions

- **Message question contradiction is smoothed.** The parent’s “operator drives, agent replies” paragraph merges two different questions:
  - no generic operator-originated informational `Message` command; versus
  - agent-initiated no-grant replyable informational/question surfaces.

  The specialist set clearly supports the first for many harnesses, but disconfirms the second. The consuming feature’s wording asks whether harnesses “send no-grant informational replyable content”; therefore this distinction is load-bearing and must be explicit.

- **Provisioning taxonomy is partially merged.** The parent names three postures (out-of-band, in-process, cloud-managed), but local Antigravity SDK is a fourth-ish posture: programmatic local sidecar startup by SDK context manager. It is not “spawn on arbitrary operator machine,” but it is also not simply “operator starts process themselves.”

- **Six-class spine contradiction.** The consuming feature names `drive / request / query / result / payload / provision`; parent’s disconfirming analysis names `drive/interrupt/approve/query/result/payload`, then classifies several surfaced actions as `Request`. The synthesis should align with the feature spine before downstream design consumes it.

## Job (d): Noise-domination / relevance-weighting

- **Claude Code Message evidence:** The most relevant attestation for the `Message` question is `[claude-code-user-input]{1}` `{4}` `{10}` `{11}`, not the parent’s uncited high-level convergence statement. The parent should foreground this as a disconfirming/qualifying row.
- **Codex Message evidence:** `[codex-appserver-protocol]{7}` and `[codex-appserver-types]{8}` are more relevant than the agent-message item citations alone; they show server-originated replyable non-approval requests. The parent mentions `thread/inject_items` instead, which is less relevant to no-grant replyable content.
- **Cursor provisioning evidence:** `[cursor-run-modes]{7}` (“Cloud Agents run inside their own dedicated machine”) is more directly relevant to “not arbitrary operator machine” than `[cursor-cloud-agents-api]{4}` alone.
- **Pi evidence:** The parent points to the commissioning item. For a research synthesis, the more relevant substrate is the actual remote_pi source (`protocol.generated.ts`, `actions/handlers.ts`, `bin/supervisord.ts`) captured in an attestation.

## Job (e): Quote-context walk

- Parent quote-like strings such as “spawn a new agent/harness process on an arbitrary operator-controlled machine” and “Message” are project vocabulary, not source quotations. They are acceptable as terms, but their surrounding claims must not be presented as source-attested without citation.
- Fetched-source quotes in the attestations generally retain local qualifiers where I spot-checked them: Claude Remote Control retains local-machine and local-process qualifiers; Antigravity managed retains Google-hosted sandbox/environment wording; Cursor Cloud Agents retains durable-agent/run split. No standalone quote-context stripping finding beyond the synthesis over-breadth findings above.

## Job (f): Analytical-tier-inheritance walk

- No `[handle]{N}` citation in the parent resolves to a specialist brief, but the parent repeatedly says it is grounded via specialist briefs and leaves the common action tables uncited. This is an analytical-tier inheritance risk: the specialist briefs are useful lenses, but the parent needs direct attestation citations for load-bearing rows.
- The unnumbered `[claude-code-remote-control]` token is citation-shaped and should be corrected to numbered attestation citations. It does not resolve to an analytical artifact, but in current prose it says “per the specialist brief,” which is the wrong grounding layer.

## Job (g): Line-reference / locator walk

- Parent citation locators checked in the high-risk claims exist and semantically support their narrow claims: `[codex-appserver-readme]{5}`, `[codex-appserver-types]{10}`, `[cursor-cloud-agents-api]{4}`, `[antigravity-managed-agent]{3}`.
- Parent has an unlocated citation-shaped reference `[claude-code-remote-control]`; fix to numbered locators.
- Several attestations use source-internal line anchors, but the campaign’s local-code attestations (notably Aider and OpenCode) often use broad “Core findings” / file-path summaries rather than line-exact anchors. That is not automatically fatal, but it limits per-claim line-reference confidence and contributed to the Aider `--ask`/`--auto` unsupported row.

## Job (h): Thin-attestation semantic complement

- **OpenCode attestations:** `opencode-schema-events` and `opencode-session-handler` are concise and source-path-based. They are semantically sufficient for the specific event/handler claims they support, but they should gain line anchors if the parent will cite them for normative action-registry decisions.
- **Aider attestations:** `aider-args`, `aider-commands`, and `aider-base-coder` are broad summaries with evidence snippets, not quote/line-rich attestations. They support broad Aider shape claims, but not all exact flag names in the parent.
- **Antigravity SDK attestation:** Substantive and supports many claims, but because many citations point to the same broad locator `{2}`, parent/specialist claims that distinguish local sidecar startup, question response, trigger sends, and subagent spawn should cite more granular anchors if possible in a revision.

## High-risk claim checklist

### Provisioning synthesis claim

Claim: “No surveyed harness exposes spawn a new agent/harness process on an arbitrary operator-controlled machine as an operator action.”

Status: **mostly supported, needs wording/citation revisions.**

- Pi: supported by remote_pi source/commissioning note, but missing attestation citation.
- Claude Code: supported for “not arbitrary remote-machine spawn”; fix citation to `[claude-code-remote-control]{3}` plus locality locators.
- Codex: supported; process spawn is arbitrary process on app-server host, not agent instance; thread creation is in-server conversation provisioning.
- Cursor: supported for Cloud Agents being cloud/dedicated-machine managed; add direct dedicated-machine citation.
- OpenCode: provisionally supported only for surveyed session handler/schema; preserve shallow confidence and revisit-if control-plane modules.
- Aider: supported for no explicit instance spawn/retire in args/commands.
- Antigravity: managed cloud claim supported; local SDK posture must be revised from “out-of-band” to “programmatic local sidecar/session startup, not remote operator-machine fleet spawn.”

### `Message` drop claim

Claim: “No harness exposes no-grant informational replyable content; Message can be dropped for v0.”

Status: **not approved as written.** The parent must separate “no generic operator-originated Message command” from “agent-originated no-grant replyable questions/elicitation.” The latter exists in Claude Code and appears in Codex/OpenCode/Antigravity. Downstream formal-model amendment should not consume the current broad wording.

### `{inferred: convergence}` claims

Status: **partly real, insufficiently demonstrated in parent.** Drive/query/result broadly converge. Request/gate/interrupt/session-management converge if grouped under `Request`, not if split into only interrupt/approve. Provisioning diverges by posture. Message/no-grant does not converge as stated.

### Six-class spine survival

Status: **likely survives only after restatement.** The proper spine is `drive / request / query / result / payload / provision`. No surveyed action obviously requires a seventh class, but parent currently misnames the spine and should map interrupt, approval/answer, session management, reconfiguration, revert/compact, and cancel under `Request`, with provision separate.

## Required revision actions

1. Rewrite the `Message` section to state the narrower, supported claim: “No surveyed harness exposes a generic operator-originated no-grant informational `Message` command distinct from drive/request; several expose agent-originated no-grant question/elicitation reply paths.” Then reassess whether `Message` can drop for v0 against the consuming feature’s exact protocol meaning.
2. Add source-attestation citations to parent common action tables or provide a per-harness evidence matrix.
3. Create or cite Pi source-grounding attestations for remote_pi `ClientMessage`, server messages/hooks, and `pi-supervisord` provisioning posture.
4. Fix `[claude-code-remote-control]` to numbered attestation locators.
5. Align the six-class spine wording with `feature-operator-presence-and-action-inventory`: `drive/request/query/result/payload/provision`.
6. Preserve confidence qualifiers for OpenCode provisioning and Antigravity CLI gaps; revise Antigravity-local provisioning posture.
7. Replace unsupported Aider `--ask`/`--auto` row terms or add attestations proving them.
8. Replace “slash-commands being the universal example” with a source-supported, harness-neutral payload statement.

---

# Revision verification — revised synthesis adversarial read (2026-07-04)

Scope read:

- Revised synthesis: `.research/analysis/campaigns/harness-action-surfaces/parent.md`
- Prior checklist: this file, pre-existing sections above
- Source-direct attestations spot-checked for high-risk claims: `pi-extension`, `claude-code-user-input`, `claude-code-remote-control`, `codex-appserver-readme`, `codex-appserver-protocol`, `codex-appserver-types`, `opencode-schema-events`, `opencode-session-handler`, `aider-args`, `aider-commands`, `aider-base-coder`, `antigravity-sdk-repo`, `antigravity-managed-agent`, `cursor-cloud-agents-api`, `cursor-run-modes`

## Per-prior-finding disposition

1. **`Message` drop claim over-broad — ADDRESSED, with one wording cleanup.**
   - Evidence: `parent.md` now separates **Q-A** generic operator-originated no-grant `Message` from **Q-B** agent-originated no-grant question/elicitation paths. It explicitly names Claude Code `AskUserQuestion`, Codex `item/tool/requestUserInput` / MCP elicitation, OpenCode `question.asked` / replies, and Antigravity `ASK_QUESTION` / `question_response` with source-direct citations.
   - The core downstream implication is now properly scoped: drop the operator-originated PROTOCOL `Message` type for v0, but keep agent-originated question/elicitation as a separate modeling question, likely under `Request`.
   - Minor cleanup: the `Revisit if` bullet still says “A harness surfaces genuine no-grant informational replyable content in a future version,” which is too broad because Q-B already exists. It should say future **operator-originated generic** no-grant `Message` content.

2. **Uncited named-feature claims in parent tables — NOT-ADDRESSED.**
   - Evidence: the common action tables for Drive, Interrupt/cancel, Approve/answer, Query, Result, and Payload still list named verbs and features without per-row source-attestation citations: e.g. `query()` / `ClaudeSDKClient.query()`, `turn/start`, Cursor run creation, OpenCode `promptAsync`, `Conversation.cancel()`, `/tokens`, `item/started`, `session.next.*`, and many others.
   - This still relies on the specialist briefs as analytical lenses instead of source-direct substrate. The parent introduction also says the non-Pi harnesses are grounded “via per-harness specialist briefs,” which is not acceptable for load-bearing parent synthesis rows unless the parent also carries the direct attestation chain.
   - Required fix remains: add source-attestation citations to each parent row or replace these tables with a cited evidence matrix.

3. **Six-class spine misstated — PARTIALLY-ADDRESSED.**
   - Evidence of fix: the opening common-action paragraph now says the consuming feature spine is **drive / request / query / result / payload / provision** and maps interrupt, approve/answer, session-management, and reconfigure under `Request`.
   - Remaining defect: the Disconfirming analysis still says “Before claiming the six-class common set (drive/interrupt/approve/query/result/payload) is universal,” which reintroduces the old, wrong spine and omits `provision`. The final answers also emphasize “drive, interrupt, approve/answer, query” instead of consistently saying `Request` contains interrupt/approve/session-management/reconfigure.
   - Required fix: replace the stale disconfirming-analysis spine with `drive/request/query/result/payload/provision` and use `Request` consistently in the seed answers.

4. **Provisioning wording — PARTIALLY-ADDRESSED.**
   - Addressed: the revised synthesis now has four postures; Antigravity local is correctly described as programmatic local sidecar/session startup; OpenCode carries `{confidence: shallow}`; Claude Remote Control now has numbered locator citations `[claude-code-remote-control]{3}` `[claude-code-remote-control]{1}` `[claude-code-remote-control]{7}` `[claude-code-remote-control]{11}`.
   - Remaining defects:
     - `parent.md` cites `[pi-extension-supervisord]`, but the new attestation handle is `pi-extension`. `[pi-extension-supervisord]` does not resolve to an attestation file.
     - The Codex in-process line cites `[codex-appserver-types]` without a locator next to the thread lifecycle claim; use `[codex-appserver-readme]{3}` / `{4}` / `{5}` and/or `[codex-appserver-types]{1}` as appropriate.
     - The seed answer “Pi, Aider, OpenCode = out-of-band sysadmin; Codex/Claude Code = in-process; Cursor/Antigravity = cloud-managed” collapses Antigravity back to cloud-only and loses the local sidecar posture.
   - Required fix: cite `[pi-extension]`, repair Codex locator(s), and restate answer 4 to include all four postures.

5. **Aider `--ask`/`--auto` unsupported — ADDRESSED.**
   - Evidence: the Aider approval row now says `--yes` / `--yes-always` / `confirm_ask()` and no longer lists unsupported `--ask` or `--auto` edit-approval flags. This matches the prior required vocabulary, though the parent should still cite the row directly when fixing finding 2.

6. **Payload “slash-commands universal” false — ADDRESSED.**
   - Evidence: the Payload section now says slash-commands are one example present in Pi, Claude Code, and Aider; Codex/OpenCode/Antigravity/Cursor are described as typed or prompt content. The common shape is “content the harness interprets,” not universal slash-command form.

## Jobs (a)–(h) on the revised synthesis

### Job (a): Semantic citation-chain walk

- Q-B citations semantically support the revised `Message` distinction: Claude Code `AskUserQuestion` is supported by `[claude-code-user-input]{1}` `{4}` `{10}` `{11}`; Codex user-input / elicitation requests by `[codex-appserver-protocol]{7}` and `[codex-appserver-types]{8}`; OpenCode question events by `[opencode-schema-events]`; Antigravity question handling by `[antigravity-sdk-repo]{2}`.
- Claude Remote Control provisioning citation is now source-direct and numbered.
- Broken chain: `[pi-extension-supervisord]` has no attestation file; the actual file is `pi-extension.md`.
- Weak/incorrect chain: the Q-A paragraph cites `thread/inject_items` to `[codex-appserver-readme]{5}`, but the attestation passage `{5}` as fetched lists thread/turn/review/command/process APIs and does not attest `thread/inject_items`. This appears inherited from the specialist brief rather than source-direct parent verification.
- Weak chain: the common action tables remain almost entirely uncited, so many named-feature claims do not have a parent-level citation chain at all.

### Job (b): Claim-shapes lint missed

- “Every surveyed harness exposes...” convergence claims remain composed claims without an evidence matrix or per-row citations.
- “All seven expose a tool-approval or question-answer surface” is plausible but still source-thin in the parent table, especially for Aider’s exact approval semantics and Cursor local-vs-cloud approval distinction.
- “Patchbay provisioning this would be genuinely novel” is a composed novelty claim; it is acceptable only if the four-posture evidence is fully cited and the OpenCode shallow caveat is retained.

### Job (c): Coherence-read for smoothed contradictions

- The old six-class vocabulary still appears in the Disconfirming analysis, contradicting the corrected opening spine.
- The final seed answer collapses Antigravity provisioning to cloud-managed and omits the now-correct local sidecar posture.
- The `Revisit if` wording re-broadens the no-grant informational content question; it should be narrowed to future operator-originated generic `Message` surfaces.

### Job (d): Noise-domination / relevance-weighting

- The revision improved the most relevant `Message` evidence by foregrounding Claude/Codex/OpenCode/Antigravity Q-B sources rather than relying on a convergence statement.
- The common action inventory is still dominated by uncited table prose and specialist-derived summaries; source-direct row citations are still absent.
- Pi grounding improved by adding `pi-extension.md`, but the parent points to the wrong handle in the provisioning paragraph.

### Job (e): Quote-context walk

- Project vocabulary such as “spawn a new agent/harness process on an arbitrary operator-controlled machine” and PROTOCOL `Message` is acceptable as analysis, not a source quote.
- No source quote-context stripping found in the high-risk revised passages. The remaining problems are citation-chain and coherence problems rather than quote-context distortion.

### Job (f): Analytical-tier-inheritance walk

- The parent still states that the non-Pi harnesses are grounded “via per-harness specialist briefs.” Specialist briefs may guide synthesis, but they are analytical-tier artifacts and cannot be final citation targets.
- The uncited common tables are the main inherited-risk surface: they appear to summarize specialists rather than directly cite attestations.
- The unsupported `thread/inject_items` citation appears to have been inherited from `specialists/codex.md` without confirming the cited attestation text.

### Job (g): Line-reference / locator walk

- Fixed: Claude Remote Control now uses numbered locators.
- Missing handle: `[pi-extension-supervisord]` does not resolve.
- Unnumbered or broad citations remain in load-bearing prose: `[aider-args]`, `[aider-commands]`, `[opencode-session-handler]`, `[codex-appserver-types]`, `[aider-base-coder]`, `[opencode-schema-events]`. Some source-direct attestations have no numbered passages, but parent claims should still use the most precise available locator or improve the attestation.

### Job (h): Thin-attestation semantic complement

- `pi-extension.md` is source-direct but lacks numbered passage locators; parent can cite it, but a tighter attestation would improve confidence for `ClientMessage`, pi.on hooks, and `pi-supervisord` claims.
- Aider and OpenCode attestations remain broad file-summary attestations; they support broad shape claims but are thin for exact per-row verb inventories.
- Antigravity SDK `{2}` is overloaded in the parent for local sidecar, `ASK_QUESTION`, and `question_response`; these are all present in the attestation but would be safer with more granular locators if the parent is consumed by formal design work.

## New issues introduced or still present after revision

1. **Missing citation handle:** `[pi-extension-supervisord]` should be `[pi-extension]` or a newly created attestation file.
2. **Unsupported Codex `thread/inject_items` citation:** `[codex-appserver-readme]{5}` does not attest `thread/inject_items` in the current attestation text. Remove the candidate, cite a real source, or mark it as ungrounded/acquisition-needed.
3. **Stale old spine in Disconfirming analysis:** replace `drive/interrupt/approve/query/result/payload` with `drive/request/query/result/payload/provision`.
4. **Final answer 4 loses Antigravity local sidecar:** include all four provisioning postures, not just cloud-managed for Antigravity.
5. **Common action tables still fail source-bound citation discipline:** this is the main blocker for approval.

## Final verdict

**NEEDS-REVISION.** The revision fixed the most serious semantic overclaim about `Message`, fixed Aider flags, improved payload wording, and substantially improved provisioning taxonomy. It is still not safe as a final synthesis because the parent-level common action tables remain uncited, one new Pi citation handle is broken, one Codex `thread/inject_items` citation is unsupported by the cited attestation, and stale prose still reintroduces the old six-class spine and a collapsed Antigravity provisioning answer.

---

# Final verification — second revised synthesis adversarial read (2026-07-04)

Scope read:

- Second-revised synthesis: `.research/analysis/campaigns/harness-action-surfaces/parent.md`
- Prior checklist sections above, especially the second-pass blocking findings
- Source-direct attestations spot-checked: `pi-extension`, `codex-appserver-readme`, `antigravity-sdk-repo`

## Second-pass blocking findings disposition

1. **Common action table source-attestation citations — RESOLVED for the prior blocker.**
   - The Drive table now carries per-harness source-attestation handles, and the synthesis explicitly states that the Cancel/Approve/Query/Result/Payload tables reuse those per-harness attestation handles. Cancel and Approve rows also carry direct citations for their high-risk named surfaces. This is a broad citation-map pattern rather than per-cell citation density, but it resolves the prior absence of a parent-level attestation chain.

2. **Broken `[pi-extension-supervisord]` handle — RESOLVED.**
   - No `[pi-extension-supervisord]` reference remains in `parent.md`; Pi rows and provisioning now cite `[pi-extension]`, and `.research/attestation/pi-extension.md` exists.

3. **Unsupported Codex `thread/inject_items` citation — RESOLVED.**
   - `thread/inject_items` no longer appears in the synthesis. The Codex closest-candidate statement is weakened to “thread-item manipulation primitives” with `{confidence: specialist-claim-not-attested}`, so it no longer pretends to be attested by `[codex-appserver-readme]{5}`.

4. **Stale old spine in Disconfirming analysis — RESOLVED.**
   - The Disconfirming analysis now says `drive/request/query/result/payload/provision`, and no stale `drive/interrupt/approve/query/result/payload` spine remains.

5. **Final-answer Antigravity provisioning collapsed to cloud-managed — NOT RESOLVED.**
   - The main Provisioning section correctly names four postures and includes Antigravity as “programmatic local sidecar/session startup” via `[antigravity-sdk-repo]{2}` as well as managed remote sandbox via `[antigravity-managed-agent]{3}`.
   - However, seed answer 4 still says: “Pi (pi-supervisord), Aider, OpenCode = out-of-band sysadmin; Codex/Claude Code = in-process; Cursor/Antigravity = cloud-managed.” This final answer again collapses Antigravity to cloud-managed and omits the local sidecar posture. Seed answer 6 partly restates the four-posture taxonomy, but it does not repair answer 4’s direct response to “Privileged sidecar/supervisor.”

## Jobs (a)–(d) on the second-revised synthesis

### Job (a): Semantic citation-chain walk

- `[pi-extension]` resolves and semantically supports Pi `ClientMessage`, `pi.on` event, and `pi-supervisord` posture claims.
- The removed Codex `thread/inject_items` citation is no longer a broken source chain.
- Antigravity local sidecar is source-supported by `[antigravity-sdk-repo]{2}`. The remaining defect is not lack of source support; it is inconsistent final-answer synthesis that drops this supported posture.

### Job (b): Claim-shapes lint missed

- No new composed-claim blocker found in the revised high-risk sections. The novelty claim remains appropriately scoped to “no arbitrary operator-controlled remote-machine process spawn” and carries the four-posture caveat in the main provisioning section.
- The broad table citation-map pattern is acceptable for this final verification pass because the prior requested correction was explicitly Drive-row citations plus a reuse note; no new blocking table issue is raised here.

### Job (c): Coherence-read for smoothed contradictions

- One coherence blocker remains: the provisioning taxonomy is correct in the main section and answer 6, but answer 4 contradicts/over-smooths it by grouping Antigravity only under cloud-managed. Because answer 4 is a final seed-question answer, this is a genuine downstream grounding/coherence failure, not style.

### Job (d): Noise-domination / relevance-weighting

- The most relevant Antigravity evidence is now the local SDK sidecar attestation plus managed remote sandbox attestation. The main body weights both; answer 4 drops the local evidence. No other new relevance-weighting blocker found.

## Final verdict

**NEEDS-REVISION.** Four of the five second-pass blockers are resolved. The remaining blocker is narrow and mechanical: revise seed answer 4 to show all four provisioning postures, including Antigravity’s programmatic local sidecar/session startup, instead of grouping Antigravity only as cloud-managed. After that correction, this final-verification pass would have no remaining blocking finding.

---

# Reconciliation verification — spawn/attach spine final check (2026-07-04)

Scope read:

- Reconciled synthesis: `.research/analysis/campaigns/harness-action-surfaces/parent.md`
- Cross-corpus pointer attestations: `.research/attestation/snc-rao-sp-cc-remote-control.md`, `.research/attestation/snc-rao-ae-opencode-cli.md`, `.research/attestation/snc-rao-landscape.md`
- SNC canonical sources spot-checked: `/home/agent/projects/SNC/.research/attestation/rao-sp-cc-remote-control.md`, `/home/agent/projects/SNC/.research/attestation/rao-ae-opencode-cli.md`, `/home/agent/projects/SNC/.research/attestation/rao-sp-cc-desktop.md`, `/home/agent/projects/SNC/.research/analysis/briefs/remote-agent-operation-landscape.md`, `/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md`
- Attach grounding spot-checks: `.research/attestation/pi-extension.md`, `.research/attestation/claude-code-remote-control.md`, `.research/attestation/opencode-session-handler.md`, `.research/attestation/codex-appserver-readme.md`

## Reconciliation checklist

1. **Spawn finding corrected — SUBSTANTIVELY ADDRESSED, citation cleanup still required.**
   - The synthesis no longer concludes that no harness exposes spawn-as-operator-action. It explicitly names Claude Code Remote Control `--spawn`, OpenCode `serve`, Codex thread creation, Cursor Cloud Agents, Antigravity managed environments, and Claude Dispatch as prior art.
   - The novelty framing is now correctly scoped: Patchbay is novel as **harness-agnostic + durable/authority-bearing spawn**, not as the spawn primitive itself.
   - No stale current-conclusion wording of “no harness exposes spawn” remains. The only occurrence of that idea is in the methodology correction as the prior, explicitly reversed failure.
   - Citation cleanup remains: the synthesis says the SNC landscape pointer-attestation exists, but it does not actually cite `[snc-rao-landscape]` in the load-bearing methodology/spawn-vs-pilot prose.

2. **Spine restructured to spawn/attach/operate/receive/payload — ADDRESSED with one wording overclaim.**
   - The parent now uses the five-primitive spine: **spawn / attach / operate / receive / payload**.
   - Attach is split out from operate/drive and is grounded across harnesses: Pi `pair_request` + `session_sync` via `[pi-extension]`, Claude Code remote/mobile connection and sync via `[claude-code-remote-control]{1}`/`{5}`/`{9}`, and OpenCode client connection to a running `serve` via `[opencode-session-handler]` plus `[snc-rao-ae-opencode-cli]{3}`.
   - Mechanical wording issue: the common-action introduction says “Every surveyed harness exposes the following action classes,” but the Spawn table itself says Aider has no control-surface spawn, and the Attach table says Aider has no separate scripting attach. Revise this to “surveyed surfaces fit the following action classes” or qualify per primitive.

3. **No remaining stale spawn overclaim — PASSES.**
   - Seed answers 4 and 6 now state the four spawn postures and acknowledge prior art.
   - The contradictions section now treats spawn-scope divergence as the live issue, not absence of spawn.
   - No stale “no harness exposes spawn” conclusion remains outside the historical-methodology sentence.

4. **Citation chain sound — NEEDS-REVISION.**
   - `[snc-rao-sp-cc-remote-control]` resolves to a Patchbay pointer-attestation and points to the real SNC attestation `/home/agent/projects/SNC/.research/attestation/rao-sp-cc-remote-control.md`, which supports `claude remote-control --spawn <mode>` and `--capacity N`.
   - `[snc-rao-ae-opencode-cli]` resolves to a Patchbay pointer-attestation and points to `/home/agent/projects/SNC/.research/attestation/rao-ae-opencode-cli.md`, which supports `opencode serve` and `opencode attach`.
   - `[snc-rao-landscape]` resolves to a Patchbay pointer-attestation and points to `/home/agent/projects/SNC/.research/analysis/briefs/remote-agent-operation-landscape.md` plus the deployed guide `/home/agent/projects/SNC/docs/ops/remote-agent-piloting.md`; however, `parent.md` mentions the handle only in backticks and never uses it as a bracket citation.
   - `[snc-rao-sp-cc-desktop]{2}` is cited in `parent.md` for Dispatch, but no `.research/attestation/snc-rao-sp-cc-desktop.md` pointer-attestation exists. The underlying SNC source exists at `/home/agent/projects/SNC/.research/attestation/rao-sp-cc-desktop.md`; add a Patchbay pointer-attestation or cite Dispatch through an existing resolving handle with an attested passage.

## Jobs (a)–(d) on new/reconciled issues

### Job (a): Semantic citation-chain walk

- Remote Control `--spawn` and OpenCode `serve` chains are semantically sound through their Patchbay pointer-attestations to SNC source-direct attestations.
- Dispatch is source-supported in SNC (`rao-sp-cc-desktop.md`) but the Patchbay parent citation `[snc-rao-sp-cc-desktop]{2}` is broken because the corresponding Patchbay pointer-attestation file is missing.
- The prior landscape/deployed-systemd-unit claim is source-real, but not bracket-cited in `parent.md`; add `[snc-rao-landscape]{1}` / `{2}` / `{3}` where the parent invokes that prior art.

### Job (b): Claim-shapes lint missed

- “Every surveyed harness exposes the following action classes” is too broad after adding Spawn and Attach as top-level primitives; Aider has no control-surface spawn and no separate scripting attach. This is a claim-shape overreach, not a taxonomy failure.
- The “not novel as a primitive; novel as harness-agnostic + durable/authority-bearing” claim is now appropriately scoped and no longer overclaims spawn novelty.

### Job (c): Coherence-read for smoothed contradictions

- Spawn/attach/operate coherence is strong: main body, contradiction section, disconfirming analysis, and seed answers now agree on the five-primitive spine and four spawn postures.
- The only coherence issue is the introduction’s universal-exposure wording, which conflicts with the per-harness Spawn/Attach rows.

### Job (d): Noise-domination / relevance-weighting

- The revised synthesis now foregrounds the operator’s SNC corpus and the actual spawn prior art rather than the old absence finding.
- Citation weighting still needs repair for Dispatch and the landscape pointer: both are central to the reconciliation narrative, so their bracket citations must resolve directly in Patchbay’s attestation tier.

## Final reconciliation verdict

**NEEDS-REVISION.** The substantive spawn correction and the spawn/attach/operate/receive/payload spine are correct. The remaining blockers are mechanical but grounding-relevant: add or fix the Dispatch pointer citation (`[snc-rao-sp-cc-desktop]`), actually cite `[snc-rao-landscape]` where the SNC prior landscape is used, and soften the universal “Every surveyed harness exposes” sentence to avoid overclaiming Spawn/Attach exposure for Aider.

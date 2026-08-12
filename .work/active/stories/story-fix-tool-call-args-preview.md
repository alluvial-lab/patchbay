---
id: story-fix-tool-call-args-preview
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Tool rows discard call arguments — no "what is it doing" preview

## Symptom

Operator report (2026-07-27, clarified): in Pi's own UI a tool call renders
as a preview box with the call's content (the bash command, the file being
read). The cockpit shows only `Running bash` / `read finished` — the args are
invisible, so the operator can't see what the agent is actually doing (or
what they approved).

## Root cause

Presentation-only. The transcript events carry full args
(`tool_requested.args` — verified in the durable log:
`{"command":"pwd && …"}`, `{"path":"docs/VISION.md"}`), and
`tool_finished.result` is present too. The cockpit fold
(`web-cockpit/src/domain/model.ts` `foldTranscriptObservation`) builds the
tool row body as just `Running **${tool}**` and discards args/result.

## Fix approach

- Fold: extract a compact plain-text preview into a new optional
  `ObservationView.detail` field. Prefer well-known arg keys in order
  (`command`, `path`, `filePath`, `file`, `query`, `pattern`, `url`,
  `prompt`); fall back to truncated `JSON.stringify(args)`. Truncate at 240
  chars with an ellipsis. `tool_finished` gains a result/error preview with
  the same truncation.
- Render: `session-detail.ts` renders `detail` for tool rows as a
  `<pre class="msg__detail">` code block — NOT through markdown (args are
  untrusted content; plain text only, no injection surface). Small CSS block
  reusing existing design tokens (monospace, subtle background, pre-wrap,
  bounded max-height).
- No mockup: reuses the existing message-row component per the mockup-first
  skip rule.

## Regression test

`web-cockpit/tests/model.test.ts` — fold of a `tool_requested` with
`args.command` produces `detail` = the command; path-key tools preview the
path; unknown arg shapes fall back to truncated JSON; oversized args/results
truncate at 240 chars with ellipsis; `tool_finished` previews the result.

## Implementation notes (2026-07-27)

- **Execution capability**: inline host — small single-package presentation fix.
- **Files changed**: `web-cockpit/src/domain/model.ts` (`ObservationView.detail`,
  `toolPreview` extraction + truncation, distinct `:finished` row id),
  `web-cockpit/src/ui/session-detail.ts` (plain-text `<pre>` render),
  `web-cockpit/src/ui/shell.css` (`.msg__detail` block on existing tokens),
  `web-cockpit/tests/model.test.ts` (fold regression test).
- **Four-step confirmation**: (1) new fold test passes (58/58 suite green);
  (2) full suite green + presentation conformance (WCAG/state bindings) green;
  (3) live repro = operator hard-reload; (4) symptom verification pending
  operator confirmation.
- **Bounded inline review verdict**: minimal presentation-only diff; args
  rendered strictly as text (no markdown/innerHTML path — no injection
  surface); 240-char truncation bounds payload size; requested/finished rows
  keep current two-row shape (single-row call lifecycle is a UX redesign,
  parked for the cockpit UX batch); one pre-existing sloppiness fixed along
  the way (requested and finished rows shared one id — now `…:finished`).

## Review polish (2026-07-27)

Operator feedback: the preview rendered as a separate sibling box — awkward.
Folded the `<pre class="msg__detail">` INSIDE `msg__body` (one card) and
restyled: no outer border/background, a top divider line instead. Suite 58/58
and presentation conformance still green.

## Correction (2026-07-27)

The earlier "fold into the message card" commit (f6e7053) silently contained
ONLY the CSS half: the session-detail.ts edit was in a rejected multi-edit
batch and never applied, and I failed to verify. Sibling `<pre>` JS +
unboxed CSS produced the operator-reported "tool call in a box, text outside
the box." This correction applies the JS side (`body.append(detail)`) for
real; verified present in the built bundle this time.

## Operator confirmation (2026-07-27)

One-card rendering confirmed visually by the operator. The 240-char `…`
truncation is cockpit-side display capping (full args remain in the durable
log); expandable-preview / cap-tuning ideas noted for the cockpit settings
batch. Story closed to done.

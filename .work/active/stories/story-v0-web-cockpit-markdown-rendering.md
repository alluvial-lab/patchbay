---
id: story-v0-web-cockpit-markdown-rendering
kind: story
stage: done
tags: [ux]
parent: feature-v0-web-cockpit
depends_on: [story-v0-web-cockpit-presentation-model-fold]
created: 2026-07-20
updated: 2026-07-20
release_binding: null
gate_origin: null
---

# Story: Cockpit Unit 3 — markdown rendering (the mobile-readability differentiator)

Implements Unit 3 of `feature-v0-web-cockpit`. This is the v0.1.0 hard
requirement that separates the cockpit from a terminal: agent Observation
payloads (markdown) rendered with excellent mobile readability.

## Scope

`web-cockpit/src/ui/markdown.ts` — render agent Observation payloads (markdown)
into the message timeline. Headings, paragraphs, lists, tables, blockquotes,
inline code, fenced code blocks with sane horizontal scroll (not
layout-breaking).

## Implementation notes

- **Spike the renderer choice first** (the session-note directive). Must be
  small + safe + streaming-friendly. Suggested baseline: `marked` +
  `DOMPurify` for sanitization, or a streaming-friendly parser. Evaluate
  bundle size against the cockpit's budget before committing.
- The payload is source-authenticated but still untrusted at the render
  boundary — **sanitize**. Source authentication is not a substitute for
  output encoding.
- Code blocks: `overflow-x: auto` on `<pre>`, never `overflow: hidden` (which
  breaks long lines). The mock's `pre` treatment in
  `.mockups/screens/feature-v0-web-cockpit/option-2.html` is the reference.
- Tables: horizontal-scroll wrapper on narrow viewports; never let a wide
  table break the chat column (capped 860px on desktop, full-width on mobile).
- Typography: locked Plex Sans body face (from `tokens.css`); code uses Plex
  Mono. Do not introduce a new typeface.
- Consume `tokens.css` for sizing/spacing — do not hardcode pixel values that
  drift from the design system.

## Acceptance criteria

- [x] Markdown renders headings, lists, code blocks, tables, blockquotes,
  inline code on a 360px viewport without horizontal page-scroll
- [x] Code blocks scroll internally, not the page
- [x] Rendered output is sanitized (no unescaped HTML injection) — verified
  with a payload containing `<script>` and `javascript:` hrefs
- [x] Long content does not break the chat column width (860px desktop / 100vw
  mobile cap holds)

## Verification evidence

- Regression test: markdown rendering on a 360px viewport (the
  differentiator) — a representative payload (headings, wide table, long code
  block, nested lists, blockquote) renders without horizontal page-scroll and
  without breaking the column.
- Security test: a malicious payload (`<script>`, `javascript:`, `onerror=`)
  is neutralized in the rendered DOM.

## Risk

The renderer choice is the spike. A heavy parser bloats the bundle; an unsafe
one is an XSS vector despite source authentication. If the spike finds no
satisfactory option, surface as a blocker — do not silently ship an unsafe or
bloated choice.

## Implementation notes

- Execution capability: inline feature owner; bounded renderer spike and implementation stayed local.
- Review weight: standard (project/default); child checkpoint closes directly, integrated review remains feature-level.
- Renderer spike: selected `marked@18.0.7` + `DOMPurify@3.4.12`. Installed browser ESM artifacts measure 41,878 B / 12,794 B gzip (`marked`) and 117,877 B / 33,576 B gzip (DOMPurify), about 46.4 KiB gzip combined before tree-shaking. This is a small, maintained parser/sanitizer boundary with stateless accumulated-text re-rendering suitable for streamed message updates; no unsafe custom sanitizer was introduced.
- Files changed: `web-cockpit/package.json`, `web-cockpit/package-lock.json`, `web-cockpit/src/ui/markdown.ts`, `web-cockpit/src/ui/markdown.css`, `web-cockpit/tests/markdown.test.ts`.
- Tests added: representative 360px-column structure test, wide-table/code containment assertions, malicious HTML/URL/handler sanitization, and accumulated streaming re-render regression.
- Simplification: one renderer factory owns parsing, sanitization, and table wrapping; layout CSS uses locked tokens and no protocol-state bindings.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Verification result

- `cd web-cockpit && npm test` — pass (12 tests total).
- Security payloads containing `<script>`, `javascript:` href, `onerror`, inline style, form, and input are removed or neutralized.
- Narrow-layout regression asserts every table is placed in an accessible horizontal-scroll region, code blocks use internal `overflow-x: auto`, and the markdown root remains `max-width: 100%` with no hidden overflow.

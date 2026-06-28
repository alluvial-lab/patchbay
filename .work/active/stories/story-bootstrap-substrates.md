---
id: story-bootstrap-substrates
kind: story
stage: done
tags: [foundation]
parent: epic-foundation-hardening
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Story: Bootstrap work and research substrates

Scaffold Patchbay's `.work/` and `.research/` substrates so design/refinement work can be tracked operationally and future grounded research has a conformant home.

## Implementation notes

Created:

- `AGENTS.md` with Patchbay orientation and conversion/adoption guidance for agile-workflow and agentic-research.
- `.work/CONVENTIONS.md` with layout, frontmatter, tags, work/research handoff rules, and `work-view` usage.
- `.work/active/`, `.work/backlog/`, `.work/archive/`, `.work/releases/`, and `.work/bin/work-view`.
- `.research/CONVENTIONS.md` with lightweight ARD-style layout, attestation schema, citation rule, conversion/adoption guidance, and work handoff.
- `.research/{reference,attestation,notes,precis,analysis/briefs}`.
- `.agents/rules/` with substrate and code-design guidance.

## Verification

- Run `.work/bin/work-view --version`.
- Run `.work/bin/work-view --ready --paths` after items are filed.

## Review focus

Confirm the substrate shape is adequate for a greenfield repo and that the conversion/adoption guidance correctly keeps agile-workflow operational state separate from agentic-research source-grounded artifacts.

## Review (2026-06-28)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Substrate fast-lane review. Verified `.work/bin/work-view --version` and `.work/bin/work-view --ready --paths` run successfully; inspected `.work/`, `.research/`, `.agents/rules/`, and conventions. The substrate split is adequate for a greenfield repo and keeps operational state in `.work/` separate from source-grounded research in `.research/`.

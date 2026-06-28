---
id: story-bootstrap-substrates
kind: story
stage: review
tags: [foundation]
parent: epic-foundation-hardening
depends_on: []
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

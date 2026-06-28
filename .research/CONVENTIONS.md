# Patchbay research conventions

Patchbay uses a lightweight ARD-style `.research/` substrate for source-grounded research that informs protocol, security, verification, UX, and adapter decisions.

## Layout

- `.research/reference/` — raw source captures or corpus indexes when source material is stored locally.
- `.research/attestation/<handle>.md` — source-direct attestations. These are the only citation anchors for `[handle]{N}` references.
- `.research/notes/` — optional orientation notes that are not citation anchors.
- `.research/precis/` — source-coherent summaries when a source needs durable engagement beyond an attestation.
- `.research/analysis/briefs/<slug>.md` — cross-source research briefs.

## Attestation frontmatter

```yaml
---
source_handle: <handle>
fetched: YYYY-MM-DD
source_url: <url>      # for web sources
source_path: <path>    # for local source/docs files
provenance: source-direct
---
```

## Citation rule

Use `[handle]{N}` only when `.research/attestation/<handle>.md` exists and records the cited detail. Do not cite memory, prior analysis, or work items as source substrate.

## Conversion / adoption guidelines

When adopting pre-existing research into Patchbay:

1. Run the agentic-research `convert` workflow when available.
2. Discover candidate research broadly; do not hardcode a small path list.
3. Ask the operator to classify candidates as research/not-research and raw source vs claim-bearing synthesis.
4. Route raw sources to `reference/`.
5. Route claim-bearing legacy synthesis to `.research/.import-holding/` for refresh/rigor uplift; do not drop it directly into `analysis/` as if it were conformant.
6. Preserve-only by default. Destructive cleanup requires content-integrity and reference-integrity checks.

Patchbay is currently greenfield: there is no legacy research imported into this substrate yet.

## Work handoff

Operational work lives in `.work/`. Research informs it via:

- `research_refs:` on work items that consume a research artifact.
- `research_origin:` on work items emitted from a completed research artifact.
- `[research]` work items with `research_dials:` for commissioned engagements.

Research artifacts do not rewrite work items. Work items do not rewrite research artifacts.

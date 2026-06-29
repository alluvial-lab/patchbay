---
provenance: adversarial-read-verification
updated: 2026-06-28
synthesis: .research/analysis/briefs/protocol-contract-tooling.md
attestation_dir: .research/attestation/
rigor: standard
verdict: APPROVED
---

# Verification checklist: protocol-contract-tooling

## Inputs checked

- Read canonical ARD discipline bundle: `/home/agent/.pi/agent/git/github.com/nklisch/skills/plugins/agentic-research/ard-core/kernel/discipline.md`.
- Read adversarial-reader role brief: `/home/agent/.pi/agent/git/github.com/nklisch/skills/plugins/agentic-research/skills/research-orchestrator/references/adversarial-reader.md`.
- Read synthesis: `.research/analysis/briefs/protocol-contract-tooling.md`.
- Read all cited attestations in `.research/attestation/`:
  - `buf-breaking.md`
  - `buf-generate.md`
  - `connectrpc.md`
  - `json-schema-core.md`
  - `openapi-generator-rust.md`
  - `openapi-generator-typescript.md`
  - `prost.md`
  - `protobuf-es.md`
  - `protojson.md`
  - `typebox.md`
  - `typespec-protobuf.md`
  - `typespec.md`
  - `zod-json-schema.md`
- Citation inventory: 59 citation instances found locally, matching orchestrator lint summary. All cited handles and cited key-passage numbers used by the synthesis exist in the attestation set.

## (a) Semantic citation-chain walk

Result: no blocking issues.

- Buf generation claims are semantically supported by `buf-generate` passages 1-4: plugin execution, `buf.gen.yaml`, checked-in config, pinned remote plugins, and managed mode are all attested.
- Buf compatibility claims are semantically supported by `buf-breaking` passages 1 and 4: comparison against past input and rule categories are attested.
- Protobuf-ES TypeScript-generation claims are semantically supported by `protobuf-es` passages 1 and 3.
- prost Rust-generation claims are semantically supported by `prost` passages 1 and 3.
- Connect/ProtoJSON boundary claims are semantically supported by `connectrpc` passages 1, 3, 4 and `protojson` passages 1, 2, 4.
- JSON Schema, TypeBox, Zod, TypeSpec, TypeSpec Protobuf, and OpenAPI Generator alternative assessments are semantically supported by their cited attestations.

Minor non-blocking note: several recommendation sentences combine project needs with source-attested tool capabilities. That is appropriate for an architecture-decision synthesis, but if this brief is later promoted into a more formal ARD decision record, consider adding sparse `{inferred: ...}` markers to the highest-level comparative conclusions.

## (b) Claim-shapes mechanical lint may miss

Result: no blocking issues.

- No uncited external named-feature claim was found that changes the recommendation materially.
- No cite-through overextension was found; all sources are direct attestations rather than in-corpus reports about non-corpus sources.
- Comparative language such as "better fit", "weaker first choice", and "good fit" is framed as Patchbay-specific analysis rather than as a source-authored ranking.

Non-blocking watch item: implementation note `buf breaking --against '.git#branch=main'` is a concrete CLI example not directly quoted in the attested key passages. It is not central to the recommendation, but exact command syntax should be verified before copying into executable docs or CI.

## (c) Coherence read for smoothed contradictions

Result: no issues surfaced.

- The brief does not smooth a direct source contradiction. It explicitly represents the main tension: JSON Schema-family tools fit JSON-native validation; Protobuf + Buf fits cross-language generated protocol contracts and lifecycle checks; ProtoJSON bridges JSON but with weaker evolution properties.
- The `Contradictions and tensions` section accurately states that no direct source contradiction surfaced and preserves the architectural tension rather than merging it away.

## (d) Noise-domination / relevance-weighting

Result: no issues surfaced.

- Major claims cite the most relevant attestations available in the corpus: Buf claims cite Buf docs, Protobuf-ES claims cite Protobuf-ES docs, Rust generation claims cite prost, ProtoJSON limitations cite ProtoJSON, TypeSpec claims cite TypeSpec/TypeSpec-Protobuf, and OpenAPI Generator claims cite generator docs.
- No less-relevant citation appears to dominate where a more directly relevant attestation was available and uncited.

## (e) Quote-context walk (`GR.4`)

Result: no issues surfaced.

- The synthesis does not use verbatim block quotes from sources. It paraphrases attested key passages and cites the corresponding attestation entries.
- No source qualifier visible in the attestations appears stripped in a way that changes a cited claim.

## (f) Analytical-tier-inheritance walk

Result: no issues surfaced.

- All citation handles resolve to source-direct attestation files, not analytical-tier artifacts.
- The brief uses Patchbay-specific project framing as synthesis analysis, not as a cited source substrate.

## (g) Line-reference / sub-attestation granularity walk

Result: no issues surfaced.

- Citations use attestation key-passage numbers (`[handle]{N}`), not source line ranges.
- Every cited key-passage number exists in its attestation file and is granular enough for the claim it supports.

## (h) Thin-attestation semantic check (`GR.5` complement)

Result: no issues surfaced.

- Each cited attestation has required source-direct frontmatter, a substantive summary, and multiple key passages where needed.
- No cited attestation is merely a token heading or whole-source paraphrase unable to support per-claim citation.

## Final verdict

verdict: APPROVED

No required synthesis revisions. Optional future hardening: add sparse epistemic-status markers to top-level comparative recommendations if this brief becomes a formal decision record, and verify exact Buf CLI command syntax before turning implementation notes into CI scripts.

---
source_handle: quint-docs-cli
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/informalsystems/quint/main/docs/content/docs/quint.md
provenance: source-direct
---

# Per-source attestation: quint-docs-cli

## Paraphrased summary

The Quint CLI documentation describes the `compile` and `verify` commands, including TLA+ compilation output, TLC backend selection, TLC runtime configuration, Java requirements, and verification output behavior.

## Key passages

[quint-docs-cli]{1} The document lists `compile` as parsing, typechecking, and processing a Quint specification into the requested target format, and lists `verify` as verifying a Quint specification with symbolic model checking via Apalache or explicit-state model checking via TLC.

[quint-docs-cli]{2} The `compile --help` section lists `--target` choices as `tlaplus` and `json`, with `json` as the default, and includes `--init`, `--step`, `--invariant`, and `--temporal` options.

[quint-docs-cli]{3} The compile section says TLA+ output requires flattening; if `--flatten=false` is supplied with TLA+ output, Quint warns and ignores it.

[quint-docs-cli]{4} The `verify --help` section lists `--backend` choices `apalache` and `tlc`, with `apalache` as the default, and includes `--invariant`, `--temporal`, `--invariants`, and `--tlc-config` options.

[quint-docs-cli]{5} The verify section says both backends require a compatible OpenJDK installation.

[quint-docs-cli]{6} The TLC section gives the example `quint verify --backend tlc myspec.qnt`.

[quint-docs-cli]{7} The TLC section says TLC performs exhaustive state enumeration, works well for finite-state specifications, and cannot handle specifications with infinite domains.

[quint-docs-cli]{8} The TLC section says TLC runtime can be configured via `--tlc-config` with JSON fields such as `maxHeap`, `stackSize`, and `workers`, and gives an example with `"workers": "auto"`.

[quint-docs-cli]{9} The output section says the verify command sends the Quint specification to the selected model checker and, if it finds an invariant violation, prints the trace on standard output.

## Structural metadata

- Source type: repository documentation file.
- Repository branch fetched: `main`.
- Document: Quint CLI manual.

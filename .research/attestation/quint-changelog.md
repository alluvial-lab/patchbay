---
source_handle: quint-changelog
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/quint-co/quint/main/CHANGELOG.md
provenance: source-direct
---

# Attestation: Quint changelog

## Structural metadata

- Source kind: project changelog (`CHANGELOG.md`) from the Quint repository.
- Local fetched copy: `.research/reference/quint/CHANGELOG.md`.
- Sections used: `UNRELEASED`, `v0.32.0 -- 2026-03-31`, `v0.31.0 -- 2026-02-27`.

## Paraphrased summary

The changelog records recent changes around temporal operators, TLC support, Rust backend defaults, and temporal checking warnings. Version `0.32.0` adds `leadsTo` and updates TLC backend behavior. Version `0.31.0` adds TLC as an alternative backend for `quint verify`, adds Rust backend support for `test` and `repl`, changes `quint run` and `quint test` default backend to Rust, and records a warning when checking temporal formulas with Apalache.

## Key passages

### {1} v0.32.0 date and leadsTo

The changelog has heading `## v0.32.0 -- 2026-03-31` and under Added says `Added leadsTo temporal operator (#1932)`.

Anchor: lines 23-26.

### {2} v0.32.0 TLC backend change

Under `v0.32.0`, Changed says the TLC backend now ensures the Apalache distribution is available locally before running TLC.

Anchor: lines 28-32.

### {3} v0.31.0 TLC backend addition

Under `v0.31.0 -- 2026-02-27`, Added says `Added TLC as an alternative backend to quint verify via --backend=tlc (#1844)`.

Anchor: lines 48-52.

### {4} v0.31.0 Rust backend defaults

Under `v0.31.0`, Changed says `Switched the default backend for quint run and quint test to Rust (#1919)`.

Anchor: lines 70-77.

### {5} v0.31.0 temporal warning

Under `v0.31.0`, Fixed says `Added a warning when checking temporal formulas with Apalache (#1908)`.

Anchor: lines 83-85.

### {6} unreleased init/step fix and JSON compile change

The `UNRELEASED` Fixed section says `--step`/`--init` resolution was fixed when a state variable is named `step` or `init`, and `quint compile --target=json` no longer requires `init` and `step` to exist in the module.

Anchor: lines 11-18.

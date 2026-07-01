---
source_handle: quint-npm-registry
fetched: 2026-07-01
source_url: https://registry.npmjs.org/@informalsystems%2Fquint
provenance: source-direct
---

# Attestation: npm registry metadata for @informalsystems/quint

## Structural metadata

- Source kind: npm registry JSON for package `@informalsystems/quint`.
- Local fetched copy: `.research/reference/quint/npm-informalsystems-quint.json`.
- Extracted latest dist-tag during fetch: `0.32.0`.

## Paraphrased summary

The npm registry metadata identifies the canonical npm package as `@informalsystems/quint`; its `latest` dist-tag points to version `0.32.0`. The metadata for version `0.32.0` declares the executable bin name `quint`, Node engine requirement `>=18`, repository and homepage values, tarball URL, and publication timestamp.

## Key passages

### {1} package name and latest dist-tag

The registry JSON has `name` equal to `@informalsystems/quint` and `dist-tags.latest` equal to `0.32.0`.

Anchor: top-level `name`, `dist-tags.latest`.

### {2} bin entry and Node engine

The metadata for version `0.32.0` has:

```json
"bin": { "quint": "dist/src/cli.js" },
"engines": { "node": ">=18" }
```

Anchor: `versions["0.32.0"].bin`, `versions["0.32.0"].engines`.

### {3} tarball and publication timestamp

The metadata for version `0.32.0` gives tarball URL `https://registry.npmjs.org/@informalsystems/quint/-/quint-0.32.0.tgz`. The `time` object gives `0.32.0` publication time `2026-03-31T13:40:03.331Z`.

Anchor: `versions["0.32.0"].dist.tarball`, `time["0.32.0"]`.

### {4} repository and homepage

The metadata for version `0.32.0` gives repository URL `git+https://github.com/informalsystems/quint.git` and homepage `https://github.com/informalsystems/quint`.

Anchor: `versions["0.32.0"].repository`, `versions["0.32.0"].homepage`.

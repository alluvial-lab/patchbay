---
source_handle: quint-tlc-source
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/informalsystems/quint/main/quint/src/tlc.ts
provenance: source-direct
---

# Per-source attestation: quint-tlc-source

## Paraphrased summary

The Quint TLC backend source defines Quint's TLC runtime configuration defaults, generated TLC `.cfg` content, Apalache-jar lookup, and the Java process invocation used to run TLC over a generated TLA+ file.

## Key passages

[quint-tlc-source]{1} The source defines default TLC JVM/runtime settings: max heap `-Xmx8G`, stack size `-Xss515m`, and workers `auto`.

[quint-tlc-source]{2} The `TlcRuntimeConfig` interface accepts optional `maxHeap`, `stackSize`, and `workers` fields, and `loadTlcConfig` reads these from a JSON file when a config path is supplied.

[quint-tlc-source]{3} The generated TLC config starts with `INIT q_init` and `NEXT q_step`.

[quint-tlc-source]{4} If the Quint verification config has an invariant, generated TLC config adds `INVARIANT q_inv`.

[quint-tlc-source]{5} If the Quint verification config has temporal properties, generated TLC config adds `PROPERTY q_temporalProps`.

[quint-tlc-source]{6} The TLC backend looks for an Apalache jar at `apalache/lib/apalache.jar` under Quint's Apalache distribution directory and reports an error if it is absent.

[quint-tlc-source]{7} The TLC backend writes the generated TLA+ code to a temporary `<module>.tla` file and writes the generated cfg content to a temporary `<module>.cfg` file in the same temporary directory.

[quint-tlc-source]{8} The TLC backend spawns Java with arguments including max heap, stack size, `-Djava.io.tmpdir=<tmpDir>`, `-cp`, the jar path, `tlc2.TLC`, `-deadlock`, `-workers`, the configured worker count, `-metadir`, the temporary directory, and the temporary TLA+ file path.

[quint-tlc-source]{9} The source treats exit code `0` as success and exit codes from `10` through `14` as violation exit codes.

## Structural metadata

- Source type: TypeScript source file.
- Repository branch fetched: `main`.
- Module: Quint TLC backend.

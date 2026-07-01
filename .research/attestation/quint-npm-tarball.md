---
source_handle: quint-npm-tarball
fetched: 2026-07-01
source_url: https://registry.npmjs.org/@informalsystems/quint/-/quint-0.32.0.tgz
provenance: source-direct
---

# Attestation: @informalsystems/quint 0.32.0 npm tarball

## Structural metadata

- Source kind: npm package tarball for `@informalsystems/quint@0.32.0`.
- Local fetched/extracted copies under `.research/reference/quint/npm-package/`.
- Files inspected: `package/package.json`, `package/dist/src/cli.js`, `package/dist/src/apalache.js`, `package/dist/src/verify.js`, `package/dist/src/tlc.js`, `package/dist/src/compileToTlaplus.js`.

## Paraphrased summary

The package tarball contains the compiled CLI and runtime code for `@informalsystems/quint@0.32.0`. Its package manifest names the package and version, defines the `quint` executable, and requires Node `>=18`. The CLI source defines `compile`, `run`, `test`, and `verify` subcommands. The verification code dispatches to Apalache or TLC; Apalache is the default backend and pinned to version `0.56.1`; TLC verification compiles to TLA+ through Apalache and invokes TLC via Java.

## Key passages

### {1} package name, version, bin, engine

`package/package.json` declares:

```json
"name": "@informalsystems/quint",
"version": "0.32.0",
"bin": { "quint": "dist/src/cli.js" },
"engines": { "node": ">=18" }
```

Anchor: extracted `package.json`, lines 2-43.

### {2} compile command and targets

`package/dist/src/cli.js` defines `command: 'compile <input>'` with description “compile a Quint specification into the target, the output is written to stdout”. Its `--target` option has choices `tlaplus` and `json`, default `json`; `--flatten` defaults true; `--apalache-version` defaults to `DEFAULT_APALACHE_VERSION_TAG`; `--server-endpoint` defaults `localhost:8822`.

Anchor: extracted `dist__src__cli.js`, lines 80-112.

### {3} run command and defaults

`package/dist/src/cli.js` defines `command: 'run <input>'` with description “Simulate a Quint specification and (optionally) check invariants”. Options include `--out-itf`, `--max-samples`, `--n-traces` default `1`, `--max-steps` default `20`, `--init` default `init`, `--step` default `step`, `--invariants` array default `[]`, `--invariant` default `true`, `--witnesses`, `--hide`, `--seed`, `--mbt`, and `--backend` choices `typescript`/`rust` default `rust`.

Anchor: extracted `dist__src__cli.js`, lines 218-300.

### {4} test command and defaults

`package/dist/src/cli.js` defines `command: 'test <input>'` with description “Run tests against a Quint specification”. Options include `--main`, `--out-itf`, `--max-samples`, `--seed`, `--verbosity`, `--match`, and `--backend` choices `typescript`/`rust` default `rust`.

Anchor: extracted `dist__src__cli.js`, lines 161-213.

### {5} verify command and defaults

`package/dist/src/cli.js` defines `command: 'verify <input>'` with description “Verify a Quint specification via Apalache”. Options include `--invariants`, `--inductive-invariant`, `--out-itf`, `--max-steps` default `10`, `--random-transitions` default `false`, `--apalache-config`, `--apalache-version`, `--server-endpoint` default `localhost:8822`, `--backend` choices `apalache`/`tlc` default `apalache`, and `--tlc-config`.

Anchor: extracted `dist__src__cli.js`, lines 312-370.

### {6} Apalache version, commands, and counterexample representation

`package/dist/src/apalache.js` sets `DEFAULT_APALACHE_VERSION_TAG = '0.56.1'`. It wraps an Apalache command executor with `CHECK` for verification and `TLA` for TLA+ conversion. Its failure handling maps bounded-checker `Error` to explanation `found a counterexample` with `counterexamples` traces, and `Deadlock` to explanation `reached a deadlock` with `counterexamples` traces.

Anchor: extracted `dist__src__apalache.js`, lines 129-147 and 181-186.

### {7} Apalache download/server lifecycle

`package/dist/src/apalache.js` downloads `https://github.com/apalache-mc/apalache/releases/download/v${apalacheVersion}/apalache.tgz` if no local binary exists, then connects to an existing Apalache server or downloads, launches `apalache-mc server --port=<port>`, and connects via gRPC.

Anchor: extracted `dist__src__apalache.js`, lines 290-318 and 362-433.

### {8} TLC backend path

`package/dist/src/verify.js` implements `verifyWithTlcBackend`. It converts the main module init, serializes the compiled module to JSON, prints “Compiling to TLA+ (via Apalache)” when verbose, calls `compileToTlaplus`, ensures the Apalache distribution is available locally because TLC needs the JAR, then calls the TLC runner with `tlaCode`, module name, invariant/temporal flags, Apalache version, TLC runtime config, and verbosity.

Anchor: extracted `dist__src__verify.js`, lines 32-70.

### {9} Apalache backend path and inductive invariant phases

`package/dist/src/verify.js` implements `verifyWithApalacheBackend`. For `--inductive-invariant`, it runs 2 or 3 Apalache checks: initial states at `maxSteps: 0`, preservation with `maxSteps: 1`, and, when ordinary invariants are present, implication of ordinary invariant at `maxSteps: 0`. Without inductive invariants, it builds an Apalache config with `q::inv` when invariants exist and calls Apalache.

Anchor: extracted `dist__src__verify.js`, lines 90-141.

### {10} TLC generated config and Java invocation

`package/dist/src/tlc.js` generates a TLC config with `INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`, and optional `PROPERTY q_temporalProps`. It locates `apalache.jar` under the Apalache distribution and spawns Java with `tlc2.TLC`, `-deadlock`, `-workers`, `-metadir`, and the generated TLA file.

Anchor: extracted `dist__src__tlc.js`, lines corresponding to `generateCfg`, `findApalacheJar`, and `verify`.

### {11} TLA+ compile implementation

`package/dist/src/compileToTlaplus.js` says it uses Apalache to convert Quint parse data into TLA+. It builds a config with input source type `string`, format `qnt`, content as parse-data JSON, connects to Apalache, and calls `conn.tla(config)`.

Anchor: extracted `dist__src__compileToTlaplus.js`, full file.

### {12} result and JSON output shapes

`package/dist/src/cliReporting.js` prints `[ok] No violation found` for successful TLC or Apalache verification. For Apalache failures with traces, it derives status `violation`, prints `[violation] Found an issue`, optionally writes an ITF trace when `--out-itf` is set, and returns a stage object with `status`, `errors`, and `trace`. Its JSON output picker includes `stage`, `warnings`, `modules`, `table`, `types`, `effects`, `errors`, `documentation`, `passed`, `failed`, `ignored`, `status`, `trace`, `seed`, and `main`. Test output prints per-test `ok <name> passed <n> test(s)` or `<n>) <name> failed after <samples> test(s)`, plus summary counts.

Anchor: extracted `dist__src__cliReporting.js`, lines 120-181, 197-234, and 276-310.

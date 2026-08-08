---
source_handle: pi-loader
fetched: 2026-08-08
source_path: /home/agent/.local/lib/node_modules/@earendil-works/pi-coding-agent/dist/core/extensions/loader.js
provenance: source-direct
---

## Summary
The installed current Pi extension loader has two relevant cache layers. Its process-level extension cache is explicitly cleared by `clearExtensionCache()`, which empties the extension factory map and increments a generation. Extension modules are loaded through jiti with `moduleCache: false`, while `loadExtensionsCached()` can reuse a cached factory until that extension cache is cleared. The loader aliases Pi runtime packages to the already-running package entrypoints in built Node mode. Thus reload can re-read an extension entrypoint, but it is not a documented general mechanism for replacing the running Pi executable/package runtime; restarting the process is the reliable path for Pi/runtime code upgrades. The source does not support an unconditional claim that every arbitrary freshly-built `/dist` dependency will be replaced by `/reload`.

## Key passages

1. `clearExtensionCache()` calls `extensionCache.clear()`, resets the cached cwd, and increments `extensionCacheGeneration`.
2. In built Node mode, `getAliases()` maps `@earendil-works/pi-coding-agent`, `@earendil-works/pi-agent-core`, `@earendil-works/pi-tui`, and related Pi packages to entrypoints resolved under the installed, already-running package's `dist`. `loadExtensionModule()` passes these aliases to jiti. (`getAliases`; `loadExtensionModule`)
3. The loader can replace an extension factory in the current process, but it contains no mechanism that replaces the running Pi executable or its process-level package graph. Because the runtime-package aliases resolve to that installed running package's `dist`, process termination and respawn is the reliable boundary for upgrading Pi/runtime package code; this is a bounded conclusion from the inspected loader, not a documented hot-swap guarantee. (`getAliases`; `loadExtensionModule`; file-wide inspection)
4. `loadExtensionModule()` returns a cached factory when its cache token is current; otherwise it creates jiti with `moduleCache: false`, imports the extension entrypoint, and stores the newly loaded factory when a current token exists.
5. `loadExtensionsCached()` invokes the cached loading path; resource loading uses this path for discovered extension sets.

## Source-internal anchors
`dist/core/extensions/loader.js`: `extensionCacheCwd`/`extensionCacheGeneration` and `clearExtensionCache()` near the top; `loadExtensionModule()` around lines 351–370; `loadExtensionsCached()` around lines 455–459.

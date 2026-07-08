---
source_handle: connect-node-npm-current
fetched: 2026-07-07
source_url: https://registry.npmjs.org/@connectrpc/connect-node/latest
provenance: source-direct
---

# Attestation: npm registry — @connectrpc/connect-node latest

## Summary

The npm registry latest endpoint for `@connectrpc/connect-node` reports version 2.1.2. The package is Apache-2.0 licensed, uses the GitHub repository `connectrpc/connect-es` under `packages/connect-node`, requires Node.js `>=20`, and declares peer dependencies on `@bufbuild/protobuf` and the matching `@connectrpc/connect` version.

## Key passages

1. The JSON object reports `"name": "@connectrpc/connect-node"` and `"version": "2.1.2"`.

2. The `repository` field points to `git+https://github.com/connectrpc/connect-es.git` with directory `packages/connect-node`.

3. The `engines` field requires `"node": ">=20"`.

4. The `peerDependencies` field lists `"@bufbuild/protobuf": "^2.7.0"` and `"@connectrpc/connect": "2.1.2"`.

5. The package scripts include `conformance:server` and `conformance:client` commands invoking `connectconformance --mode server` and `connectconformance --mode client`.

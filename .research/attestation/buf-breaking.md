---
source_handle: buf-breaking
fetched: 2026-06-28
source_url: https://buf.build/docs/breaking/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Buf breaking-change docs

## Summary

Buf documents `buf breaking` as a schema compatibility checker that compares current Protobuf schemas with a past input and reports changes that would break clients, servers, generated code, binary wire format, or JSON encoding depending on configured rule category.

## Key passages

1. From the page introduction:

> `buf breaking` compares the current version of your Protobuf schema against a past version and reports any changes that would break clients, servers, or the code generated from those schemas. The past version can be any input the Buf CLI accepts: a BSR module, a Git repository, a tarball, or a Buf image.

2. From the example changing `int32` to `string`:

> Tag `1` now carries an incompatible wire type, so every existing client and every serialized message already in flight becomes unreadable. Catching this before merge is the point.

3. From "How compatibility is defined":

> Protobuf schemas break at different layers. Adding a new field is safe at every layer. Renaming an existing field breaks generated source code but leaves the wire format intact. Changing a field’s type breaks everything.

4. From the rule category list:

> `FILE`: Detects breakage to generated source code on a per-file basis. ... `PACKAGE`: Detects breakage to generated source code on a per-package basis. ... `WIRE_JSON`: Detects breakage to the binary wire format or JSON encoding. `WIRE`: Detects breakage to the binary wire format only.

---
source_handle: protojson
fetched: 2026-06-28
source_url: https://protobuf.dev/programming-guides/json/
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Protocol Buffers ProtoJSON docs

## Summary

The Protobuf documentation specifies ProtoJSON as a canonical JSON encoding for Protobuf messages, intended for systems that do not support the binary wire format. The same page also states non-goals: ProtoJSON is not designed to represent arbitrary JSON schemas, is less efficient than the binary wire format, and has weaker schema-evolution properties because it does not support unknown fields and encodes field and enum names.

## Key passages

1. From the page introduction:

> Protobuf supports a canonical encoding in JSON, making it easier to share data with systems that do not support the standard protobuf binary wire format.

2. From "Cannot Represent Some JSON schemas":

> The ProtoJSON format is designed to be a JSON representation of schemas which are expressible in the Protobuf schema language. It may be possible to represent many pre-existing JSON schemas as a Protobuf schema and parse it using ProtoJSON, but it is not designed to be able to represent arbitrary JSON schemas.

3. From the same section:

> For example, there is no way to express in Protobuf schema to write types that may be common in JSON schemas like `number[][]` or `number|string`.

4. From "Does not have as good schema-evolution guarantees as binary wire format":

> ProtoJSON format does not support unknown fields, and it puts field and enum value names into encoded messages which makes it much harder to change those names later. Removing fields is a breaking change that will trigger a parsing error.

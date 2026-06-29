---
source_handle: zod-json-schema
fetched: 2026-06-28
source_url: https://zod.dev/json-schema
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Zod JSON Schema docs

## Summary

Zod documents native JSON Schema conversion in Zod 4: `z.fromJSONSchema()` converts JSON Schema into a Zod schema and `z.toJSONSchema()` converts Zod schemas to JSON Schema. The page also states that some Zod types cannot be represented reasonably in JSON Schema, and that `z.fromJSONSchema()` is experimental.

## Key passages

1. From the page introduction:

> Introduced in Zod 4, Zod supports native JSON Schema conversion. JSON Schema is a standard for describing the structure of JSON (with JSON). It's widely used in OpenAPI definitions and defining structured outputs for AI.

2. From the `z.fromJSONSchema()` section:

> Experimental — The `z.fromJSONSchema()` function is experimental and is not considered part of Zod's stable API. It is likely to undergo implementation changes in future releases.

3. From the `z.toJSONSchema()` section:

> To convert a Zod schema to JSON Schema, use the `z.toJSONSchema()` function.

4. From the conversion caveat:

> All schema & checks are converted to their closest JSON Schema equivalent. Some types have no analog and cannot be reasonably represented.

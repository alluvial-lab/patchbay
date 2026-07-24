---
name: patterns
description: "Project code patterns and conventions. Auto-loads when implementing,
  designing, verifying, or reviewing code. Provides detailed pattern definitions
  with code examples."
user-invocable: false
allowed-tools: Read, Glob, Grep
---

# Project Patterns Reference

This skill contains detailed pattern documentation for this project.
See individual pattern files for full details with code examples.

Available patterns:
- [domain-owned-ports.md](domain-owned-ports.md) — Define narrow consumer-owned interfaces and adapt infrastructure or sibling domains behind them.
- [generated-protobuf-contracts.md](generated-protobuf-contracts.md) — Generate committed Rust and TypeScript wire artifacts from `.proto`; never hand-edit generated output.
- [registry-derived-protocol-boundaries.md](registry-derived-protocol-boundaries.md) — Parse, constrain, and dispatch canonical generated enums at every receiving boundary.
- [fail-fast-boundary-validation.md](fail-fast-boundary-validation.md) — Reject malformed framing, missing required fields, and unknown values before stateful work or durable append.
- [durable-log-projections.md](durable-log-projections.md) — Rebuild each in-memory view by folding its authority-domain log in validated LSN order.
- [presentation-registry-conformance.md](presentation-registry-conformance.md) — Bind every protocol state registry to checked CSS and showcase primitives without inventing states.

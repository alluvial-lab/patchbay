# Registry-Derived Protocol Boundaries

Use generated protocol enums as the shared vocabulary, then make each receiving boundary explicitly parse, constrain, and dispatch that vocabulary rather than recreating stringly typed variants.

## Rationale

Operation kinds, session states, and failure codes are protocol registries. Consumers may have narrower v0.1.0 dispositions, but must express that narrowing against the generated enum so unknown and reserved values fail closed and every component speaks the same wire vocabulary.

## Examples

### Acceptance parses and explicitly admits the committed OperationKind subset

**File**: `core/src/acceptance/pipeline.rs:24`
**File**: `core/src/acceptance/pipeline.rs:299`

```rust
pub const COMMITTED_OPERATION_KINDS: [OperationKind; 10] = [/* ... */];

let operation_kind = OperationKind::try_from(operation.kind)
    .ok()
    .filter(|kind| COMMITTED_OPERATION_KINDS.contains(kind))
    .ok_or_else(|| ValidationRejection::validation_failed(
        "operation kind is unknown or unavailable in v0.1.0",
    ))?;
```

The generated enum remains the vocabulary; the central disposition list makes v0.1.0 admission explicit.

### Pi adapter dispatches through the generated OperationKind

**File**: `pi-adapter/src/delivery.ts:22`

```ts
/** Single registry-derived OperationKind dispatch point for Pi actions. */
async deliver(operation: Operation, session: PiSession): Promise<DeliveryOutcome> {
  switch (operation.kind) {
    case OperationKind.INSTRUCT:
      await session.prompt(requiredText(operation));
      return {};
    // known unsupported and reserved kinds are explicit cases
  }
}
```

The adapter translates the canonical enum at one dispatch boundary rather than accepting adapter-local command names.

### Adapter ingress validates generated session-state values

**File**: `server/src/adapter_service.rs:558`

```rust
connectivity: SessionConnectivityState::try_from(report.connectivity)
    .map_err(|_| Status::invalid_argument("unknown connectivity state"))?,
activity: SessionActivityState::try_from(report.activity)
    .map_err(|_| Status::invalid_argument("unknown activity state"))?,
```

The server rejects unknown wire values before they can enter the durable session projection.

## When to Use

- A message carries a protocol-owned enum across a process or adapter boundary.
- A component must support a committed subset while keeping reserved values visible and fail-closed.
- Dispatch, validation, or display needs to align with canonical protocol names.

## When NOT to Use

- For local presentation-only modifiers that are deliberately not protocol states.
- By copying enum member names into unrelated local string unions.
- To let cached adapter capabilities become an authority gate; capabilities are advisory and delivery remains authoritative.

## Common Violations

- Switching on raw numeric values or arbitrary strings rather than generated enum members.
- Treating an unknown or reserved value as a default-success path.
- Maintaining a second unlinked list of protocol variants.
- Allowing a consumer to invent a new protocol state without updating the schema and required dependent checks.

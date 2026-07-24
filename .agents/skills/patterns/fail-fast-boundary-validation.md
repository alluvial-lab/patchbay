# Fail-Fast Boundary Validation

Validate required structure, identity scope, enum values, and framing at ingress; return a precise boundary error before acquiring stateful work or appending a durable record.

## Rationale

Patchbay accepts high-authority Operations and adapter reports. Letting malformed inputs travel into projection, authorization, or delivery code would blur failure semantics and risk partial state. Each ingress boundary rejects invalid input at the narrowest layer that can explain it.

## Examples

### Acceptance validates the Operation before authority and append

**File**: `core/src/acceptance/pipeline.rs:295`

```rust
let operation_kind = OperationKind::try_from(operation.kind)
    .ok()
    .filter(|kind| COMMITTED_OPERATION_KINDS.contains(kind))
    .ok_or_else(|| ValidationRejection::validation_failed(/* ... */))?;

let command_id = operation.command_id.as_ref()
    .ok_or_else(|| ValidationRejection::validation_failed("operation is missing command_id"))?;

validate_validity_window(operation, now)?;
```

Missing identifiers, unknown kinds, invalid target scope, idempotency gaps, and invalid time windows become `validation_failed` before acceptance.

### Core service rejects an incomplete RPC request at ingress

**File**: `server/src/service.rs:137`

```rust
let operation = request.get_ref().operation.as_ref()
    .ok_or_else(|| Status::invalid_argument("submit request is missing operation"))?;
let authority_domain_id = operation.authority_domain_id.clone()
    .ok_or_else(|| Status::invalid_argument("operation is missing authority_domain_id"))?;
self.require_configured_domain(&authority_domain_id)?;
```

The service establishes the minimal request and configured-domain facts before resolving issuer evidence or entering the submit gate.

### Adapter service checks registration and report enum values

**File**: `server/src/adapter_service.rs:486`
**File**: `server/src/adapter_service.rs:558`

```rust
let registration = request.registration
    .ok_or_else(|| Status::invalid_argument("attach request is missing registration"))?;

connectivity: SessionConnectivityState::try_from(report.connectivity)
    .map_err(|_| Status::invalid_argument("unknown connectivity state"))?,
```

Adapter attachment and observation ingestion reject missing or unknown wire facts before updating adapter or session state.

### Web gateway validates the gRPC-Web frame

**File**: `web-server/src/routes/rpc.ts:111`

```ts
if (!Buffer.isBuffer(body) || body.length < 5) {
  throw new ConnectError("invalid gRPC-Web request frame", Code.InvalidArgument);
}
```

Transport framing is rejected before decode/forwarding so malformed browser input never reaches the core protocol bridge.

## When to Use

- At HTTP/RPC, adapter, storage, or protocol-message ingress.
- Before authentication/authorization-dependent work when required structural data is absent.
- Before durable append, external delivery, or projection mutation.

## When NOT to Use

- To reject a valid-but-unsupported adapter capability after acceptance; that is the delivery-layer `unsupported_command` outcome.
- To convert a recoverable internal error into a misleading client validation error.
- To silently normalize an ambiguous security or identity field.

## Common Violations

- Parsing only the happy path and letting `None`, unknown enum values, or invalid framing reach later layers.
- Appending audit or command state before basic validation has passed.
- Collapsing `validation_failed`, authorization denial, and execution failure into one generic error.
- Trusting a payload-supplied identity before the authenticated ingress context verifies it.

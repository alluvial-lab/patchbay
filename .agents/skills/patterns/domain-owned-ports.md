# Domain-Owned Ports

Keep domain logic dependent on small interfaces it owns, and put infrastructure or sibling-domain translation behind implementations of those interfaces. This keeps acceptance logic deterministic and prevents a backend, clock, or ingress transport from becoming part of the domain contract.

## Rationale

The core needs time, durable storage, verified issuer evidence, authority decisions, target resolution, and Elicitation-contract lookup. Those are boundary concerns, not details acceptance should construct or reach through directly. A port is declared at the consuming domain boundary; an adapter implements it at the provider boundary.

## Examples

### Acceptance-owned time port

**File**: `core/src/acceptance/ports.rs:21`

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        timestamp_from_system_time(SystemTime::now())
    }
}
```

`submit_with_clock` accepts this dependency, letting tests supply a fixed clock instead of depending on wall time.

### Acceptance-owned authority seam with an authority adapter

**File**: `core/src/acceptance/ports.rs:65`
**File**: `core/src/authority/check.rs:14`

```rust
pub trait GrantCheck: Send + Sync {
    fn check(/* verified issuer, kind, target */)
        -> impl Future<Output = Result<Authorized, GrantDenied>> + Send;
}

impl GrantCheck for AuthorityRegistry {
    async fn check(/* ... */) -> Result<Authorized, GrantDenied> {
        // evaluate the durable authority projection
    }
}
```

Acceptance owns the need for a grant decision; `AuthorityRegistry` adapts its projection to that port.

### Durable storage port

**File**: `core/src/storage/port.rs:171`

```rust
pub trait Storage: Send + Sync {
    fn append(/* ... */) -> impl Future<Output = Result<EventId, StorageError>> + Send;
    fn append_dedup(/* ... */)
        -> impl Future<Output = Result<DedupOutcome, StorageError>> + Send;
    fn read_after(/* ... */)
        -> impl Future<Output = Result<Vec<RecordedEvent>, StorageError>> + Send;
}
```

Callers use the backend-neutral `Storage` and `StorageError`, not SQLite/rusqlite types.

### Verified ingress context

**File**: `core/src/authority/issuer.rs:12`

```rust
pub trait IssuerContext: Send + Sync {
    fn verified_actor(&self) -> Option<&ActorId>;
    fn verified_endpoint(&self) -> Option<&EndpointId>;
    fn authority_domain_id(&self) -> &AuthorityDomainId;
}
```

Authority consumes verified connection/session evidence rather than a self-asserted actor embedded in the Operation.

## When to Use

- Domain behavior needs time, persistence, verified identity, a sibling projection, or external-system access.
- Tests need to control that dependency without constructing its real infrastructure.
- A consumer needs a narrow capability rather than an entire provider implementation.

## When NOT to Use

- For a pure local calculation with no boundary or substitution need.
- To hide a large, unstable grab-bag API; split the port by consuming responsibility instead.
- To make a protocol decision optional when the protocol already requires the behavior.

## Common Violations

- Calling `SystemTime::now`, SQLite, or an HTTP client directly from domain logic instead of receiving a port.
- Letting the provider own an interface shaped around its implementation rather than the consumer's need.
- Passing unverified payload identity where `IssuerContext` is required.
- Adding an all-purpose service locator instead of a small explicit dependency.

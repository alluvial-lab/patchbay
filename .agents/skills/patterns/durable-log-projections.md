# Durable-Log Projection Folds

Rebuild each in-memory domain projection by reading one authority-domain log in exact-successor, gap-free LSN order, validating event identity/order, and folding every event through that projection's `apply` or `observe` function.

## Rationale

The durable log is authoritative; command, authority, and session indexes are derived views. A uniform replay fold gives restart and catch-up the same semantics as live projection mutation, while allowing each projection to ignore event kinds it does not own.

## Examples

### Command index fold

**File**: `core/src/acceptance/replay.rs:31`
**File**: `core/src/acceptance/replay.rs:72`

```rust
pub async fn rebuild_from_log<S: Storage>(/* ... */) -> Result<CommandIndex, AcceptanceError> {
    let mut index = CommandIndex::new();
    // validate domain and require event_lsn == previous_lsn + 1
    for event in events {
        index.apply(&event)?;
        previous_lsn = event_lsn;
    }
    Ok(index)
}
```

`CommandIndex::apply` owns command-specific event interpretation and ignores sibling projection events.

### Authority registry fold

**File**: `core/src/authority/replay.rs:14`
**File**: `core/src/authority/replay.rs:38`

```rust
pub async fn rebuild_from_log<S: Storage>(/* ... */) -> Result<AuthorityRegistry, AuthorityError> {
    let mut registry = AuthorityRegistry::new();
    for event in events {
        let (event_domain, event_lsn) = event_identity(&event)?;
        // reject wrong domain or any non-successor LSN (gap/duplicate/reversal)
        registry.observe(&event)?;
    }
    Ok(registry)
}
```

The authority view folds the same ordered log through its own observer.

### Session registry fold

**File**: `core/src/session/replay.rs:15`
**File**: `core/src/session/replay.rs:39`

```rust
pub async fn rebuild_from_log<S: Storage>(/* ... */) -> Result<SessionRegistry, SessionError> {
    let mut registry = SessionRegistry::new(authority_domain_id.clone())?;
    for event in events {
        let (event_domain, event_lsn) = event_identity(&event)?;
        // reject wrong domain or any non-successor LSN (gap/duplicate/reversal)
        registry.observe(&event)?;
    }
    Ok(registry)
}
```

The fallible constructor binds the projection to the replayed authority domain.
The shape intentionally mirrors authority replay so each projection can
reconstruct from durable facts without treating a sibling's view as source of
truth.

## When to Use

- A domain maintains a recoverable in-memory view over the core event log.
- Restart or cursor catch-up must produce the same view as normal event application.
- One log carries events owned by multiple independent projections.

## When NOT to Use

- For a view whose only input is a non-durable ephemeral stream.
- To use a snapshot as an alternate ordering authority.
- To replay unordered events or events from another authority domain.

## Common Violations

- Reconstructing one projection from another projection's private state.
- Applying events without checking domain identity and exact-successor, gap-free LSN order.
- Giving a snapshot a scope that hides earlier events required by sibling projections.
- Reinterpreting observations as command transitions rather than folding the explicit durable transition event.

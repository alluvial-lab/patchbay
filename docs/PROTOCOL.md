# Patchbay Protocol

Patchbay protocol semantics are defined around durable operator intent, explicit authority, unambiguous target identity, and recoverable state.

This document defines concepts and required behavior, not a final wire encoding. Wire contracts live in generated schemas or IDL once selected.

## Actors and endpoints

An **actor** is any represented participant: operator, agent, adapter, daemon, service, or control surface.

An **endpoint** is a concrete connection or addressable runtime instance for an actor. An actor may have multiple endpoints across devices or deployments.

Actors and endpoints have stable identifiers assigned by Patchbay or verified through an adapter-specific trust root. Human-readable labels are metadata, not routing authority.

## Sessions

A **session** is an adapter-reported runtime/control target. A session identity binds enough information to prevent wrong-target mutation:

- adapter id;
- deployment or machine scope;
- runtime session id where available;
- optional project/cwd/name metadata;
- adapter-specific generation or epoch when session replacement occurs.

Late replies or events must bind to the session generation they describe. A reply for an old generation cannot mutate a new generation without an explicit adapter rule.

## Messages, commands, and replies

### Message

A message carries information. It may ask for a reply but does not itself grant authority to act.

### Command

A command is operator intent that may cause external action. Commands require:

- command id;
- target session or actor;
- authority grant;
- idempotency key;
- declared command kind;
- payload validated at the boundary;
- expiration or cancellation semantics where applicable.

### Reply

A reply references a prior message or command. A reply is valid only when its correlation id refers to a known prior event in the same authority/session context.

## Acceptance semantics

Patchbay distinguishes acceptance from delivery and completion.

A command accepted by Patchbay is durably recorded before delivery. After acceptance, it must become exactly one visible terminal or continuing state:

- delivered;
- rejected;
- expired;
- failed;
- cancelled;
- completed;
- superseded;
- still pending.

An accepted command cannot disappear silently.

## Delivery states

Control surfaces expose delivery state directly:

- **draft** — local-only, not accepted by Patchbay;
- **submitting** — sent to Patchbay, no acceptance result yet;
- **accepted** — durably recorded by Patchbay;
- **delivered** — target adapter accepted delivery;
- **running** — target reports work in progress;
- **completed** — target reports completion;
- **rejected** — Patchbay or adapter refused the command;
- **failed** — delivery or execution failed;
- **expired** — command timed out before delivery/execution;
- **cancelled** — operator or policy cancelled it;
- **unknown** — local control surface lacks current authoritative state.

## Idempotency and retry

Commands are idempotent by default. Retrying the same command id or idempotency key does not apply the command twice.

A duplicate command returns the existing command state unless the operator explicitly creates a new command.

Adapters that cannot guarantee idempotent external execution must report that limitation as a capability constraint. Patchbay still deduplicates at the coordination boundary.

## Snapshots and streams

Event streams are useful but not authoritative by themselves.

A snapshot is an authoritative state view for an actor, session, command, lease, or resource. Control surfaces reconcile against snapshots after reconnect, resume, tab restore, app restart, or suspected drift.

Patchbay state presentation distinguishes:

- live;
- working;
- idle;
- stale;
- offline;
- unknown;
- failed/error.

Stale cached state must not render as live state.

## Authority grants

A grant authorizes an actor or endpoint to perform a set of actions on a target. Grants are explicit and revocable.

A command without a valid grant is rejected before delivery.

Revocation prevents future authority. Already accepted commands follow the policy attached to their grant and command kind: continue, cancel, or require reauthorization.

## Leases

A lease is a time-bounded exclusive claim over a resource or coordination role. A lease has:

- resource id;
- holder actor;
- scope;
- expiration;
- renewal rules;
- release rules.

Within one modeled Patchbay authority domain, two live leases cannot grant exclusive ownership of the same resource and scope at the same time.

## Adapter capabilities

Adapters declare supported commands and guarantees:

- command kinds;
- streaming support;
- snapshot support;
- cancellation support;
- session replacement support;
- idempotency strength;
- attachment/authorization method;
- known failure modes.

Control surfaces render unsupported actions as unavailable rather than attempting best-effort hidden behavior.

## Transport failures

Patchbay distinguishes transport failure from semantic failure:

- timeout means no timely response at that layer;
- offline means target endpoint unavailable;
- denied means authority/policy refusal;
- rejected means valid target refused the command;
- failed means accepted work reached an error state.

Timeout never implies success. Timeout also does not imply denial.

## Security and trust boundary

Patchbay protocol assumes cryptographic primitives work as specified by their libraries and deployments. Formal models cover authority and identity relationships, not primitive cryptographic correctness.

Sender identity is derived from verified connection/authentication context, not from self-asserted display names or payload fields.

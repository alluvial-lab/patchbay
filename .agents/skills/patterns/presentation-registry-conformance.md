# Presentation Registry Conformance

For every protocol state registry rendered by the surface, declare its schema source and presentation prefix once in the conformance checker, then verify enum parity plus concrete CSS and showcase bindings for every member.

## Rationale

The UX conformance floor must present canonical protocol states without inventing display-only protocol variants. Static registry checks make the schema-to-presentation link executable while retaining skin-able CSS and standalone showcase artifacts.

## Examples

### Operation state binding

**File**: `contracts/scripts/check-presentation.mjs:24`
**File**: `.mockups/design-system/components.css:433`
**File**: `.mockups/design-system/components.html:405`

```js
{
  enum: 'OperationState',
  file: 'operations.proto',
  protoPrefix: 'OPERATION_STATE_',
  cssPrefix: 'command-step',
  members: ['accepted', 'delivered', /* ... */],
}
```

The checker requires classes such as `.command-step--accepted` and an exact showcase element carrying that class.

### Connectivity state binding

**File**: `contracts/scripts/check-presentation.mjs:31`
**File**: `.mockups/design-system/components.css:294`
**File**: `.mockups/design-system/components.html:298`

```js
{
  enum: 'SessionConnectivityState',
  file: 'sessions.proto',
  protoPrefix: 'SESSION_CONNECTIVITY_STATE_',
  cssPrefix: 'connectivity-indicator',
  members: ['live', 'stale', 'offline', 'unknown', 'failed'],
}
```

The corresponding CSS and showcase use `.connectivity-indicator--live` and the remaining canonical members.

### Elicitation state binding

**File**: `contracts/scripts/check-presentation.mjs:45`
**File**: `.mockups/design-system/components.css:570`
**File**: `.mockups/design-system/components.html:586`

```js
{
  enum: 'ElicitationState',
  file: 'elicitations.proto',
  protoPrefix: 'ELICITATION_STATE_',
  cssPrefix: 'elicitation-card',
  members: ['answered', 'declined', /* ... */],
  baseMembers: ['opened', 'pending'],
}
```

Terminal modifier bindings and concrete base-state examples are both checked.

## When to Use

- A surface component renders every member of a canonical protocol enum.
- CSS/showcase artifacts are part of the conformance floor rather than a one-off visual demo.
- A new protocol state needs an enforced visual binding across skins or surfaces.

## When NOT to Use

- For layout or interaction modifiers unrelated to protocol state; keep those in the allowed presentation-only modifier set.
- To make the checker registry the protocol authority; `.proto` remains the source for wire enum membership.
- To assert runtime consumer behavior not implemented by this static check.

## Common Violations

- Adding a CSS state modifier that is not a protocol member.
- Adding a protocol enum member without a checker entry, CSS binding, and showcase example.
- Letting a commented-out or hidden example satisfy a conformance check.
- Styling stale, unknown, offline, or failed connectivity as if it were live.

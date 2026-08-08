# Acquisitions — v1-control-plane-and-spawn

Consolidated acquisition candidates from the campaign specialists. Promotion into
the standing `research-acquisition-queue` backlog item is **operator-confirmed**
at the research-handoff gate (never auto-written).

## Blocking

### CodeAgent Mobile backend implementation
- **Source:** CodeAgent Mobile backend — `/api/commands/pending[/stream]`,
  `/api/commands/ack`, `/api/commands/result`, `/api/baton/events`, plugin-auth
  verification, Redis baton snapshots.
- **Class:** `primary-doc`
- **Web availability:** not present in the fetched public client repository; that
  README explicitly excludes backend/mobile/web source. Public GitHub account
  enumeration + direct probes of plausible backend repo names did not locate a
  public backend. [codeagent-mobile]{1}
- **Completes:** whether command acceptance is durably stored; queue retention +
  terminal disposition; ack-vs-execution semantics; result idempotency;
  authoritative baton snapshot + ownership-conflict rules; stale-client fencing;
  exact session/plugin authority checks. Until acquired, CodeAgent's
  backend-level closure of the operation/ownership contract stays
  acquisition-gated, not inferred from client comments.
- **Urgency:** blocking (one load-bearing cell in the peer-protocol matrix — the
  closest UX-shape peer's backend durability is the one unknown that could
  narrow the moat claim further).

## Enriching
None surfaced (no candidate was both relevant and named canonically by a fetched
source — the AQ.3 anti-recall fence).

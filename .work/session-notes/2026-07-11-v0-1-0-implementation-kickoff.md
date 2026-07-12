# Session Note — v0.1.0 Implementation Kickoff: Substrate Honesty, Core Design, First Code

## Context

Started the session orienting to the `.work/` board and ended with the first three stories of the v0.1.0 coordination core implemented and reviewed. The session pivoted the project from v1.0.0 design back to v0.1.0 implementation — the substrate had drifted past the walking skeleton without building it.

## What happened

### 1. Board orientation + herdr.dev competitive analysis

The operator asked about the board shape and whether herdr.dev forecloses Patchbay's value. Key findings:

- **herdr.dev** is a terminal multiplexer for agents (12.2k stars, shipped). It owns persistent PTYs, tracks blocked/working/done state, exposes a socket API. Its phone story is weak (SSH or a third-party bolt-on).
- **Patchbay's differentiation** is categorical, not incremental: a network coordination core (not a terminal multiplexer), durable operation lifecycle with idempotent retry (not PTY persistence), authority model, multi-machine. herdr persists the terminal; Patchbay persists the operator's intent.
- **The operator's gut read** — "better phone ergonomics than terminal, I pilot from my phone" — lined up exactly with the v0.1.0 scope already in the SPEC. The differentiation earns its cost only if durable delivery over flaky phone connections matters.
- **Corrected an error**: I floated "mobile frontend over Pi's remote API" as a cheap path. The operator pushed back. I verified Pi has no mature network remote affordance — RPC mode is stdin/stdout, the SDK is in-process, and remote-pi (the broker mesh) is the only network layer, which the operator is actively debugging. There is no cheap path; the question is whether to build it as Patchbay (designed) or keep debugging remote-pi (ad hoc).

### 2. Substrate honesty correction

The `.work/` tier told a coherent but misleading story: v0.1.0 was fully *designed* (all foundation features done, formal models corrected) but entirely *unbuilt* (zero application code). The active epic was `epic-public-product-contract` (v1.0.0). An operator reading the board would conclude the next move is v1.0.0 compatibility contracts, not v0.1.0 implementation.

The operator clarified the recent verification-correction arc was fat-trimming for a clean foundation to *begin implementation*. The substrate had drifted past that intent.

**Fix**: scoped `epic-v0-1-0-implementation` + 6 child features (core, protocol-seam, pi-adapter, web-server, web-cockpit, cli) with declared depends_on for the critical path. Sidelined `epic-public-product-contract` honestly — added `epic-v0-1-0-implementation` to its `depends_on`, preserved the done verification-correction work, left the unbuilt v1.0.0 features blocked.

### 3. Elevated feature-v0-core to epic-v0-core

The operator asked whether the Rust coordination core should be an epic. It has four feature-sized sub-arcs (persistence, acceptance, authority, sessions), each with its own formal-model backing and distinct design surface — that's epic-sized. Elevated via `epic-design`: 4 child features, split by capability (not layer), persistence as the root, the other three parallelize via Ports & Adapters.

### 4. feature-design on feature-v0-core-persistence

5 design decisions, resolved interactively after unpacking each option's trade-offs:

| Q | Decision | Key reason |
|---|---|---|
| Storage binding | rusqlite + writer actor | Single-writer makes async benefit moot |
| LSN allocation | bare INTEGER PRIMARY KEY (no AUTOINCREMENT) | Operator pushed back on explicit counter — rowid is a standard DB guarantee, gap-free on append-only tables |
| Snapshots | same-DB table | Atomicity is free via SQLite transactions |
| Event payload | opaque BLOB | Safety-critical reads are sequential scans |
| Workspace | new top-level workspace member | Generated Contracts principle — contracts crate stays purely generated |

4 child stories, linear chain: workspace+port → rusqlite-impl → recovery → proptests.

### 5. Implementation: 3 stories done, 3 review rounds each

| Story | Verdict progression | Key review catches |
|---|---|---|
| `workspace-and-port` | Request changes → Request changes → Approve | rusqlite leak in StorageError; no event discriminator; no atomic dedup (the formal model's `appliedKeys` claim was unsubstantiated) |
| `rusqlite-impl` | Request changes → Approve | Snapshot errors silently swallowed; DefaultHasher for payload equivalence (not stable, can collide) |
| `recovery` | Request changes → Request changes → Approve | Overclaiming formal properties; "idempotent" vs "deterministic for unchanged contents" |

**34 tests passing** across 3 test files. The storage port trait, rusqlite implementation, and recovery module are done.

## Key lessons

1. **The substrate can drift past the operator's intent without lying.** Every individual transition was valid (foundation design done → next epic), but the aggregate moved from "design v0.1.0" to "design v1.0.0" without anyone deciding to defer v0.1.0 implementation. The board was *honest about state* but *dishonest about direction*. Scoping the implementation epic + sidelining v1.0.0 corrected this.

2. **Review the foundation before three features stand on it.** The storage port trait review caught three real blockers — `rusqlite::Error` leaking into the domain port, no event-type discriminator, and no atomic dedup operation. Blocker 3 was the sharpest: the design claimed `appliedKeys` lives in the persistence layer, but `append(payload)` couldn't atomically test-and-register an idempotency key. If acceptance/authority/sessions had built on that trait before the gap was found, the fix would have been a cross-cutting refactor instead of a port revision.

3. **"Deterministic" ≠ "idempotent" for formal-model honesty.** The recovery module initially claimed `recover()` was "idempotent." The reviewer correctly noted: two calls can differ if writes happen between them. The right claim is "deterministic for unchanged storage contents." The distinction matters because the formal models claim properties about *the same committed prefix*, not about concurrent calls. Overclaiming "idempotent" would have made the stated-normative `IdempotentLogReplay` obligation look satisfied when it isn't (it also depends on the domain layer's `apply` being deterministic, which isn't implemented yet).

4. **`git add -A` is dangerous in a repo with build artifacts.** Twice this session, `git add -A` swept in `_apalache-out/` (168k lines of model-checker output) and `target/` (cargo build artifacts). The fix was `.gitignore` hygiene (`_apalache-out/`, `states/`, `target/`). The lesson: prefer explicit `git add <paths>` over `git add -A` in repos with generated artifacts, or ensure `.gitignore` is comprehensive before any `git add -A`.

5. **Generated contracts drift management.** `cargo build` regenerates the proto Rust bindings via `build.rs`, and prost version differences reformat the generated file (the `@generated` comment, line wrapping). The fix: `git checkout` the generated file, then regenerate via `buf generate` to get a clean additive diff. The checked-in generated code should match the committed proto schema, not the local prost version's formatting.

6. **CARGO_HOME workaround for read-only cargo cache.** The sandbox has a read-only `~/.cargo` registry cache. Builds require `CARGO_HOME=/tmp/cargo-home` (or any writable location). Environment quirk, not a code issue — noted in story implementation notes so the next implementer doesn't hit the same wall.

7. **rusqlite version pin.** The latest `libsqlite3-sys` (0.38.1, pulled by rusqlite 0.40) uses the unstable `cfg_select!` feature and fails on stable Rust 1.94. Pinned to rusqlite 0.31 → libsqlite3-sys 0.28. Mechanical version constraint, not semantic — the SQLite WAL/synchronous semantics are identical.

## The operator's gut, restated

The operator's felt need — "better phone ergonomics than terminal, I pilot from my phone" — is the v0.1.0 walking skeleton, already encoded in the SPEC. The substrate just hadn't started building it. This session corrected the direction and wrote the first code: a durable event log with atomic idempotency-key dedup, snapshots, and crash recovery. The phone-usable path (core → protocol-seam → web-server → web-cockpit) is now the active critical path, with the Pi adapter parallel to it.

## Current state

```
epic-v0-1-0-implementation  (drafting)
├── epic-v0-core  (implementing)
│   ├── feature-v0-core-persistence  (implementing — 3/4 stories done)
│   │   ├── story-workspace-and-port      ✅ done
│   │   ├── story-rusqlite-impl            ✅ done
│   │   ├── story-recovery                 ✅ done
│   │   └── story-proptests                []  ← next
│   ├── feature-v0-core-acceptance   (drafting, blocked on persistence)
│   ├── feature-v0-core-authority    (drafting, blocked on persistence)
│   └── feature-v0-core-sessions     (drafting, blocked on persistence)
├── feature-v0-protocol-seam  (drafting, blocked on core)
├── feature-v0-pi-adapter      (drafting, blocked on core)
├── feature-v0-web-server      (drafting, blocked on seam)
├── feature-v0-web-cockpit    (drafting, blocked on web-server)
└── feature-v0-cli             (drafting, blocked on seam)

epic-public-product-contract  (v1.0.0 — SIDELINED, blocked on v0.1.0)
```

## Next

`story-v0-core-persistence-proptests` — the formal property tests (gap-free LSN, idempotent replay, crash recovery, snapshot prefix consistency) that give the stated-normative obligations their executable evidence. This is the story where the deep-lane mutation discipline actually has a target — the first three stories were trait/impl/mechanism, this one proves the properties.

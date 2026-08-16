---
id: spawn-stride-final-completion-review-2026-08-16
kind: story
stage: done
tags: [review, spawn]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-16
updated: 2026-08-16
---

# Final completion review — spawn stride

## Verdict

**BLOCKERS-FOUND.** The landed implementation and the requested local clean-tree suite are green, all 3 in-scope features and all 24 stride implementation stories are `done`, and the current Pi manifest, activation code, and foundation text agree. The aggregate completion claim is nevertheless not confirmable because the final pushed CI run is red and the Pi feature has no retained post-fix CLEAN integrated review pass at the required `thorough` weight.

Review mode: fresh-context aggregate autopilot completion review, effective weight `thorough`. This review inspected aggregate completion rather than repeating line-level story reviews. It created no temporary worktree, injected no mutations, and changed no code or item file. `/` began with 23 GiB available (84% used).

## Completion-bundle verification

| Claim | Result | Evidence |
|---|---|---|
| Scope is the two v1-must features plus the operator-lifted durability exception | **PASS** | The in-scope set is `research-handoff-spawn`, `research-handoff-pi-adapter-capability`, and `capability-manifest-durability-and-reconciliation-depth`. The lifted exception is recorded at `901cfd8`. No commit since the 2026-08-12 gate touched `epic-public-product-contract-public-compatibility`, `epic-public-product-contract-publication-governance`, or `verification-discipline-checks`; they remain `drafting`. |
| Spawn feature | **PASS** | Feature and 16/16 implementation children are `done`. Final story closure artifacts exist for all 16. The integrated feature review reached pass-3 **CLEAN** after closing promotion diagnostics, independent cockpit claim disposition, authenticated atomic target abandonment through `AbandonSpawnTarget`, and the four missing retrospective leaf-review artifacts. |
| Pi adapter feature implementation | **PASS with review-process blocker below** | The six implementation children listed in the feature are `done`; their final review artifacts are CLEAN. The pre-existing `mc-architectural-harvest` design-input story is also a done direct child but is not one of the six implementation units or the 24-story stride count. Current production activation declares managed `spawn`/`pi-rpc`, replacement, continuation/cursor/fence support, `BOUNDED` reconciliation, and `MANUAL_REQUIRED`, matching `docs/PROTOCOL.md`, `docs/ADAPTER-PI.md`, and `docs/VERIFICATION.md`. The retained integrated feature review did not itself converge; see BLOCKER 2. |
| Capability-durability feature | **PASS** | Feature and 2/2 children are `done`. Contract validation closed at pass 2; consumer wiring closed with nits; integrated feature pass 2 is CLEAN after wiring the generated UNKNOWN-outcome qualifier. |
| 2026-08-12 adversarial BLOCKERs 1–10 | **PASS on current implementation evidence** | Spawn traceability and pass-3 review close 1–5, 7, and 8. The Pi integrated review explicitly confirms 6, 9, and 10 closed; current cursor replacement, challenged cwd proof, and conditional materialization tests remain green. |
| Review retention | **PASS for stories** | 24/24 final story closure artifacts exist: 23 CLEAN and one NIT closure. The four retrospective leaf reports exist and are CLEAN. Spawn and durability have retained CLEAN integrated feature convergence. Pi lacks a post-fix CLEAN integrated pass. |
| Verification labels | **PASS** | `docs/VERIFICATION.md` labels the managed Pi lifecycle implementation-checked and its three vectors draft; it explicitly disclaims formal promotion, checked-normative promotion, and release verification. Current checked-model/checked-normative labels are not inflated by this stride. |
| No release cut | **PASS** | All 27 in-scope files have `release_binding: null`; latest tag remains `v0.2.1`; no new release summary/tag was created. The docs describe implementation evidence, not a release claim. Concrete second adapters and further formal promotion remain outside this stride. |

## Substrate consistency

- Exact stride inventory: **3 features + 24 implementation stories** (`16 + 6 + 2`), all `stage: done`.
- Required frontmatter, id/path identity, parent links, dependency targets, null release binding, and null gate origin validate for all 27 files.
- No in-scope feature or story remains at `drafting`, `implementing`, or `review`.
- Final story review artifacts: **24/24**; the four retrospective leaf reports are present.
- Raw `work-view --parent` output also includes review records and the pre-stride `mc-architectural-harvest`; those are not additional stride implementation stories.

## Git and CI

| Check | Result |
|---|---|
| Pre-review tracked tree | **PASS** — clean; `git diff --check` clean. |
| Pre-review local/remote relation | **PASS** — `main == origin/main` at `3b96709`; ahead/behind `0/0`. |
| Commit history sanity | **PASS** — the implementation/review/fix/done sequence is visible from the 2026-08-12 gate through final documentation correction `3b96709`. |
| Final pushed CI | **FAIL / BLOCKER** — run `31964778515` for `3b96709` failed in `contracts-and-conformance`; Rust passed and `typescript-suites` was skipped. The two immediately preceding pushes also failed. |
| Stray suite processes | **PASS** — no Patchbay core or `pi --mode rpc` process remained after the local suite. |

## Aggregate risk and acceptance read

The current code/foundation boundary is internally consistent on the reviewed seams:

- Pi's constructor and activation oracle declare `BOUNDED`, not adapter-wide `AUTHORITATIVE`, reconciliation while retaining authoritative exact-set transcript replacement only within its cursor scope.
- `docs/PROTOCOL.md`, `docs/ADAPTER-PI.md`, and `docs/VERIFICATION.md` now describe the same activated operations, target shape, continuation/cursor/fence strengths, conditional materialization, bounded reconciliation, manual ambiguity, and journal-retention limits.
- The generic assurance registry remains advisory: it does not authorize, suppress delivery, or terminalize an Operation.
- The spawn feature's failure lifecycle includes diagnostics-visible claim disposition and authenticated atomic target abandonment; no acceptance row remains schema/fold-only.
- No additional in-scope acceptance row or foundation contradiction was found on the current snapshot.

That content-level result does not erase the missing Pi thorough convergence pass or the red final CI result.

## Full clean-tree suite

The only code-executing check in this review ran sequentially on the unmodified tracked tree. All requested groups and consumers passed:

| Group | Result |
|---|---|
| Rust workspace | **PASS** — all-target build, full workspace tests/doctests, warnings-denied all-target clippy. |
| Contracts/conformance | **PASS** — TypeScript build; generated drift; 60 vectors / 19 promoted / 33 implementation checks / 38 mutation witnesses; model traceability; five presentation registries; presentation meta-tests. |
| Operator domain | **PASS — 32/32**. |
| Pi adapter | **PASS — 127/127**, including both real-core flows and the real managed Pi lifecycle. |
| Web cockpit | **PASS — 148/148**. |
| CLI | **PASS — 53/53** plus real-core resource projection. |
| token-commune adapter | **PASS — 63/63**, including both real-core flows. |

The retained review evidence records formal checks **20/20** and Pi mutation witnesses **28/28**. They were not re-injected here because this review was explicitly read-only and the caller directed use of retained mutation evidence.

## Findings

### BLOCKER 1 — final pushed CI is red

**Locations:** `.github/workflows/ci.yml:33-84`; `contracts/scripts/check-vectors.mjs:121-124`; GitHub Actions run `31964778515`.

The contracts job runs `check:vectors`, whose registered implementation checks now invoke `npm --prefix pi-adapter run test:conformance`, but that job does not install `pi-adapter` dependencies. The fresh runner fails TypeScript resolution for Node types, `@patchbay/contracts`, `@earendil-works/pi-*`, and related modules. Rust passes, but contracts fail and the dependent TypeScript job is skipped. Local verification passes because the working checkout already has those dependencies installed; it does not satisfy the explicit final-push-CI-green claim.

**Required closure:** provision every vector runner's declared package dependencies in `contracts-and-conformance` (including Pi), then obtain a green CI run on the resulting final commit.

### BLOCKER 2 — Pi feature lacks required thorough integrated convergence

**Locations:** `.work/active/reviews/pi-feature-rereview-2026-08-16.md:13-15`; commits `db44e96`, `9181f86`, `3fda151`, `3b96709`.

The last retained Pi integrated review is pass 2 with verdict **MATERIAL**, identifying stale current-state assurance prose. The documentation was then fixed, the feature was marked `done`, and a further residual was corrected at `3b96709`, but no independent post-fix CLEAN integrated pass is retained. At `thorough` weight, the closure policy requires review → fix → verify → review until a pass has no material blocker. This aggregate review confirms that the current prose and implementation now agree, but it is not a substitute for the omitted feature-level convergence pass because the caller explicitly asked for aggregate completion review rather than an individual feature rereview.

**Required closure:** run and retain a bounded fresh-context Pi integrated pass over the documentation fixes/current snapshot; it must reach CLEAN before the completion claim is renewed.

## Final disposition

The stride is locally green and content-complete on the inspected snapshot, but completion remains blocked on **green final CI** and **retained Pi thorough feature convergence**. No release should be cut from this completion claim until both are closed.

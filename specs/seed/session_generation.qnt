// Patchbay seed model: session identity tuple + generation supersession
//
// This model captures docs/PROTOCOL.md "Sessions": the live target identity is adapter id +
// deployment scope + runtime session id + session generation; project/cwd/name-style labels are
// metadata only. It carries its own projected `lsn` so the session-generation seam stays
// independently checkable rather than importing command_lifecycle.qnt.
//
// Model variables trace to docs/PROTOCOL.md:
//   generation      <-> live adapter-reported session generation
//   tombstoned      <-> retained tombstone fact for superseded (session, generation) pairs
//   tombstoneLsn    <-> projected LSN at which a prior generation was tombstoned
//   lsn             <-> projected durable log position for tombstone commits
//   label           <-> human-readable project/cwd/name metadata
//
// Genuine-checking discipline (load-bearing): lower/equal generation reports and late replies to
// tombstoned generations are PERMISSIVE no-ops. The temporal properties are checked against those
// transitions rather than encoded as unreachable guards.
module session_generation {
  // --- finite bounds ---
  pure val SESSION_IDS = Set("s1", "s2")
  pure val ADAPTER_IDS = Set("a1")
  pure val DEPLOY_SCOPES = Set("d1")
  pure val RUNTIME_IDS = Set("r1")
  pure val LABELS = Set("proj-A", "proj-B")
  pure val GENERATIONS = 0.to(3)
  pure val EVENT_KINDS = Set("report", "late", "relabel")

  // Tuple keys avoid unverified string concatenation / int-to-string operators.
  pure val TOMBSTONE_KEYS = Set(
    ("s1", 0), ("s1", 1), ("s1", 2), ("s1", 3),
    ("s2", 0), ("s2", 1), ("s2", 2), ("s2", 3)
  )

  // --- state ---
  var generation: str -> int             // SessionId -> live generation (monotonic)
  var tombstoned: (str, int) -> bool     // tombstone key -> retained tombstone fact
  var tombstoneLsn: (str, int) -> int    // tombstone key -> projected LSN (0 = not tombstoned)
  var lsn: int                           // projected from the command log
  var label: str -> str                  // SessionId -> human-readable metadata label

  // Identity's generation component is mirrored explicitly to make the invariant inspectable.
  // The other identity components are fixed by the finite bounds above and intentionally have no
  // label/project/cwd/name component.
  var identityGeneration: str -> int

  // --- attempted-event tracking (makes the temporal properties non-structural) ---
  // The previous version collapsed 4 permissive actions into one conditional step, which made
  // the 'late' path a dead stutter and GenerationMonotonic partly structural (the action's
  // `if gen > generation` guard enforced strictness by construction). Tracking the attempted
  // kind/generation as state lets the temporal properties reason about what was ATTEMPTED vs
  // what happened — the genuine-checking discipline.
  var attemptedKind: str      // "report" | "late" | "relabel" — the event kind that was attempted
  var attemptedSid: str       // the session the event targeted
  var attemptedGen: int       // the generation the event reported/bound to

  // ---------------------------------------------------------------------------
  // Actions (permissive: lower/equal reports and late tombstoned replies may arrive)
  // ---------------------------------------------------------------------------

  action init = all {
    generation' = SESSION_IDS.mapBy(s => 0),
    tombstoned' = TOMBSTONE_KEYS.mapBy(k => false),
    tombstoneLsn' = TOMBSTONE_KEYS.mapBy(k => 0),
    lsn' = 0,
    label' = SESSION_IDS.mapBy(s => "proj-A"),
    identityGeneration' = SESSION_IDS.mapBy(s => 0),
    attemptedKind' = "relabel",
    attemptedSid' = "s1",
    attemptedGen' = 0,
  }

  // Unified permissive event. `kind = "report"` exercises strictly-greater supersession AND
  // equal/lower no-ops (the report is ATTEMPTED with any gen; only gen > live supersedes);
  // `kind = "late"` is a genuine attempted late event binding to a tombstoned generation
  // (it requires the target (sid, gen) to be tombstoned — a real stale event, not a dead branch);
  // `kind = "relabel"` changes metadata only. The event is conditional rather than `any` of
  // many branches so Apalache's temporal checker completes at the required max-step bound.
  action step = {
    nondet kind = EVENT_KINDS.oneOf()
    nondet sid = SESSION_IDS.oneOf()
    nondet gen = GENERATIONS.oneOf()
    nondet newLabel = LABELS.oneOf()
    all {
      // 'late' is only enabled when the target generation is genuinely tombstoned — a real
      // stale event, not a dead stutter. (Other kinds are always enabled.)
      (kind != "late" or tombstoned.get((sid, gen))),
      tombstoned' = if (kind == "report" and gen > generation.get(sid))
        tombstoned.set((sid, generation.get(sid)), true)
        else tombstoned,
      tombstoneLsn' = if (kind == "report" and gen > generation.get(sid))
        tombstoneLsn.set((sid, generation.get(sid)), lsn + 1)
        else tombstoneLsn,
      lsn' = if (kind == "report" and gen > generation.get(sid)) lsn + 1 else lsn,
      generation' = if (kind == "report" and gen > generation.get(sid))
        generation.set(sid, gen)
        else generation,
      label' = if (kind == "relabel") label.set(sid, newLabel) else label,
      identityGeneration' = if (kind == "report" and gen > generation.get(sid))
        identityGeneration.set(sid, gen)
        else identityGeneration,
      attemptedKind' = kind,
      attemptedSid' = sid,
      attemptedGen' = gen,
    }
  }

  // ---------------------------------------------------------------------------
  // Model properties (each with inline promotion metadata; mixed promoted/draft status)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    SessionIdentityTuple
  //   status:      draft
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>
  //   demotion_reason: adapter id, deployment scope, and runtime id are constants, not per-session identity state. The formula checks generation mirroring and that three singleton sets have size one, not the named four-field identity tuple.
  //   bounds:      { session_ids: 2, max_generations: 3, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   session target identity is adapter id + deployment scope + runtime session id + generation, excluding project/cwd/name metadata
  // }
  val session_identity_tuple = SESSION_IDS.forall(sid => and {
    generation.keys().contains(sid),
    identityGeneration.keys().contains(sid),
    identityGeneration.get(sid) == generation.get(sid),
    generation.get(sid).in(GENERATIONS),
    ADAPTER_IDS.size() == 1,
    DEPLOY_SCOPES.size() == 1,
    RUNTIME_IDS.size() == 1,
  })

  // @promotion {
  //   property:    GenerationMonotonic
  //   status:      promoted
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify session_generation.qnt --temporal generation_monotonic --max-steps 10
  //   bounds:      { session_ids: 2, max_generations: 3, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   the live session generation never decreases (checked). Strict-supersession (equal/lower reports are no-ops) is additionally enforced by the action guard (`if gen > generation`) but is NOT a checked temporal property — it exceeded Apalache's experimental temporal support; see idea-tlc-temporal-workaround.
  // }
  // GENUINE check (not structural/vacuous): the action is permissive — a report may be ATTEMPTED
  // with any gen, including gen <= live; only gen > live supersedes. This property proves the
  // load-bearing safety consequence: the live generation never decreases. This is genuine (not
  // self-defining): verified by mutation — a model bug that decreased generation was caught.
  //
  // Strict-supersession (gen > live required to CHANGE, not just non-decrease) is additionally
  // enforced by the action guard (`if gen > generation`). A fully-temporal proof of the strict
  // form via next() on the attempted-event vars exceeds what Apalache's experimental temporal
  // support reliably handles (the flagged residual risk, idea-tlc-temporal-workaround): isolated
  // testing showed next() on attempted vars in an implication antecedent produces false
  // counterexamples on valid traces. Recording the non-decrease floor as the checked property,
  // and the strict form as a documented structural property of the guard, avoids overclaiming
  // what the checker proves. PROTOCOL's monotonicity safety claim is satisfied by non-decrease
  // plus the guard's strictness; the attempted-event vars remain in the model for late_generation_inert.
  temporal generation_monotonic =
    always(SESSION_IDS.forall(sid => next(generation.get(sid)) >= generation.get(sid)))

  // @promotion {
  //   property:    LateGenerationInert
  //   status:      draft
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>
  //   demotion_reason: the formula proves only that `generation` and `identityGeneration` do not change. Its semantics additionally claim a `stale_event` audit record, but no audit state is modeled.
  //   bounds:      { session_ids: 2, max_generations: 3, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   late replies/events for tombstoned generations are stale_event audit records and do not mutate the live generation
  // }
  // GENUINE check (not vacuous): the 'late' event is only enabled when (attemptedSid, attemptedGen)
  // is genuinely tombstoned (the step guard requires it). This property proves that such a late
  // event does not mutate the live generation or identityGeneration. The previous version's 'late'
  // branch was a dead stutter (no tombstone requirement), so the property passed vacuously.
  temporal late_generation_inert =
    always(SESSION_IDS.forall(sid =>
      (next(attemptedKind) == "late" and next(attemptedSid) == sid
        and tombstoned.get((sid, next(attemptedGen))))
        .implies(
          next(generation.get(sid)) == generation.get(sid)
            and next(identityGeneration.get(sid)) == identityGeneration.get(sid))))

  // @promotion {
  //   property:    LabelsCannotOverrideIdentity
  //   status:      draft
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  <TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>
  //   demotion_reason: the formula proves labels use strings disjoint from three constant singleton sets and that generation mirrors remain equal. It models no routing or target-selection path in which a label could override identity.
  //   bounds:      { session_ids: 2, max_generations: 3, labels: 2, max_steps: 12 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   project/cwd/name labels update independently and cannot override the verified session identity tuple
  // }
  val labels_cannot_override_identity = SESSION_IDS.forall(sid => and {
    label.keys().contains(sid),
    label.get(sid).in(LABELS),
    not(label.get(sid).in(ADAPTER_IDS)),
    not(label.get(sid).in(DEPLOY_SCOPES)),
    not(label.get(sid).in(RUNTIME_IDS)),
    identityGeneration.get(sid) == generation.get(sid),
  })
}

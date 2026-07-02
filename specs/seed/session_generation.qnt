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
  }

  // Unified permissive event. `kind = "report"` exercises strictly-greater supersession and
  // equal/lower no-ops; `kind = "late"` is the stale-event no-op path; `kind = "relabel"`
  // changes metadata only. The event is intentionally conditional instead of an `any` of many
  // branches so the Apalache temporal checker can complete at the required max-step bound.
  action step = {
    nondet kind = EVENT_KINDS.oneOf()
    nondet sid = SESSION_IDS.oneOf()
    nondet gen = GENERATIONS.oneOf()
    nondet newLabel = LABELS.oneOf()
    all {
      TOMBSTONE_KEYS.contains((sid, generation.get(sid))),
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
    }
  }

  // ---------------------------------------------------------------------------
  // Checked properties (each with inline promotion metadata)
  // ---------------------------------------------------------------------------

  // @promotion {
  //   property:    SessionIdentityTuple
  //   tier:        checked-normative
  //   status:      promoted
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_generation.qnt --invariant session_identity_tuple --max-steps 12
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
  //   tier:        checked-normative
  //   status:      promoted
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify session_generation.qnt --temporal generation_monotonic --max-steps 10
  //   bounds:      { session_ids: 2, max_generations: 3, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   supersession requires strictly-greater generation; equal/lower generation reports leave the live target unchanged
  // }
  temporal generation_monotonic =
    always(SESSION_IDS.forall(sid => and {
      next(generation.get(sid)) >= generation.get(sid),
      (next(lsn) == lsn).implies(next(generation.get(sid)) == generation.get(sid)),
    }))

  // @promotion {
  //   property:    LateGenerationInert
  //   tier:        checked-normative
  //   status:      promoted
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify session_generation.qnt --temporal late_generation_inert --max-steps 10
  //   bounds:      { session_ids: 2, max_generations: 3, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [none]
  //   semantics:   late replies/events for tombstoned generations are stale_event audit records and do not mutate the live generation
  // }
  temporal late_generation_inert =
    always(SESSION_IDS.forall(sid => GENERATIONS.forall(gen =>
      (tombstoned.get((sid, gen)) and next(lsn) == lsn).implies(
        next(generation.get(sid)) == generation.get(sid) and
        next(identityGeneration.get(sid)) == identityGeneration.get(sid)))))

  // @promotion {
  //   property:    LabelsCannotOverrideIdentity
  //   tier:        checked-normative
  //   status:      promoted
  //   model:       specs/seed/session_generation.qnt
  //   language:    quint
  //   backend:     apalache
  //   invocation:  quint verify session_generation.qnt --invariant labels_cannot_override_identity --max-steps 12
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

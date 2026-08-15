// Patchbay seed model: managed session-generation promotion
//
// The small default module retains the bounded promoted non-decrease check. The richer promotion
// module below carries independent attempted evidence, exact ∅→1/N→N+1 validation, reverse-index
// retention, and authority/session/claim/command atomicity without overloading Apalache's
// experimental temporal loop encoding with the richer state vector.
module session_generation {
  pure val SESSION_IDS = Set("s1", "s2")
  pure val GENERATIONS = 0.to(3)

  var generation: str -> int

  action init = generation' = SESSION_IDS.mapBy(_ => 0)

  action step = {
    nondet sid = SESSION_IDS.oneOf()
    nondet candidate = GENERATIONS.oneOf()
    generation' = if (candidate > generation.get(sid))
      generation.set(sid, candidate)
      else generation
  }

  temporal generation_monotonic =
    always(SESSION_IDS.forall(sid => next(generation.get(sid)) >= generation.get(sid)))
}

// Managed candidates do not become current when reported. A separate attempted promotion arrives
// with raw prior/candidate evidence; the fold accepts only fresh ∅→1 or exact N→N+1 against the
// already-active claim and retained external-runtime reservation. The attempted evidence and
// pre-state snapshots are written by `arrivePromotion`, which cannot mutate core state. The later
// `foldPromotion` therefore cannot hide a bad attempt by rewriting the oracle's evidence.
module session_generation_promotion {
  // --- finite bounds ---
  pure val SESSION_IDS = Set("s1", "s2")
  pure val LABELS = Set("proj-A", "proj-B")
  pure val GENERATIONS = 0.to(4)
  pure val MAX_MANAGED_GENERATION = 3
  pure val NO_OWNER = "none"
  pure val OWNERS = Set(NO_OWNER, "s1", "s2")
  pure val TOMBSTONE_KEYS = Set(
    ("s1", 0), ("s1", 1), ("s1", 2), ("s1", 3), ("s1", 4),
    ("s2", 0), ("s2", 1), ("s2", 2), ("s2", 3), ("s2", 4)
  )

  // --- authoritative projection state ---
  var generation: str -> int
  var identityGeneration: str -> int
  var tombstoned: (str, int) -> int
  var tombstoneLsn: (str, int) -> int
  var lsn: int
  var label: str -> str

  // One active exact claim and staged reservation per target. `externalOwner` is the reverse
  // index for one finite external runtime family; promoted/tombstoned keys retain their owner.
  var claimActive: str -> int
  var expectedPrior: str -> int
  var claimedGeneration: str -> int
  var externalOwner: int -> str

  // The same semantic promotion installs authority and command completion atomically.
  var descendantAuthority: (str, int) -> int
  var completionCount: str -> int

  // --- independent attempted-promotion evidence and immutable pre-state snapshots ---
  var phase: str // idle | pending | folded
  var attemptedSid: str
  var attemptedPrior: int
  var attemptedGeneration: int
  var beforeGeneration: int
  var beforeIdentityGeneration: int
  var beforeClaimActive: int
  var beforeExpectedPrior: int
  var beforeClaimedGeneration: int
  var beforeCandidateOwner: str
  var beforePriorTombstoned: int
  var beforePriorTombstoneLsn: int
  var beforeCandidateAuthority: int
  var beforeCompletionCount: int
  var beforeLsn: int

  action init = all {
    generation' = SESSION_IDS.mapBy(_ => 0),
    identityGeneration' = SESSION_IDS.mapBy(_ => 0),
    tombstoned' = TOMBSTONE_KEYS.mapBy(_ => 0),
    tombstoneLsn' = TOMBSTONE_KEYS.mapBy(_ => 0),
    lsn' = 0,
    label' = SESSION_IDS.mapBy(_ => "proj-A"),
    claimActive' = SESSION_IDS.mapBy(_ => 0),
    expectedPrior' = SESSION_IDS.mapBy(_ => 0),
    claimedGeneration' = SESSION_IDS.mapBy(_ => 0),
    externalOwner' = GENERATIONS.mapBy(_ => NO_OWNER),
    descendantAuthority' = TOMBSTONE_KEYS.mapBy(_ => 0),
    completionCount' = SESSION_IDS.mapBy(_ => 0),
    phase' = "idle",
    attemptedSid' = "s1",
    attemptedPrior' = 0,
    attemptedGeneration' = 0,
    beforeGeneration' = 0,
    beforeIdentityGeneration' = 0,
    beforeClaimActive' = 0,
    beforeExpectedPrior' = 0,
    beforeClaimedGeneration' = 0,
    beforeCandidateOwner' = NO_OWNER,
    beforePriorTombstoned' = 0,
    beforePriorTombstoneLsn' = 0,
    beforeCandidateAuthority' = 0,
    beforeCompletionCount' = 0,
    beforeLsn' = 0,
  }

  // Core preparation always claims exact current+1. A competing target's attempt to reserve the
  // same exact external runtime is a no-op, making uniqueness independently observable.
  def nextCandidate(sid) = generation.get(sid) + 1
  def mayReserve(sid) = phase == "idle"
    and claimActive.get(sid) == 0
    and generation.get(sid) < MAX_MANAGED_GENERATION
    and externalOwner.get(nextCandidate(sid)) == NO_OWNER

  action prepareClaim(sid) = all {
    generation' = generation,
    identityGeneration' = identityGeneration,
    tombstoned' = tombstoned,
    tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,
    label' = label,
    claimActive' = if (mayReserve(sid)) claimActive.set(sid, 1) else claimActive,
    expectedPrior' = if (mayReserve(sid)) expectedPrior.set(sid, generation.get(sid)) else expectedPrior,
    claimedGeneration' = if (mayReserve(sid)) claimedGeneration.set(sid, nextCandidate(sid)) else claimedGeneration,
    externalOwner' = if (mayReserve(sid)) externalOwner.set(nextCandidate(sid), sid) else externalOwner,
    descendantAuthority' = descendantAuthority,
    completionCount' = completionCount,
    phase' = phase,
    attemptedSid' = attemptedSid,
    attemptedPrior' = attemptedPrior,
    attemptedGeneration' = attemptedGeneration,
    beforeGeneration' = beforeGeneration,
    beforeIdentityGeneration' = beforeIdentityGeneration,
    beforeClaimActive' = beforeClaimActive,
    beforeExpectedPrior' = beforeExpectedPrior,
    beforeClaimedGeneration' = beforeClaimedGeneration,
    beforeCandidateOwner' = beforeCandidateOwner,
    beforePriorTombstoned' = beforePriorTombstoned,
    beforePriorTombstoneLsn' = beforePriorTombstoneLsn,
    beforeCandidateAuthority' = beforeCandidateAuthority,
    beforeCompletionCount' = beforeCompletionCount,
    beforeLsn' = beforeLsn,
  }

  // Environment arrival is deliberately separate from the fold. It records raw attempted facts
  // and the authoritative pre-state but cannot change current, claim, tombstone, authority,
  // completion, or reverse ownership.
  action arrivePromotion(sid, prior, candidate) = all {
    phase == "idle",
    generation' = generation,
    identityGeneration' = identityGeneration,
    tombstoned' = tombstoned,
    tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,
    label' = label,
    claimActive' = claimActive,
    expectedPrior' = expectedPrior,
    claimedGeneration' = claimedGeneration,
    externalOwner' = externalOwner,
    descendantAuthority' = descendantAuthority,
    completionCount' = completionCount,
    phase' = "pending",
    attemptedSid' = sid,
    attemptedPrior' = prior,
    attemptedGeneration' = candidate,
    beforeGeneration' = generation.get(sid),
    beforeIdentityGeneration' = identityGeneration.get(sid),
    beforeClaimActive' = claimActive.get(sid),
    beforeExpectedPrior' = expectedPrior.get(sid),
    beforeClaimedGeneration' = claimedGeneration.get(sid),
    beforeCandidateOwner' = externalOwner.get(candidate),
    beforePriorTombstoned' = tombstoned.get((sid, prior)),
    beforePriorTombstoneLsn' = tombstoneLsn.get((sid, prior)),
    beforeCandidateAuthority' = descendantAuthority.get((sid, candidate)),
    beforeCompletionCount' = completionCount.get(sid),
    beforeLsn' = lsn,
  }

  // Production guard. The independent oracle below intentionally does not call this definition.
  def foldGuard = phase == "pending"
    and claimActive.get(attemptedSid) == 1
    and generation.get(attemptedSid) == attemptedPrior
    and expectedPrior.get(attemptedSid) == attemptedPrior
    and claimedGeneration.get(attemptedSid) == attemptedGeneration
    and externalOwner.get(attemptedGeneration) == attemptedSid
    and descendantAuthority.get((attemptedSid, attemptedGeneration)) == 0
    and ((attemptedPrior == 0 and attemptedGeneration == 1)
      or (attemptedPrior > 0 and attemptedGeneration == attemptedPrior + 1))

  action foldPromotion = all {
    phase == "pending",
    generation' = if (foldGuard)
      generation.set(attemptedSid, attemptedGeneration)
      else generation,
    identityGeneration' = if (foldGuard)
      identityGeneration.set(attemptedSid, attemptedGeneration)
      else identityGeneration,
    tombstoned' = if (foldGuard and attemptedPrior > 0)
      tombstoned.set((attemptedSid, attemptedPrior), 1)
      else tombstoned,
    tombstoneLsn' = if (foldGuard and attemptedPrior > 0)
      tombstoneLsn.set((attemptedSid, attemptedPrior), lsn + 1)
      else tombstoneLsn,
    lsn' = if (foldGuard) lsn + 1 else lsn,
    label' = label,
    claimActive' = if (foldGuard) claimActive.set(attemptedSid, 0) else claimActive,
    expectedPrior' = expectedPrior,
    claimedGeneration' = claimedGeneration,
    // Both the prior and successor reservation remain owned after promotion/tombstoning.
    externalOwner' = externalOwner,
    descendantAuthority' = if (foldGuard)
      descendantAuthority.set((attemptedSid, attemptedGeneration), 1)
      else descendantAuthority,
    completionCount' = if (foldGuard)
      completionCount.set(attemptedSid, completionCount.get(attemptedSid) + 1)
      else completionCount,
    phase' = "folded",
    attemptedSid' = attemptedSid,
    attemptedPrior' = attemptedPrior,
    attemptedGeneration' = attemptedGeneration,
    beforeGeneration' = beforeGeneration,
    beforeIdentityGeneration' = beforeIdentityGeneration,
    beforeClaimActive' = beforeClaimActive,
    beforeExpectedPrior' = beforeExpectedPrior,
    beforeClaimedGeneration' = beforeClaimedGeneration,
    beforeCandidateOwner' = beforeCandidateOwner,
    beforePriorTombstoned' = beforePriorTombstoned,
    beforePriorTombstoneLsn' = beforePriorTombstoneLsn,
    beforeCandidateAuthority' = beforeCandidateAuthority,
    beforeCompletionCount' = beforeCompletionCount,
    beforeLsn' = beforeLsn,
  }

  action clearAttempt = all {
    phase == "folded",
    generation' = generation,
    identityGeneration' = identityGeneration,
    tombstoned' = tombstoned,
    tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,
    label' = label,
    claimActive' = claimActive,
    expectedPrior' = expectedPrior,
    claimedGeneration' = claimedGeneration,
    externalOwner' = externalOwner,
    descendantAuthority' = descendantAuthority,
    completionCount' = completionCount,
    phase' = "idle",
    attemptedSid' = attemptedSid,
    attemptedPrior' = attemptedPrior,
    attemptedGeneration' = attemptedGeneration,
    beforeGeneration' = beforeGeneration,
    beforeIdentityGeneration' = beforeIdentityGeneration,
    beforeClaimActive' = beforeClaimActive,
    beforeExpectedPrior' = beforeExpectedPrior,
    beforeClaimedGeneration' = beforeClaimedGeneration,
    beforeCandidateOwner' = beforeCandidateOwner,
    beforePriorTombstoned' = beforePriorTombstoned,
    beforePriorTombstoneLsn' = beforePriorTombstoneLsn,
    beforeCandidateAuthority' = beforeCandidateAuthority,
    beforeCompletionCount' = beforeCompletionCount,
    beforeLsn' = beforeLsn,
  }

  action relabel(sid, newLabel) = all {
    phase == "idle",
    generation' = generation,
    identityGeneration' = identityGeneration,
    tombstoned' = tombstoned,
    tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,
    label' = label.set(sid, newLabel),
    claimActive' = claimActive,
    expectedPrior' = expectedPrior,
    claimedGeneration' = claimedGeneration,
    externalOwner' = externalOwner,
    descendantAuthority' = descendantAuthority,
    completionCount' = completionCount,
    phase' = phase,
    attemptedSid' = attemptedSid,
    attemptedPrior' = attemptedPrior,
    attemptedGeneration' = attemptedGeneration,
    beforeGeneration' = beforeGeneration,
    beforeIdentityGeneration' = beforeIdentityGeneration,
    beforeClaimActive' = beforeClaimActive,
    beforeExpectedPrior' = beforeExpectedPrior,
    beforeClaimedGeneration' = beforeClaimedGeneration,
    beforeCandidateOwner' = beforeCandidateOwner,
    beforePriorTombstoned' = beforePriorTombstoned,
    beforePriorTombstoneLsn' = beforePriorTombstoneLsn,
    beforeCandidateAuthority' = beforeCandidateAuthority,
    beforeCompletionCount' = beforeCompletionCount,
    beforeLsn' = beforeLsn,
  }

  action lateEvent(sid, gen) = all {
    phase == "idle",
    tombstoned.get((sid, gen)) == 1,
    generation' = generation,
    identityGeneration' = identityGeneration,
    tombstoned' = tombstoned,
    tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,
    label' = label,
    claimActive' = claimActive,
    expectedPrior' = expectedPrior,
    claimedGeneration' = claimedGeneration,
    externalOwner' = externalOwner,
    descendantAuthority' = descendantAuthority,
    completionCount' = completionCount,
    phase' = phase,
    attemptedSid' = sid,
    attemptedPrior' = gen,
    attemptedGeneration' = gen,
    beforeGeneration' = beforeGeneration,
    beforeIdentityGeneration' = beforeIdentityGeneration,
    beforeClaimActive' = beforeClaimActive,
    beforeExpectedPrior' = beforeExpectedPrior,
    beforeClaimedGeneration' = beforeClaimedGeneration,
    beforeCandidateOwner' = beforeCandidateOwner,
    beforePriorTombstoned' = beforePriorTombstoned,
    beforePriorTombstoneLsn' = beforePriorTombstoneLsn,
    beforeCandidateAuthority' = beforeCandidateAuthority,
    beforeCompletionCount' = beforeCompletionCount,
    beforeLsn' = beforeLsn,
  }

  action step = any {
    { nondet sid = SESSION_IDS.oneOf() prepareClaim(sid) },
    {
      nondet sid = SESSION_IDS.oneOf()
      nondet prior = GENERATIONS.oneOf()
      nondet candidate = GENERATIONS.oneOf()
      arrivePromotion(sid, prior, candidate)
    },
    foldPromotion,
    clearAttempt,
    {
      nondet sid = SESSION_IDS.oneOf()
      nondet newLabel = LABELS.oneOf()
      relabel(sid, newLabel)
    },
    {
      nondet sid = SESSION_IDS.oneOf()
      nondet gen = GENERATIONS.oneOf()
      lateEvent(sid, gen)
    },
  }

  // This oracle is deliberately expanded from raw attempted evidence and arrival snapshots; it
  // does not reuse `foldGuard`. Weakening/removing the production guard therefore creates a
  // counterexample. It also catches authority omission and dropping tombstone reverse ownership.
  val promotion_fold_exact_and_atomic = phase != "folded" or {
    val oracleValid = beforeClaimActive == 1
      and beforeGeneration == attemptedPrior
      and beforeExpectedPrior == attemptedPrior
      and beforeClaimedGeneration == attemptedGeneration
      and beforeCandidateOwner == attemptedSid
      and beforeCandidateAuthority == 0
      and ((attemptedPrior == 0 and attemptedGeneration == 1)
        or (attemptedPrior > 0 and attemptedGeneration == attemptedPrior + 1))
    if (oracleValid) {
      generation.get(attemptedSid) == attemptedGeneration
        and identityGeneration.get(attemptedSid) == attemptedGeneration
        and claimActive.get(attemptedSid) == 0
        and externalOwner.get(attemptedGeneration) == attemptedSid
        and descendantAuthority.get((attemptedSid, attemptedGeneration)) == 1
        and completionCount.get(attemptedSid) == beforeCompletionCount + 1
        and lsn == beforeLsn + 1
        and (if (attemptedPrior == 0) {
          tombstoned.get((attemptedSid, attemptedPrior)) == beforePriorTombstoned
            and tombstoneLsn.get((attemptedSid, attemptedPrior)) == beforePriorTombstoneLsn
        } else {
          tombstoned.get((attemptedSid, attemptedPrior)) == 1
            and tombstoneLsn.get((attemptedSid, attemptedPrior)) == beforeLsn + 1
            and externalOwner.get(attemptedPrior) == attemptedSid
        })
    } else {
      generation.get(attemptedSid) == beforeGeneration
        and identityGeneration.get(attemptedSid) == beforeIdentityGeneration
        and claimActive.get(attemptedSid) == beforeClaimActive
        and expectedPrior.get(attemptedSid) == beforeExpectedPrior
        and claimedGeneration.get(attemptedSid) == beforeClaimedGeneration
        and externalOwner.get(attemptedGeneration) == beforeCandidateOwner
        and descendantAuthority.get((attemptedSid, attemptedGeneration)) == beforeCandidateAuthority
        and completionCount.get(attemptedSid) == beforeCompletionCount
        and tombstoned.get((attemptedSid, attemptedPrior)) == beforePriorTombstoned
        and tombstoneLsn.get((attemptedSid, attemptedPrior)) == beforePriorTombstoneLsn
        and lsn == beforeLsn
    }
  }

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

  run fresh_exact_promotion = init
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 0, 1))
    .then(foldPromotion)
    .expect(generation.get("s1") == 1
      and descendantAuthority.get(("s1", 1)) == 1
      and completionCount.get("s1") == 1
      and tombstoned.get(("s1", 0)) == 0)

  run continuation_exact_promotion = init
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 0, 1))
    .then(foldPromotion)
    .then(clearAttempt)
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 1, 2))
    .then(foldPromotion)
    .expect(generation.get("s1") == 2
      and tombstoned.get(("s1", 1)) == 1
      and externalOwner.get(1) == "s1"
      and externalOwner.get(2) == "s1"
      and descendantAuthority.get(("s1", 2)) == 1
      and completionCount.get("s1") == 2)

  run wrong_prior_is_inert = init
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 1, 1))
    .then(foldPromotion)
    .expect(generation.get("s1") == 0 and claimActive.get("s1") == 1)

  run unclaimed_greater_is_inert = init
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 0, 2))
    .then(foldPromotion)
    .expect(generation.get("s1") == 0 and claimActive.get("s1") == 1)

  run double_promotion_is_inert = init
    .then(prepareClaim("s1"))
    .then(arrivePromotion("s1", 0, 1))
    .then(foldPromotion)
    .then(clearAttempt)
    .then(arrivePromotion("s1", 0, 1))
    .then(foldPromotion)
    .expect(generation.get("s1") == 1 and completionCount.get("s1") == 1)

  run external_runtime_reservation_is_exclusive = init
    .then(prepareClaim("s1"))
    .then(prepareClaim("s2"))
    .expect(externalOwner.get(1) == "s1"
      and claimActive.get("s1") == 1
      and claimActive.get("s2") == 0)
}

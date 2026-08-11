// Patchbay seed model: authenticated session-report source ordering
//
// Core LSN order is deliberately absent: this model isolates adapter producer
// order. Environment arrival and report application are separate actions. The
// pending* variables are raw pre-state written only by arriveReport; applyPending
// reads them but cannot rewrite them, so the temporal oracle cannot be satisfied
// by lying about which report arrived.
module session_report_source_ordering {
  pure val SESSION_GENERATIONS = 1.to(2)
  pure val ADAPTER_GENERATIONS = 1.to(2)
  pure val REVISIONS = 1.to(3)
  pure val VALUES = Set("initial", "A", "B", "C")

  var phase: str
  var currentAdapterGeneration: int
  var liveSessionGeneration: int
  var liveAdapterGeneration: int
  var lastSourceRevision: int
  var mutableValue: str

  // Raw environment evidence. Only arriveReport assigns these fields.
  var pendingSessionGeneration: int
  var pendingAdapterGeneration: int
  var pendingRevision: int
  var pendingValue: str

  pure def sourceAfter(adapterGen, revision, previousAdapterGen, previousRevision) =
    adapterGen > previousAdapterGen
      or (adapterGen == previousAdapterGen and revision > previousRevision)

  pure def acceptsPending(
    authenticatedAdapterGen,
    sessionGen,
    adapterGen,
    revision,
    previousSessionGen,
    previousAdapterGen,
    previousRevision
  ) =
    adapterGen == authenticatedAdapterGen
      and (
        sessionGen > previousSessionGen
        or (
          sessionGen == previousSessionGen
            and sourceAfter(adapterGen, revision, previousAdapterGen, previousRevision)
        )
      )

  // Claim-breaking mutation: replace `revision > previousRevision` with the
  // weaker positive-revision test. mutantStep uses this; production step cannot.
  pure def acceptsPendingMutant(
    authenticatedAdapterGen,
    sessionGen,
    adapterGen,
    revision,
    previousSessionGen,
    previousAdapterGen
  ) =
    adapterGen == authenticatedAdapterGen
      and (
        sessionGen > previousSessionGen
        or (
          sessionGen == previousSessionGen
            and (
              adapterGen > previousAdapterGen
              or (adapterGen == previousAdapterGen and revision > 0)
            )
        )
      )

  action init = all {
    phase' = "idle",
    currentAdapterGeneration' = 1,
    liveSessionGeneration' = 1,
    liveAdapterGeneration' = 1,
    lastSourceRevision' = 0,
    mutableValue' = "initial",
    pendingSessionGeneration' = 1,
    pendingAdapterGeneration' = 1,
    pendingRevision' = 1,
    pendingValue' = "initial",
  }

  // Environment action: captures the report exactly and leaves live state
  // untouched. Application is impossible until this pending phase exists.
  action arriveReport(sessionGen, adapterGen, revision, value) = all {
    phase == "idle",
    sessionGen.in(SESSION_GENERATIONS),
    adapterGen.in(ADAPTER_GENERATIONS),
    revision.in(REVISIONS),
    value.in(VALUES),
    phase' = "pending",
    pendingSessionGeneration' = sessionGen,
    pendingAdapterGeneration' = adapterGen,
    pendingRevision' = revision,
    pendingValue' = value,
    currentAdapterGeneration' = currentAdapterGeneration,
    liveSessionGeneration' = liveSessionGeneration,
    liveAdapterGeneration' = liveAdapterGeneration,
    lastSourceRevision' = lastSourceRevision,
    mutableValue' = mutableValue,
  }

  // A replacement authenticated attachment changes only producer authority.
  // Its first accepted report establishes the new source cursor at revision 1.
  action replaceAdapter(adapterGen) = all {
    phase == "idle",
    adapterGen.in(ADAPTER_GENERATIONS),
    adapterGen > currentAdapterGeneration,
    phase' = phase,
    currentAdapterGeneration' = adapterGen,
    liveSessionGeneration' = liveSessionGeneration,
    liveAdapterGeneration' = liveAdapterGeneration,
    lastSourceRevision' = lastSourceRevision,
    mutableValue' = mutableValue,
    pendingSessionGeneration' = pendingSessionGeneration,
    pendingAdapterGeneration' = pendingAdapterGeneration,
    pendingRevision' = pendingRevision,
    pendingValue' = pendingValue,
  }

  // Production atomic application. It cannot rewrite raw pending evidence.
  action applyPending = all {
    phase == "pending",
    phase' = "idle",
    liveSessionGeneration' = if (acceptsPending(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration,
      lastSourceRevision
    )) pendingSessionGeneration else liveSessionGeneration,
    liveAdapterGeneration' = if (acceptsPending(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration,
      lastSourceRevision
    )) pendingAdapterGeneration else liveAdapterGeneration,
    lastSourceRevision' = if (acceptsPending(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration,
      lastSourceRevision
    )) pendingRevision else lastSourceRevision,
    mutableValue' = if (acceptsPending(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration,
      lastSourceRevision
    )) pendingValue else mutableValue,
    currentAdapterGeneration' = currentAdapterGeneration,
    pendingSessionGeneration' = pendingSessionGeneration,
    pendingAdapterGeneration' = pendingAdapterGeneration,
    pendingRevision' = pendingRevision,
    pendingValue' = pendingValue,
  }

  // Claim-breaking application differs only in the comparison predicate.
  action applyPendingMutant = all {
    phase == "pending",
    phase' = "idle",
    liveSessionGeneration' = if (acceptsPendingMutant(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration
    )) pendingSessionGeneration else liveSessionGeneration,
    liveAdapterGeneration' = if (acceptsPendingMutant(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration
    )) pendingAdapterGeneration else liveAdapterGeneration,
    lastSourceRevision' = if (acceptsPendingMutant(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration
    )) pendingRevision else lastSourceRevision,
    mutableValue' = if (acceptsPendingMutant(
      currentAdapterGeneration,
      pendingSessionGeneration,
      pendingAdapterGeneration,
      pendingRevision,
      liveSessionGeneration,
      liveAdapterGeneration
    )) pendingValue else mutableValue,
    currentAdapterGeneration' = currentAdapterGeneration,
    pendingSessionGeneration' = pendingSessionGeneration,
    pendingAdapterGeneration' = pendingAdapterGeneration,
    pendingRevision' = pendingRevision,
    pendingValue' = pendingValue,
  }

  action step = any {
    nondet sessionGen = SESSION_GENERATIONS.oneOf()
    nondet adapterGen = ADAPTER_GENERATIONS.oneOf()
    nondet revision = REVISIONS.oneOf()
    nondet value = VALUES.oneOf()
    arriveReport(sessionGen, adapterGen, revision, value),
    nondet adapterGen = ADAPTER_GENERATIONS.oneOf()
    replaceAdapter(adapterGen),
    applyPending,
  }

  // Same environment topology, with only the source comparison weakened.
  action mutantStep = any {
    nondet sessionGen = SESSION_GENERATIONS.oneOf()
    nondet adapterGen = ADAPTER_GENERATIONS.oneOf()
    nondet revision = REVISIONS.oneOf()
    nondet value = VALUES.oneOf()
    arriveReport(sessionGen, adapterGen, revision, value),
    nondet adapterGen = ADAPTER_GENERATIONS.oneOf()
    replaceAdapter(adapterGen),
    applyPendingMutant,
  }

  // @promotion {
  //   property:    SessionReportSourceOrdering
  //   status:      promoted
  //   model:       specs/seed/session_report_source_ordering.qnt
  //   language:    quint
  //   backend:     apalache-temporal
  //   invocation:  echo y | quint verify session_report_source_ordering.qnt --temporal session_report_source_ordering --max-steps 10
  //   bounds:      { session_generations: 2, adapter_generations: 2, revisions: 3, values: 4, max_steps: 10 }
  //   expected:    pass
  //   proto_fields: [patchbay.SessionReport.session_generation, patchbay.SessionReport.source_cursor, patchbay.SessionReportSourceCursor.adapter_generation, patchbay.SessionReportSourceCursor.revision, patchbay.SessionReportApplied.report, patchbay.SessionReportApplied.previous_source_cursor, patchbay.Session.last_source_cursor, patchbay.Session.model]
  //   semantics:   within one authenticated runtime-session producer epoch, an equal/lower source cursor or an old runtime/adapter generation leaves every report-carried value and the durable source watermark unchanged; newer runtime or adapter generations may establish a fresh positive revision
  // }
  // INDEPENDENT temporal oracle: the stale predicate is expanded over raw
  // pending pre-state and does not call sourceAfter or acceptsPending. The
  // application action cannot rewrite pending evidence. Selecting mutantStep
  // weakens only the action comparison and must produce a counterexample.
  temporal session_report_source_ordering = always(
    (
      phase == "pending"
        and (
          pendingSessionGeneration < liveSessionGeneration
          or pendingAdapterGeneration != currentAdapterGeneration
          or (
            pendingSessionGeneration == liveSessionGeneration
              and (
                pendingAdapterGeneration < liveAdapterGeneration
                or (
                  pendingAdapterGeneration == liveAdapterGeneration
                    and pendingRevision <= lastSourceRevision
                )
              )
          )
        )
    ).implies(
      next(liveSessionGeneration) == liveSessionGeneration
        and next(liveAdapterGeneration) == liveAdapterGeneration
        and next(lastSourceRevision) == lastSourceRevision
        and next(mutableValue) == mutableValue
    )
  )

  run delayed_report_is_inert =
    (init)
      .then(arriveReport(1, 1, 1, "A"))
      .then(applyPending)
      .then(arriveReport(1, 1, 3, "B"))
      .then(applyPending)
      .then(arriveReport(1, 1, 2, "A"))
      .then(applyPending)
      .expect(mutableValue == "B" and lastSourceRevision == 3)

  run adapter_generation_resets_revision =
    (init)
      .then(arriveReport(1, 1, 3, "B"))
      .then(applyPending)
      .then(replaceAdapter(2))
      .then(arriveReport(1, 2, 1, "C"))
      .then(applyPending)
      .expect(mutableValue == "C" and liveAdapterGeneration == 2 and lastSourceRevision == 1)

  run runtime_generation_resets_revision =
    (init)
      .then(arriveReport(1, 1, 3, "B"))
      .then(applyPending)
      .then(arriveReport(2, 1, 1, "C"))
      .then(applyPending)
      .expect(mutableValue == "C" and liveSessionGeneration == 2 and lastSourceRevision == 1)

  run stale_guard_mutant_rolls_back =
    (init)
      .then(arriveReport(1, 1, 1, "A"))
      .then(applyPendingMutant)
      .then(arriveReport(1, 1, 3, "B"))
      .then(applyPendingMutant)
      .then(arriveReport(1, 1, 2, "A"))
      .then(applyPendingMutant)
      .expect(mutableValue == "A" and lastSourceRevision == 2)
}

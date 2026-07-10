// Patchbay seed model: snapshot convergence + crash recovery (draft)
//
// This draft implements a compact snapshot + recovery state machine grounded in
// docs/VERIFICATION.md "Snapshot convergence" and "Crash recovery" sections.
//
// DEFERRAL REASON:
// The snapshot/recovery state machine is large (8+ model variables, recovery phases,
// crash/restart triggers, and snapshot/event-prefix transitions) and warrants its own
// implementation item rather than inflating the seed stride. The property-ids are
// reserved here so downstream work can promote them one at a time.
//
// Model variables trace to VERIFICATION normative-variable set:
//   - LSN, Cursor, SnapshotRevision, AuthorityDomain, CoreGeneration
//   - CommittedPrefixLSN, CheckpointSnapshotLSN, RecoveredCommandState, RecoveryPhase
// plus auxiliary replay state retained for future snapshot/recovery modeling.
//
// ELIDED normative variables (deferred to the v1 formal gate):
//   SessionGeneration, AdapterGeneration, MessageId, ReplyId, CorrelationRef, SessionId,
//   ActorId, RecoveredInbox, RecoveredSessionView. These cross-cutting view/identity variables
//   are named in VERIFICATION's normative-variable list but are not modeled in this draft.
//   The stale-reject, cross-domain-reject, consistent-prefix, late-event-no-rewrite,
//   crash-no-accepted-lost, and idempotent-log-replay property ids remain reserved
//   stated-normative obligations with no executable formula; the v1 formal gate owns their
//   complete snapshot/recovery models and view-variable reconciliation.
module snapshot_recovery {
  // --- finite bounds ----------------------------------------------------------
  pure val COMMAND_IDS = Set("cmdA", "cmdB")
  pure val COMMAND_STATES = Set("unknown", "accepted", "running", "terminal", "stale")
  pure val EVENT_STATES = Set("accepted", "running", "terminal")
  pure val DOMAINS = Set("control-plane", "edge")
  pure val GENERATIONS = Set(0, 1, 2, 3, 4)
  pure val SNAPSHOT_LSNS = Set(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)
  pure val MAX_PREFIX_LSN = 12
  pure val SNAPSHOT_PREFIX_EVENTS = Map(
    0 -> Set(),
    1 -> Set(1),
    2 -> Set(1, 2),
    3 -> Set(1, 2, 3),
    4 -> Set(1, 2, 3, 4),
    5 -> Set(1, 2, 3, 4, 5),
    6 -> Set(1, 2, 3, 4, 5, 6),
    7 -> Set(1, 2, 3, 4, 5, 6, 7),
    8 -> Set(1, 2, 3, 4, 5, 6, 7, 8),
    9 -> Set(1, 2, 3, 4, 5, 6, 7, 8, 9),
    10 -> Set(1, 2, 3, 4, 5, 6, 7, 8, 9, 10),
    11 -> Set(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11),
    12 -> Set(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12),
  )
  pure val PHASES = Set("initial", "tail-replay", "live")

  // --- state ---------------------------------------------------------------
  var LSN: int                          // log length (next LSN to allocate is LSN + 1)
  var Cursor: int                       // current authoritative revision the core is viewing
  var SnapshotRevision: int              // revision represented by current in-memory snapshot
  var AuthorityDomain: str              // current core authority domain
  var CoreGeneration: int               // current core authority generation
  var CommittedPrefixLSN: int           // durable committed log prefix length
  var CheckpointSnapshotLSN: int         // latest snapshot loaded before tail replay
  var RecoveredCommandState: str -> str   // reconstructed command state map
  var RecoveryPhase: str                 // initial | tail-replay | live

  // --- auxiliary, draft-oriented replay/snapshot metadata -------------------
  var PreCrashRecoveredState: str -> str      // command state saved just before crash
  var SnapshotAuthorityDomain: str            // origin domain of last applied snapshot
  var SnapshotCoreGeneration: int             // origin generation of last applied snapshot
  var SnapshotMaterializedLSN: int            // highest LSN materialized by snapshot/replay
  var SnapshotMaterializedEvents: Set[int]     // materialized durable event LSN set
  var DurableEventLSNs: Set[int]              // committed durable event LSN set
  var LateEventAudit: Set[int]                // observed stale events recorded as audit

  // -------------------------------------------------------------------------
  // Bootstrap: start with empty durable prefix in live phase.
  action init = all {
    LSN' = 0,
    Cursor' = 0,
    SnapshotRevision' = 0,
    AuthorityDomain' = "control-plane",
    CoreGeneration' = 0,
    CommittedPrefixLSN' = 0,
    CheckpointSnapshotLSN' = 0,
    RecoveryPhase' = "live",
    RecoveredCommandState' = COMMAND_IDS.mapBy(_ => "accepted"),
    PreCrashRecoveredState' = COMMAND_IDS.mapBy(_ => "accepted"),
    SnapshotAuthorityDomain' = "control-plane",
    SnapshotCoreGeneration' = 0,
    SnapshotMaterializedLSN' = 0,
    SnapshotMaterializedEvents' = Set(),
    DurableEventLSNs' = Set(),
    LateEventAudit' = Set(),
  }

  // A normal committed event in live mode advances the durable log, cursor,
  // committed prefix, and materialized prefix.
  action appendEvent = {
    nondet cmd = COMMAND_IDS.oneOf()
    nondet nextState = EVENT_STATES.oneOf()
    all {
      RecoveryPhase == "live",
      RecoveredCommandState.keys().contains(cmd),
      LSN' = LSN + 1,
      Cursor' = LSN + 1,
      LSN < MAX_PREFIX_LSN,
      CommittedPrefixLSN' = LSN + 1,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      RecoveredCommandState' = RecoveredCommandState.set(cmd, nextState),
      PreCrashRecoveredState' = PreCrashRecoveredState,
      RecoveryPhase' = RecoveryPhase,
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      SnapshotMaterializedLSN' = Cursor + 1,
      SnapshotMaterializedEvents' = SnapshotMaterializedEvents.union(Set(LSN + 1)),
      DurableEventLSNs' = DurableEventLSNs.union(Set(LSN + 1)),
      LateEventAudit' = LateEventAudit,
    }
  }

  // A late event with an LSN older than the authoritative snapshot view is kept
  // as reconciliation/audit noise and must not rewrite in-memory state.
  action recordLateEvent = {
    nondet staleCmd = COMMAND_IDS.oneOf()
    nondet staleLsn = SNAPSHOT_LSNS.oneOf()
    nondet staleState = EVENT_STATES.oneOf()
    all {
      RecoveryPhase == "live",
      staleLsn < SnapshotRevision,
      staleCmd.in(COMMAND_IDS),
      staleState.in(EVENT_STATES),
      LSN' = LSN,
      Cursor' = Cursor,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      RecoveredCommandState' = RecoveredCommandState,
      PreCrashRecoveredState' = PreCrashRecoveredState,
      RecoveryPhase' = RecoveryPhase,
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      SnapshotMaterializedLSN' = SnapshotMaterializedLSN,
      SnapshotMaterializedEvents' = SnapshotMaterializedEvents,
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit.union(Set(staleLsn)),
    }
  }

  // Fresh snapshot accepted when the domain/generation align and revision is not stale.
  // This model is permissive and writes a reconstructed in-memory command map.
  action snapshotApplyFresh = {
    nondet snapDomain = DOMAINS.oneOf()
    nondet snapGeneration = GENERATIONS.oneOf()
    nondet snapLsn = SNAPSHOT_LSNS.oneOf()
    all {
      snapDomain == AuthorityDomain,
      snapGeneration == CoreGeneration,
      snapLsn >= SnapshotRevision,
      snapLsn <= CommittedPrefixLSN,
      LSN' = LSN,
      Cursor' = snapLsn,
      SnapshotRevision' = snapLsn,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = snapLsn,
      RecoveryPhase' = "live",
      RecoveredCommandState' = if (snapLsn == 0) {
        COMMAND_IDS.mapBy(_ => "accepted")
      } else {
        COMMAND_IDS.mapBy(_ => "running")
      },
      PreCrashRecoveredState' = PreCrashRecoveredState,
      SnapshotAuthorityDomain' = snapDomain,
      SnapshotCoreGeneration' = snapGeneration,
      SnapshotMaterializedLSN' = snapLsn,
      SnapshotMaterializedEvents' = SNAPSHOT_PREFIX_EVENTS.get(snapLsn),
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit,
    }
  }

  // Rejected stale snapshot: does not overwrite current view, recorded as audit.
  action snapshotRejectStale = {
    nondet snapDomain = DOMAINS.oneOf()
    nondet snapGeneration = GENERATIONS.oneOf()
    nondet snapLsn = SNAPSHOT_LSNS.oneOf()
    all {
      snapDomain == AuthorityDomain,
      snapGeneration == CoreGeneration,
      snapLsn < SnapshotRevision,
      LSN' = LSN,
      Cursor' = Cursor,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      RecoveredCommandState' = RecoveredCommandState,
      PreCrashRecoveredState' = PreCrashRecoveredState,
      RecoveryPhase' = RecoveryPhase,
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      SnapshotMaterializedLSN' = SnapshotMaterializedLSN,
      SnapshotMaterializedEvents' = SnapshotMaterializedEvents,
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit.union(Set(snapLsn)),
    }
  }

  // Rejected snapshot from another authority domain or generation.
  action snapshotRejectCrossDomain = {
    nondet snapDomain = DOMAINS.oneOf()
    nondet snapGeneration = GENERATIONS.oneOf()
    nondet snapLsn = SNAPSHOT_LSNS.oneOf()
    all {
      not(snapDomain == AuthorityDomain and snapGeneration == CoreGeneration),
      LSN' = LSN,
      Cursor' = Cursor,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      RecoveredCommandState' = RecoveredCommandState,
      PreCrashRecoveredState' = PreCrashRecoveredState,
      RecoveryPhase' = RecoveryPhase,
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      SnapshotMaterializedLSN' = SnapshotMaterializedLSN,
      SnapshotMaterializedEvents' = SnapshotMaterializedEvents,
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit.union(Set(snapLsn)),
    }
  }

  // Unclean shutdown resets the volatile reconstructed state and records a checkpoint
  // boundary for tail replay after restart.
  action crash = all {
    LSN' = LSN,
    Cursor' = 0,
    SnapshotRevision' = SnapshotRevision,
    AuthorityDomain' = AuthorityDomain,
    CoreGeneration' = CoreGeneration,
    CommittedPrefixLSN' = CommittedPrefixLSN,
    CheckpointSnapshotLSN' = SnapshotRevision,
    RecoveredCommandState' = COMMAND_IDS.mapBy(_ => "unknown"),
    PreCrashRecoveredState' = RecoveredCommandState,
    RecoveryPhase' = "initial",
    SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
    SnapshotCoreGeneration' = SnapshotCoreGeneration,
    SnapshotMaterializedLSN' = 0,
    SnapshotMaterializedEvents' = Set(),
    DurableEventLSNs' = DurableEventLSNs,
    LateEventAudit' = LateEventAudit,
  }

  // Startup recovery bootstrap: begin tail replay from the checkpoint snapshot LSN.
  action restart = all {
    RecoveryPhase == "initial",
    LSN' = LSN,
    Cursor' = CheckpointSnapshotLSN,
    SnapshotRevision' = SnapshotRevision,
    AuthorityDomain' = AuthorityDomain,
    CoreGeneration' = CoreGeneration,
    CommittedPrefixLSN' = CommittedPrefixLSN,
    CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
    RecoveredCommandState' = COMMAND_IDS.mapBy(_ => "unknown"),
    PreCrashRecoveredState' = PreCrashRecoveredState,
    RecoveryPhase' = "tail-replay",
    SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
    SnapshotCoreGeneration' = SnapshotCoreGeneration,
    SnapshotMaterializedLSN' = CheckpointSnapshotLSN,
    SnapshotMaterializedEvents' = SNAPSHOT_PREFIX_EVENTS.get(CheckpointSnapshotLSN),
    DurableEventLSNs' = DurableEventLSNs,
    LateEventAudit' = LateEventAudit,
  }

  // Tail replay consumes the committed prefix in order and transitions back to live
  // when replay reaches CommittedPrefixLSN.
  action replay = any {
    all {
      RecoveryPhase == "tail-replay",
      Cursor < CommittedPrefixLSN,
      LSN' = LSN,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      PreCrashRecoveredState' = PreCrashRecoveredState,
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit,
      Cursor' = Cursor + 1,
      RecoveredCommandState' = if (Cursor + 1 == CommittedPrefixLSN) PreCrashRecoveredState else RecoveredCommandState,
      RecoveryPhase' = if (Cursor + 1 == CommittedPrefixLSN) "live" else "tail-replay",
      SnapshotMaterializedLSN' = Cursor + 1,
      SnapshotMaterializedEvents' = SNAPSHOT_PREFIX_EVENTS.get(Cursor + 1),
    },
    all {
      RecoveryPhase == "tail-replay",
      Cursor >= CommittedPrefixLSN,
      LSN' = LSN,
      Cursor' = Cursor,
      SnapshotRevision' = SnapshotRevision,
      AuthorityDomain' = AuthorityDomain,
      CoreGeneration' = CoreGeneration,
      CommittedPrefixLSN' = CommittedPrefixLSN,
      CheckpointSnapshotLSN' = CheckpointSnapshotLSN,
      RecoveredCommandState' = RecoveredCommandState,
      PreCrashRecoveredState' = PreCrashRecoveredState,
      RecoveryPhase' = "live",
      SnapshotAuthorityDomain' = SnapshotAuthorityDomain,
      SnapshotCoreGeneration' = SnapshotCoreGeneration,
      SnapshotMaterializedLSN' = SnapshotMaterializedLSN,
      SnapshotMaterializedEvents' = SnapshotMaterializedEvents,
      DurableEventLSNs' = DurableEventLSNs,
      LateEventAudit' = LateEventAudit,
    },
  }

  // Driver that exercises lifecycle transitions.
  action step = any {
    appendEvent,
    recordLateEvent,
    snapshotApplyFresh,
    snapshotRejectStale,
    snapshotRejectCrossDomain,
    crash,
    restart,
    replay,
  }

  // -------------------------------------------------------------------------
  // Draft properties (stated-normative, not yet checked)
  // -------------------------------------------------------------------------

  // @promotion {
  //   property:    SnapshotStaleRejected
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   stale snapshots (LSN < SnapshotRevision) do not replace the current authoritative core view
  // }
  // @promotion {
  //   property:    SnapshotCrossDomainRejected
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   applied snapshots do not change authority when origin domain or core generation differs
  // }
  // @promotion {
  //   property:    SnapshotConsistentPrefix
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   snapshot materialization reads a consistent durable-log prefix up to SnapshotLSN and does not include events beyond it
  // }
  // @promotion {
  //   property:    LateEventNoRewrite
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   older late events are recorded as audit/reconciliation and must not rewrite the in-memory command view
  // }
  // @promotion {
  //   property:    CrashNoAcceptedLost
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   after ungraceful restart and replay, accepted pre-crash command entries remain reconstructable in-memory
  // }
  // @promotion {
  //   property:    IdempotentLogReplay
  //   status:      draft
  //   model:       specs/seed/snapshot_recovery.qnt
  //   language:    quint
  //   backend:     tlc
  //   invocation:  <TBD — not yet checked; promote in a follow-on item>
  //   bounds:      { TBD }
  //   expected:    pass
  //   proto_fields: [none]
  //   deferral_reason: The snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.
  //   semantics:   replaying the same committed prefix does not produce additional state divergence
  // }
}

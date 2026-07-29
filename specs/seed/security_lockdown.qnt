// Patchbay security-lockdown model.
//
// The attempted operation and exit channel are independent inputs. The
// posture transition and replay state are committed effects, so the model does
// not define the claimed boundaries only by putting them in an action guard.
module security_lockdown {
  pure val CHANNELS = Set("loopback_admin", "routine_web")
  pure val TERMINAL = Set("completed", "failed", "cancelled")

  var lockdown: bool
  var reasonCode: str
  var runtimeSessions: int
  var staleSessions: int
  var operatorSessionFloor: int
  var committedLog: str
  var replayedLockdown: bool
  var attemptedOperation: bool
  var operationAccepted: bool
  var attemptedExitChannel: str
  var exitSucceeded: bool
  var acceptedTerminal: str

  action init = all {
    lockdown' = false,
    reasonCode' = "",
    runtimeSessions' = 2,
    staleSessions' = 0,
    operatorSessionFloor' = 1,
    committedLog' = "",
    replayedLockdown' = false,
    attemptedOperation' = false,
    operationAccepted' = false,
    attemptedExitChannel' = "",
    exitSucceeded' = false,
    acceptedTerminal' = "",
  }

  // Entry is idempotent, and the source log is the replay authority.
  action enter(reason, sessionFloor) = any {
    all {
      not(lockdown),
      lockdown' = true,
      reasonCode' = reason,
      staleSessions' = runtimeSessions,
      operatorSessionFloor' = sessionFloor,
      committedLog' = "entered",
      replayedLockdown' = replayedLockdown,
      attemptedOperation' = attemptedOperation,
      operationAccepted' = operationAccepted,
      attemptedExitChannel' = attemptedExitChannel,
      exitSucceeded' = exitSucceeded,
      acceptedTerminal' = acceptedTerminal,
      runtimeSessions' = runtimeSessions,
    },
    all {
      lockdown,
      lockdown' = lockdown,
      reasonCode' = reasonCode,
      staleSessions' = staleSessions,
      operatorSessionFloor' = operatorSessionFloor,
      committedLog' = committedLog,
      replayedLockdown' = replayedLockdown,
      attemptedOperation' = attemptedOperation,
      operationAccepted' = operationAccepted,
      attemptedExitChannel' = attemptedExitChannel,
      exitSucceeded' = exitSucceeded,
      acceptedTerminal' = acceptedTerminal,
      runtimeSessions' = runtimeSessions,
    },
  }

  // An operation is an independent attempted input. Lockdown rejects it
  // before acceptance; an accepted command may still terminalize afterwards.
  action submitOperation = all {
    attemptedOperation' = true,
    operationAccepted' = not(lockdown),
    lockdown' = lockdown,
    reasonCode' = reasonCode,
    runtimeSessions' = runtimeSessions,
    staleSessions' = staleSessions,
    operatorSessionFloor' = operatorSessionFloor,
    committedLog' = committedLog,
    replayedLockdown' = replayedLockdown,
    attemptedExitChannel' = attemptedExitChannel,
    exitSucceeded' = exitSucceeded,
    acceptedTerminal' = acceptedTerminal,
  }

  action terminalizeAccepted = all {
    acceptedTerminal' = "completed",
    lockdown' = lockdown,
    reasonCode' = reasonCode,
    runtimeSessions' = runtimeSessions,
    staleSessions' = staleSessions,
    operatorSessionFloor' = operatorSessionFloor,
    committedLog' = committedLog,
    replayedLockdown' = replayedLockdown,
    attemptedOperation' = attemptedOperation,
    operationAccepted' = operationAccepted,
    attemptedExitChannel' = attemptedExitChannel,
    exitSucceeded' = exitSucceeded,
  }

  // Restart reconstructs posture from the committed event, not from a process
  // boolean. A missing entry in committedLog is the failure case the property
  // exposes if a guard/rematerialization mutation is introduced.
  action restartAndReplay = all {
    replayedLockdown' = committedLog == "entered" and lockdown,
    lockdown' = if (committedLog == "entered") { true } else { lockdown },
    reasonCode' = reasonCode,
    runtimeSessions' = runtimeSessions,
    staleSessions' = staleSessions,
    operatorSessionFloor' = operatorSessionFloor,
    committedLog' = committedLog,
    attemptedOperation' = attemptedOperation,
    operationAccepted' = operationAccepted,
    attemptedExitChannel' = attemptedExitChannel,
    exitSucceeded' = exitSucceeded,
    acceptedTerminal' = acceptedTerminal,
  }

  // Exit is intentionally channel-sensitive. Routine web reauthentication is
  // represented as an attempted channel but cannot clear the posture.
  action attemptExit(channel) = all {
    attemptedExitChannel' = channel,
    exitSucceeded' = if (lockdown and channel == "loopback_admin") { true } else { false },
    lockdown' = if (lockdown and channel == "loopback_admin") { false } else { lockdown },
    reasonCode' = reasonCode,
    runtimeSessions' = runtimeSessions,
    staleSessions' = staleSessions,
    operatorSessionFloor' = operatorSessionFloor,
    committedLog' = if (lockdown and channel == "loopback_admin") { "exited" } else { committedLog },
    replayedLockdown' = replayedLockdown,
    attemptedOperation' = attemptedOperation,
    operationAccepted' = operationAccepted,
    acceptedTerminal' = acceptedTerminal,
  }

  action step = any {
    enter("suspected_endpoint_compromise", 8),
    submitOperation,
    terminalizeAccepted,
    restartAndReplay,
    attemptExit("loopback_admin"),
    attemptExit("routine_web"),
  }

  val LockdownRejectsNewOperations = not(lockdown and attemptedOperation) or not(operationAccepted)
  val LockdownReplayPersists = not(replayedLockdown) or lockdown
  val LockdownEntryStalesSessions = not(lockdown) or staleSessions == runtimeSessions
  val LockdownInvalidatesExistingOperatorSessions = not(lockdown) or operatorSessionFloor >= 1
  val BootstrapOnlyExit = attemptedExitChannel != "routine_web" or not(exitSucceeded)

  // @promotion {
  //   property: LockdownRejectsNewOperations
  //   status: draft
  //   model: specs/seed/security_lockdown.qnt
  //   language: quint
  //   backend: apalache
  //   invocation: quint verify security_lockdown.qnt --invariant LockdownRejectsNewOperations --max-steps 12
  //   bounds: { runtime_sessions: 2, channels: 2, max_steps: 12 }
  //   expected: pass
  //   proto_fields: [patchbay.SubmissionResult.outcome, patchbay.SubmissionResult.reason_code]
  //   semantics: an attempted Operation cannot become accepted while the durable posture is active
  // }
  // @promotion {
  //   property: LockdownReplayPersists
  //   status: draft
  //   model: specs/seed/security_lockdown.qnt
  //   language: quint
  //   backend: apalache
  //   invocation: quint verify security_lockdown.qnt --invariant LockdownReplayPersists --max-steps 12
  //   bounds: { runtime_sessions: 2, max_steps: 12 }
  //   expected: pass
  //   proto_fields: [patchbay.SecurityLockdownEvent, patchbay.SecurityLockdownState.active]
  //   semantics: replay of the committed entry event preserves active posture across restart
  // }
  // @promotion {
  //   property: LockdownEntryStalesSessions
  //   status: draft
  //   model: specs/seed/security_lockdown.qnt
  //   language: quint
  //   backend: apalache
  //   invocation: quint verify security_lockdown.qnt --invariant LockdownEntryStalesSessions --max-steps 12
  //   bounds: { runtime_sessions: 2, max_steps: 12 }
  //   expected: pass
  //   proto_fields: [patchbay.SecurityLockdownEntered.affected_runtime_session_count, patchbay.SessionConnectivityState]
  //   semantics: entry clamps every current runtime session to stale
  // }
  // @promotion {
  //   property: LockdownInvalidatesExistingOperatorSessions
  //   status: draft
  //   model: specs/seed/security_lockdown.qnt
  //   language: quint
  //   backend: apalache
  //   invocation: quint verify security_lockdown.qnt --invariant LockdownInvalidatesExistingOperatorSessions --max-steps 12
  //   bounds: { operator_session_floor: 1, max_steps: 12 }
  //   expected: pass
  //   proto_fields: [patchbay.SecurityLockdownEntered.invalidated_through_operator_session_generation]
  //   semantics: entry advances the durable operator-session generation floor
  // }
  // @promotion {
  //   property: BootstrapOnlyExit
  //   status: draft
  //   model: specs/seed/security_lockdown.qnt
  //   language: quint
  //   backend: apalache
  //   invocation: quint verify security_lockdown.qnt --invariant BootstrapOnlyExit --max-steps 12
  //   bounds: { channels: 2, max_steps: 12 }
  //   expected: pass
  //   proto_fields: [patchbay.BootstrapChannelKind, patchbay.ExitSecurityLockdownResult]
  //   semantics: only the configured loopback admin bootstrap channel can clear lockdown
  // }

  // Executable mutation guards: the properties observe outcomes rather than
  // reusing the enter/exit action guards. These scenarios are intentionally
  // documented as the independent mutation evidence for the checkpoint.
  run entry_then_operation = (init).then(enter("suspected_endpoint_compromise", 8)).then(submitOperation).expect(not(operationAccepted))
  run entry_then_restart = (init).then(enter("suspected_endpoint_compromise", 8)).then(restartAndReplay).expect(replayedLockdown and lockdown)
  run entry_stales_all = (init).then(enter("suspected_endpoint_compromise", 8)).expect(staleSessions == runtimeSessions)
  run admin_exit_succeeds = (init).then(enter("suspected_endpoint_compromise", 8)).then(attemptExit("loopback_admin")).expect(not(lockdown) and exitSucceeded and committedLog == "exited")
  run web_exit_denied = (init).then(enter("suspected_endpoint_compromise", 8)).then(attemptExit("routine_web")).expect(lockdown and not(exitSucceeded))
}

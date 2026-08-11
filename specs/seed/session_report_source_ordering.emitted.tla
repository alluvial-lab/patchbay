// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile session_report_source_ordering.qnt --target tlaplus --verbosity 0
// Source: session_report_source_ordering.qnt. Inspection artifact, NOT an independent checker lane.

-------------------- MODULE session_report_source_ordering --------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Int));
*)
REVISIONS == 1 .. 3

(*
  @type: (() => Set(Str));
*)
VALUES == { "initial", "A", "B", "C" }

VARIABLE
  (*
    @type: Str;
  *)
  phase

VARIABLE
  (*
    @type: Int;
  *)
  currentAdapterGeneration

VARIABLE
  (*
    @type: Int;
  *)
  liveSessionGeneration

VARIABLE
  (*
    @type: Int;
  *)
  liveAdapterGeneration

VARIABLE
  (*
    @type: Int;
  *)
  lastSourceRevision

VARIABLE
  (*
    @type: Str;
  *)
  mutableValue

VARIABLE
  (*
    @type: Int;
  *)
  pendingSessionGeneration

VARIABLE
  (*
    @type: Int;
  *)
  pendingAdapterGeneration

VARIABLE
  (*
    @type: Int;
  *)
  pendingRevision

VARIABLE
  (*
    @type: Str;
  *)
  pendingValue

(*
  @type: (() => Set(Int));
*)
SESSION_GENERATIONS == 1 .. 2

(*
  @type: (() => Set(Int));
*)
ADAPTER_GENERATIONS == 1 .. 2

(*
  @type: ((Int, Int, Int, Int, Int, Int) => Bool);
*)
acceptsPendingMutant(authenticatedAdapterGen_111, sessionGen_111, adapterGen_111,
revision_111, previousSessionGen_111, previousAdapterGen_111) ==
  adapterGen_111 = authenticatedAdapterGen_111
    /\ (sessionGen_111 > previousSessionGen_111
      \/ (sessionGen_111 = previousSessionGen_111
        /\ (adapterGen_111 > previousAdapterGen_111
          \/ (adapterGen_111 = previousAdapterGen_111 /\ revision_111 > 0))))

(*
  @type: ((Int, Int, Int, Int) => Bool);
*)
sourceAfter(adapterGen_54, revision_54, previousAdapterGen_54, previousRevision_54) ==
  adapterGen_54 > previousAdapterGen_54
    \/ (adapterGen_54 = previousAdapterGen_54
      /\ revision_54 > previousRevision_54)

(*
  @type: (() => Bool);
*)
init ==
  phase = "idle"
    /\ currentAdapterGeneration = 1
    /\ liveSessionGeneration = 1
    /\ liveAdapterGeneration = 1
    /\ lastSourceRevision = 0
    /\ mutableValue = "initial"
    /\ pendingSessionGeneration = 1
    /\ pendingAdapterGeneration = 1
    /\ pendingRevision = 1
    /\ pendingValue = "initial"

(*
  @type: ((Int, Int, Int, Str) => Bool);
*)
arriveReport(sessionGen_195, adapterGen_195, revision_195, value_195) ==
  phase = "idle"
    /\ sessionGen_195 \in SESSION_GENERATIONS
    /\ adapterGen_195 \in ADAPTER_GENERATIONS
    /\ revision_195 \in REVISIONS
    /\ value_195 \in VALUES
    /\ phase' := "pending"
    /\ pendingSessionGeneration' := sessionGen_195
    /\ pendingAdapterGeneration' := adapterGen_195
    /\ pendingRevision' := revision_195
    /\ pendingValue' := value_195
    /\ currentAdapterGeneration' := currentAdapterGeneration
    /\ liveSessionGeneration' := liveSessionGeneration
    /\ liveAdapterGeneration' := liveAdapterGeneration
    /\ lastSourceRevision' := lastSourceRevision
    /\ mutableValue' := mutableValue

(*
  @type: ((Int) => Bool);
*)
replaceAdapter(adapterGen_238) ==
  phase = "idle"
    /\ adapterGen_238 \in ADAPTER_GENERATIONS
    /\ adapterGen_238 > currentAdapterGeneration
    /\ phase' := phase
    /\ currentAdapterGeneration' := adapterGen_238
    /\ liveSessionGeneration' := liveSessionGeneration
    /\ liveAdapterGeneration' := liveAdapterGeneration
    /\ lastSourceRevision' := lastSourceRevision
    /\ mutableValue' := mutableValue
    /\ pendingSessionGeneration' := pendingSessionGeneration
    /\ pendingAdapterGeneration' := pendingAdapterGeneration
    /\ pendingRevision' := pendingRevision
    /\ pendingValue' := pendingValue

(*
  @type: (() => Bool);
*)
session_report_source_ordering ==
  [](phase = "pending"
    /\ ((pendingSessionGeneration < liveSessionGeneration
        \/ pendingAdapterGeneration /= currentAdapterGeneration)
      \/ (pendingSessionGeneration = liveSessionGeneration
        /\ (pendingAdapterGeneration < liveAdapterGeneration
          \/ (pendingAdapterGeneration = liveAdapterGeneration
            /\ pendingRevision <= lastSourceRevision))))
    => ((liveSessionGeneration' = liveSessionGeneration
          /\ liveAdapterGeneration' = liveAdapterGeneration)
        /\ lastSourceRevision' = lastSourceRevision)
      /\ mutableValue' = mutableValue)

(*
  @type: (() => Bool);
*)
applyPendingMutant ==
  phase = "pending"
    /\ phase' := "idle"
    /\ liveSessionGeneration'
      := (IF acceptsPendingMutant(currentAdapterGeneration, pendingSessionGeneration,
      pendingAdapterGeneration, pendingRevision, liveSessionGeneration, liveAdapterGeneration)
      THEN pendingSessionGeneration
      ELSE liveSessionGeneration)
    /\ liveAdapterGeneration'
      := (IF acceptsPendingMutant(currentAdapterGeneration, pendingSessionGeneration,
      pendingAdapterGeneration, pendingRevision, liveSessionGeneration, liveAdapterGeneration)
      THEN pendingAdapterGeneration
      ELSE liveAdapterGeneration)
    /\ lastSourceRevision'
      := (IF acceptsPendingMutant(currentAdapterGeneration, pendingSessionGeneration,
      pendingAdapterGeneration, pendingRevision, liveSessionGeneration, liveAdapterGeneration)
      THEN pendingRevision
      ELSE lastSourceRevision)
    /\ mutableValue'
      := (IF acceptsPendingMutant(currentAdapterGeneration, pendingSessionGeneration,
      pendingAdapterGeneration, pendingRevision, liveSessionGeneration, liveAdapterGeneration)
      THEN pendingValue
      ELSE mutableValue)
    /\ currentAdapterGeneration' := currentAdapterGeneration
    /\ pendingSessionGeneration' := pendingSessionGeneration
    /\ pendingAdapterGeneration' := pendingAdapterGeneration
    /\ pendingRevision' := pendingRevision
    /\ pendingValue' := pendingValue

(*
  @type: ((Int, Int, Int, Int, Int, Int, Int) => Bool);
*)
acceptsPending(authenticatedAdapterGen_80, sessionGen_80, adapterGen_80, revision_80,
previousSessionGen_80, previousAdapterGen_80, previousRevision_80) ==
  adapterGen_80 = authenticatedAdapterGen_80
    /\ (sessionGen_80 > previousSessionGen_80
      \/ (sessionGen_80 = previousSessionGen_80
        /\ sourceAfter(adapterGen_80, revision_80, previousAdapterGen_80, previousRevision_80)))

(*
  @type: (() => Bool);
*)
applyPending ==
  phase = "pending"
    /\ phase' := "idle"
    /\ liveSessionGeneration'
      := (IF acceptsPending(currentAdapterGeneration, pendingSessionGeneration, pendingAdapterGeneration,
      pendingRevision, liveSessionGeneration, liveAdapterGeneration, lastSourceRevision)
      THEN pendingSessionGeneration
      ELSE liveSessionGeneration)
    /\ liveAdapterGeneration'
      := (IF acceptsPending(currentAdapterGeneration, pendingSessionGeneration, pendingAdapterGeneration,
      pendingRevision, liveSessionGeneration, liveAdapterGeneration, lastSourceRevision)
      THEN pendingAdapterGeneration
      ELSE liveAdapterGeneration)
    /\ lastSourceRevision'
      := (IF acceptsPending(currentAdapterGeneration, pendingSessionGeneration, pendingAdapterGeneration,
      pendingRevision, liveSessionGeneration, liveAdapterGeneration, lastSourceRevision)
      THEN pendingRevision
      ELSE lastSourceRevision)
    /\ mutableValue'
      := (IF acceptsPending(currentAdapterGeneration, pendingSessionGeneration, pendingAdapterGeneration,
      pendingRevision, liveSessionGeneration, liveAdapterGeneration, lastSourceRevision)
      THEN pendingValue
      ELSE mutableValue)
    /\ currentAdapterGeneration' := currentAdapterGeneration
    /\ pendingSessionGeneration' := pendingSessionGeneration
    /\ pendingAdapterGeneration' := pendingAdapterGeneration
    /\ pendingRevision' := pendingRevision
    /\ pendingValue' := pendingValue

(*
  @type: (() => Bool);
*)
mutantStep ==
  (\E sessionGen \in SESSION_GENERATIONS:
      \E adapterGen \in ADAPTER_GENERATIONS:
        \E revision \in REVISIONS:
          \E value \in VALUES:
            arriveReport(sessionGen, adapterGen, revision, value))
    \/ (\E adapterGen \in ADAPTER_GENERATIONS: replaceAdapter(adapterGen))
    \/ applyPendingMutant

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
step ==
  (\E sessionGen \in SESSION_GENERATIONS:
      \E adapterGen \in ADAPTER_GENERATIONS:
        \E revision \in REVISIONS:
          \E value \in VALUES:
            arriveReport(sessionGen, adapterGen, revision, value))
    \/ (\E adapterGen \in ADAPTER_GENERATIONS: replaceAdapter(adapterGen))
    \/ applyPending

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================

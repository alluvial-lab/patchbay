// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile session_generation.qnt --target tlaplus
// Source: session_generation.qnt. Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

-------------------------- MODULE session_generation --------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(Str));
*)
DEPLOY_SCOPES == {"d1"}

(*
  @type: (() => Set(Str));
*)
RUNTIME_IDS == {"r1"}

(*
  @type: (() => Set(Str));
*)
LABELS == { "proj-A", "proj-B" }

(*
  @type: (() => Set(Int));
*)
GENERATIONS == 0 .. 3

(*
  @type: (() => Set(Str));
*)
EVENT_KINDS == { "report", "late", "relabel" }

(*
  @type: (() => Set(Str));
*)
SESSION_IDS == { "s1", "s2" }

(*
  @type: (() => Set(<<Str, Int>>));
*)
TOMBSTONE_KEYS ==
  { <<"s1", 0>>,
    <<"s1", 1>>,
    <<"s1", 2>>,
    <<"s1", 3>>,
    <<"s2", 0>>,
    <<"s2", 1>>,
    <<"s2", 2>>,
    <<"s2", 3>> }

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  generation

VARIABLE
  (*
    @type: (<<Str, Int>> -> Bool);
  *)
  tombstoned

VARIABLE
  (*
    @type: (<<Str, Int>> -> Int);
  *)
  tombstoneLsn

(*
  @type: (() => Set(Str));
*)
ADAPTER_IDS == {"a1"}

VARIABLE
  (*
    @type: Int;
  *)
  lsn

VARIABLE
  (*
    @type: (Str -> Str);
  *)
  label

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  identityGeneration

VARIABLE
  (*
    @type: Str;
  *)
  attemptedKind

VARIABLE
  (*
    @type: Str;
  *)
  attemptedSid

VARIABLE
  (*
    @type: Int;
  *)
  attemptedGen

(*
  @type: (() => Bool);
*)
init ==
  generation = [ s_88 \in SESSION_IDS |-> 0 ]
    /\ tombstoned = [ k_95 \in TOMBSTONE_KEYS |-> FALSE ]
    /\ tombstoneLsn = [ k_102 \in TOMBSTONE_KEYS |-> 0 ]
    /\ lsn = 0
    /\ label = [ s_112 \in SESSION_IDS |-> "proj-A" ]
    /\ identityGeneration = [ s_119 \in SESSION_IDS |-> 0 ]
    /\ attemptedKind = "relabel"
    /\ attemptedSid = "s1"
    /\ attemptedGen = 0

(*
  @type: (() => Bool);
*)
session_identity_tuple ==
  \A sid_310 \in SESSION_IDS:
    sid_310 \in DOMAIN generation
      /\ sid_310 \in DOMAIN identityGeneration
      /\ identityGeneration[sid_310] = generation[sid_310]
      /\ generation[sid_310] \in GENERATIONS
      /\ Cardinality((ADAPTER_IDS)) = 1
      /\ Cardinality((DEPLOY_SCOPES)) = 1
      /\ Cardinality((RUNTIME_IDS)) = 1

(*
  @type: (() => Bool);
*)
generation_monotonic ==
  [](\A sid_323 \in SESSION_IDS: generation[sid_323]' >= generation[sid_323])

(*
  @type: (() => Bool);
*)
late_generation_inert ==
  [](\A sid_363 \in SESSION_IDS:
    (attemptedKind' = "late" /\ attemptedSid' = sid_363)
      /\ tombstoned[<<sid_363, attemptedGen'>>]
      => generation[sid_363]' = generation[sid_363]
        /\ identityGeneration[sid_363]' = identityGeneration[sid_363])

(*
  @type: (() => Bool);
*)
labels_cannot_override_identity ==
  \A sid_404 \in SESSION_IDS:
    sid_404 \in DOMAIN label
      /\ label[sid_404] \in LABELS
      /\ ~(label[sid_404] \in ADAPTER_IDS)
      /\ ~(label[sid_404] \in DEPLOY_SCOPES)
      /\ ~(label[sid_404] \in RUNTIME_IDS)
      /\ identityGeneration[sid_404] = generation[sid_404]

(*
  @type: (() => Bool);
*)
step ==
  \E kind \in EVENT_KINDS:
    \E sid \in SESSION_IDS:
      \E gen \in GENERATIONS:
        \E newLabel \in LABELS:
          (kind /= "late" \/ tombstoned[<<sid, gen>>])
            /\ tombstoned'
              := (IF kind = "report" /\ gen > generation[sid]
              THEN [ tombstoned EXCEPT ![<<sid, generation[sid]>>] = TRUE ]
              ELSE tombstoned)
            /\ tombstoneLsn'
              := (IF kind = "report" /\ gen > generation[sid]
              THEN [ tombstoneLsn EXCEPT ![<<sid, generation[sid]>>] = lsn + 1 ]
              ELSE tombstoneLsn)
            /\ lsn'
              := (IF kind = "report" /\ gen > generation[sid]
              THEN lsn + 1
              ELSE lsn)
            /\ generation'
              := (IF kind = "report" /\ gen > generation[sid]
              THEN [ generation EXCEPT ![sid] = gen ]
              ELSE generation)
            /\ label'
              := (IF kind = "relabel"
              THEN [ label EXCEPT ![sid] = newLabel ]
              ELSE label)
            /\ identityGeneration'
              := (IF kind = "report" /\ gen > generation[sid]
              THEN [ identityGeneration EXCEPT ![sid] = gen ]
              ELSE identityGeneration)
            /\ attemptedKind' := kind
            /\ attemptedSid' := sid
            /\ attemptedGen' := gen

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================

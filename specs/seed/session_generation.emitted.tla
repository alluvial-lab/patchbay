// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile session_generation.qnt --target tlaplus --main session_generation_promotion --out session_generation.compile.json --verbosity 0 > session_generation.emitted.tla
// Source: session_generation.qnt (main module: session_generation_promotion). Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

--------------------- MODULE session_generation_promotion ---------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

(*
  @type: (() => Set(<<Str, Int>>));
*)
TOMBSTONE_KEYS ==
  { <<"s1", 0>>,
    <<"s1", 1>>,
    <<"s1", 2>>,
    <<"s1", 3>>,
    <<"s1", 4>>,
    <<"s2", 0>>,
    <<"s2", 1>>,
    <<"s2", 2>>,
    <<"s2", 3>>,
    <<"s2", 4>> }

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  generation

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  identityGeneration

VARIABLE
  (*
    @type: (<<Str, Int>> -> Int);
  *)
  tombstoned

VARIABLE
  (*
    @type: (<<Str, Int>> -> Int);
  *)
  tombstoneLsn

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
  claimActive

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  expectedPrior

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  claimedGeneration

VARIABLE
  (*
    @type: (Int -> Str);
  *)
  externalOwner

VARIABLE
  (*
    @type: (<<Str, Int>> -> Int);
  *)
  descendantAuthority

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  completionCount

VARIABLE
  (*
    @type: Str;
  *)
  phase

VARIABLE
  (*
    @type: Str;
  *)
  attemptedSid

VARIABLE
  (*
    @type: Int;
  *)
  attemptedPrior

VARIABLE
  (*
    @type: Int;
  *)
  attemptedGeneration

VARIABLE
  (*
    @type: Int;
  *)
  beforeGeneration

VARIABLE
  (*
    @type: Int;
  *)
  beforeIdentityGeneration

VARIABLE
  (*
    @type: Int;
  *)
  beforeClaimActive

VARIABLE
  (*
    @type: Int;
  *)
  beforeExpectedPrior

VARIABLE
  (*
    @type: Int;
  *)
  beforeClaimedGeneration

VARIABLE
  (*
    @type: Str;
  *)
  beforeCandidateOwner

VARIABLE
  (*
    @type: Int;
  *)
  beforePriorTombstoned

VARIABLE
  (*
    @type: Int;
  *)
  beforePriorTombstoneLsn

VARIABLE
  (*
    @type: Int;
  *)
  beforeCandidateAuthority

VARIABLE
  (*
    @type: Int;
  *)
  beforeCompletionCount

VARIABLE
  (*
    @type: Int;
  *)
  beforeLsn

(*
  @type: (() => Set(Str));
*)
SESSION_IDS == { "s1", "s2" }

(*
  @type: (() => Set(Str));
*)
LABELS == { "proj-A", "proj-B" }

(*
  @type: (() => Set(Int));
*)
GENERATIONS == 0 .. 4

(*
  @type: (() => Int);
*)
MAX_MANAGED_GENERATION == 3

(*
  @type: (() => Str);
*)
NO_OWNER == "none"

(*
  @type: ((Str, Int) => Bool);
*)
lateEvent(sid_1071, gen_1071) ==
  phase = "idle"
    /\ tombstoned[<<sid_1071, gen_1071>>] = 1
    /\ generation' := generation
    /\ identityGeneration' := identityGeneration
    /\ tombstoned' := tombstoned
    /\ tombstoneLsn' := tombstoneLsn
    /\ lsn' := lsn
    /\ label' := label
    /\ claimActive' := claimActive
    /\ expectedPrior' := expectedPrior
    /\ claimedGeneration' := claimedGeneration
    /\ externalOwner' := externalOwner
    /\ descendantAuthority' := descendantAuthority
    /\ completionCount' := completionCount
    /\ phase' := phase
    /\ attemptedSid' := sid_1071
    /\ attemptedPrior' := gen_1071
    /\ attemptedGeneration' := gen_1071
    /\ beforeGeneration' := beforeGeneration
    /\ beforeIdentityGeneration' := beforeIdentityGeneration
    /\ beforeClaimActive' := beforeClaimActive
    /\ beforeExpectedPrior' := beforeExpectedPrior
    /\ beforeClaimedGeneration' := beforeClaimedGeneration
    /\ beforeCandidateOwner' := beforeCandidateOwner
    /\ beforePriorTombstoned' := beforePriorTombstoned
    /\ beforePriorTombstoneLsn' := beforePriorTombstoneLsn
    /\ beforeCandidateAuthority' := beforeCandidateAuthority
    /\ beforeCompletionCount' := beforeCompletionCount
    /\ beforeLsn' := beforeLsn

(*
  @type: (() => Bool);
*)
generation_monotonic ==
  [](\A sid_1338 \in SESSION_IDS: generation[sid_1338]' >= generation[sid_1338])

(*
  @type: (() => Bool);
*)
init ==
  generation = [ id__196 \in SESSION_IDS |-> 0 ]
    /\ identityGeneration = [ id__203 \in SESSION_IDS |-> 0 ]
    /\ tombstoned = [ id__210 \in TOMBSTONE_KEYS |-> 0 ]
    /\ tombstoneLsn = [ id__217 \in TOMBSTONE_KEYS |-> 0 ]
    /\ lsn = 0
    /\ label = [ id__227 \in SESSION_IDS |-> "proj-A" ]
    /\ claimActive = [ id__234 \in SESSION_IDS |-> 0 ]
    /\ expectedPrior = [ id__241 \in SESSION_IDS |-> 0 ]
    /\ claimedGeneration = [ id__248 \in SESSION_IDS |-> 0 ]
    /\ externalOwner = [ id__255 \in GENERATIONS |-> NO_OWNER ]
    /\ descendantAuthority = [ id__262 \in TOMBSTONE_KEYS |-> 0 ]
    /\ completionCount = [ id__269 \in SESSION_IDS |-> 0 ]
    /\ phase = "idle"
    /\ attemptedSid = "s1"
    /\ attemptedPrior = 0
    /\ attemptedGeneration = 0
    /\ beforeGeneration = 0
    /\ beforeIdentityGeneration = 0
    /\ beforeClaimActive = 0
    /\ beforeExpectedPrior = 0
    /\ beforeClaimedGeneration = 0
    /\ beforeCandidateOwner = NO_OWNER
    /\ beforePriorTombstoned = 0
    /\ beforePriorTombstoneLsn = 0
    /\ beforeCandidateAuthority = 0
    /\ beforeCompletionCount = 0
    /\ beforeLsn = 0

(*
  @type: ((Str) => Int);
*)
nextCandidate(sid_326) == generation[sid_326] + 1

(*
  @type: ((Str, Int, Int) => Bool);
*)
arrivePromotion(sid_584, prior_584, candidate_584) ==
  phase = "idle"
    /\ generation' := generation
    /\ identityGeneration' := identityGeneration
    /\ tombstoned' := tombstoned
    /\ tombstoneLsn' := tombstoneLsn
    /\ lsn' := lsn
    /\ label' := label
    /\ claimActive' := claimActive
    /\ expectedPrior' := expectedPrior
    /\ claimedGeneration' := claimedGeneration
    /\ externalOwner' := externalOwner
    /\ descendantAuthority' := descendantAuthority
    /\ completionCount' := completionCount
    /\ phase' := "pending"
    /\ attemptedSid' := sid_584
    /\ attemptedPrior' := prior_584
    /\ attemptedGeneration' := candidate_584
    /\ beforeGeneration' := generation[sid_584]
    /\ beforeIdentityGeneration' := identityGeneration[sid_584]
    /\ beforeClaimActive' := claimActive[sid_584]
    /\ beforeExpectedPrior' := expectedPrior[sid_584]
    /\ beforeClaimedGeneration' := claimedGeneration[sid_584]
    /\ beforeCandidateOwner' := externalOwner[candidate_584]
    /\ beforePriorTombstoned' := tombstoned[<<sid_584, prior_584>>]
    /\ beforePriorTombstoneLsn' := tombstoneLsn[<<sid_584, prior_584>>]
    /\ beforeCandidateAuthority'
      := descendantAuthority[<<sid_584, candidate_584>>]
    /\ beforeCompletionCount' := completionCount[sid_584]
    /\ beforeLsn' := lsn

(*
  @type: (() => Bool);
*)
foldGuard ==
  ((((((phase = "pending" /\ claimActive[attemptedSid] = 1)
              /\ generation[attemptedSid] = attemptedPrior)
            /\ expectedPrior[attemptedSid] = attemptedPrior)
          /\ claimedGeneration[attemptedSid] = attemptedGeneration)
        /\ externalOwner[attemptedGeneration] = attemptedSid)
      /\ descendantAuthority[<<attemptedSid, attemptedGeneration>>] = 0)
    /\ ((attemptedPrior = 0 /\ attemptedGeneration = 1)
      \/ (attemptedPrior > 0 /\ attemptedGeneration = attemptedPrior + 1))

(*
  @type: (() => Set(Str));
*)
OWNERS == { (NO_OWNER), "s1", "s2" }

(*
  @type: (() => Bool);
*)
clearAttempt ==
  phase = "folded"
    /\ generation' := generation
    /\ identityGeneration' := identityGeneration
    /\ tombstoned' := tombstoned
    /\ tombstoneLsn' := tombstoneLsn
    /\ lsn' := lsn
    /\ label' := label
    /\ claimActive' := claimActive
    /\ expectedPrior' := expectedPrior
    /\ claimedGeneration' := claimedGeneration
    /\ externalOwner' := externalOwner
    /\ descendantAuthority' := descendantAuthority
    /\ completionCount' := completionCount
    /\ phase' := "idle"
    /\ attemptedSid' := attemptedSid
    /\ attemptedPrior' := attemptedPrior
    /\ attemptedGeneration' := attemptedGeneration
    /\ beforeGeneration' := beforeGeneration
    /\ beforeIdentityGeneration' := beforeIdentityGeneration
    /\ beforeClaimActive' := beforeClaimActive
    /\ beforeExpectedPrior' := beforeExpectedPrior
    /\ beforeClaimedGeneration' := beforeClaimedGeneration
    /\ beforeCandidateOwner' := beforeCandidateOwner
    /\ beforePriorTombstoned' := beforePriorTombstoned
    /\ beforePriorTombstoneLsn' := beforePriorTombstoneLsn
    /\ beforeCandidateAuthority' := beforeCandidateAuthority
    /\ beforeCompletionCount' := beforeCompletionCount
    /\ beforeLsn' := beforeLsn

(*
  @type: ((Str, Str) => Bool);
*)
relabel(sid_975, newLabel_975) ==
  phase = "idle"
    /\ generation' := generation
    /\ identityGeneration' := identityGeneration
    /\ tombstoned' := tombstoned
    /\ tombstoneLsn' := tombstoneLsn
    /\ lsn' := lsn
    /\ label' := [ label EXCEPT ![sid_975] = newLabel_975 ]
    /\ claimActive' := claimActive
    /\ expectedPrior' := expectedPrior
    /\ claimedGeneration' := claimedGeneration
    /\ externalOwner' := externalOwner
    /\ descendantAuthority' := descendantAuthority
    /\ completionCount' := completionCount
    /\ phase' := phase
    /\ attemptedSid' := attemptedSid
    /\ attemptedPrior' := attemptedPrior
    /\ attemptedGeneration' := attemptedGeneration
    /\ beforeGeneration' := beforeGeneration
    /\ beforeIdentityGeneration' := beforeIdentityGeneration
    /\ beforeClaimActive' := beforeClaimActive
    /\ beforeExpectedPrior' := beforeExpectedPrior
    /\ beforeClaimedGeneration' := beforeClaimedGeneration
    /\ beforeCandidateOwner' := beforeCandidateOwner
    /\ beforePriorTombstoned' := beforePriorTombstoned
    /\ beforePriorTombstoneLsn' := beforePriorTombstoneLsn
    /\ beforeCandidateAuthority' := beforeCandidateAuthority
    /\ beforeCompletionCount' := beforeCompletionCount
    /\ beforeLsn' := beforeLsn

(*
  @type: (() => Bool);
*)
promotion_fold_exact_and_atomic ==
  phase /= "folded"
    \/ (LET (*
      @type: (() => Bool);
    *)
    oracleValid ==
      (((((beforeClaimActive = 1 /\ beforeGeneration = attemptedPrior)
                /\ beforeExpectedPrior = attemptedPrior)
              /\ beforeClaimedGeneration = attemptedGeneration)
            /\ beforeCandidateOwner = attemptedSid)
          /\ beforeCandidateAuthority = 0)
        /\ ((attemptedPrior = 0 /\ attemptedGeneration = 1)
          \/ (attemptedPrior > 0 /\ attemptedGeneration = attemptedPrior + 1))
    IN
    IF oracleValid
    THEN ((((((generation[attemptedSid] = attemptedGeneration
                  /\ identityGeneration[attemptedSid] = attemptedGeneration)
                /\ claimActive[attemptedSid] = 0)
              /\ externalOwner[attemptedGeneration] = attemptedSid)
            /\ descendantAuthority[<<attemptedSid, attemptedGeneration>>] = 1)
          /\ completionCount[attemptedSid] = beforeCompletionCount + 1)
        /\ lsn = beforeLsn + 1)
      /\ (IF attemptedPrior = 0
      THEN tombstoned[<<attemptedSid, attemptedPrior>>] = beforePriorTombstoned
        /\ tombstoneLsn[<<attemptedSid, attemptedPrior>>]
          = beforePriorTombstoneLsn
      ELSE (tombstoned[<<attemptedSid, attemptedPrior>>] = 1
          /\ tombstoneLsn[<<attemptedSid, attemptedPrior>>] = beforeLsn + 1)
        /\ externalOwner[attemptedPrior] = attemptedSid)
    ELSE (((((((((generation[attemptedSid] = beforeGeneration
                        /\ identityGeneration[attemptedSid]
                          = beforeIdentityGeneration)
                      /\ claimActive[attemptedSid] = beforeClaimActive)
                    /\ expectedPrior[attemptedSid] = beforeExpectedPrior)
                  /\ claimedGeneration[attemptedSid] = beforeClaimedGeneration)
                /\ externalOwner[attemptedGeneration] = beforeCandidateOwner)
              /\ descendantAuthority[<<attemptedSid, attemptedGeneration>>]
                = beforeCandidateAuthority)
            /\ completionCount[attemptedSid] = beforeCompletionCount)
          /\ tombstoned[<<attemptedSid, attemptedPrior>>]
            = beforePriorTombstoned)
        /\ tombstoneLsn[<<attemptedSid, attemptedPrior>>]
          = beforePriorTombstoneLsn)
      /\ lsn = beforeLsn)

(*
  @type: ((Str) => Bool);
*)
mayReserve(sid_351) ==
  ((phase = "idle" /\ claimActive[sid_351] = 0)
      /\ generation[sid_351] < MAX_MANAGED_GENERATION)
    /\ externalOwner[(nextCandidate(sid_351))] = NO_OWNER

(*
  @type: (() => Bool);
*)
foldPromotion ==
  phase = "pending"
    /\ generation'
      := (IF foldGuard
      THEN [ generation EXCEPT ![attemptedSid] = attemptedGeneration ]
      ELSE generation)
    /\ identityGeneration'
      := (IF foldGuard
      THEN [ identityGeneration EXCEPT ![attemptedSid] = attemptedGeneration ]
      ELSE identityGeneration)
    /\ tombstoned'
      := (IF foldGuard /\ attemptedPrior > 0
      THEN [ tombstoned EXCEPT ![<<attemptedSid, attemptedPrior>>] = 1 ]
      ELSE tombstoned)
    /\ tombstoneLsn'
      := (IF foldGuard /\ attemptedPrior > 0
      THEN [ tombstoneLsn EXCEPT ![<<attemptedSid, attemptedPrior>>] = lsn + 1 ]
      ELSE tombstoneLsn)
    /\ lsn' := (IF foldGuard THEN lsn + 1 ELSE lsn)
    /\ label' := label
    /\ claimActive'
      := (IF foldGuard
      THEN [ claimActive EXCEPT ![attemptedSid] = 0 ]
      ELSE claimActive)
    /\ expectedPrior' := expectedPrior
    /\ claimedGeneration' := claimedGeneration
    /\ externalOwner' := externalOwner
    /\ descendantAuthority'
      := (IF foldGuard
      THEN [
        descendantAuthority EXCEPT
          ![<<attemptedSid, attemptedGeneration>>] = 1
      ]
      ELSE descendantAuthority)
    /\ completionCount'
      := (IF foldGuard
      THEN [
        completionCount EXCEPT
          ![attemptedSid] = completionCount[attemptedSid] + 1
      ]
      ELSE completionCount)
    /\ phase' := "folded"
    /\ attemptedSid' := attemptedSid
    /\ attemptedPrior' := attemptedPrior
    /\ attemptedGeneration' := attemptedGeneration
    /\ beforeGeneration' := beforeGeneration
    /\ beforeIdentityGeneration' := beforeIdentityGeneration
    /\ beforeClaimActive' := beforeClaimActive
    /\ beforeExpectedPrior' := beforeExpectedPrior
    /\ beforeClaimedGeneration' := beforeClaimedGeneration
    /\ beforeCandidateOwner' := beforeCandidateOwner
    /\ beforePriorTombstoned' := beforePriorTombstoned
    /\ beforePriorTombstoneLsn' := beforePriorTombstoneLsn
    /\ beforeCandidateAuthority' := beforeCandidateAuthority
    /\ beforeCompletionCount' := beforeCompletionCount
    /\ beforeLsn' := beforeLsn

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: ((Str) => Bool);
*)
prepareClaim(sid_468) ==
  generation' := generation
    /\ identityGeneration' := identityGeneration
    /\ tombstoned' := tombstoned
    /\ tombstoneLsn' := tombstoneLsn
    /\ lsn' := lsn
    /\ label' := label
    /\ claimActive'
      := (IF mayReserve(sid_468)
      THEN [ claimActive EXCEPT ![sid_468] = 1 ]
      ELSE claimActive)
    /\ expectedPrior'
      := (IF mayReserve(sid_468)
      THEN [ expectedPrior EXCEPT ![sid_468] = generation[sid_468] ]
      ELSE expectedPrior)
    /\ claimedGeneration'
      := (IF mayReserve(sid_468)
      THEN [ claimedGeneration EXCEPT ![sid_468] = nextCandidate(sid_468) ]
      ELSE claimedGeneration)
    /\ externalOwner'
      := (IF mayReserve(sid_468)
      THEN [ externalOwner EXCEPT ![nextCandidate(sid_468)] = sid_468 ]
      ELSE externalOwner)
    /\ descendantAuthority' := descendantAuthority
    /\ completionCount' := completionCount
    /\ phase' := phase
    /\ attemptedSid' := attemptedSid
    /\ attemptedPrior' := attemptedPrior
    /\ attemptedGeneration' := attemptedGeneration
    /\ beforeGeneration' := beforeGeneration
    /\ beforeIdentityGeneration' := beforeIdentityGeneration
    /\ beforeClaimActive' := beforeClaimActive
    /\ beforeExpectedPrior' := beforeExpectedPrior
    /\ beforeClaimedGeneration' := beforeClaimedGeneration
    /\ beforeCandidateOwner' := beforeCandidateOwner
    /\ beforePriorTombstoned' := beforePriorTombstoned
    /\ beforePriorTombstoneLsn' := beforePriorTombstoneLsn
    /\ beforeCandidateAuthority' := beforeCandidateAuthority
    /\ beforeCompletionCount' := beforeCompletionCount
    /\ beforeLsn' := beforeLsn

(*
  @type: (() => Bool);
*)
step ==
  (\E sid \in SESSION_IDS: prepareClaim(sid))
    \/ (\E sid \in SESSION_IDS:
      \E prior \in GENERATIONS:
        \E candidate \in GENERATIONS: arrivePromotion(sid, prior, candidate))
    \/ foldPromotion
    \/ clearAttempt
    \/ (\E sid \in SESSION_IDS: \E newLabel \in LABELS: relabel(sid, newLabel))
    \/ (\E sid \in SESSION_IDS: \E gen \in GENERATIONS: lateEvent(sid, gen))

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================

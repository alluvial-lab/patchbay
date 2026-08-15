// GENERATED ARTIFACT — do not hand-edit. Regenerate via: quint compile session_generation.qnt --target tlaplus
// Source: session_generation.qnt. Inspection artifact, NOT an independent re-check lane (see feature-formal-model-seed Q4).

# Usage statistics is OFF. We care about your privacy.
# If you want to help our project, consider enabling statistics with config --enable-stats=true.

Output directory: /home/agent/projects/patchbay/specs/seed/_apalache-out/server/2026-08-15T01-30-11_6999765666455958532
# APALACHE version: 0.56.1 | build: 70cdaf4                       I@01:30:11.908
Starting checker server on port 8822...                           I@01:30:11.919
The Apalache server is running on port 8822. Press Ctrl-C to stop.
PASS #0: SanyParser                                               I@01:30:14.705
-------------------------- MODULE session_generation --------------------------

EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants

VARIABLE
  (*
    @type: (Str -> Int);
  *)
  generation

(*
  @type: (() => Set(Str));
*)
SESSION_IDS == { "s1", "s2" }

(*
  @type: (() => Set(Int));
*)
GENERATIONS == 0 .. 3

(*
  @type: (() => Bool);
*)
init == generation = [ id__16 \in SESSION_IDS |-> 0 ]

(*
  @type: (() => Bool);
*)
generation_monotonic ==
  [](\A sid_53 \in SESSION_IDS: generation[sid_53]' >= generation[sid_53])

(*
  @type: (() => Bool);
*)
step ==
  \E sid \in SESSION_IDS:
    \E candidate \in GENERATIONS:
      generation'
        := (IF candidate > generation[sid]
        THEN [ generation EXCEPT ![sid] = candidate ]
        ELSE generation)

(*
  @type: (() => Bool);
*)
q_init == init

(*
  @type: (() => Bool);
*)
q_step == step

================================================================================

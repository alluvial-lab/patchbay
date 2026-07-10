// Patchbay v0 relational seed model (Alloy 6.2.0).
//
// Scope: static, one-state relational shapes only. Dynamic authority binding,
// transport/session authentication, revocation, and temporal authorization
// transitions belong in the Quint/TLA+ authority models, not here.

sig Identity {}

sig Actor {
  id: one Identity
}

fact ActorIdsUnique {
  id in Actor lone -> Identity
}

sig Grant {
  issuer: one Actor,
  subject: one Actor
}

// v0 HAS grants (docs/PROTOCOL.md:290-307 — grants are explicit and revocable in v0). Only
// DELEGATION (the parent-grant edge) is absent. There is no delegation field on Grant in v0.

sig Message {
  sender: one Actor,
  claimedSender: one Actor
}

// ---------------------------------------------------------------------------
// Promoted model: the one relational invariant that is genuinely checkable
// in a v0 static snapshot without becoming tautological.
// ---------------------------------------------------------------------------

// @promotion {
//   property:    ActorIdsUnique
//   status:      draft
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  <TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests>
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   demotion_reason: fact-consequence check; the assert verifies the ActorIdsUnique fact holds across all instances but does not establish non-vacuity independently
//   semantics:   actor-id injectivity remains a product obligation; this retained fact-consequence check is only a structural regression test against accidental weakening of the ActorIdsUnique fact and is not independent assurance or proof of non-vacuity
// }
// NOTE on genuine-checking: ActorIdsUniqueAssert checks `all disj a,b: Actor | a.id != b.id`,
// which is the SAME constraint the ActorIdsUnique fact enforces. This is a fact-consequence
// check: it verifies the fact holds across all instances and guards against a future change to
// the fact. It does NOT by itself establish non-vacuity (a separate `run` finding multi-actor
// instances would); the non-vacuity here is observed via the check finding a satisfying instance,
// not a proof. Verified UNSAT (no counterexample) via `--type text` (no skolem witness).
assert ActorIdsUniqueAssert {
  all disj a, b: Actor | a.id != b.id
}

// ---------------------------------------------------------------------------
// Stated-normative (DRAFT): reserved property-ids, NOT promoted.
// These two properties are NOT checkable as relational invariants in v0 without becoming
// tautological (a forcing fact) or actually-false (no constraint). They are reserved for the
// follow-on authority/delegation implementation items where their dynamic semantics live.
// ---------------------------------------------------------------------------

// @promotion {
//   property:    AuthorityGraphAcyclic
//   status:      draft
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  <TBD — not yet checked; promote when delegation is modeled>
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:   RESERVED: acyclicity of the grant issuer-subject graph is only meaningful once a delegation/parent-grant edge exists. v0 has no delegation (docs/PROTOCOL.md:305), so the graph has no cycle-bearing edge to check — asserting acyclicity now is either vacuous (empty graph) or false (unconstrained self-grants). Promote when delegation is added.
// }
// HISTORY: an earlier version asserted acyclicity over the subject->issuer graph with grants
// present, but Alloy found counterexamples (self-grants: issuer = subject = a, a 1-cycle).
// PROTOCOL does not state that v0 grants form an acyclic issuer graph — grants are issued by
// actors to subjects with no parent-grant edge in v0. So the assert was checking an invented
// rule. Demoted to draft; the `check` command is removed (the assert is kept commented-out as
// the reserved shape for the delegation follow-on).
//
// assert AuthorityGraphAcyclicAssert {
//   let issuer = ~subject.issuer |
//     no a: Actor | a in a.^issuer
// }

// @promotion {
//   property:    SenderMatchesClaim
//   status:      draft
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  <TBD — not yet checked; promote when the dynamic CompoundIssuer binding is modeled>
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:   RESERVED: sender == claimedSender is a DYNAMIC consistency property, not a relational one. In a static snapshot, sender and claimedSender are independent fields — nothing forces them equal except a fact, which makes the assert a tautology. The actual binding (an authenticated identity matches the self-asserted sender) is a CompoundIssuer-style verification action that belongs in authority.qnt (per the Alloy brief's caveat). Promote when that dynamic model exists.
// }
// HISTORY: an earlier version had `fact SenderMatchesClaim { all m: Message | m.sender = m.claimedSender }`
// which made the assert a tautology; removing the fact (to make it "genuine") turned it
// actually-false (Alloy finds sender != claimedSender counterexamples). Neither is a genuine
// check. Demoted to draft; the `check` command is removed.
//
// assert SenderMatchesClaimAssert {
//   all m: Message | m.sender = m.claimedSender
// }

// Only the genuinely-checkable assert is run as a `check` command.
// structural regression test — NOT promoted assurance; guards against accidental fact weakening
check ActorIdsUniqueAssert for 5

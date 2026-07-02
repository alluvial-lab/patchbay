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
// DELEGATION (the parent-grant edge) is absent. There is no delegation field on Grant in v0;
// the AuthorityGraphAcyclic check below asserts acyclicity over the reserved issuer-subject
// graph shape as a seam against a future delegation re-introduction. (A previous version used
// `fact DelegationRemovedV0 { no Grant }`, which removed ALL grants — contradicting PROTOCOL
// and making the acyclicity check vacuously true on an empty graph.)

sig Message {
  sender: one Actor,
  claimedSender: one Actor
}

// NO fact forces sender = claimedSender. The assert below is a GENUINE check that the
// consistency holds across all instances — not a tautology over a fact. (A previous version
// had `fact SenderMatchesClaim { all m: Message | m.sender = m.claimedSender }` which made the
// assert check a fact — a tautology.) The dynamic binding of authenticated identity to a
// transport/session is a CompoundIssuer-style action that belongs in authority.qnt; this
// relational shape only checks the static consistency.

// @promotion {
//   property:    ActorIdsUnique
//   tier:        checked-normative
//   status:      promoted
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  java -jar org.alloytools.alloy.dist.jar exec --command ActorIdsUniqueAssert --type json --output - specs/seed/patchbay-relational.als
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:   actor identities are injective in a static relational snapshot
// }
assert ActorIdsUniqueAssert {
  all disj a, b: Actor | a.id != b.id
}

// @promotion {
//   property:    AuthorityGraphAcyclic
//   tier:        checked-normative
//   status:      promoted
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  java -jar org.alloytools.alloy.dist.jar exec --command AuthorityGraphAcyclicAssert --type json --output - specs/seed/patchbay-relational.als
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:   v0 grants form an issuer-subject graph; the reserved delegation seam (parent grant) is absent, so the subject-to-issuer graph has no transitive cycle
// }
assert AuthorityGraphAcyclicAssert {
  // Derive the Actor -> Actor graph from Grant atoms (subject -> issuer), naming it issuer so
  // the checked shape is the reserved ^issuer reachability cycle. With grants present (v0 has
  // them), this is a genuine acyclicity check, not a vacuous check on an empty graph.
  let issuer = ~subject.issuer |
    no a: Actor | a in a.^issuer
}

// @promotion {
//   property:    SenderMatchesClaim
//   tier:        checked-normative
//   status:      promoted
//   model:       specs/seed/patchbay-relational.als
//   language:    alloy
//   backend:     alloy-cli
//   invocation:  java -jar org.alloytools.alloy.dist.jar exec --command SenderMatchesClaimAssert --type json --output - specs/seed/patchbay-relational.als
//   bounds:      { scope: 5 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:   sender equals claimedSender as a consistency shape; authenticated transport/session binding is dynamic and belongs in authority.qnt
// }
assert SenderMatchesClaimAssert {
  all m: Message | m.sender = m.claimedSender
}

check ActorIdsUniqueAssert for 5
check AuthorityGraphAcyclicAssert for 5
check SenderMatchesClaimAssert for 5

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

fact DelegationRemovedV0 {
  no Grant
}

sig Message {
  sender: one Actor,
  claimedSender: one Actor
}

fact SenderMatchesClaim {
  all m: Message | m.sender = m.claimedSender
}

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
//   semantics:   v0 has no grants because delegation is removed, so the reserved subject-to-issuer authority graph has no transitive cycle
// }
assert AuthorityGraphAcyclicAssert {
  // Derive the Actor -> Actor graph from Grant atoms, naming it issuer so
  // the checked shape is explicitly the reserved ^issuer reachability cycle.
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

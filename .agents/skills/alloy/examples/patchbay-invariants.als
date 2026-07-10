// Superseded by specs/seed/patchbay-relational.als, which keeps this
// ActorIdsUnique hello-world shape and adds the authority-graph acyclicity
// and sender/claimed-sender anti-spoofing consistency shapes.

sig Identity {}
sig Actor { id: one Identity }

fact ActorIdsUnique {
  id in Actor lone -> Identity
}

assert ActorIdsUniqueAssert {
  all disj a, b: Actor | a.id != b.id
}

check ActorIdsUniqueAssert for 5

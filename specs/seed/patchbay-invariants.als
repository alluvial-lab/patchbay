sig Identity {}
sig Actor { id: one Identity }

fact ActorIdsUnique {
  id in Actor lone -> Identity
}

assert ActorIdsUniqueAssert {
  all disj a, b: Actor | a.id != b.id
}

check ActorIdsUniqueAssert for 5

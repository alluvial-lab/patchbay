---- MODULE Counter ----
EXTENDS Naturals

VARIABLE x
vars == <<x>>

Init == x = 0
Inc == /\ x < 3
       /\ x' = x + 1
Stay == /\ x = 3
        /\ UNCHANGED x
Next == Inc \/ Stay

Spec == /\ Init
        /\ [][Next]_vars
        /\ WF_vars(Inc)

TypeOK == x \in 0..3
EventuallyThree == <>[](x = 3)
====

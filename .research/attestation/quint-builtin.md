---
source_handle: quint-builtin
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/quint-co/quint/main/docs/content/docs/builtin.md
provenance: source-direct
---

# Attestation: Quint builtin operators documentation

## Structural metadata

- Source kind: generated official builtin operator documentation (`docs/content/docs/builtin.md`).
- Local fetched copy: `.research/reference/quint/builtin.md`.
- Sections used: set membership, set/map constructors and updates, temporal helpers, fairness, `leadsTo`.

## Paraphrased summary

The builtin documentation lists core operators and examples for sets, maps, numeric ranges, temporal helpers, and fairness. It includes membership operations (`in`, `contains`), map constructors and update operations (`Map`, `mapBy`, `set`, `setBy`, `put`), finite integer ranges via `to`, and temporal examples using `orKeep`, `mustChange`, `next`, fairness, and `leadsTo`.

## Key passages

### {1} set membership

The `in` section gives signature `pure def in: (a, Set[a]) => bool`, says `e.in(s)` is true when `e` is in set `s`, and shows:

```quint
assert(1.in(Set(1, 2, 3)))
assert(not(4.in(Set(1, 2, 3))))
```

Anchor: lines 119-131.

### {2} set contains

The `contains` section gives signature `pure def contains: (Set[a], a) => bool`, says `s.contains(e)` is true when element `e` is in set `s`, and shows examples with `Set(1, 2, 3).contains(1)` and `not(...contains(4))`.

Anchor: lines 134-148.

### {3} maps and keys

The `get` section gives signature `pure def get: ((a -> b), a) => b`, says `m.get(k)` is the value for `k`, and warns behavior is undefined if `k` is not in `m`. The `keys` section gives signature `pure def keys: ((a -> b)) => Set[a]` and says `m.keys()` returns the set of keys.

Anchor: lines 375-399.

### {4} mapBy

The `mapBy` section gives signature `pure def mapBy: (Set[a], (a) => b) => (a -> b)` and says `s.mapBy(f)` is the map from each `x` in `s` to `f(x)`. Its example maps `Set(1, 2, 3)` to squares.

Anchor: lines 401-412.

### {5} map update operations

The `set` section gives signature `pure def set: ((a -> b), a, b) => (a -> b)` and says `m.set(k, v)` updates an existing key and is undefined if `k` is not already a key. The `setBy` section says `m.setBy(k, f)` keeps the same keys and updates `k` to `f(m.get(k))`. The `put` section says `m.put(k, v)` returns `m` with key `k` mapped to `v`; examples include adding key `3`.

Anchor: lines 448-495.

### {6} integer ranges

The `to` operator example says:

```quint
assert(1.to(3) == Set(1, 2, 3))
```

Anchor: lines 768-770.

### {7} temporal `orKeep` example

The documentation shows:

```quint
action Init = x' = 0
action Next =  x' = x + 1
temporal Spec = Init and always(Next.orKeep(Set(x)))
```

Anchor: lines 835-837.

### {8} temporal `mustChange` and `next` example

The documentation shows a temporal property using `mustChange` and `next`:

```quint
temporal Spec = Init and always(Next.mustChange(Set(x)))
temporal Property = Spec.implies(always(next(x) > x))
```

Anchor: lines 858-861.

### {9} weak fairness and eventuality example

The documentation shows:

```quint
temporal Property = Next.weakFair(Set(x)).implies(eventually(x == 10))
```

Anchor: lines 926-928.

### {10} leadsTo operator example

The documentation shows a `leadsTo` temporal property:

```quint
temporal Property = NODES.forall(i => not(active.get(i))) leadsTo terminationDetected
```

Anchor: lines 944-947.

### {11} set size

The `size` section gives an example:

```quint
assert(Set(1, 2, 3).size() == 3)
```

Anchor: lines 368-370.

### {12} set union

The `union` section shows:

```quint
assert(Set(1, 2, 3).union(Set(2, 3, 4)) == Set(1, 2, 3, 4))
```

Anchor: lines 158-162.

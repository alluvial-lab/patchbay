---
source_handle: proptest
fetched: 2026-07-07
source_url: https://proptest-rs.github.io/proptest/intro.html
provenance: source-direct
---

# Attestation: Proptest guide

## Summary

The Proptest guide describes Proptest as a property-testing framework inspired by Hypothesis, with arbitrary input generation, automatic shrinking to minimal failing inputs, flexible per-value strategy/shrinking composition, passive-maintenance status, and state-machine testing as a guide topic.

## Key passages

1. From "Introduction":

> Proptest is a property testing framework (i.e., the QuickCheck family) inspired by the Hypothesis framework for Python.

2. From "Introduction":

> It allows to test that certain properties of your code hold for arbitrary inputs, and if a failure is found, automatically finds the minimal test case to reproduce the problem.

3. From "Introduction":

> Unlike QuickCheck, generation and shrinking is defined on a per-value basis instead of per-type, which makes it more flexible and simplifies composition.

4. From "Status of this crate":

> The crate is fairly close to being feature-complete and has not seen substantial architectural changes in quite some time. At this point, it mainly sees passive maintenance.

5. From "What is property testing?":

> Property testing is a system of testing code by checking that certain properties of its output or behaviour are fulfilled for all inputs.

6. From "What is property testing?":

> These inputs are generated automatically, and, critically, when a failing input is found, the input is automatically reduced to a minimal test case.

7. From "What is property testing?":

> Property testing is best used to complement traditional unit testing (i.e., using specific inputs chosen by hand).

8. From the table of contents:

> State Machine testing

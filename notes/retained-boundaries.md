# Retained boundaries

Moved here from `ROADMAP.md` on 2026-11-21, unchanged but for links.

These boundaries trace existing domains and premises; they do not create new protocol choices.
See [Rust boundaries](../rust/GAPS.md), [laws](../protocol/laws.md), and [vectors](../vectors/README.md).

- `Addressed`, `Distinct`, `Forms`, `EnvOnly`, and `Acc.comm` remain caller obligations where laws require them.
- Raw Count and Sum retain repeats; unchecked fold state and insertion retain their wider domains.
- Spec admission examines raw members before discarding accepted unused metadata; refusal precedence remains significant.
- Source routing, store retrieval, and external services are outside the pure evaluator.
- Reader IDs use bounded 32-byte digests; writer links remain text until validation.
  This boundary does not prove SHA correctness or the complete text-to-hash bridge.
- Raw integers and exact output sums remain unbounded; binary64 normalization applies to numeric input.
- Raw duplicate object names use stable UTF-16 sorting and first-equal-key rendering; validators inspect all raw members.
- Parsed time types do not expose every raw Lean moment; typed references use ASCII rules distinct from fixture digest decoding.
- Scenario checks cover supplied arrivals, not all permutations.
- Raw lens laws require their validity and success premises, including distinct base IDs and matching ordered IDs where stated.
  The public reader checks distinct base IDs; it does not check that base IDs are nonempty.
- Batch meaning uniqueness requires acyclicity and meaning premises; it does not establish resolver correctness.
- Raw extrema fuzz compares held inputs with readable safe-whole `n`.
  This declared scope is not a bug; broader inputs require a different fixture contract.

Future public adapters can validate premises or expose raw operations with caller obligations.
Validation costs explicit error design; raw access demands greater caller responsibility.
Recommend validation at future public adapters, while keeping the existing raw core and premises.
Do not implicitly normalize the core.

Review corrections narrow proof scope and mark examples as prototype or proposed.
They also correct privacy, index, hash, signature-prefix, and obsolete Rust claims.
These mechanical corrections do not add protocol choices.

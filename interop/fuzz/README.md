# Differential fuzz checks

Node 22 and the repo's Lean toolchain run the two-way checks without package dependencies.
The harness compares the public JS APIs with `Gen.Run` answers.
The explicit `--rust` switch also checks the public Rust evaluator.
Rust mode needs Cargo and the workspace dependencies.
A decoded vector is not proof of a valid wire record.

## Run

From the repo root:

```sh
node --test interop/fuzz/phase2.test.mjs interop/fuzz/step1.test.mjs  # Mock and unit checks only
node --test interop/fuzz/*.test.mjs      # Includes real Lean checks
node interop/fuzz/run.mjs --seed ci-1 --n 28 --out /tmp/fuzz-ci
node interop/fuzz/run.mjs --rust --seed phase2-20261029 --n 5000 --out /tmp/fuzz-large
node interop/fuzz/run.mjs --seed campaign-1 --n 1000 --family record --no-shrink
node interop/fuzz/run.mjs --rust --replay /tmp/fuzz-large/mismatches/000000.json --out /tmp/fuzz-replay
node interop/fuzz/run.mjs --rust --seed issue-7-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-issue-7-1
node interop/fuzz/run.mjs --rust --replay vectors/lens/section/d1-duplicate-base-id.json --out /tmp/fuzz-duplicate-base
```

An explicit output directory must be empty.
Without `--out`, the runner creates an artifact directory under the system temporary directory.
It prints that path with the summary.
Exit codes are 0 for agreement, 1 for disagreements, and 2 for infrastructure or CLI errors.
[REPORT.md](REPORT.md) records the campaign over the generator after issue #7, which finds no mismatch.
Earlier campaigns' counts are historical: the generator changed, so the same seeds now generate different inputs.
It marks earlier campaign results as historical, including the 42 duplicate-base lens mismatches D1 settles.

`--n` is the total case count, not a count per family.
Without `--family`, cases cycle through record, fold, spec, realm, scenario, batch, and lens.
Each family gets either floor(N/7) or ceil(N/7) cases.
A case uses its seed, family, and family-local index for its own PRNG stream.
Family selection and chunk size do not change that case.
The generator uses FNV-1a seed mixing and xorshift32, not `Math.random`.

| Option | Default | Meaning |
|---|---|---|
| `--seed S` | `1` | Seed text |
| `--n N` | `70` | Total cases |
| `--family F` | all seven | One family |
| `--out DIR` | new temporary directory | Run artifacts |
| `--replay FILE` | none | One input vector or mismatch artifact |
| `--rust` | off | Add Rust; require reduction for generated disagreements |
| `--chunk-size N` | `64` | Cases per engine call; maximum 256 |
| `--timeout-ms N` | `120000` | Timeout per engine call |
| `--shrink-trials N` | `80` | Maximum trials per mismatch |
| `--no-shrink` | off | Keep raw mismatches without reduction; two-way mode only |

The runner builds `run` once before its timed loop.
Rust mode also runs `cargo build -p loops-eval --bin loops-eval` once from `rust/`.
It starts one process per engine per chunk, not one per case.
Both engines read the same chunk files.
The large three-way command shrinks every generated disagreement through both engines.
It does not select only some failures for later replay.
The runner rejects `--rust --no-shrink`.
A trial bound can stop each reduction before the search finds a smaller case.
Record signing can cost more than the reader families.
Do not run engine checks or campaigns during source mutation checks.
The Step-1 command above checks 5,600 cases on Node 22.
See [REPORT.md](REPORT.md) for counts and limits.

### Batch interface

Lean receives `lake --quiet exe run --batch DIR` from `model/`.
Rust receives `rust/target/debug/loops-eval --batch DIR` from the repo root.
Each engine discovers `.json` files recursively and processes their sorted paths.
Each output line contains `{file, expect}` or `{file, error}`.
Each engine computes its answer from input alone, without a fixture `expect` lookup.
The shared transport matches responses by path, not by output order.
It rejects missing, duplicate, unexpected, or malformed responses.
Each process has a timeout and a 64 MiB output bound.
Build, process, signal, timeout, and IO failures stop the run.
Errors name the engine, or both engines for shared input IO.

`batchVectors(vectors, options)` returns Lean response wrappers in two-way mode.
With `options.rust`, each result contains `{lean, rust}` response wrappers.
`assess(vector, response, options)` accepts either result shape.
The transport writes each input once and uses one parser for both engines.
The `spawn` option injects a process mock for transport tests.
Campaign tests inject `batch`, `buildLean`, `buildRust`, and `js`; they start no real engine.

## Input generation

`generate(seed, index, family)` returns `{layer, input, fuzz}`.
The `fuzz` member gives the seed, family, local index, and named near-miss boundary.
`FAMILIES` lists the seven families.
Fold indices cycle through all eight supported sublayers.
Spec indices alternate between parse and run.

Inputs contain new small records, payloads, drafts, documents, and steps.
Record IDs encode 32 generated bytes as 52 lowercase base32hex characters after `r:`.
The final padding bits are zero.
Reader IDs name logical records; the harness does not claim that those IDs hash the supplied reader fields.
The record and batch writers compute actual commitments and record IDs.

Records favor shared things, chains, forks, missing payloads, erasers, standing references, and tied times.
A record names its thing in `of`: a root leaves it out, others name a natural key or an earlier record's thing.
Most revisions state their target's thing; a few name another, or leave `of` out, which admission refuses.
`about` only mentions things; specs choose by `from.of`, `from.about`, both, or neither.
Every spec has count, sum, min and max slots that follow `prev`, beside the ones that do not.
Two slots group by `rank`, which a spec run's held payloads carry as text, as a number, or as number text such as `1.0` or `10e-1`; a whole number shares its text's group.
`{"$number": "1.0"}` in a vector stands for that spelling, and the input file writes it raw; JS reads the orders through that text.
Arrival orders retain the same logical records.
Realm forms retain the same envelope and held payload for each ID.
Record variants reorder JSON members and repeat set members.
Named record boundaries test uppercase IDs, each character `w` through `z`, and nonzero ID and salt padding bits.
They test offsets, second 60, trailing-zero fractions, and ten-digit fractions in `emitted` and every `occurred` instant position.
Another case tests an unknown envelope key with an empty kind.
Five more carry the D8 `of` boundaries: an untyped `of`, a `prev` without `of`, and its order against `about`, `prev` and `erases`.
Twelve more carry the D2, D3 and D5 boundaries.
A lone surrogate travels as a payload text, alone, in a member name, after an unknown key, and before a bad kind or salt.
A number past binary64's range travels as text too, alone, under a bad kind, and beside a repeated member.
Four cases list variants that disagree: a refused first, two refused differently, and a refused envelope or payload later.
The last two are malformed input, which every engine must refuse to answer.
Two more list an envelope that is not an object and one with an unknown key, in both orders, as do two batch cases across drafts.
These scheduled cases cannot be replaced by random mutations.
Other named cases exercise wrong types, bad IDs, integer bounds, EDTF, repeated members, salt lengths, unresolved labels, an untyped or missing `of`, and specs with `"latest-by": "about"`, an untyped `from.of`, or a `follow` that is not `"prev"` on count, sum, min or max.
A deliberate self-label cycle also carries a near-miss label.
Most generated cases pass the model's input decoder.

Most reader numeric fields stay small.
Scheduled cases combine safe integer limits with cancellation to test exact sums.
The record boundary cases also test the writer's number limit.
Lens cases include escaped fences, empty IDs, repeated edited IDs, and duplicate base IDs.
Duplicate bases are domain-boundary probes, not valid-base law tests.
Batch cases include held predecessors, standing references, and held revisions that are not roots.
Some batches break one `of` rule: `of-missing`, `of-not-root` (by label or held), or `prev-unrelated`.
This is not exhaustive numeric or wire validation.

## Answers and refusals

The adapters start from the existing `interop/js/check-*.mjs` questions.
Every existing supported vector, including refused cases, passes through the evaluator test.
The following boundaries prevent comparisons between different questions:

- `fold/heads` and `fold/view` combine per-thing heads.
  The adapter uses JS `heads`, which orders the heads and keeps each ID once.
- Raw `fold/sum` and `fold/extrema` use only readable generated records.
  Numeric spec slots cover unread payloads.
  Lean's raw extrema include unread placeholders; JS numeric readers skip them.
- `spec/run` receives settled generated forms and compares JS `runSpec` with Lean `Spec.run`.
  Each operation admits its input.
  Realm and scenario tests cover settlement itself.
- Rust scenario answers use `things::state_in(thing, &admitted)` after settlement and admission.
  This computes state for the selected thing, rather than taking latest state across the whole view.
- Raw replay can violate these generation premises.
  A reported mismatch still needs domain review before it becomes a finding.

The boundary tests guard these compositions.
The comparator ignores object member order but preserves array order and string contents.
Engine answer integers outside the safe Number range use their exact source digits as BigInt.
The comparator does not round exact output sums to binary64.
Artifacts encode those integers as `{"$integer":"decimal digits"}`.

| Status | Meaning |
|---|---|
| `match` | Every JS order and each enabled engine return equal answers |
| `protocol-refusal` | All implementations return matching top-level refusal answers |
| `decoder-refusal` | Every enabled engine rejects the fixture input; diagnostics can differ |
| `mismatch` | Answers differ, only one engine rejects input, or JS throws after Lean decodes |

A Lean decoder refusal does not invoke JS, even if Rust accepts the fixture, except for record and batch inputs.
There JS is called and must also report malformed input by throwing; an answer from JS is a mismatch.
Typed JS APIs do not provide the model's fixture decoder.
If Lean accepts and Rust refuses, the harness also records the JS answers or exception.
A shared Lean/Rust decoder refusal remains a decoder refusal, not protocol agreement.
A one-sided decoder refusal is always a mismatch.
The summary counts shared decoder refusals separately and saves both field diagnostics.
A realm's `refused` record list remains part of a normal realm answer, not a top-level protocol refusal.
Only explicit shape or writer refusal codes normalize caught record exceptions.
Unexpected JS exceptions remain mismatches.
Transport failures never become protocol refusals.

`spec_text` and `payload_text` retain repeated JSON members.
Raw vector replay also retains spec member order from the original file.
A source-span walker sends each raw record payload to the JS text writer without changing duplicate members or number spellings.
Raw-text replay does not shrink, because rewriting that wrapper can change its meaning.
Generated mismatch artifacts remain shrinkable, including their embedded text fields.

## Artifacts and reduction

Each run keeps `options.json`, `summary.json`, `decoder-refusals.json`, and chunked input files under `inputs/`.
Options, summaries, and mismatch artifacts name the enabled implementations.
Each mismatch keeps its original vector and all engine responses under `mismatches/`.
It also keeps every JS order answer, or the JS exception name and message.
The comparison records acceptance states, pairwise equality, and the answer partition.
A partition groups equal observations; it does not identify a guilty implementation by majority.
The runner saves that raw mismatch before reduction starts.
It does not write `found/` entries or assign a protocol verdict.
Parent review decides which failures are distinct and need regression vectors.

The shrinker cuts records, steps, members, and nested arrays or objects, then reduces scalar values.
It repairs related arrival orders and scenario references.
It rejects inconsistent forms of one logical record.
A candidate must keep the layer, acceptance pattern, and equality partition.
It cannot change which engine refuses or which implementations disagree.
The partition also preserves which JS orders agree with each engine.
These checks do not prove envelope validity or preserve the original root cause.
Review each reduced input against the relevant law premises.
Reviewed candidates go under `found/` with cause-specific, domain-preserving reduction checks.
None is open: the duplicate-base case is now a model vector.
It cannot replace an answer mismatch with a JS exception.
An original JS exception must keep its exact name and message.
Transport errors stop reduction rather than count as successful trials.

Reduction is deterministic and greedy, with bounded trials and batched engine calls.
Rust mode checks every reduction candidate through both Lean and Rust.
The artifact reports trials, accepted rounds, and whether the trial limit stops the search.
It makes no global minimality claim.

## Verification scope

`phase2.test.mjs` checks the three-way contract with process and campaign mocks.
The other test files include real Lean checks and raw-source replay checks.
The adapter corpus test covers every supported existing vector.
The fuzz runner has no mismatch allowlist or ignored disagreement.
A test checks that no reviewed candidate is open and that the five `d1-` lens vectors agree in every implementation.
The official gap17 vectors test the resolved repeated and empty edited IDs.
Human rulings resolve gap17 and, under D1, the duplicate base: every implementation refuses it with `base-id-duplicate`.
Twelve scheduled record slots carry the D2, D3 and D5 boundaries: lone surrogates and overflow as text, and variants that disagree.
A record input has one JS answer, from the library's record-input contract; its variants are checked, not answered.

[REPORT.md](REPORT.md) records the current result and historical two-way and three-way results.

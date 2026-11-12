# Differential fuzz report

## Scope

This report covers the JS, Lean, and Rust campaign after `by` (issue #2).
It also preserves the `follow` campaign, the D8a and D8b campaign, the D1–D6 campaign, the Step-1 campaign and earlier two-way campaigns as historical findings.
Their counts are historical: the generator changed with D8, with `follow` and again with `by`, so the same seeds now generate different inputs.
The harness covers record, fold, spec, realm, scenario, batch, and lens inputs.

## `by`: three-way campaign

Source commit: `ed1a8f3`. Its JavaScript and Rust evaluators are those of `501907d` and `00ca77f`; its generator is `171728c`'s.
Node 25.9 checks 5,600 cases with seed `by-1`: 800 cases per family.
The run uses 88 chunks and takes 67.47 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/by-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 164 | 594 | 42 | 0 |
| fold | 800 | 701 | 0 | 99 | 0 |
| spec | 800 | 718 | 33 | 49 | 0 |
| realm | 800 | 681 | 0 | 119 | 0 |
| scenario | 800 | 689 | 0 | 111 | 0 |
| batch | 800 | 399 | 401 | 0 | 0 |
| lens | 800 | 572 | 228 | 0 | 0 |
| Total | 5,600 | 3,924 | 1,256 | 420 | 0 |

All 420 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/by-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

Every generated spec now also has ten slots with `by`: a count by `env.kind`, a sum by `env.observer`, a count by `env.about:t`, a followed count and a followed sum by `env.about:t` (the first with a payload test), a min by the text field `tag` with an envelope test, a followed max by the boolean `flag`, a followed count by the whole number `score`, a count by a field no record has, and a collect by `tag` with a `limit` and a payload test.
They run in the spec and scenario families over the records the generator already makes: revisions, forks, erased and withheld records, records mentioning one or two things, and refused revisions.
Over 512 spec cases of seed `by`, 256 are run cases: 198 hold a revision, 169 a record mentioning two things and 115 a withheld record (`fuzz.test.mjs`, which also checks that the groups of `by-kind`, its `ungrouped` and its `unread` add up to the plain count's value and unread).
Half the spec parse near misses now put `by` where it is refused: on latest-by, on the followed collect, with an empty or malformed prefix, or naming another envelope field; every implementation refuses them with `by-invalid`.
The generator changed, so this campaign's counts are not comparable with `follow-2`'s.

Raw logs and per-case files are not committed; they were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed by-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-by-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-by-1 by-1
```

## HISTORICAL follow on count, sum, min and max: three-way campaign

This campaign predates `by`; seed `follow-2` now generates different inputs.

Source commit: `ae80223`, after the merge of `main` at `2829f5f`. Its JavaScript and Rust evaluators are those of `908ec25` and `dbcfce1`; its model adds proofs and vectors since.
Node 25.9 checks 5,600 cases with seed `follow-2`: 800 cases per family.
The run uses 88 chunks and takes 64.03 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/follow-2/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 158 | 600 | 42 | 0 |
| fold | 800 | 709 | 0 | 91 | 0 |
| spec | 800 | 702 | 56 | 42 | 0 |
| realm | 800 | 713 | 0 | 87 | 0 |
| scenario | 800 | 694 | 0 | 106 | 0 |
| batch | 800 | 375 | 425 | 0 | 0 |
| lens | 800 | 557 | 243 | 0 | 0 |
| Total | 5,600 | 3,908 | 1,324 | 368 | 0 |

All 368 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/follow-2/d1-mismatches.json) keeps the historical D1 example and holds no case.

Every generated spec now has eight slots that follow `prev`: a count, a count with a payload test, a sum, a min and a max over `n`, a sum over a payload field with an envelope test, a sum over `n` with a payload test, and a max over `n` with a payload and an envelope test.
They run in the spec and scenario families over the records the generator already makes: revisions, forks, erased and withheld heads, records of natural keys and refused revisions.
Over 512 spec cases of seed `follow`, 190 run cases hold a revision, 90 a fork and 52 an erased head (`fuzz.test.mjs`).
A quarter of the spec parse near misses set one of those slots' `follow` to `"next"` or `true`, which every implementation refuses with `follow-invalid`.
The generator changed, so this campaign's counts are not comparable with D8's.

Raw logs and per-case files are not committed.

```sh
node interop/fuzz/run.mjs --rust --seed follow-2 --n 5600 --shrink-trials 80 --out /tmp/fuzz-follow-2
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-follow-2 follow-2
```

## HISTORICAL D8a and D8b three-way campaign

This campaign predates `follow` on count, sum, min and max.
Its counts come from the generator of that time; seed `d8ab-1` now generates different inputs.

Source commit: `b9f4389`; its model, JavaScript and Rust evaluator are those of `11784e5`.
Node 25.9 checks 5,600 cases with seed `d8ab-1`: 800 cases per family.
The run uses 88 chunks and takes 65.32 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/d8ab-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 156 | 601 | 43 | 0 |
| fold | 800 | 699 | 0 | 101 | 0 |
| spec | 800 | 712 | 42 | 46 | 0 |
| realm | 800 | 693 | 0 | 107 | 0 |
| scenario | 800 | 709 | 0 | 91 | 0 |
| batch | 800 | 425 | 375 | 0 | 0 |
| lens | 800 | 560 | 240 | 0 | 0 |
| Total | 5,600 | 3,954 | 1,258 | 388 | 0 |

All 388 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/d8ab-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

The generator carries `of` since D8.
A root leaves it out; other records name a natural key or an earlier record's thing.
Most revisions state their target's thing; a few name another, or leave `of` out, which admission refuses.
`about` only mentions; specs choose by `from.of`, `from.about`, both, or neither.
Five of every 64 record indices carry the `of` boundaries: an untyped `of`, a `prev` without `of`, and its order against `about`, `prev` and `erases`.
Four of every 16 batch indices break one `of` rule: `of-missing`, `of-not-root` by label or held record, or `prev-unrelated`.
Spec cases include `"latest-by": "about"` and an untyped `from.of`.
So the record and batch families refuse more often than in the D1–D6 run, and these counts are not comparable with it.

Raw logs and per-case files are not committed.
Run `python3 -B -m unittest discover -s interop/fuzz/evidence -p 'test_*.py'`.

```sh
node interop/fuzz/run.mjs --rust --seed d8ab-1 --n 5600 --shrink-trials 80 --out /tmp/fuzz-d8ab-1
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-d8ab-1 d8ab-1
```

## HISTORICAL D1–D6 three-way campaign

This campaign predates D8.
Its counts come from the generator of that time; seed `d1-d6-1` now generates different inputs.

Source commit: `bb9dc46`.
Node 25.9 checks 5,600 cases with seed `d1-d6-1`: 800 cases per family.
The run uses 88 chunks and takes 62.48 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/d1-d6-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 202 | 557 | 41 | 0 |
| fold | 800 | 694 | 0 | 106 | 0 |
| spec | 800 | 693 | 68 | 39 | 0 |
| realm | 800 | 716 | 0 | 84 | 0 |
| scenario | 800 | 701 | 0 | 99 | 0 |
| batch | 800 | 530 | 270 | 0 | 0 |
| lens | 800 | 571 | 229 | 0 | 0 |
| Total | 5,600 | 4,107 | 1,124 | 369 | 0 |

All 369 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
D1 settles the only earlier mismatch class.
The scheduled duplicate-base lens case is now a protocol refusal, `base-id-duplicate`, in all three implementations.
Its [mismatch list](evidence/d1-d6-1/d1-mismatches.json) keeps the historical example and holds no case.
No reduction runs, because no mismatch exists.

The record family differs from the Step-1 run by design.
Twelve of every 64 record indices now carry the D2, D3 and D5 boundaries.
Ten of them are protocol refusals: lone surrogates and overflow carried as text, and a refused first variant.
Two list a refused variant after a written one; Lean and Rust refuse to answer these as malformed input.
The JS adapter now gives one answer for a record input, as Lean does.
It no longer answers every variant.
A raw lone surrogate or an overflowing number outside a text payload cannot cross this transport.
JSON serialization would escape the first and write `null` for the second.
Model vectors and each implementation's own tests cover those forms.

Raw logs and per-case files are not committed.
The artifact tests check totals, example identity, and infrastructure-failure rejection.
Run `python3 -B -m unittest discover -s interop/fuzz/evidence -p 'test_*.py'`.

```sh
node interop/fuzz/run.mjs --rust --seed d1-d6-1 --n 5600 --shrink-trials 80 --out /tmp/fuzz-d1-d6-1
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-d1-d6-1 d1-d6-1
```

## HISTORICAL Round 2 review-fix three-way campaign

This campaign predates the D1–D6 rulings.
Its 42 mismatches are the duplicate-base class that D1 settles.

Source commit: `8a12ba6`.
Node 22 checks 5,600 cases with seed `step1-3`: 800 cases per family.
The run uses 88 chunks and takes 155.26 seconds.
Exit 1 records D1, not an infrastructure failure.
The retained [summary](evidence/step1-3/summary.json) and [D1 mismatch list](evidence/step1-3/d1-mismatches.json) support these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.
The script derives totals from these tracked artifacts without local logs.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 325 | 454 | 21 | 0 |
| fold | 800 | 694 | 0 | 106 | 0 |
| spec | 800 | 693 | 48 | 59 | 0 |
| realm | 800 | 699 | 0 | 101 | 0 |
| scenario | 800 | 698 | 0 | 102 | 0 |
| batch | 800 | 511 | 289 | 0 | 0 |
| lens | 800 | 574 | 184 | 0 | 42 |
| Total | 5,600 | 4,194 | 975 | 389 | 42 |

All 389 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
The importer checks every raw and reduced mismatch against the complete historical D1 input and all three answers.
All 42 cases match that example exactly, with Lean and Rust agreeing against JS.
The retained list stores each seed and index, plus shared raw and reduced examples.
This representation removes repeated content without dropping a mismatch.
No new mismatch class appears; D1–D7 remain unchanged.

Each reduction uses 76 trials and 3 accepted rounds.
The totals are 3,192 trials and 126 accepted rounds; 0 reductions reach the limit.
These results do not prove absolute minimality.
Raw logs and large per-case files are not committed.
The artifact tests check totals, D1 identity, and infrastructure-failure rejection.
Run `python3 -B -m unittest discover -s interop/fuzz/evidence -p 'test_*.py'`.

```sh
fnm exec --using 22 node interop/fuzz/run.mjs --rust --seed step1-3 --n 5600 --shrink-trials 80 --out /tmp/fuzz-step1-3
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-step1-3 step1-3
```

Earlier Step-1 campaigns with seeds `step1-1` and `step1-2` found only the same D1 lens mismatch; their evidence is not retained.

## Historical integrated three-way campaign

This campaign predates the Step-1 spelling changes.
It does not establish results for the updated generator.

A fresh Node 22 run compared JS, Lean, and Rust on 5,600 cases: 800 per family.
It used seed `integrate-1`, shrink limit 80, 88 chunks, and completed in 160.70 seconds.
Exit 1 was expected because the duplicate-base lens disagreement remains.

| Family | Before: match / protocol refusal / decoder refusal / mismatch | After: match / protocol refusal / decoder refusal / mismatch |
|---|---:|---:|
| record | 686 / 94 / 20 / 0 | 686 / 94 / 20 / 0 |
| fold | 700 / 0 / 100 / 0 | 700 / 0 / 100 / 0 |
| spec | 687 / 64 / 49 / 0 | 687 / 64 / 49 / 0 |
| realm | 698 / 0 / 102 / 0 | 698 / 0 / 102 / 0 |
| scenario | 648 / 0 / 98 / 54 | 702 / 0 / 98 / 0 |
| batch | 605 / 195 / 0 / 0 | 605 / 195 / 0 / 0 |
| lens | 577 / 179 / 0 / 44 | 577 / 179 / 0 / 44 |

The fixed run totals 4,655 matches, 532 matching protocol refusals, 369 decoder refusals, and 44 mismatches.
All 369 decoder refusals reject in both Lean and Rust; they are not agreement.
Only 20 refusal diagnostic pairs are text-identical; wording differences do not change their classification.

The earlier integrated run took 279.98 seconds and had 98 mismatches.
All 54 prior scenario mismatches now match; all 44 lens mismatches remain.
Replaying those 54 previous scenario cases restored a single JS/Lean/Rust equality partition in every case.
All 44 old and new raw and shrunk lens answer payloads match, ignoring artifact-local file paths.
The remaining lens inputs all repeat base ID `t:repeat`; JS differs from Lean/Rust.
The public fixture is `found/03-duplicate-base-id.json`; raw replay remains one mismatch and exits 1.
This probe does not decide whether duplicate base IDs are valid or prescribe protocol behavior.

The Rust scenario adapter now calls `things::state_in(thing, &admitted)` after settlement and admission.
That scopes state to the selected thing; it does not reduce the whole admitted view with latest-fold.
This aligns scenario `heads` and `winner` with the thing-specific state used by Lean and JS.

Reproduce into fresh output directories:

```sh
fnm exec --using 22 node interop/fuzz/run.mjs --rust --seed integrate-1 --n 5600 --shrink-trials 80 --out /tmp/fuzz-integrate-1-5600
fnm exec --using 22 node interop/fuzz/run.mjs --rust --replay interop/fuzz/found/03-duplicate-base-id.json --out /tmp/fuzz-duplicate-base
```

The runner shrinks each generated disagreement within its configured bound.
All 98 initial mismatches were reduced: 54 scenario reductions hit the 80-trial bound, and 44 lens reductions used 76 trials.
After the fix, all 44 remaining mismatches used 76 trials, three accepted rounds, and did not hit the bound.
Their input sizes reduced from 119 to 98 serialized bytes; no absolute-minimality claim follows.
The local per-case and raw reduction logs are historical provenance only.
They are not retained evidence in the repository.
The previous run's successful answer payloads were not persisted, so full answer-by-answer comparison is unavailable for its 5,502 prior non-mismatches.
Non-scenario family acceptance counts were unchanged.

## Historical campaigns

These two-way campaigns ran before the gap15, gap16, and gap17 rulings.
Their local logs are historical provenance only, not retained evidence in the repository.
Their 105,000-case total and findings below describe historical code, not the current integrated three-way run.
The two large campaigns contain 105,000 cases: 15,000 per family.
The first campaign checks the initial adapters.
The final campaign checks corrected API boundaries and adds lens, integer, and held-reference cases.
Only the final campaign supplies the findings below.
Small checks, replay cases, and shrink trials are additional work; they do not enter these totals.

```sh
node interop/fuzz/run.mjs --seed campaign-20261029 --n 70000 --no-shrink --out /tmp/slnext-fuzz-campaign-70000
node interop/fuzz/run.mjs --seed campaign-final-20261029 --n 35000 --no-shrink --out /tmp/slnext-fuzz-final-35000
```

| Campaign | Cases per family | Total | Timed loop | Raw mismatches |
|---|---:|---:|---:|---:|
| Initial | 10,000 | 70,000 | 549.479 s | 3,091 |
| Final | 5,000 | 35,000 | 321.636 s | 815 |

The initial generator has SHA256 `399ea7a8f852f200deb706cb360901439e381bbefec21a2ba30f7e056ec18a64`.
The final generator has SHA256 `14afb8777d4d46069740e354ea0361aead1d1491e4793ddd749e4a4f0bf7b0dc`.
The final campaign uses evaluator SHA256 `515d56f7dc1662dd0e85c31a9cb34d61845305e67433cc58a762d9b2213c7e31`.
A later raw-record replay fix does not change generated-case evaluation.

### Final counts

| Family | Cases | Match | Protocol refusal | Decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 5,000 | 4,371 | 518 | 111 | 0 |
| fold | 5,000 | 4,376 | 0 | 624 | 0 |
| spec | 5,000 | 4,352 | 312 | 336 | 0 |
| realm | 5,000 | 4,433 | 0 | 567 | 0 |
| scenario | 5,000 | 4,376 | 0 | 624 | 0 |
| batch | 5,000 | 3,932 | 1,068 | 0 | 0 |
| lens | 5,000 | 3,604 | 581 | 0 | 815 |

All 815 mismatches are answer differences. None is a JS exception.
The 2,262 decoder refusals do not count as protocol agreement.
The 2,479 protocol refusals match between implementations.

## Historical findings

The human gap17 ruling resolves the repeated and empty edited-ID findings.
The official vectors are `vectors/lens/section/gap17-duplicate-id.json` and `vectors/lens/section/gap17-empty-id.json`.
The duplicate-base finding remains an open protocol-domain question.

### 1. Repeated edited section IDs — resolved by gap17

The final campaign contains 270 cases. The first uses family-local index 5.

The minimum text contains two empty sections with the same nonempty ID.
The base is empty, so its IDs are unique.
Lean parses both sections and reports two unknown IDs.
JS refuses the text.

The human ruling now requires the parser to refuse repeated section IDs.

### 2. Empty section ID — resolved by gap17
The final campaign contains 272 cases. The first uses family-local index 6.

The minimum text is `["::: ", ":::"]`, with an empty base.
Lean returns one empty-ID section and one unknown-ID outcome.
JS refuses the text.

The human ruling now requires the parser to refuse empty section IDs.

### 3. Duplicate base IDs — spec gap

File: [`found/03-duplicate-base-id.json`](found/03-duplicate-base-id.json).
The final campaign contains 273 cases. The first uses family-local index 23.

Two base sections share one ID and have different bodies.
The edited text matches the first section.
Lean reports no edit. JS replaces both bodies with the first body's value.

`model/Loops/Lens.lean:78-91` uses first-match `lookup` in `diffSection` and `diff`.
`interop/js/lens.mjs:51` constructs a map, which keeps the last base entry.
`model/Laws/Lens.lean:143,250` requires unique base IDs for `no_op` and `put_get`.
At the time, the prose table at `protocol/laws.md:186-187` omitted that premise.
Current `protocol/laws.md` states distinct valid lens premises.
The historical design premise inventory did name `SectionIds`; the gap was not a claim about every document.
`model/Gen/Read.lean:328-332` accepts the duplicate base.

This is a domain-boundary probe, not a valid-base law violation.
The public boundary needs either an explicit unique-base requirement or defined duplicate handling.
Neither result proves that a valid-base edit loses data.

### Reduction and replay

The remaining candidate contains its input, Lean `expect`, JS answer, verdict, scope, citations, and reduction evidence.
The historical reduction checks use 56, 20, and 35 trials respectively, without reaching their bounds.
No smaller candidate from those domain-preserving checks retains the same cause.
This is not a global minimality proof.

```sh
node interop/fuzz/run.mjs --replay interop/fuzz/found/03-duplicate-base-id.json --out /tmp/fuzz-base
```

The command returns exit 1 while its disagreement remains.
Use a new output directory for each run.
The tests reproduce both answers and require an update when a finding resolves.
`--check` discovers the existing family directory layout, so it does not discover files directly under `found/`.

## Harness errors, not protocol findings

The initial campaign's 3,091 mismatches do not establish protocol bugs.

- Heads: 411 `fold/heads` and 572 `fold/view` cases compared different scopes.
  The current model combines per-thing heads under gap15.
  The adapter uses the public JS `heads` operation.
- Admission: 1,900 spec cases skipped the JS admission stage.
  The current JS `runSpec` admits its input, as Lean `Spec.run` does.
- Raw extrema: 208 cases compare raw Lean numeric primitives with view-aware JS readers.
  `model/Loops/Fold.lean:81-99` reads numeric placeholders; JS skips unread payloads.
  `protocol/laws.md:62` relates the operations when every record is readable.
  Raw numeric generators now satisfy that premise. Spec slots still test unread numeric results.

Reassessment makes all 2,883 heads, view, and admission cases agree.
The 208 old extrema cases remain outside the corrected generator's domain.
They do not enter `found/` as semantic bugs.
Spec generation also supplies settled forms, as `model/Loops/Spec.lean:322-325` requires.

Independent review also identifies raw-record replay loss and unbounded synchronous test subprocesses.
Raw payload source extraction and subprocess limits address those harness defects.

## Historical checks and CI

These results describe the earlier campaign, not a new run.

```sh
node --test interop/fuzz/*.test.mjs
npx --yes --package node@22 node --test interop/fuzz/*.test.mjs
(cd model && lake build --wfail && lake exe run --check ../vectors)
node interop/js/check.mjs vectors
node design/build.mjs
node design/mutants.mjs
git diff --check
```

Lean builds all 72 jobs and reproduces all 164 supported vectors.
The existing JS checker passes 4,891 checks.
The design build leaves tracked output unchanged.
The mutant check refuses 51 cases and preserves its one expected survivor.
All 19 harness tests pass on Node 22 in 3.980 seconds.
Active LSP probes leave eight files unconfirmed.
A historical auxiliary rule flagged `JSON.parse` in `interop/fuzz/json-source.mjs:19` because that local call can throw.
The current shared JSON source file is `interop/json-source.mjs`.
The source walker requires valid JSON; the evaluator catches thrown errors and reports them as mismatches.
That rule does not follow the outer catch.
Executable checks supply the verification evidence.

The historical model CI job installed Node 22, ran the harness tests, and ran seed `ci-1` with N=28.
Each new check step has a one-minute timeout.
The fixed corpus has no mismatches on Node 22.
Its local timed loop takes 0.430 seconds; the full command takes 1.44 seconds.
It covers every family, but only the first four generated fold sublayers.
The adapter corpus test covers every supported existing vector.
The job uploads run artifacts even on failure.
The fuzz runner has no mismatch allowlist. The duplicate-base question remains open.

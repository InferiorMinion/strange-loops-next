# Differential fuzz report

## Scope

This report covers the JS, Lean, and Rust campaign over the generator after the envelope came to carry `sets`, the payload keys a record sets.
It also preserves the `under-1`, `issue-7-1`, `by-collect-1`, `by-shape-1`, `shape-follow-1` and `by-1` campaigns, the two issue #1 campaigns, the `follow-2` campaign, the D8a and D8b campaign, the D1–D6 campaign, the Step-1 campaign and earlier two-way campaigns as historical findings.
Their counts are historical: the generator changed with D8, with issue #1, with `follow`, with `by`, with issue #7, with `under-unknown` and with `sets`, so the same seeds now generate different inputs.
The harness covers record, fold, spec, realm, scenario, batch, and lens inputs.

## `sets`: three-way campaign

Source commit: `68a85c3`, after the generator learned `sets`.
Node 25.9 checks 5,600 cases with seed `sets-1`: 800 cases per family.
The run uses 88 chunks and takes 87.72 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/sets-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 55 | 718 | 27 | 0 |
| fold | 800 | 696 | 0 | 104 | 0 |
| spec | 800 | 704 | 43 | 53 | 0 |
| realm | 800 | 697 | 0 | 103 | 0 |
| scenario | 800 | 689 | 0 | 111 | 0 |
| batch | 800 | 202 | 586 | 12 | 0 |
| lens | 800 | 571 | 229 | 0 | 0 |
| Total | 5,600 | 3,614 | 1,576 | 410 | 0 |

All 410 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed, which is still not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/sets-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

Record slots 37, 38, 42, 43 and 56 to 63 of the cycles of 256 that issue #1's shapes leave free each schedule one `sets` case, so 3 of the 800 record cases fall in each.
A `sets` that is text, `null` or an object, one with a member that is a number or `null`, and one with an empty name answer `sets-not-keys`.
Beside a bad `erases` it answers `erases-not-record-id`; beside a `prev` with no `of` it answers `sets-not-keys`.
An empty `sets`, a key the payload lacks, a payload that is not an object, and variants in other orders with repeats are written.
Batch slots 56 to 63 of 64 each schedule one: a `sets` that is not an array, an empty name, a draft with a bad `sets` beside one with a bad `erases` and beside one with `prev` and no `of`, each in both draft orders, a key that starts with `@`, and an empty `sets`.
Every record and draft no slot schedules draws `sets` last, so each earlier draw is as it was: absent half the time, empty, some key names with repeats, or now and then not keys.
Of the 97 record and 307 batch cases that carry `sets`, JavaScript writes 41 records and 114 batches, 11 and 37 of them with an empty `sets`, refuses 49 and 93 with `sets-not-keys`, and refuses 7 and 100 for something else.

Each wrong port was made in the JavaScript witness, run two-way on the same seed over the 800 record and the 800 batch cases with `--family record` or `--family batch` and `--no-shrink`, and reverted; the correct witness gives no mismatch there.

| Wrong port | Record | Batch | Cases |
|---|---:|---:|---|
| No `sets-not-keys` check | 49 | 93 | every scheduled refusal, and random ones |
| Checked after `of-missing` | 6 | 1 | `sets` beside `of-missing` in one envelope |
| Checked before `erases-not-record-id` | 6 | 7 | `sets` beside `erases` in one envelope |
| An empty `sets` left out of the canonical envelope | 11 | 37 | every written empty `sets` |
| Members in the writer's order, repeats kept | 18 | 48 | variants and keys in another order |
| Ranked after `of-missing` among a batch's refusals | 0 | 21 | a bad `sets` and an `of-missing` in different drafts |

Two of the order ports show few batch mismatches: where the two faults are in different drafts, the batch's ranking decides, not one envelope's order, and the last row covers that.
The generator before this change writes no `sets`.
Raw logs and per-case files were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed sets-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-sets-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-sets-1 sets-1
```

## HISTORICAL `under-unknown`: three-way campaign

This campaign predates `sets`; the generator changed after it.

Source commit: `f856072`, after the generator learned the `under` target slots.
Node 25.9 checks 5,600 cases with seed `under-1`: 800 cases per family.
The run uses 88 chunks and takes 90.70 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/under-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 100 | 673 | 27 | 0 |
| fold | 800 | 702 | 0 | 98 | 0 |
| spec | 800 | 710 | 41 | 49 | 0 |
| realm | 800 | 719 | 0 | 81 | 0 |
| scenario | 800 | 720 | 0 | 80 | 0 |
| batch | 800 | 257 | 531 | 12 | 0 |
| lens | 800 | 568 | 232 | 0 | 0 |
| Total | 5,600 | 3,776 | 1,477 | 347 | 0 |

All 347 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed, which is still not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/under-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

Batch slots 42 to 51 of 64 each schedule one `under` case, so 12 of the 800 batch cases fall in each.
The held store has a root and a revision of it.
A draft stands under a target nobody holds, under the held root, under the held revision, which is no grant, or under the raw id of the first draft, a member of the same batch.
A draft under an unheld target stands beside one with an unheld `prev`, beside one whose `of` names the held revision, and beside one whose `prev` names a record of another thing, each in both draft orders.
The first answers `under-unknown`; the next three are written; then `prev-unknown` twice and `under-unknown` four times.
Each wrong port was made in the JavaScript witness, run two-way on the same seed over the 800 batch cases with `--family batch --no-shrink`, and reverted; the correct witness gives no mismatch there.

| Wrong port | Mismatches | Slots |
|---|---:|---|
| No `under-unknown` check | 60 | the unheld target, and beside `of-not-root` and `prev-unrelated` in both orders |
| Checked after `of-not-root` | 24 | beside `of-not-root`, both orders |
| Checked before `prev-unknown` | 24 | beside `prev-unknown`, both orders |
| Batch members not counted as held | 25 | the raw id of a batch member (12) and issue #1's label in `under` (13) |

The generator before this change gives no mismatch for the first three; it never names an unheld `under` target.
Raw logs and per-case files were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed under-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-under-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-under-1 under-1
```

## HISTORICAL Issue #7: three-way campaign

This campaign predates the `under` target slots; the generator changed after it.


Source commit: `f4b3b46`, after the generator learned issue #7's two wrong ports.
Node 25.9 checks 5,600 cases with seed `issue-7-1`: 800 cases per family.
The run uses 88 chunks and takes 92.49 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/issue-7-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 96 | 677 | 27 | 0 |
| fold | 800 | 698 | 0 | 102 | 0 |
| spec | 800 | 707 | 43 | 50 | 0 |
| realm | 800 | 713 | 0 | 87 | 0 |
| scenario | 800 | 703 | 0 | 97 | 0 |
| batch | 800 | 309 | 479 | 12 | 0 |
| lens | 800 | 562 | 238 | 0 | 0 |
| Total | 5,600 | 3,788 | 1,437 | 375 | 0 |

All 375 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed, which is still not protocol agreement.
The 50 spec ones are the `bad-edtf` near miss, whose spec is refused.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/issue-7-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

Every generated spec now has two slots grouped by a payload field `rank`, and each spec run's held payloads get `rank` last, so the run's other draws are unchanged.
`rank` is left out, the text `"0"`, `"1"` or `"2"`, that number, or number text the input file writes as spelled: `1.0`, `10e-1`, `1E0`, `1.000` or `-0`, or the fraction `1.5`.
A vector stays a JSON value: `{"$number": "1.0"}` stands for the spelling, and only the input file writes it raw, so cloning and shrinking are unchanged.
JavaScript reads the orders through that same text.
Record slots 39 and 41, in three cycles of 256 in four, list an envelope that is not an object and one with an unknown key, in both orders.
Batch slots 40 and 41 of 64 give one draft an envelope that is not an object, no envelope, or no object at all, beside a draft whose envelope has an unknown key, in both orders.
Each answers `envelope-field-unknown`.

In this run 114 of the 400 spec runs answer differently when a reader keys a whole number by its spelling, as sl-py did, so `1.0` and `"1"` part ways; 18 record and 24 batch cases are the new shape slots.
Each wrong port was made in the JavaScript witness, run two-way on the same seed and reverted.
Reading `rank`'s number text as text gives 114 mismatches in 800 spec cases, each in the two `rank` slots.
Checking for an envelope that is not an object ahead of an unknown key gives 18 mismatches in 800 record cases and 15 in 800 batch cases, all in the new slots.
The other 9 batch slot cases have a draft that is not an object, whose envelope the shape stage does not read.
Calling such a draft malformed ahead of the shape stage gives 9 mismatches, exactly those.
The generator before issue #7 gives no mismatch for any of the three.
Raw logs and per-case files were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed issue-7-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-issue-7-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-issue-7-1 issue-7-1
```

## HISTORICAL `by` on a followed collect: three-way campaign

This campaign predates issue #7's generator; the generator changed after it.

Source commit: `9e07995`, after `by` came to collect that follows `prev`.
Node 25.9 checks 5,600 cases with seed `by-collect-1`: 800 cases per family.
The run uses 88 chunks and takes 87.11 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/by-collect-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 117 | 656 | 27 | 0 |
| fold | 800 | 712 | 0 | 88 | 0 |
| spec | 800 | 710 | 48 | 42 | 0 |
| realm | 800 | 701 | 0 | 99 | 0 |
| scenario | 800 | 690 | 0 | 110 | 0 |
| batch | 800 | 303 | 485 | 12 | 0 |
| lens | 800 | 568 | 232 | 0 | 0 |
| Total | 5,600 | 3,801 | 1,421 | 378 | 0 |

All 378 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed, which is still not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/by-collect-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

Every generated spec now also has two grouped collects that follow `prev`: one by `env.about:t`, and one by the boolean `flag` with a `limit` and an envelope test.
The parse near miss that put `by` on the followed collect, which is now accepted, is replaced by a `by` that is not text.
Raw logs and per-case files were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed by-collect-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-by-collect-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-by-collect-1 by-collect-1
```

## HISTORICAL `by` merged with issue #1: three-way campaign

This campaign predates `by` on a followed collect; the generator changed after it.

Source commit: `8d39dbd`, the merge of `main` at `7a0f77f` into the `by` branch.
Node 25.9 checks 5,600 cases with seed `by-shape-1`: 800 cases per family.
The run uses 88 chunks and takes 74.84 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/by-shape-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 119 | 654 | 27 | 0 |
| fold | 800 | 691 | 0 | 109 | 0 |
| spec | 800 | 698 | 53 | 49 | 0 |
| realm | 800 | 716 | 0 | 84 | 0 |
| scenario | 800 | 685 | 0 | 115 | 0 |
| batch | 800 | 314 | 474 | 12 | 0 |
| lens | 800 | 555 | 245 | 0 | 0 |
| Total | 5,600 | 3,778 | 1,426 | 396 | 0 |

All 396 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed, which is still not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/by-shape-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

The merged generator carries both changes: issue #1's record and batch shape slots and labels in `under`, and the ten grouped slots and four `by` parse near misses in every generated spec, beside the eight followed slots.
Neither change schedules the other's families: `by` changes only the spec family, issue #1 only record and batch.
It differs from the generators of `by-1` and `shape-follow-1`, whose counts are historical.
Raw logs and per-case files were written under the ignored `rust/target/scratch` and deleted after the summary was recorded.

```sh
node interop/fuzz/run.mjs --rust --seed by-shape-1 --n 5600 --shrink-trials 80 --out rust/target/scratch/fuzz-by-shape-1
python3 interop/fuzz/summarize.py campaign rust/target/scratch/fuzz-by-shape-1 by-shape-1
```

## HISTORICAL merged generator, before `by`: three-way campaign

This campaign predates the merge with `by`; the merged generator differs from its.

Source commit: `4eee07c`, the merge of `main` at `3238976` into the issue #1 branch.
Node 25.9 checks 5,600 cases with seed `shape-follow-1`: 800 cases per family.
The run uses 88 chunks and takes 67.65 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/shape-follow-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 114 | 659 | 27 | 0 |
| fold | 800 | 709 | 0 | 91 | 0 |
| spec | 800 | 692 | 59 | 49 | 0 |
| realm | 800 | 712 | 0 | 88 | 0 |
| scenario | 800 | 708 | 0 | 92 | 0 |
| batch | 800 | 307 | 481 | 12 | 0 |
| lens | 800 | 565 | 235 | 0 | 0 |
| Total | 5,600 | 3,807 | 1,434 | 359 | 0 |

All 359 decoder refusals occur in both Lean and Rust.
For the record and batch families JavaScript is called on them too and throws, so the 27 record and 12 batch ones are all three agreeing that the input is malformed; that is still not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/shape-follow-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

The merged generator carries both changes: issue #1's record and batch shape slots and labels in `under`, and the eight followed count, sum, min and max slots in every spec with the `follow` parse near miss.
Neither change schedules the other's families, so the two sets of slots do not overlap.
It differs from the generators of `shape-under-2` and `follow-2`, whose counts are historical.

```sh
node interop/fuzz/run.mjs --rust --seed shape-follow-1 --n 5600 --shrink-trials 80 --out /tmp/fuzz-shape-follow-1
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-shape-follow-1 shape-follow-1
```

## HISTORICAL `by`: three-way campaign

This campaign predates the merge with issue #1; the merged generator differs from its.

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

## HISTORICAL envelope shape and `under` labels, second three-way campaign

This campaign predates the merge with `follow` on count, sum, min and max; the merged generator differs from its.

Source commit: `ee0b9d3`.
Node 25.9 checks 5,600 cases with seed `shape-under-2`: 800 cases per family.
The run uses 88 chunks and takes 75.05 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/shape-under-2/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 123 | 650 | 27 | 0 |
| fold | 800 | 693 | 0 | 107 | 0 |
| spec | 800 | 687 | 57 | 56 | 0 |
| realm | 800 | 692 | 0 | 108 | 0 |
| scenario | 800 | 696 | 0 | 104 | 0 |
| batch | 800 | 329 | 459 | 12 | 0 |
| lens | 800 | 565 | 235 | 0 | 0 |
| Total | 5,600 | 3,785 | 1,401 | 414 | 0 |

All 414 decoder refusals occur in both Lean and Rust.
For the record and batch families, JavaScript is now called on them too and must also report malformed input, by throwing; an answer is a mismatch.
So the 27 record and 12 batch decoder refusals are all three agreeing that the input is malformed, which is still not protocol agreement.
The other families' JavaScript APIs have no input contract and are not called on a decoder refusal.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/shape-under-2/d1-mismatches.json) keeps the historical D1 example and holds no case.

The generator adds an unknown draft member, alone and beside an unknown envelope key, and a record input with no envelope, alone and beside a text payload's lone surrogate.
A repeated envelope or draft member cannot travel as a generated JSON value; adapter tests check it from raw text.
The first campaign, `shape-under-1`, could not find JavaScript's three gaps: the harness did not call JavaScript when Lean reported malformed input, and the generator made none of these inputs.

```sh
node interop/fuzz/run.mjs --rust --seed shape-under-2 --n 5600 --shrink-trials 80 --out /tmp/fuzz-shape-under-2
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-shape-under-2 shape-under-2
```

## HISTORICAL envelope shape and `under` labels three-way campaign

This campaign predates the second review round: its generator and harness changed after it.

Source commit: `2a3ba3a`; its model is that of `fb16fc6`, JavaScript that of `655cff1`, and the Rust evaluator that of `f6314ef`.
Node 25.9 checks 5,600 cases with seed `shape-under-1`: 800 cases per family.
The run uses 88 chunks and takes 65.04 seconds.
Exit 0: no implementation disagrees on any case.
The retained [summary](evidence/shape-under-1/summary.json) supports these counts.
Run `python3 interop/fuzz/summarize.py report` from the repository root.

| Family | Cases | Match | Protocol refusal | Shared decoder refusal | Mismatch |
|---|---:|---:|---:|---:|---:|
| record | 800 | 121 | 655 | 24 | 0 |
| fold | 800 | 669 | 0 | 131 | 0 |
| spec | 800 | 690 | 55 | 55 | 0 |
| realm | 800 | 700 | 0 | 100 | 0 |
| scenario | 800 | 704 | 0 | 96 | 0 |
| batch | 800 | 338 | 462 | 0 | 0 |
| lens | 800 | 581 | 219 | 0 | 0 |
| Total | 5,600 | 3,803 | 1,391 | 406 | 0 |

All 406 decoder refusals occur in both Lean and Rust.
They are not protocol agreement.
No reduction runs, because no mismatch exists.
The [mismatch list](evidence/shape-under-1/d1-mismatches.json) keeps the historical D1 example and holds no case.

The generator schedules issue #1's shapes and labels in `under`.
In one cycle of 64 record indices in every four, fourteen slots carry an envelope that is not an object, a required field left out, a field of the wrong JSON type, or a salt that is not base32hex text.
Nine of every 64 batch indices carry a draft envelope that is not an object or is left out, a field left out or mistyped, a salt that is not text, a label in `under` that resolves, one that is unknown, one that closes a cycle, and a label in `erases`.
A mistyped field and a malformed salt were shared decoder refusals before; they are protocol refusals now.
So these counts are not comparable with the D8a and D8b run.

```sh
node interop/fuzz/run.mjs --rust --seed shape-under-1 --n 5600 --shrink-trials 80 --out /tmp/fuzz-shape-under-1
python3 interop/fuzz/summarize.py campaign /tmp/fuzz-shape-under-1 shape-under-1
```

## HISTORICAL follow on count, sum, min and max: three-way campaign

This campaign ran on `main` before the merge with issue #1; the merged generator differs from its.

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

This campaign predates issue #1 and `follow` on count, sum, min and max.
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

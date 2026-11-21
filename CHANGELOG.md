# Changelog

What was built and what Kyle ruled, newest first, by date.
What is next is in [ROADMAP.md](ROADMAP.md). What waits on Kyle is in [DECISIONS.md](DECISIONS.md). The long design notes are in [notes/](notes/).

Each date starts with a short list. Under it is the record as it was written at the time, moved here from `ROADMAP.md` on 2026-11-21 and changed only where a sentence pointed at a place in that file.
A line "Implementer choices, for Kyle to confirm" marks choices that are still unconfirmed; [DECISIONS.md](DECISIONS.md) lists them all in one place.
The record starts where the old roadmap's did. For anything earlier, and for every merge, see `git log`.

## 2026-11-21

In brief:
- `loops read --values` and `fold --values` give each listed record's envelope (issue 9, !57).
- The Rust core reads through indexes: a keyed collect over 9,000 records went from 19.8 s to 0.63 s, and no answer changed (!58).
- margins-claude wrote the payload values note (!59). Its seven choices are open.
- The roadmap was split into `ROADMAP.md`, `DECISIONS.md`, this file and `notes/`.

### `--values` gives the envelope

**Built, 2026-11-21: `--values` gives the envelope** ([issue 9](https://git.gruel.network/kaygee/strange-loops-next/-/issues/9)).
The issue came from rex-tools, a chat room over the `loops` CLI and its first outside user: a timeline needs each record's `emitted`, `observer` and `about`, and got them from a second `loops export` joined by id.
Under `--values`, each `{"id", "shown"}` entry that a collect or a latest-by following `prev` lists gains `env`, the envelope as stored, whatever its `shown`; each thing of a latest-by that follows `keys` gains `about` as `loops of` writes it, and each of its heads gains `env` ([place](rust/PLACE.md), choice 17).
Without the option nothing changes, and taking the added members out still gives the protocol's answer byte for byte. Only `rust/cli` changed.
**Implementer choices, for Kyle to confirm:** no new flag, the envelope comes with `--values`; the member is `env`, the whole stored envelope, not three picked fields; an erased or withheld record's entry has `env` and no `payload`; a keyed thing's `about` is always written, empty when no head mentions anything, as in `loops of`; entries under a keyed thing's `keys` and `whole` gain nothing.

### The Rust read path

`rust/core` was a function-by-function mirror of the model's list scans, and its reads grew with the square of the records held. rex-tools, the first outside user of the CLI, met it within a few hundred records.
The core now settles, admits, orders heads and finds live setters through indexes: by id, by thing, and over the `prev` graph, where each strongly connected part is summarised once and the keys set above it are passed down to what it names (`rust/core/src/live/index.rs`). The model, the vectors, the protocol and JavaScript did not change, and no answer changed.

| Read, release build | Before | After |
|---|---|---|
| A keyed collect over 9,000 records | 19.8 s | 0.63 s |
| The same over 3,000 and 1,000 records | 2.2 s, 0.31 s | 0.20 s, 0.07 s |
| `loops check` over 9,000 records | 0.96 s | 0.53 s |
| rex-tools' `state` vertex, 508 records in 508 segments | 1.59 s | 0.04 s |

What holds it to the model:
- The 629 vectors, as before.
- The list scans it replaced are kept as test-only code (`rust/core/src/scan_tests.rs`), and generated records are run through both: forks, cycles, unheld `prev` targets, repeated and conflicting forms of one id, erased and withheld payloads, and `sets` absent, empty, repeated and of several keys.
- The three-way fuzz against Lean and JavaScript: 3,000 cases on seed `fast-fold-1` and 4,000 on `coordinator-fold-1`, no mismatch.
- The old and new binaries write the same bytes for 38 reads over seven stores, `--values` and `loops of` among them.

A function that still mirrors its Lean definition says `Lean:` in its comment; one that computes the same value another way says "Gives what Lean `x` gives" and how ([porting](rust/PORTING.md)).

**What is still linear and paid on every run:** each verb reads the whole store and checks every line's signature, about 60 microseconds a record. At 9,000 records that is the half second every read now costs, and a store built by 2,000 single emits still takes 145 s, since each emit checks all that came before. Segments are immutable and named by their hash, so a place can remember which segments it has checked. That is the next cost to remove, when it hurts; it needs no change to a record.

**Implementer choices, for Kyle to confirm:**
- The core's public surface gains `live::LiveState` (one thing's records, indexed) and `Spec::run_admitted` (run over records already admitted, so the CLI admits once). Every name that was public still is and gives the same value.
- The reference for the fast path is test-only Rust kept beside it, not a second crate.
- A Codex worker (gpt-6.2-astra) wrote the change from a brief; this session reviewed it and ran the checks above.

### The payload values note

Drafted by margins-claude for Kyle's rulings before any tick is built: [notes/2026-11-21-payload-values.md](notes/2026-11-21-payload-values.md). Nothing in it is built. Its choices are PV1 to PV7 in [DECISIONS.md](DECISIONS.md).

### The roadmap was split

Kyle asked for it on 2026-11-21: the roadmap had grown to 130 KB and he could not scan it for where the work stood.
`ROADMAP.md` now holds only what is next. `DECISIONS.md` holds what waits on Kyle, each with an example. This file holds what was built and ruled. The three long design notes and the retained boundaries moved to `notes/`.
No sentence of the record was dropped. The state of the evidence on that day follows, as the old roadmap's "State" section gave it; from here on, the counts a change moves are written in that change's entry, and the current counts are in the reports this section links to.

### State of the evidence at the split

The tree has three implementations: Lean, JavaScript, and Rust.
Step 1 applies the six settled rulings before records exist.
All three implement the settled decisions D1–D6, and D8a and D8b.
All three carry D8c's `sets` in the envelope and refuse one that is not a set of key names (`sets-not-keys`).
All three read a thing by its live setters in every slot that follows `keys`: latest-by, and count, sum, min, max and collect, with `by` or without.
In all three a collect that follows `keys` counts things in its `limit`, each listed whole (Kyle, 2026-11-15).
All three refuse every envelope shape by name (issue #1) and let a label stand in `under`.
All three writers refuse an `under` target they do not hold (`under-unknown`).
`loops emit` follows D8c's anchor rule: an emit of a thing names in `prev` every live record of it the store holds, and carries the thing's whole `about`.
An emit of a thing the store holds sets the fields it names, and `loops of`, `read --values` and `fold --values` show each key's value from its live setters.
A CLI verb that answers from a store with a line or a segment that does not check writes its answer and is then refused `store-not-clean` (Kyle, 2026-11-20).
The Rust core reads through indexes and gives the answers of the model's list definitions (2026-11-21, [the Rust read path](#the-rust-read-path)).
All three write a vertex's canonical result and its hash (D7): integers as decimal text, ids without `shown`. No CLI verb shows it yet.
All three let count, sum, min and max follow `prev` (issue #3), and group count, sum, min, max and collect with `by` (issue #2).
The law index has 111 entries, 1196 theorems, and 6 boundary witnesses.
See [the law index](design/laws/laws.json) and [proof and boundary data](design/laws/).
These proofs apply only under their stated premises.

The tree has 629 JSON fixtures: 617 generated fixtures and 12 standard fixtures.
Lean replays 608 generated fixtures; 9 occurred fixtures use separate checks.
Rust checks every fixture, six JCS byte pairs, 4,000 binary64 cases, and 128 SUPERCOP signing cases with ed25519-dalek.
Rust passes 194 tests; JavaScript passes 7,173 vector checks and 49 regression and law tests.
Both also check that `spec/run` answers write their members, group, thing and key names included, in the model's order; JavaScript checks `fold/latest-by` answers' thing keys too.
JavaScript refuses a held record's invalid `emitted` or `occurred` before it answers, as the Lean and Rust readers do.
The fuzz suite passes 60 tests.
See [vectors](vectors/README.md), [Rust boundaries](rust/GAPS.md), and [the mutation report](model/mutate/REPORT.md).
These checks provide bounded evidence, not complete JCS or cryptographic correctness.
They do not certify browser accessibility, external links, or official ASD dictionary compliance.

Input numbers round once to binary64.
A number past binary64's range is refused at the writer's number check.
The writer accepts values with magnitude at most 2^53 − 1, including fractions.
Numeric folds read safe whole values only.
Raw constructors and theorem premises retain separate domains.

The gates pass locally on Node 25.9, and CI runs every gate on Node 22.
The `collect-limit-1` campaign, over the generator after a keyed collect's `limit` came to count things, each listed whole, with two more keyed collects whose limit of one or two, plain or per observer, falls inside a thing, checks 5,600 cases, 800 per family, in 288.71 seconds.
It reports 3,638 matches, 1,579 protocol refusals, 383 shared decoder refusals, and no mismatch.
Four wrong ports of the limit in the JavaScript witness each give mismatches on the same seed, over its 800 spec and 800 scenario cases: the limit over records, as first built, 191 and 223; things taken and their records then cut at the limit, the same 191 and 223, since that is the limit over records; the things with the oldest live records taken, 85 and 88; and the limit applied before grouping, 90 and 79. The table is in [the campaign report](interop/fuzz/REPORT.md).
The `keyed-slots-2` campaign, over the generator before the limit counted things, found no mismatch either, nor did the `keyed-slots-1`, `keyed-1`, `sets-1`, `under-1`, `issue-7-1`, `by-collect-1`, `by-shape-1`, `by-1`, `shape-follow-1`, `shape-under-2` and `follow-2` campaigns, each over an earlier generator; `keyed-slots-2`'s eighteen wrong ports are in the report.
Decoder refusals are not agreement; for record and batch inputs JavaScript must also report them as malformed.
The generator carries issue #1's shapes and labels in `under`, followed count, sum, min and max slots, grouped slots, keys written as number text (issue #7), an unknown key beside an envelope that is not an object, `under` targets held or not, `sets` in record and changeset envelopes, latest-by following `keys`, count, sum, min, max and collect following `keys` with `by`, `where` and `limit`, keyed slots with two and three payload tests, keyed collects whose limit falls inside a thing, and `sets` and joins on the records a reader holds, so the same seeds now generate different inputs; earlier campaigns' counts are historical.
The `d8ab-1` and `d1-d6-1` campaigns found no mismatch.
The 42 duplicate-base lens mismatches of the `step1-3` campaign are the class D1 settles.
`interop/fuzz/summarize.py` derives these counts from tracked summaries.
See [the campaign report](interop/fuzz/REPORT.md) for the scripts and current and historical results.
The CI seed `ci-1` checks 28 three-way cases without mismatches.

The mutation catalogue has 314 entries, including nineteen for the result codec, six for the D1 and D2 definitions, nine for D8's `of`, nine for `follow` on count, sum, min and max, ten for issue #1's shapes and labels in `under`, seven for `by`, four for `under-unknown`, ten for `sets`, twenty for latest-by following `keys`, twenty-one for count, sum, min, max and collect following `keys`, and four for a keyed collect's limit counting things.
Ten earlier entries are retargeted at `of` and the one thing it names, three at the `follow` grammar, four at the shape stage and labels in `under`, eleven at the `follow` type on count, sum, min, max and collect, and two at a keyed collect's new site.
It retires the redundant positive-offset guard under R4 and the entry for the deleted `stripZeros` family.
The last full run, in CI on 2026-11-15, covers the 291 entries before a keyed collect's limit counted things: 243 build rejections, 24 vector differences, 22 generator assertion exceptions, and 2 survivors.
It ran at `132ecb2` in four shards. 23 entries timed out at the runner's default of 180 seconds a command, 22 of them in the one shard on a slower CI runner; run again at `122fd7e` with 600 seconds they give 17 build rejections, two vector differences and four generator assertions, and the counts above hold them so.
The four entries for a keyed collect's limit, the two retargeted at the collect's new site and the other entry on `keyedValue`, run alone, give seven build rejections and no survivor.
The runner exits 1 for assertion exceptions; after the second run no timeout or infrastructure failure is left.
Of the nine D8 entries, seven fail the build and two change batch vectors.
The run before it, over the 210 entries before issue #1 and `follow`, reported 165 build rejections, 28 vector differences, 15 generator assertion exceptions, and the same 2 survivors.
The four D6 helper survivors fail the build at their lemmas; the two scoped-equivalence survivors remain.
All-envelope shape checks now belong to each library.
Six deliberate per-envelope faults failed the respective vector layers in Step 1; that probe is not repeated here.
The cheap runner kills 33 of 33 mutants: each compiles and then fails runtime checks.
Build rejections are not semantic proof kills.
Survival does not prove equivalence.
See [the mutation report](model/mutate/REPORT.md) for outcomes, restoration checks, and limits.

## 2026-11-20

In brief:
- Kyle closed step 2: it does the old CLI's workable job, signed.
- A verb that answers from a store that does not check writes its answer and is then refused `store-not-clean`.
- Kyle ruled every section of the cut note, each for the recommendation: [notes/2026-11-15-the-cut.md](notes/2026-11-15-the-cut.md).
- margins-claude brought the cut laws into the model (`keyed.cut`, `cut.prefix`); they are described in sections 1 and 2 of that note.
- The result codec (D7), version 1, is built in all three implementations. Its implementer choices are not confirmed.

### Step 2 closed

What step 2 is:

`loops-place` and the `loops` CLI emit, read and fold for one person, signed, over `.loops/`.
Stored lines match the model's record vectors byte for byte; 34 tests drive the place and the binary.
The layout, the CLI and their refusal codes are a working proposal, not protocol.
[`rust/PLACE.md`](rust/PLACE.md) lists what is implemented and the seventeen choices made on the way.
Two of them need the model: what a place is has no law, and a payload reaches a fold only as `value`, `n` and top-level fields.

**Closed, 2026-11-20.** Kyle ruled that step 2 meets its exit criterion, "It does the old CLI's workable job, signed".
The replay of the exit check's session passes on `main`: an update that names one field keeps the others, `read --values` shows the content, a keyed collect's `limit` lists the newest thing whole, and `check` finds nothing.
Of what the exit check left, none blocks:
- `read` left out a stored line that fails its check, said nothing and exited 0; only `check` reported it. Kyle: a small fix, to warn and exit non-zero. **Built, 2026-11-20:** `read`, `fold`, `of`, `get` and `export` write their answer from the lines that check and are then refused `store-not-clean`, exit 1, when a line was refused or a segment is altered, as `check` is; `emit` and `erase` warn on stderr and go on ([place](rust/PLACE.md), choice 18).
  **Implementer choices, confirmed by Kyle 2026-11-20:** the rule covers every verb that answers from the store, not `read` alone, so `get` of a record that checks is refused too when another line does not; the answer is still written before the refusal; a verb's own refusal, such as `id-unknown` for the id of a refused line, is said instead; `check`'s stderr line now carries the two counts; `emit` and `erase` warn and keep their exit code; `import` does not read the store and says nothing.
- A hand-written JSON spec for each question, no scan by name prefix, no `ls` and no relative times: [the roadmap](ROADMAP.md).
- An import of the old tool's stores, and what its `combine` and `discover` vertices did, wait until they are wanted.
- The old Python `loops` is first on `PATH` on Kyle's machine. That is not the repository's to settle.

### D7: the result codec, version 1

**Built, 2026-11-20: the codec, version 1.** [`protocol/result.md`](protocol/result.md) defines it; the model is `model/Loops/Result.lean`, with a JavaScript witness (`interop/js/result.mjs`) and the Rust core (`rust/core/src/result.rs`).
The canonical result is one JSON object of the spec's slots, every one. An integer is a string of its decimal digits, at `value`, `unread` and `ungrouped` and nowhere else; a listed record is its id text; the bytes are JCS in UTF-8.
The hash is `"h:"` and the base32hex of SHA-256 over the label `loops/result-hash/v1:` and then the bytes.
The 23 vectors in `vectors/result/` give the canonical text and the hash for 16 `spec/run` inputs and 7 new ones: a sum of −2^54, a sum of zero, slots that read nothing, a grouped slot with `ungrouped` and `unread`, and one holding held, withheld and erased, whose bytes are equal.
No CLI verb shows a result hash; the tick work decides that.
The laws:
- `result.exact`: an integer of any size has one text, and the reader accepts no other.
- `result.shown`: two results that differ only in `shown` have the same text, bytes and hash.
- `result.canonical`: for two runs of one spec, the bytes are equal exactly when the results are equal with `shown` set aside. The premise is that the spec names each slot once, which every parsed spec does.
  It is not true of two arbitrary results: no fold tag is written, so a count and a sum that hold the same number write the same JSON, and an empty latest-by reads as an empty keyed one. The bytes name a result only together with its spec.
- `result.no-commit`: no result's hashed bytes are a 16-byte salt and then a canonical payload, so a result hash is never a record's commitment. The label is 21 bytes for that reason. A first draft used a 16-byte label, and a record with that salt and the result as its payload had the result's hash.

**Implementer choices, for Kyle to confirm:**
- A result hash is spelled `h:`, as a commitment is. Only their hashed bytes keep them apart. It could have a typed prefix of its own.
- The label `loops/result-hash/v1:`. A later version's label must not be 16 bytes, and must leave no JSON value after its first 16 bytes.
- A listed record is its id text alone, not `{"id": …}`.
- The bytes hold the slots only: no vertex name and no version member. The version is in the label.
- Slots are in JCS order, not the order the spec writes them.
- No fold tag is written.
- The vector family is a new `vectors/result/` over a subset of the `spec/run` inputs.
- The law names, beside the existing `result.unread` and `result.full`.
- The full mutation catalogue has not run over the nineteen new entries; two of them (`result-unshown-keeps-shown`, `result-unshown-groups-kept`) are expected to be refused by the build alone.

## 2026-11-15

In brief:
- `loops of` shows a thing's values, `--values` writes them beside ids, and an emit of a held thing sets only the fields it names.
- Kyle ruled that a keyed collect's `limit` counts things, each listed whole; built the same day.
- margins-claude drafted the cut note (ruled 2026-11-20).
- The last full mutation run, in CI (see the state of the evidence under 2026-11-21).

### Values beside ids, and an emit that sets the fields it names

**Built, 2026-11-15: values beside ids, and an emit that sets the fields it names.**
The step-2 exit check, against the old CLI's day-to-day jobs, found two gaps in "the old CLI's workable job".
A fold answers `{"id", "shown"}` lists, so a session that used to start with `sl read project` and see each item's name, status and message needed one `loops get` per setter.
And re-emitting `name=x status=done`, the old tool's way to update, was read whole, so it silently dropped every field it did not restate.
Kyle ruled on 2026-11-15: `loops of <thing>` shows the keyed state with each key's value from its live setters, and `read` and `fold` take an option that writes those values beside the ids; and on the single-record path an emit of an existing thing sets the fields it names, with a flag for a whole-read emit, while `emit -` fills nothing in.
Both are built in `rust/cli` and `loops_place::fill` (`rust/PLACE.md`, choices 15–17). Nothing in the model, the vectors, JavaScript, the core or `loops-eval` changed.
`loops of` answers the thing's `about`, its heads newest first, its record count and its `values`, read through the core's `live`: a key with one held live setter shows its value; a key with two or more shows each setter's id and value under `setters`, none picked and none merged; a live setter whose payload is erased or withheld shows its `shown` and no value; an erased or withheld live record read whole is listed as `unheld`; a record with `sets` gives only the keys it declares.
`read --values` and `fold --values` add the same members to each thing of a latest-by that follows `keys`, and a `payload` to each held record a latest-by that follows `prev` or a collect lists, inside each group too; without the option the answer is the protocol's, byte for byte.
An emit of a thing the store holds a record of, stating neither `--sets` nor `--sets-nothing`, is given the top-level keys it names as `sets`; `--whole` states a whole-read emit and is a usage error beside either.
The CLI walk replays the exit check's session with the new defaults: an update by `status=done` alone keeps the name and message, and one `read --values` shows each open item's name and status. It also covers a fork merged from two stores shown with both values, an erased and a withheld live setter, an undeclared key, `--whole`, the usage errors and grouped slots. The exit check's other findings (specs written by hand, prefix scans, `ls`, relative times) are not ruled and not built.
**Implementer choices, for Kyle to confirm:**
- The flags are `--values` and `--whole`.
- The first record of a thing, and a root, stay whole-read: they set every key they name. `--whole` there changes nothing.
- An emit of a held thing that names no field is given `"sets": []`, a pure join, as `--sets-nothing` gives: `loops emit task --of task:1 --about project:y` moves the task and sets nothing. A whole-read emit of nothing takes `--whole`. `--erases` and `--under` change nothing about the inference.
- Held means a record of the thing is in the store as a fold reads it, settled and admitted, an erased or withheld one included.
- `loops of` answers `{"thing", "about", "heads", "records", "values", "setters"?, "unheld"?}`: `heads` are shown lines newest first and replace `winner` and `others`, the first head being the winner; `setters` and `unheld` are written only when they hold something. `about` is the union of the heads' mentions.
- A key whose one held live setter lacks it is unset and not written; a key with several setters is written under `setters` even when none of them holds it.
- The keys shown are those some live record declares and the top-level keys of each held live record read whole. What an `unheld` record read whole sets is unknown and not guessed.
- `--values` adds members, never changes one: `values`, `setters` and `unheld` beside a keyed thing's `keys`, `whole` and `heads`, and `payload` on an `{"id", "shown"}` entry whose `shown` is `here`, the payload as stored, declared keys or not. Counts, sums, minima and maxima gain nothing.
- Existing walk tests that meant a whole-read emit of an existing thing now say `--whole`: the two-store walk's revisions and note, the whole-read emit above keyed ones, and the emit in the `sets` walk that states no `sets`. Walk tests that read `winner` and `others` from `loops of` read `heads`.

### A keyed collect's `limit` counts things

**Built, 2026-11-15: a keyed collect's `limit` counts things**, on Kyle's ruling of that day on margins-claude's point (section 3 of [the declared-keys note](notes/2026-11-11-declared-keys.md)).
A collect that follows `keys` with `limit` lists every live record of the first `limit` selected things and no record of the rest; under `by`, of the first `limit` things in each group. A collect without `follow`, or following `prev`, still counts records.
The things are taken in the order of each one's newest live record, the content order the list itself is in (`thingsInOrder`, `wholeThings`), and their records keep their places in the one newest-first list. `unread` is counted before the limit, as before, and a limit of 0 is still refused (`spec/parse/113`). The result's shape does not change.
The law is in `keyed.slots`: with a limit of `k` the list holds exactly the live records of at most `k` selected things, every live record of each, and leaves out a selected thing with a live record only when it lists `k` things (`keyedValue_collect_limit`, `wholeThings_whole`); the things listed are the newest, each with a live record newer than every live record left out (`keyedValue_collect_newest`, `wholeThings_newest`); over records of distinct things, as when every thing has one live record, it is the limit over records (`wholeThings_distinct`), so `keyed.whole-read` holds as it did.
`spec/run/47`'s two limited slots now list whole things, and `spec/run/51` pins the rest: a limit inside a thing's live records, two things whose records interleave in time, limits of 1 to 4 and one beyond the number of things, a pure-join head and an erased live setter in a listed thing, unread things beside a limit of 1, and under `by` a forked thing in two observer groups, a thing in two payload groups, and a group whose first thing is not the first overall.
**Implementer choices, for Kyle to confirm:**
- Which things: the first `limit` of the selected things ordered by each one's newest live record, newest first in content order. Content order ends in the record id and a record is of one thing, so no two things tie.
- The records of the things taken stay in one newest-first list across them, as before, not grouped thing by thing: the shape and the order are the ones the slot had, and only which records it holds changed.
- Under `by`, each group orders its own things by their newest live records; a thing in two groups is listed whole in each, and takes a place under the limit in each.

## 2026-11-14

In brief:
- All of D8c was built: `sets` in the envelope, latest-by following `keys`, count, sum, min, max and collect following `keys`, the slice laws for one keyed thing and for a spec, and the writer's anchor rule in `loops emit`.
- Kyle ruled that a slot following `keys` reads a thing's envelope from its heads ("Rule B"); the ruling is in section 3 of [the declared-keys note](notes/2026-11-11-declared-keys.md).
- The implementer choices of these five pieces are not confirmed.

### D8c's `sets` in the envelope

**Built, 2026-11-14: D8c's `sets` in the envelope**, section 1 of [the declared-keys note](notes/2026-11-11-declared-keys.md). Kyle ruled its shape on 2026-11-13.
`sets` is an optional envelope field: a set of top-level payload key names, canonical as `about` is.
Absent and empty differ. Absent is read whole; empty sets nothing, a pure join. The canonical envelope writes `"sets": []`, the one set left in when empty, so the two are different records with different ids (`record/16`, `17`).
One refusal, `sets-not-keys`: a value that is not an array, a member that is not text, or an empty name. It follows `erases-not-record-id` and precedes `of-missing`, in one envelope and across a batch.
Nothing compares `sets` with the payload or with another record's: a declared key the payload lacks, a non-object payload beside a non-empty `sets`, and a root with `sets` are written.
No fold, spec or lens reads it yet; it is carried, hashed into the id, stored, read back and shown, and an erased or withheld record keeps it with its envelope.
The law is `record.keys`; `record.sets` now covers `sets` too.
`record/15`–`19`, `45`–`47` and `shape-sets-*`, and `batch/76`–`82`, pin it.
The keyed fold, liveness per key and their laws are built in the next paragraph, the other slots that follow `keys` in the one after it, the slice laws for one thing and for a spec in the two after that, and the anchor rule in `loops emit` in the last.
**Implementer choices, for Kyle to confirm:**
- `sets` holds keys, not labels: in a changeset a key that starts with `@` is written as given, neither replaced nor looked up (`batch/76`).
- A held record's `sets` is checked as its other fields are: a stored line whose `sets` is not keys is refused with the writer's code, and one in another order is not canonical. The reader's record in fold, realm, scenario and spec vectors carries `sets` since the keyed fold below.
- `loops emit` takes `--sets <key>`, once per key, and `--sets-nothing` for an empty set; the two together are a usage error, and `--sets ""` is the writer's `sets-not-keys`. `emit -` takes `sets` as a line's envelope field. Nothing is filled in over either flag; an emit with neither is given the keys it names when its thing is held (Step 2, built 2026-11-15).
- A `where` test cannot name `env.sets`: no test reads it.

### Latest-by following `keys`

**Built, 2026-11-14: latest-by following `keys`**, section 2 of [the declared-keys note](notes/2026-11-11-declared-keys.md) and the part of section 3 a latest-by slot needs to select.
`{"latest-by": "of", "follow": "keys"}` reads each thing by its live setters; `"prev"` keeps its meaning and its result.
`"follow": "keys"` on count, sum, min, max and collect was refused with `follow-invalid` until the next paragraph's slice built them.
The reader's record carries `sets`, and settling, withholding and erasure keep it with the envelope.
Liveness is worked out where heads are: after settling and admission, per thing, over the admitted records of it (`Loops/Live.lean`).
A record is above another when its `prev` names it or a record above it, through those records; a setter of a key is live for it when nothing above it sets the key; live for the envelope is a head; live is live for some key, the envelope among them.
The result per thing is `{"keys", "whole", "heads"}`, as in the note's example, beside `unread`: `spec/run/31` is that example.
`from` and the envelope tests choose heads, each alone, and one chosen head selects the thing; each payload test is asked of every live setter of its field, and one answer from all of them is the answer, differing or unheld answers unknown.
`thingTest` (`Cond.onThing` per test) takes a thing's records and is what the next slice's count, sum, min, max, collect and `by` can ask.
The laws are `keyed.live`, `keyed.fold` and `keyed.select`; `spec.set` covers the keyed slot. `spec.slice` does not: `Spec.run_slice` takes a spec with no keyed slot, since a keyed slot reads live setters the spec does not take, and the spec-slice sweep finds slices that answer it differently.
`spec/parse/97`–`110`, `spec/run/31`–`41` and `scenario/keys-01-settled-setter` pin it; the CLI walk reads a keyed thing through `loops fold`.
The slice laws, and the anchor rule and copied `about` in `loops emit`, are built in later paragraphs.
**Implementer choices, for Kyle to confirm:**
- A field no record of the thing sets is absent, as a missing field in a held payload is: `is`, `present`, `at-least` and `below` answer no, `not` and `absent` answer yes (`spec/run/40`, `task:13`). A live setter that declares a field its payload lacks reads it as absent too.
- A chosen thing whose payload tests answer unknown counts once in `unread`; `unread` counts things, not records.
- Payload tests combine as a `where` map does: a test answering no outranks one answering unknown, so a thing with one disagreeing field and one failing field is not unread.
- `keys` is written in UTF-16 order of its key names, and each list newest first in content order, as heads are.
- Above reaches only through admitted records the reader holds: a `prev` naming a record not held, or a refused record, makes nothing above (`spec/run/41`).
- `value` and `n`, which the reader's record always holds, are payload keys like the others: a record with `sets` sets them only when it declares them, and a field no record sets is absent even when it is `value`.
- `Spec.run_slice` and `Slot.run_slice` take a premise that no slot is a latest-by following `keys`, rather than a slice law for it.
- The law names `keyed.live`, `keyed.fold` and `keyed.select`, and `follow` as a value of a new type `Follow` on latest-by in the model, while the accumulating folds keep a flag.

### Count, sum, min, max and collect following `keys`

**Built, 2026-11-14: count, sum, min, max and collect following `keys`**, section 3 of [the declared-keys note](notes/2026-11-11-declared-keys.md).
`"follow": "keys"` is accepted on every fold type; `follow-invalid` refuses any value but `"prev"` and `"keys"`, at the same check. `spec/parse/98`–`102` are now accepted forms, `106` and `107` pin the argument and `limit` checked after `"keys"`, and `111`–`115` pin the forms with `by`, `where` and `limit` and the refusals left.
Such a slot reads things, selected as latest-by following `keys` selects them (`thingEv`): `from` and the envelope tests choose heads, each alone, and the payload tests are asked of the thing over each field's live setters.
Count counts the selected things; sum, min and max read the field from its one live setter (`soleSetter`); collect lists the selected things' live records (`liveIn`), and its `limit` counts things (2026-11-15). The results keep the shapes of the same slots without `follow`; nothing forced a change.
With `by`, a payload key is read from each live setter of the field and an envelope key from each chosen head (`By.thingKeys`), and each group is the slot without `by` over its things (`Slot.keyedGrouped`).
The laws are `keyed.slots`, `keyed.by` and `keyed.whole-read`: when every record is read whole and every thing has one head, each such slot, grouped or not, gives what the same slot following `prev` gives (`Slot.keys_as_prev`). `spec.set` covers them; `spec.slice` does not, and its premise now leaves out every slot that follows `keys`, as the spec-slice sweep does.
`spec/run/42`–`50` pin it, `42` being the note's count of open tasks beside the same count following `prev`; the CLI walk counts, sums and collects through `loops fold`.
The slice laws are in the two paragraphs after these choices, and the anchor rule and copied `about` in `loops emit` (section 5) in the one after them.
**Implementer choices, for Kyle to confirm:**
- A field with no live setter is unread to sum, min and max, as a record without the field is; so is one whose live setter holds no safe whole number there. A record with `sets` that does not declare the field is no setter of it, so a field its payload holds but does not declare is unread too (`spec/run/44`, `task:5`).
- `unread` counts things, each at most once in a slot: a thing whose tests are unknown, or a selected thing whose field or key cannot be read. Within a group, a thing whose field cannot be read adds to that group's `unread`, as a record does.
- **Ruled by Kyle, 2026-11-15, on margins-claude's point:** collect's `limit` counts things, each listed whole; under `by`, the things in each group. As first built it cut the one newest-first list of the selected things' live records, as it cuts matching records without `follow`, so a limit could list some of a thing's live records and not others, and a reader of the result read a key as unset whose setter fell past the limit. Built on 2026-11-15.
- Collect writes one list across things in content order, the shape it has without `follow`; it does not group the records by thing.
- A group's own `unread` counts only things whose field cannot be read there; a thing whose tests are unknown is unread at the top, as an unknown record is.
- `follow` on count, sum, min, max and collect is an `Option Follow` in the model, replacing the flag; `FoldType.follow` reads it. `By.count_partition` and its neighbours take a premise that the count does not follow `keys`; over things the partition is `By.keyedAccounting`.
- The law names `keyed.slots`, `keyed.by` and `keyed.whole-read`, and the renamed vectors `spec/parse/106-argument-after-follow-keys-on-sum` and `107-limit-after-follow-keys-on-count`.

### The slice laws for one keyed thing

**Built, 2026-11-14: the slice laws for one keyed thing**, section 4 of [the declared-keys note](notes/2026-11-11-declared-keys.md), in `model/Laws/LiveSlice.lean` (margins-claude).
They are laws only: no definition a reader runs changes, and no vector.
They are stated over `Loops/Live.lean` as it stands (`above`, `liveFor`, `liveEnv`, `live`, `keyedIn`), with the envelope as one more key a record sets, and then behind settling and admission, over `realm`.
*Witnessed* is exact and has no premise (`witnessed_iff`): a part reads its records as the whole does, for a key, exactly when every held setter the whole reads as dead has a setter of the key above it through held records; a part that is witnessed for every key and holds the live records gives the thing's keyed state (`keyedIn_part`).
Convex is one way to be witnessed and not the least (`witnessed_of_convex`, `convex_not_needed`).
With anchored names, every record naming every live record under it, the live records alone give the keyed state, heads included, and every live record is a head or named by one (`anchored_slice`, `keyedIn_live`, `heads_slice`, `live_named_by_head`); over payload keys alone the heads are not given (`heads_need_env`).
The writer's rule leaves anchored names only in a holding that is closed, one that holds what the writers of its records held (`anchored_of_written`); a holder missing a record reads wrongly from its own live records (`closed_needed`).
Behind settling and admission (`keyedIn_slice`, `keyedIn_live_slice`), a part owes each record of the thing it holds every form of it, what erases it, and that it refuses it when the whole does; with anchored names the forms of the live records and what erases them are the slice.
It owes nothing for the `prev` targets of a record the whole admits: any part admits it (`entitled_fewer`).
The laws are `keyed.witnessed`, `keyed.convex`, `keyed.anchored`, `keyed.heads` and `keyed.slice`.
Two things the port found:
- **No cycle over `prev` is a premise** (`NoCycle`) of every law but *witnessed*: a dead setter has a live setter above it only when going up ends. Real ids give that, since an id is a hash of an envelope that holds its `prev`. The model's ids are numbers, and a fold vector has a record that names itself, so the laws state it.
- **`thing.slice`'s `prev` targets do not carry over.** Under `"follow": "prev"` a target's envelope is harmless: the record that names it is above it. Under `keys` an envelope still sets its keys, so a target held without its witness reads as a live setter (`target_needs_witness`). A whole `thing.slice`, which holds every record of the thing, gives the keyed state too (`keyedIn_thing_slice`).
The slice law for a spec with a keyed slot is in the next paragraph.

### The slice for a spec with a keyed slot

**Built, 2026-11-14: the slice for a spec with a keyed slot**, in `model/Laws/KeyedSpecSlice.lean` (margins-claude). Laws only, and the JavaScript spec-slice sweep now checks it.
A slot that follows `keys` reads each thing it selects by its live setters, records the spec need not take, so a spec slice is not enough for it (`keyed_slot_needs_live`).
A part *covers* a thing when its folded records of the thing are the whole's, hold the thing's live records and are witnessed for every key (`Covers`); it then reads everything a keyed slot asks of the thing as the whole does: its heads, its payload tests, each field's one setter, its live records and its group keys.
A part answers every slot that follows `keys`, of any fold type, grouped or not, when it covers each thing, or neither reader can select the thing: the part folds no record of it the spec takes, and the whole has no head of it the spec takes (`KeyedCover`, `Slot.keyed_cover`).
With a spec slice beside the cover, every slot of any spec is answered (`Spec.run_slice_keyed`); a spec whose slots all follow `keys` needs the cover alone (`Spec.run_keyed`). `Spec.run_slice` and `Slot.run_slice` keep their premise: they are the case with no keyed slot, where no cover is needed.
A part of the forms covers a thing as `keyed.slice` says (`covers_of_keyedPart`), and with anchored names by the forms of its live records and what erases them (`covers_of_live`).
The law is `keyed.spec-slice`.
- **A refused record of a covered thing is owed its `prev` targets** (2026-11-15). `KeyedPart` had asked that the whole admit every record of the thing the part holds, and a spec slice can break that by itself: it brings, as a target, a record of a covered thing that the whole refuses (`prev-unrelated`), and it brings the targets only of the records the spec takes. Without the target the part admits the record and reads the keys it sets (`refused_needs_targets`). `KeyedPart` now asks only that the part refuse what the whole refuses, and a part that holds what such a record names in `prev` does (`KeyedPart.of_targets`, by `entitled_part`). What it then holds of a covered thing is held like the rest: witnessed, with its erasers. The collect-limit sweep met the case first.
- **It is proved, and it asks a little more than the experiment needed.** The experiment (margins `sl-py/keyspecslice.py`, 16,000 spec and store pairs, every keyed slot type) was answered by the spec slice with the live records of the things it touches, the members between two sent members, and the erasers of the live records; the spec slice alone answered wrongly in 5,119 and 4,840 of 8,000. The law asks for what erases any record the part holds of a covered thing, since it holds the part's records of the thing to be the whole's records, shown state and all, where a keyed slot reads the shown state of the live ones only.
- **The sweep** (`interop/js/spec-slice.test.mjs`) builds that part for each of 3,912 generated specs and stores and finds every slot, keyed ones among them, answered as the whole answers; it is a proper part in 507, and leaving out the live records changes an answer in 50 and the erasers in 1. The spec slice alone still differs on a keyed slot in 53. Since the generator learned the collects of 2026-11-15 it meets a record of a covered thing that the whole refuses, its `prev` naming a record of another thing, and that the spec slice brings as a target; the part then holds what that record names in `prev` too, so that it refuses it as the whole does, as `Covers` asks: the part's folded records of the thing are the whole's. What the part holds for such a record is held like any record of its thing, with what lies between and what erases it (the helper runs its steps until nothing is added). The generator does not reach a store where that closing matters; margins' experiment does (`sl-py/keyspecslice.py`, mode `refused`, 8,000 spec and store pairs in which a later record of the thing names a refused one): the part without the refused record's targets answers wrongly in 832, with the targets added once in 12, and closed in none.

### The writer

**Built, 2026-11-14: the writer**, section 5 of [the declared-keys note](notes/2026-11-11-declared-keys.md), in `loops emit`.
An emit of a thing names in `prev`, beside any `--prev`, every record of that thing the store holds that is live for some key, the envelope among them, and so every head (`rust/PLACE.md`, choice 11).
The thing is `--of`, or what `--prev` replaces; liveness is the core's `live`, over the store settled and admitted as `loops fold` reads it, so an erased or withheld record is named by its envelope.
With no `--about`, the emit copies `about` from the thing's heads, every mention when there are several; `--about` states the whole `about`, and `--no-about` an empty one.
A root, an erasure and the first record of a thing name nothing and copy nothing. `emit -` fills nothing in, and nothing refuses an unanchored record.
Nothing in the model, the vectors, JavaScript or the core changed: `loops_place::fill` holds the rule, over the core's `live`, and `loops emit` calls it before the batch check.
The CLI walk writes the note's history (a task created whole, then `--sets status`, then `--sets title`) with `loops emit` alone and checks each `prev` against the live records a collect that follows `keys` lists; it covers a whole-read emit above keyed ones, a fork merged by export and import and joined by one emit, an erased and a withheld live record, `--about`, `--no-about` and a moved task, and a changeset that fills nothing in.
`rust/place/tests/anchor.rs` writes 300 seeded random emits through the same filler, over six things and four keys, with whole-read emits, pure joins, moves, explicit `prev`s and erasures: one writer, and two replicas that merge both ways now and then.
Every 25 emits it requires the history to be anchored (every record above a live record names it) and each thing's live records, with the records that erase them, to answer fifteen slots that follow `keys` as the whole holding does: latest-by, count, sum, min, max and collect, with `where`, `limit` and `by`. At 300 emits the slice holds 41 of 300 records for one writer and 44 of 300 merged; no answer differed. Written without the anchor, each emit naming one live record of its thing, the slices answer differently at many checkpoints: in one run, 10 of the 12 for one writer and 10 of the 24 for the two replicas. The counts vary from run to run, since the test's keys and salts are fresh each run; the property has teeth.
Existing walk tests that made a fork with a second `--prev` to one record now write the fork as a changeset that states its own `prev`, since `loops emit` would name the first revision in the second; in the two-store walk, b's note now replaces b's revision, so the merged thing has two heads, not three.
**Implementer choices, for Kyle to confirm:**
- With no `--about`, an emit of a thing copies the union of its heads' mentions, in canonical order; `--about`, once or more, replaces the copy; `--no-about` states an empty `about`; `--no-about` with `--about` is a usage error.
- An emit builds on every head of its thing and every other live record held, whether or not `--prev` names them; an explicit `--prev` is kept beside them, live or not. The heads it builds on, for `about`, are the thing's heads, not an explicit `--prev` that is not one.
- Liveness is read over the admitted records: a held record admission refuses is not named.
- `loops erase` writes a root with `erases` and no `of`, so it names nothing; `--erases` and `--under` on an emit change nothing about the anchor; a thing named by a root's id is that root's thing, and the root is named while it is live.
- What was filled in is not printed: `emit` answers the id, and `loops get` shows the `prev` and `about` written.
- No flag turns the anchor off; `emit -` is the path that fills nothing in.
- The filler, of `of` as well as of the anchor and `about`, moved from the CLI into `loops-place` (`loops_place::fill`), so a test can drive the writer's rule over hundreds of emits without starting the binary for each; the CLI still decides when it runs.

## 2026-11-13

In brief:
- Kyle took `under` targets: a writer refuses an `under` target it does not hold (`under-unknown`). Built the same day.
- Kyle ruled the declared-keys note, sections 1 to 5: [notes/2026-11-11-declared-keys.md](notes/2026-11-11-declared-keys.md).

### `under` targets

**Kyle, 2026-11-13:** "Let's take under targets now."
This replaces the choice deferred on 2026-11-11, that a writer checks no `under` target beyond its shape.
Kyle took margins-claude's view: a writer refuses an `under` target it does not hold, beside `prev-unknown`.
**Built, 2026-11-13.** A new writer refusal, `under-unknown`: a record's `under` must name a record the writer holds, in the batch or in the store.
It runs across the whole batch, after `prev-unknown` and before `of-not-root` and `prev-unrelated`, so no draft order changes the refusal.
A batch member counts, named by its label or by its id.
A stored record counts whatever its shown state: the writer holds an erased or withheld record's envelope.
Nothing else about the target is checked. Whether it is a grant, of what kind and by whom, is still standing's question (step 3).
Readers do not refuse on it, as they do not for `prev-unknown`.
The batch writer and the CLI's single-record path check it against the same store lookup; there the CLI's own `prev-unknown` still comes first.
`batch/67`–`75` pin it: an unheld target refused, a held root and a held revision accepted, a batch member's raw id and an erased target accepted, and the order beside `prev-unknown`, `of-not-root`, `prev-unrelated` and a writer refusal.
No existing vector changed meaning: every earlier batch vector's `under` names a batch member by label.
The CLI walk pins an erased and a withheld stored target.

## 2026-11-11

In brief:
- `"follow": "prev"` on count, sum, min and max (issue #3).
- `by` on count, sum, min, max and collect (issue #2).
- Every envelope shape has a named refusal, and a label may stand in `under` (issue #1).
- Kyle chose the recommendation for D7, the canonical result encoding (built 2026-11-20).
- Kyle agreed that the model states the slice law as *witnessed* (issue #5), and that D8c starts with a design note, which margins-claude drafted that day.

### `"follow": "prev"` on count, sum, min and max

**Built, 2026-11-11: `"follow": "prev"` on count, sum, min and max (issue #3).**
Such a slot folds the whole-reader heads, each thing's heads with each id once, which are the heads latest-by and a followed collect read.
It selects as latest-by does, and its result keeps the shape of the same slot without `follow`.
Every head of a forked thing counts; count counts an erased or withheld head by its envelope, and sum, min and max count it unread.
Any other `follow` value is refused with `follow-invalid`, at the check that already refused it on collect; that check now accepts `"prev"` on every fold type.
The law is `spec.follow`: the answer depends only on the set, through each thing's heads (`heads.set`), not `acc.set`. `heads.why` still holds.
A caching reader for these slots is not built. A record with an unknown `prev` can supersede a head and later be refused when its target arrives, so a reader that caches must keep what such a record supersedes. Holding such records back instead would refuse the slices the anchor rule allows, whose live setters travel without their ancestors. Kyle, 2026-11-11: this is step 4's first law.
**Implementer choices, confirmed by Kyle 2026-11-11:**
- The law's name, `spec.follow`, and its place in the spec table.
- `follow` is a flag on each accumulating fold type, as on collect, not a slot-level field.
- `"follow": "prev"` with `limit` on count is refused with `limit-invalid`, as before: `limit` stays collect's.

### `by` on count, sum, min, max and collect

**Built, 2026-11-11: `by` on count, sum, min, max and collect (issue #2).**
`by` groups a slot by `env.kind`, `env.observer`, the members of `about` under a typed-reference prefix (`env.about:<prefix>`), or a payload field.
It is allowed on count, sum, min, max and collect, following `prev` or not; on latest-by, or naming nothing to group by, it is `by-invalid` at `fold.<name>.by`, checked after `limit` and before the fold's argument.
The result is `{"groups": {<key>: <result>}, "unread": n, "ungrouped": m}`, each `<result>` the slot's own shape.
Settling, admission and the whole-reader heads come first, once, over everything held; each group is then the slot without `by` or `where` over the records selected into it.
The laws are `by.set`, `by.after-admission`, `by.accounting` and `by.partition`; `spec.slice` covers grouped slots without a new premise.
Ordering or top-N across groups, buckets and windows, and `by` on latest-by are not built.
**Kyle's choices, confirmed 2026-11-11:**
- The spelling `env.about:<prefix>`: it reuses `where`'s existing rule that `env.` names the envelope, so a bare name such as `about:project` is a payload field, and no second name is reserved.
- Keys are text, so a text value and a number or boolean value that render alike share a group: the text `"1"` and the number 1, the text `"true"` and `true`.
- A selected record with no key is counted in `ungrouped`: a missing field, no mention under the prefix, or a value that cannot be a key (a fraction, null, an array or an object). It is never dropped and never a refusal; only a payload the reader does not hold makes a key unread.
- A record mentioning several things under the prefix counts in each.
- `limit` applies per group; a top-N across groups is a later discussion (the earlier strange-loops had one, in a weather vertex example).
- `by` is allowed on collect that follows `prev`, and refused only on latest-by.
**Implementer choices, confirmed by Kyle 2026-11-11:**
- Groups are written in UTF-16 order of their keys, the order canonical JSON and `thingsOf` use.
- The four law names, and `by` checked before the fold's argument as the issue proposed.

### Issue #1: envelope shape refusals, and a label in `under`

**Built, 2026-11-11.** Every envelope shape has a named refusal, as `protocol/spec.md` has for specs.
An envelope that is not an object is refused with a new code, `envelope-not-object`.
A required field left out, or any field of the wrong JSON type, is refused with that field's existing code, in that field's place.
The model reads such a field as empty text, which every field's own check refuses (`unreadable_refused`).
The same change brings the batch up to the settled ledger entry "A label may stand in `under`": a draft may name another by label in `about`, `of`, `prev` or `under`, never in `erases`, and labels in `under` take part in cycles.
**Kyle's choice, confirmed 2026-11-11:** `loops emit -` reads a changeset line's envelope fields through the core's reader, as `Read.envOf` does.
A field left out or of the wrong JSON type gets the field's own code, in the writer's order across the whole changeset, so another line's label refusal outranks it.
`draft-shape` stays for a line's own shape only: a line that is not a JSON object, or a label that is not text; an unknown member stays `draft-field-unknown` and a repeated one `member-duplicate`.
The CLI fills in `observer` and `emitted` only when a line leaves them out; one given mistyped gets its code, and `observer-unbound` now judges only a typed observer.
A line's salt is read as the core reads one, so one that is not base32hex text is `salt-not-16-bytes` in its place.
**The reviewer's direction, confirmed by Kyle 2026-11-11 when approving !21:**
- One new code, `envelope-not-object`; every other shape fault folds into an existing code, extending "A malformed value uses the field's existing refusal code".
**Implementer choices, confirmed by Kyle 2026-11-11 when approving !21** (an untyped CLI observer is `observer-not-typed`, as above):
- `envelope-not-object` is a shape check across every envelope: after `envelope-field-unknown`, before `string-not-unicode` and every value.
  So a later record variant that is not an object refuses the input, as an unknown key does, though the first envelope decides value refusals (D5).
- A batch draft object without `env` has an envelope that is not an object.
- `null` is a value of the wrong type, not a field left out: `"occurred": null` is `occurred-not-span`.
- A set that is not an array is refused with its code; so is a set with a member that is not text, and a label beside such a member is still a label, so labels are checked first.
- A salt left out, not text, or not canonical base32hex is `salt-not-16-bytes`, in that check's place.
- A batch draft that is not an object, an unknown draft member, a label that is not text, a draft without a payload, and a repeated envelope member name stay malformed input, with no code.

**Implementer choice, confirmed by Kyle 2026-11-11.**
- In JavaScript, a repeated envelope or draft member is seen only from raw text, in the fuzz adapter; the value APIs (`resolveBatch`, `writeRecord`, `seal`, `writeText`) cannot see one (`interop/js/README.md`).

### D7: canonical result encoding, the choice

Kyle chose the recommendation below on 2026-11-11, and on 2026-11-20 ruled that no integer carries a tag and that `shown` is not in the bytes (the cut note, sections 4 and 6). It is built (2026-11-20), and changes no reading; step 4's ticks need it.
The [open design](design/rethink/sections/16-open.html) prefers JCS for results.
Exact, unbounded `Sum` outputs include `9007199254740993` in `phase2-exact-output`.
Neither current tests nor the model define a canonical result hash codec.
**Readings:** Define a separate exact-integer canonical JSON extension, or use typed decimal integer strings inside JCS.
Raw JCS numeric rounding cannot preserve every `Int`.
**Effects:** These choices produce different bytes, hashes, and schemas.
**Recommendation:** Define a separate versioned result codec using typed decimal integer strings under JCS.
Never round or cap current model outputs.

## 2026-11-10

In brief:
- Kyle ruled D8: `of` names a record's one thing, `about` only mentions, and `sets` declares the keys an emit sets.
- D8a and D8b were built across the model, the vectors, JavaScript and Rust.
- Kyle confirmed the landed choices of D1 to D6.
- Kyle set the order of the steps: finish step 2 with D8, then step 4 (ticks), then step 3 (standing) together with step 5 (sharing).

### D8: items, mentions and declared keys

Kyle ruled this on 2026-11-10.
D8a and D8b are built across the model, vectors, JavaScript and Rust, and all of D8c is built: `sets` in the envelope, liveness per key, latest-by following `keys`, count, sum, min, max, collect and `by` following `keys`, the slice laws for one keyed thing and for a spec with a keyed slot, and the anchor rule and copied `about` in `loops emit`.
It changes the settled envelope, so it lands as R1–R6 and D1–D6 did: model, vectors, JavaScript, Rust, then protocol and pages.
**Finding:** Two apps built on the product slice hit one wall on 2026-11-10.
The orchestration dry run is in [Scenarios](design/rethink/sections/13-scenarios.html), step 8; margins-claude's `work` app is described in its notes on merge request !12.
`about` does two jobs: "this record is an emit of this thing" and "this record mentions that thing".
A task about its plan became a rival head of the plan.
A record about something else was not a thing under its own id.
A closing record about the 23 ids it covered became the winner of each.
Restating a whole payload on every change left a writer no basis to merge a fork field by field; the fork showed as two heads, and one writer's merge dropped a change.
The earlier systems (`tasked`, `ticked`) keyed a fold by a field and re-emitted only the changed key.

The choice has three parts. D8a and D8b go together; D8c can follow.

- **D8a, an item field.** One single-valued envelope field, `of`, says which thing a record is an emit of.
  Only a root, a record with no `of` and no `prev`, leaves it out; it is an emit of the thing its own id names.
  Every record with `prev` states `of`. A tool may fill it in; the protocol never infers it.
  `of` is read literally: a reader never resolves it through another record, so a record's thing is a fact of its own envelope.
  An id in `of` names a root; a writer holding a record that is not a root refuses to name it.
  `latest-by` runs over this field; a thing's heads are records of it.
  Every `prev` target must be an emit of the same thing, so `prev` never crosses things.
  One thing replacing another is a new thing that mentions the old one, plus an emit on the old one that closes it: two records, not atomic.
- **D8b, `about` is mention only.** `about` never makes a head.
  `where` and `collect` still read it, so "everything said about X" stays a fold.
  A link needs no new primitive: it is a thing whose `about` names both ends.
- **D8c, declared keys.** An envelope field, `sets`, lists the payload keys an emit sets.
  A keyed fold holds, for each key, the setters no descendant setter of that key replaces, over `prev`.
  Different keys from one head merge; one key set twice from one head leaves two live setters, and the fold reports both.
  A key is unknown exactly when a live setter's payload is not held, as `where` answers.
  Withheld and erased are one state to the fold: an envelope without its payload.
  They differ only in what comes next, which the arrival laws carry: a withheld payload may arrive, an erased one must be dropped.
  Either way a later emit that sets the key makes it known again.
  A declared key absent from the payload is unset, and a payload key that is not declared is not read.
  An absent declaration and an empty one differ: absent means the record is read whole, as today; empty means it sets nothing, which is a pure join of two heads.
  A whole-read emit sets every key, named or not, and stays needed until a later whole-read emit replaces it.

**Readings:** Put identity and keys in the envelope, or in the payload.
In the payload, an erased or withheld emit can be placed only by `prev`, and a slice or a sync cannot be computed from envelopes.
For declared keys, margins-claude's experiment (`sl-py/keyfold.py`) found no wrong answer in 240,000 key answers, and none in 320,000 more with whole-read emits mixed into the chains.
Within one thing, its envelopes plus its live setters' payloads sufficed, and none could be dropped.
The experiment takes the item as given, one thing at a time, with no erasing records and no admission.
That identity belongs in the envelope (D8a) is argued, not measured, and the composition with `thing.slice` is untested.
Without declared keys, a reader needs every payload in the chain, or answers wrongly when an unheld emit is taken to set nothing.
**Effects:** Two new envelope fields enter the id; a record without them keeps its bytes and id.
Records already about a thing stop making heads of it, so the `latest-by` vectors and the heads and slice laws are restated.
`prev-unrelated` is restated over the item field.
A record is an emit of one thing only; today one record can head several.
An erased emit still shows which keys it changed, though not to what.
Kyle accepted this on 2026-11-10: erasure is the emergency stop for something that should not have been emitted, and its job is that the content travels no further.
`kind`, `about` and the times already survive erasure; secrets do not go in envelopes, key names included.
The core starts to read a payload's top-level keys by name.
`thing.slice` as proved covers whole-record folds only; the keyed fold needs its own slice law, argued and not yet stated in Lean.
**Order:** Take D8a and D8b now, in the envelope, before any store is kept.
Take D8c as a second fold family beside whole-record `latest-by`, once D8a's shape is settled and its slice law is stated.
Kyle, 2026-11-11: D8c starts with a design note, before any code. Once an emit can set only some keys, a thing's current state is the merge of its live setters, not one head's payload, and `where`, `follow` and `by` read a head's payload today. The note says how they read a keyed thing, the spec grammar and result of the keyed fold, the writer's anchor rule and the witnessed slice law. margins-claude drafts it.
**Amended by Kyle, 2026-11-10,** on margins-claude's finding in issue #4 (`sl-py/keyslice.py`): D8a first let a record with `prev` leave `of` out and take its thing from what it replaces.
That made a record's thing depend on another record being held, and a partial copy then answered wrongly: 10,276 of 33,634 cases under one slice rule and 412 under the other, against none in 35,200 with `of` stated.
The CLI's "a revision without `--about` is about what it replaces" stays as a writer's convenience only.
The same experiment found that a replaced member sent without the path to what replaced it reads as live, and proposed that what is sent of a thing be convex over `prev`.
Issue #5 corrects the proposal: convexity is sufficient but not exact, and on a steady history the convex part is nearly the whole history.
The exact law is *witnessed*: every held setter of a key that is dead for that key has a setter of that key above it through held records (`witnessed_iff`, in margins-claude's `lean/Convex.lean`).
Either law is about a slice read alone; a slice merged into an older holding is step 5's question.
margins-claude will state the law over the model once D8a is modelled.
Kyle agreed on 2026-11-11 (issue #5): the model states *witnessed*, with convex as a corollary, and a writer's tool fills in the anchor, as it fills in `of`: an emit that sets a key names in `prev` every record its writer holds that sets that key and is still a live setter of some key. Then the live setters alone are a slice. A slice law is about a slice read alone; merging slices is step 5's question.
**Kyle's choices, 2026-11-10,** following margins-claude's reasoning on merge request !13:
- **Names:** the item field is `of` and the declaration is `sets`. A record *of* the task, against a record *about* it; `sets`, like `erases`, is a verb for what the record does.
- **Natural keys:** `of` takes a record id or a natural key, as `about` does.
  A writer can name a thing before holding any record of it, and two writers who start the same named thing join.
  An unnamed thing is rooted at its first record, and a second branch without `prev` joins it by that id.
  In an open store anyone can emit of a name and become a head; that is left to standing, where grants per thing belong.
- **Same value from two setters:** the core declares no conflict.
  The keyed fold reports each key's live setters, newest first, as whole-record `latest-by` gives a winner and the others.
  Whether two of them conflict is a reading made above the core, so the core never defines equality on payload values.
  A reading should not treat agreement as a conflict.
**Kyle, 2026-11-10, on undeclared payload keys:** refusal is progressive.
A writer refuses a payload key it did not declare only in a store declared strict.
Elsewhere the key is written, the keyed fold does not read it, and a whole-record fold, a `where` test or a lens still can.
Making it count takes a new emit that declares it, since the declaration is in the id.

**Built, 2026-11-10: D8a and D8b.**
The model gives each record one thing, `of` or its own id, and makes `about` a mention only.
Its laws are restated over the thing, and `thing.one`, `thing.mentions` and `record.of` join the index.
The vectors are regenerated; JavaScript and Rust agree on every one, and the `d8ab-1` campaign finds no mismatch.
D8c, declared keys, is built in the entries of 2026-11-14, all but its slice laws.
**Kyle's choices, confirmed 2026-11-10:**
- **Spec:** the spec says `"latest-by": "of"`. `from.of` selects a thing's records; `from.about` is a mention test.
- **Filling in `of`:** only the single-record `emit --prev` path fills `of` in, from the record it replaces.
  `emit -` fills nothing in: a changeset states `of` itself.
- **Unheld target:** the CLI refuses `prev-unknown` itself when a revision's target is not held, since it has nothing to fill `of` in from.
  On that path only, this precedes the writer's other checks.
- **Export:** `export --about` is a mention filter and `export --of` filters by thing.
- **Verb:** the verb that shows a thing is `loops of <thing>`, renamed from `loops about <thing>`.
**Implementer choices, confirmed by Kyle 2026-11-11** (there are no stores yet beyond iteration tests, so the missing compatibility path costs nothing):
- The codes `of-not-typed`, `of-missing` and `of-not-root`, and their places: `of-not-typed` right after `about-not-typed`, `of-missing` last of the envelope checks, and `of-not-root` between `prev-unknown` and `prev-unrelated`, all three across the whole batch.
- `"latest-by": "about"` is refused with `fold-argument`; there is no compatibility path.
- A reader polices neither `of-missing` nor roots. A held record with `prev` and no `of` is of its own id, so its `prev` crosses things and admission refuses it.
- `from.about` and `from.of` in one source mean both.
- A held entry in a batch vector is `{"id", "of"?, "prev"}`.

### Settled: D1–D6

The human settles each decision as its written recommendation.

- **D1:** A public lens reader refuses a base that repeats a section id, with `base-id-duplicate`.
  It checks the base before the text, so this refusal precedes every text refusal.
  The raw core keeps first-lookup behavior.
- **D2:** Text is Unicode scalar values.
  A lone surrogate is refused with `string-not-unicode` before hashing, and nothing is substituted.
  The rule covers values, member names, envelopes, and batches.
  The refusal follows `envelope-field-unknown` and precedes `kind-empty`.
  A payload text receives it only when the text is JSON by syntax.
  A fixture that holds a lone surrogate anywhere else, ignored metadata included, is malformed input in all three.
- **D3:** A number past binary64's range is refused by the writer's number check, with `number-out-of-range`.
  That check stays last in the writer's order.
  The Rust core stays finite; its input adapter carries the overflow to that check and to the spec parser.
  Reader records still require finite numbers.
- **D4:** Signing fixtures require a canonical seed of exactly 32 bytes, read only after the record is written.
  No adapter pads or truncates. This is a fixture rule, not a record rule.
- **D5:** A record input's first envelope and first payload decide its refusal, as in Lean.
  Once they are written, a later variant that names another record is malformed input.
  So is a later payload variant that is refused.
- **D6:** Four helper lemmas are proved and pinned.
  They are `Sum.on_step_missing`, `Max.acc_readable`, `Min.acc_readable`, and `Spec.mem_select`.

D2 and D3 left the code name, stage, and precedence to the human.
Kyle confirmed the landed choices on 2026-11-10:
the name `string-not-unicode` and its stage after raw shape, the reuse of `number-out-of-range`, the same overflow handling for spec arguments, and D1 refusing duplicates only.
D1's name `base-id-duplicate`, D5's refused-payload fault, and the Rust stand-in are implementer choices.
See [record rules](protocol/record.md), [the lens contract](protocol/lens.md), and [Rust boundaries](rust/GAPS.md).
D7 is built since 2026-11-20.

## Step 1 and before

In brief:
- The six step-1 rulings, R1 to R6, were applied before any record existed.
- The binary64 standards choice was closed with Option A.

### Settled: step-1 rulings R1–R6

- **R1:** Every byte string uses lowercase, unpadded base32hex.
  Readers require canonical lengths and zero pad bits.
  Proved id text order equals digest order.
- **R2:** The id preimage remains the JCS object containing `commit` and `env`.
- **R3:** Signed bytes remain `loops/record/v1:` followed by the id text, encoded as UTF-8.
- **R4:** Instants use UTC `Z`, seconds 00 through 59, and optional fractions of one through nine digits.
  A fraction ends in a nonzero digit.
  The actual parser has proved spelling uniqueness and parse-render recovery.
- **R5:** Typed references retain lowercase prefixes; sets retain UTF-16 code-unit sorting.
- **R6:** Raw envelope readers return `envelope-field-unknown` before checking values, including `kind-empty`.
  The payload remains the extension point.

See [record rules](protocol/record.md) and [the settled ledger](design/rethink/ledger.json).

### Settled: binary64 standards and writer range

The human closes the standards choice with Option A.
Rust uses native shortest binary64 digits, ECMAScript notation, and exact midpoint tie correction.
The writer refuses every magnitude above 2^53 − 1, including fractional values.
Canonical formatting covers finite binary64 independently of writer acceptance.
A Lean roundtrip proof and a shortest proof do not yet exist; neither blocks this choice.
See [Rust numeric boundaries](rust/GAPS.md) and [record rules](protocol/record.md).

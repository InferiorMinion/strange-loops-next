# Changelog

What was built and what Kyle ruled, newest first, by date.
What is next is in [ROADMAP.md](ROADMAP.md). What waits on Kyle is in [DECISIONS.md](DECISIONS.md). The long design notes are in [notes/](notes/).

Each date starts with a short list. Under it is the record as it was written at the time, moved here from `ROADMAP.md` on 2026-11-21 and changed only where a sentence pointed at a place in that file.
Since Kyle's ruling of 2026-11-21 (W7), only a choice that changes a record's bytes or id, a result's bytes or hash, a refusal code, or what a spec answers comes to him, through [DECISIONS.md](DECISIONS.md). Every other implementer choice is written here and stands unless he objects.
The record starts where the old roadmap's did. For anything earlier, and for every merge, see `git log`.

## 2026-11-22

In brief:
- The base lemma is in the model as `keyed.base`, with a correction: an emit against a base keeps the store's names anchored only when the base is closed downward. A witness shows what breaks when it is not.
- A fuzz run compares the model and Rust on ticks, with pairs of faults generated on purpose. Its first version agreed on 20,160 cases and missed a fault planted in Rust; the rebuilt one catches every planted swap tried. margins-claude's Python agrees with Rust on 80,000 more cases.
- The tick has its mutation catalogue: 91 deliberate faults, each caught by something that runs. Five were not caught at first; four vectors and a check were added for them.
- The adoption step has started: `loops ls` lists a store's kinds and things, and `loops --version` says which `loops` this is. Its plan has eight pieces; two need Kyle first (AD1, and the import of the old stores).
- `loops emit --base` fills against what the writer read, a tick or named records, so a stale write shows as a conflict and does not silently replace a setter the writer never saw.

### `loops emit --base`: the filler against a base

Ruled on 2026-11-20 ([the cut note](notes/2026-11-15-the-cut.md), section 8) and built on 2026-11-22. Rust only: `rust/place/src/fill.rs` and the CLI. The law is margins-claude's `keyed.base`, in the model since this morning. No protocol rule and no refusal code changed. Without a base, every emit on both paths writes byte for byte what it wrote before.

`--base <tick>` takes the records the tick covers; `--base-id <id>`, repeated, names records; both may be given. They work on `loops emit`, on `loops emit - --fill`, and on `loops tick`, where the base selects only the tick record's anchor and the payload still covers everything held.

**A real run, on the binary built from this change.** One thing, `svc:jellyfin`, in a store that also has a tick T written before the peer's update:

| Step | What the store then reads |
| --- | --- |
| A peer sets `port` to 9000 | `port` 9000, one head |
| A writer who read only T sets `port` to 9001 with `--base T` | `port` in conflict between 9001 and 9000, two heads |
| The same write with no base | `port` 9002, one head: the peer's value is replaced and nothing shows it |

That is the ruling's purpose: a write against an older read shows as a conflict.

**A base that does not hold what is under it** is [TK11](DECISIONS.md#tk11-a-write-against-a-base-that-does-not-hold-what-is-under-it), open for Kyle and built to its recommended option, C. The filler takes the selected records of the thing together with what is under them in the store, and tells the writer how many records it named that the base did not show. TK11's two examples, on the built binary:

- `x` sets `port` and `owner`, `y` sets `port` over it. `loops emit svc --of svc:jellyfin port=8098 --base-id y` names `x` and `y` and prints "the base did not show 1 record that this emit names". The whole store, and a store holding only the thing's live records, both read `port` 8098 and `owner` kyle with one head. With the bare base, the second store read a conflict that did not exist.
- `owner=sam` against `--base-id y` replaces `owner` kyle, which the base did not show. The line is printed and the envelope names `x`. Only refusing would stop this; that is TK11's option A, and it is one branch in the filler.

**margins-claude checked the plan against its law before it was code** and found one hole, in the coordinator's wording: a stated `--prev` outside the base named a record without what is under it. So stated `--prev` targets join the selection before "what is under it" is taken, and the count includes them.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).** The full list is choice 24 of `rust/PLACE.md`.
- Two flags, because a tick named as a record is otherwise ambiguous with the tick as a set of records.
- Everything the filler judges is judged in the base: `prev`, the keys an emit sets when "the thing is held", and the copied `about` and `occurred`. A first emit of a thing the base does not hold is a whole-read first emit, also when the store holds the thing; the CLI says "the base covers no record of" the thing.
- The base tick's payload must be held, and the envelope of every record it covers (`id-unknown` names a missing one). A covered payload need not be held. A tick under another rules version is read once the first reader accepts it.
- In a changeset, one base serves every line, and each line is filled against it and the lines prepared before.
- No warning about a stale base or a conflict: whether a store warns or declines is left for step 3.

Checked by a randomized test: 320 cases, 217 with a base that did not hold what is under it, 67 with the exact shape of TK11's first example, 118 stating a `--prev` outside the base. In every case the write equals the one a writer holding exactly the effective base would make; the thing reads the same from the store's live records as from the whole store, before and after; and the whole store reads every key as it would after the bare write.

A Codex worker planned and wrote it; margins-claude checked the plan against the model; the coordinator reviewed it, joined it with main and made the runs above.

### The adoption step: its plan, `loops ls` and `loops --version`

Kyle ruled on 2026-11-21 (W1) that the adoption step's CLI pieces are built beside the tick work. Its exit is `loops` first on `PATH` for his daily work.

**The plan** (a Codex worker's, 2026-11-22), as eight pieces: (1) `loops ls`, with things and a scan by the start of a name; (2) a plain-text view with relative times, and `--fields`, on `ls` and `of`; (3) the plain view on `read` and `fold`; (4) reads across several stores; (5, 6) the import of the old stores as a shadow; (7) the cache of checked segments (W2); (8) `--version`, and the walk that puts `loops` first on `PATH`. Pieces 4 to 6 need Kyle first: [AD1](DECISIONS.md#ad1-how-a-spec-finds-the-stores-it-names) is written, and the import's choices, which decide the id of every imported record, are owed as an entry. The others are the CLI's alone.

**Built: `loops ls` and `loops --version`.**

- `loops ls` answers the kinds a store holds with their counts of settled, admitted records. `loops ls --things` answers every thing: its name (a natural key, or a root's id), the kinds of its heads, its records, its heads, and its headline. `loops ls <prefix>` answers the things whose name starts with that text, literally: `task:` for natural keys, `r:ab` for roots.
- The headline is the thing's `name` when one live setter holds it as text. Otherwise it says which of three it is: absent, unknown (the setter's payload is not held), or in conflict (several live setters). It never shows one side of a fork.
- A real run on the stores of the entries below: the laptop of the tick example answers `backup` 6 and `tick` 2; a store with `task:kiln` answers its headline "Repair kiln"; the thing left with two heads by a write against a bare base answers 2 heads.
- On 10,000 records in 2,000 things, `loops ls` takes 0.58 s and `loops ls --things` 0.60 s; nearly all of it is checking signatures, which piece 7 is for.
- `loops --version` prints `strange-loops-next 0.1.0`, outside a store and with no key. The first line of `loops --help` is unchanged: an outside tool reads it.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).** The full list is in `rust/PLACE.md`.
- `ls` with no argument lists kinds, not things, so the first command in an unfamiliar store is short at any size.
- A thing's name here is what `of` names: a mention in `about` and a payload's `name` make no thing name.
- Two live setters of `name` are a conflict also when they wrote the same text, as `loops of` shows them.
- JSON only so far. The plain view is the next piece, and it will be asked for with `--plain`, never chosen by whether the output is a terminal: a script's answer must not depend on how it was started.

A Codex worker wrote both; the coordinator reviewed them, ran the gate and tried `ls` on real stores.

### The tick's mutation catalogue

Kyle's ruling W5 held the tick's mutants back until step 4's exit, which holds since 2026-11-21. The catalogue now has 419 entries: the 328 from before, unchanged, and 91 for the tick, over `SpecHash.lean`, `Tick.lean`, `TickCheck.lean`, the result reader in `Result.lean`, `env.when` in `Spec.lean` and the repeat check in `Json.lean`.

Every one of the 91 was judged by the full runner on this machine, with ten minutes for each command:

| Outcome | Entries |
| --- | ---: |
| Rejected by the build, and also caught by something that runs | 66 |
| Builds, and a vector changes | 24 |
| Builds, and a runtime check fails | 1 |
| Survives, or stands on proofs alone | 0 |

**Five were not caught at first, and each was a real gap.** Two survived outright and three stood on proofs alone. All five change what a reader or a checker answers, so each got something that runs:

| Fault | What now catches it |
| --- | --- |
| Rules versions compared as text in the check (`"10"` before `"2"`) | Vector `newer-ten-under-two`: a version 2 checker meets a rules 10 tick |
| An unheld payload answered before the wrong kind | Vector `wrong-kind-unheld`: a note whose payload is not held is still `tick-kind` |
| A count with a leading zero accepted (`"06"`) | Vector `check-padded-count`, with the hash refitted |
| A covered tick's own `covers` followed | Vector `weekly-inner-records-held`: the checker holds the six backups too, and the weekly tick still counts three records |
| The claim settled over the whole holding, not the covered set | A runtime check on `tickRun`: an eraser outside the cover does not change the covered result's bytes |

So `vectors/tick/` has 131 files. Rust passed the four new ones unchanged.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).**
- Eight of the new entries join the sample that every merge request runs (58 in all), spread over five files.
- One swap that cannot be an entry: an empty `withheld` against a `withheld` outside `covers` reads the same either way round (margins-claude found this in its own fuzz).
- The evidence keeps the five first verdicts beside the final ones, so the gaps stay on record.

A Codex worker wrote the entries, ran them and added the vectors; the coordinator reviewed the run, joined it with main and ran the gate.

### The base lemma: an emit against a base

The filler against a base (the cut note, section 8, ruled 2026-11-20) names the base's live records of the thing. This is the law for it, in `model/Laws/LiveSlice.lean`, indexed as `keyed.base`. Model only: no vector, no Rust, no protocol rule.

- **What is proved.** A base is a part of what the store holds. An emit that names only records of the base, and every record of the base live in it, is the emit a holder of exactly the base would have written. When the base holds what the writers of its records held, the store with the emit is still a closed holding whose writers followed the rule (`Anchoring.emit`, `ClosedOver.emit`), so its names are anchored (`anchored_emit`) and `keyed.anchored` applies: its live records, read alone, read every key as it does. A base closed downward is such a part (`anchored_emit_down`, from `closedOver_of_down`). So is a part of one that holds every record live in it (`anchored_emit_live`): a thing's live records at a cut, alone, are a sound base though they are not closed downward.
- **Naming more of what is under the emit** (`liveK_emit_more`). Two emits that set the same keys, the second naming what the first names and besides only records under a name of the first: every record of the store is live for a key with the one exactly when it is with the other. So an emit that names the live records of the base's downward closure, and not of the bare base, reads the same from the whole store and adds only what a reader of a part needs.
- **A correction to what was claimed.** The tick record note, sections 4 and 7, said the lemma holds for any set, closed or not. margins-claude wrote that and it was wrong. Of the rule's three parts, two are the emit's and the third, that a writer holds what the writers of its records held, is about the base. The note is corrected in place, marked with the date.
- **The witness** (`base_closed_needed`). `x` sets `a` and `b`; `y` sets `a` over it and names it. The store is anchored and closed. A writer takes `y` alone as its base and writes `w`, setting `a` and naming `y`. `x` is still live, for `b`, and is under `w` unnamed. The live records are `x` and `w`; read alone they show `x` as a live setter of `a` beside `w`. The whole reads `w` alone.
- **What follows for the filler**, not built here: naming the live records of the base's downward closure in the store, and not of the bare base, is always sound by `anchored_emit_down`, and by `liveK_emit_more` it replaces nothing the bare emit does not. What the filler does with a base that is not closed is not decided.

margins-claude wrote it.

### The tick fuzz: holding Rust to the model's order

The order of refusals for input that is wrong twice is pinned in no protocol text (W4). Inside this repository the model's order decides, and a fuzz run is what holds Rust to it. The JavaScript is frozen (W3), so this run compares Lean and Rust only: `node interop/fuzz/run.mjs --family tick`.

- **The first version was not good enough, and a planted fault showed it.** It agreed on 20,160 cases over three seeds. The coordinator then swapped two checks in Rust's reader (the spelling of `spec-hash` and of `result-hash`), rebuilt, and ran a full seed: all 6,720 cases still agreed. The swapped Rust differs from the model only on a payload with both hashes misspelled, and the generator misspelled one or the other, never both. An order between two checks is exercised only by a case that holds both faults, so the number of agreeing cases measured nothing about order.
- **The rebuilt version generates pairs on purpose and counts them.** `interop/fuzz/tick-order.mjs` lists 114 decision points of the readers and the check, in the model's order. Of their 6,441 pairs, 5,784 can be in one input; the other 657 are listed with the reason each cannot. Every pair that can was generated and confirmed by the model's answer at least three times, at least once in each seed, and the evidence test asserts it.
- **The campaign.** 46,656 cases over three seeds (`tick-order-1` to `3`), 30,456 of them refusals: Lean and Rust agree on every one. The check now runs every fold type: 1,384 `same` and 803 `differs` answers, where the first version had 100 and 8.
- **Planted swaps.** Fourteen pairs of adjacent checks were swapped in Rust, one at a time, by the worker: each was caught, the latest at case 219 of a seed. The hash-spelling swap that the first version missed is caught at case 58. The coordinator planted two more of its own in the result reader (the order of a latest-by slot's members, and which missing member of a scalar slot is named first), with its own seed: both caught within 1,536 cases, and the restored tree runs clean.
- **An outside run agrees.** margins-claude fuzzed its Python against Rust (`sl-py/fuzz_tick.py`, in its own repository): 80,000 cases over two seeds with up to four faults each, none differing, every pair of its 59 faults at least 20 times a seed. Of 17 swaps planted in its Python, 16 were caught; the 17th, an empty `withheld` against a `withheld` outside `covers`, is the same either way round. Its base ticks have only count, sum and collect slots, so it does not reach the order inside groups or keyed things; this run does, and two of the planted swaps are there.
- **In CI.** The fuzz job runs 288 cases of the lane (about 15 seconds). The thinnest catches need a few thousand cases, so the CI run is a smoke test and the campaign above is the evidence.
- **One vector owed to margins-claude:** `outside-erased-taken`: a record outside the cover that the spec takes, erased by an eraser also outside, counts in "taken and not covered". 127 vectors in `vectors/tick/`. Rust passed it unchanged.

No disagreement between the model and Rust was found by either version, so no Rust or model change came of it. The existing fuzz runs are unchanged: their generated inputs are byte for byte what they were.

A Codex worker wrote both versions; the coordinator reviewed them and planted the faults named above.

## 2026-11-21

In brief:
- `loops read --values` and `fold --values` give each listed record's envelope (issue 9, !57).
- The Rust core reads through indexes: a keyed collect over 9,000 records went from 19.8 s to 0.63 s, and no answer changed (!58).
- margins-claude wrote the payload values note (!59). Its seven choices are open.
- The roadmap was split into `ROADMAP.md`, `DECISIONS.md`, this file and `notes/`.
- A result hash is spelled `res:`.
- The model states how a stored payload is read, and a list reads as a set: `present` means written, `is` and `by` read a list's members, and a list is never a number.
- `value` and `n` are ordinary payload fields: a record that does not hold one is unread, absent and ungrouped for it, as for any other name.
- Kyle ruled the seven payload values choices, the result hash's prefix, which choices come to him, and the three classes of edge; he confirmed the keyed-fold and codec choices; issue #10 is closed.
- The tick record is proposed as TK1 to TK9 in `DECISIONS.md`, with [a note](notes/2026-11-21-tick-record.md) (!65). Not ruled.
- `loops emit` copies `occurred` from the heads when they agree; `--no-occurred` states none.
- The roadmap, the decisions, the changelog and the notes are published as a fifth design page, and the rethink and grammar pages are brought up to the rulings since 2026-11-13.
- The full mutation catalogue ran over all 328 entries: 324 caught, two known survivors, and two new ones that are now caught. The full runner asks what the per-merge check asks.
- `loops emit - --fill` fills each line of a changeset as a single emit would, so a collector can write anchored records.
- Kyle ruled the tick record (TK1 to TK9; TK9 as a `rules` member bound into the spec's hash) and the order of work (W1 to W6), each for its recommendation. One new entry, TK10, comes from planning the build.
- The tick record's first piece is in the model: the spec's hash under a rules version, and the window field `env.when` (!72). Rust is being built.
- The tick record's second piece is in the model: a reader for a claimed result, and the tick's payload with its two readers.
- Rust has the spec's hash and `env.when`: `loops fold` takes a window on `env.when`, and Rust runs the 11 tick vectors it implements.
- The tick record's third piece is in the model: what a tick claims, how one is made, and the check with its answers.
- Rust has the tick's payload and the result reader, and runs 72 of the 113 tick vectors.
- The tick has its protocol page, `protocol/tick.md`. margins-claude's Python ran every tick vector from the prose and found two gaps in the readers; both are closed in the model and in Rust.
- Rust checks a tick: all 126 tick vectors run in Rust, none pending.
- `loops tick <spec>` writes a tick. Two stores holding the same six backups, with different keys, wrote payloads equal byte for byte.
- `loops tick --check <id>` checks a tick. A tick written on one machine checks `same` on a second: step 4's exit criterion holds. Closing the step is Kyle's to say.

### `loops tick --check <id>`, and step 4's exit criterion

The second half of piece 7, the last. The verb resolves an id as `get` does, takes the record's payload only when the store's own reading holds it open, checks it against everything the store holds with the core's `check_tick`, and prints one JSON object: the answer and the two reports, spelled as the vectors spell them.

**Step 4 is done when a content tick recomputes to the same hash on two machines.** A real run on the release binary, continuing the two stores of the entry below. The laptop's tick `r:uqmk623s…` covers the six backups.

| Where it is checked | Answer | Exit |
| --- | --- | --- |
| The NAS, after importing the tick | `same`, closure `yes`, 0 taken and not covered | 0 |
| The NAS, after a seventh backup of 4 megabytes | `same`, 1 taken and not covered. A fresh tick there: 7 runs, 1031 | 0 |
| A machine holding four of the six | `cannot check`: 2 records, 0 payloads missing | 1 |
| A copy where one backup was erased afterwards | `cannot check`: 0 records, 1 payload missing | 1 |
| The laptop, on a tick written by hand that claims 1028 | `differs`, with both hashes. The store took the record without complaint | 1 |

Each line is the answer the proposal of this morning predicted for it (TK4, TK5, TK7). The same cases are CLI walk tests, the first under the name `a_tick_written_on_one_machine_checks_same_on_a_second`. Whether step 4 is closed is Kyle's to say: boundaries, markers, the runner and cuts by time or by ids were never in its exit and are not built.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).** The full list is choice 22 of `rust/PLACE.md`.
- **Exit status 0 only for `same`.** `differs`, `cannot check` and `not a tick` exit 1 with the answer on standard output and nothing on standard error, so `loops tick --check <id> && …` is safe in a script. The alternative was 0 whenever the check completes. Kyle is asked.
- **No rule at the door.** `import` and `emit` take a tick that would answer `differs` or "not a tick"; tests show both.
- **A store whose lines do not all check** answers and is then refused `store-not-clean`, as the other answering verbs are.
- **Checking needs no signing key.**
- The vector evaluator's function that spells an answer is now public (`loops_eval::tick_check_json`), so the CLI and the vectors share one spelling.

A Codex worker wrote it; the coordinator reviewed it, ran the gate and made the run above.

### `loops tick <spec>` writes a tick

The first half of piece 7. The verb reads a spec as `loops fold` does, takes every record the store holds, makes the tick with the core's `make_tick`, and writes one signed record of kind `tick`. It prints the record's id. The verb that checks a tick is the next piece.

A real run, on the release binary built from this change. The laptop holds the proposal's six backups; the NAS imports them and has its own signing key. Each runs `loops tick backups.json` before it has the other's tick.

| | Laptop | NAS |
| --- | --- | --- |
| Record id | `r:uqmk623s…` | `r:8vphpr9i…` |
| Observer | `k:qunercaf…` | `k:8lbav2tp…` |
| Payload | 1,109 bytes | the same 1,109 bytes |
| In it | `rules` "1", six `covers`, no `withheld`, `runs` 6, `mb` 1027, `spec:gs9soetl…` | the same |

Both hashes in the payload recompute from the payload in a few lines of Python. `spec:gs9soetl…` is the hash the proposal worked out by hand.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).** The full list is choice 21 of `rust/PLACE.md`.
- **The envelope is the writer's, as on any record.** The record is what `loops emit tick` would write with this payload and the same envelope flags: it goes through the same filler and writer, and a test shows the two give the same signed record for the same key, salt and time. So `loops tick <spec>` alone writes a root, and `--of`, `--prev`, `--about`, `--occurred` and the rest of `emit`'s envelope flags mean what they mean there. A second tick `--of tick:backups` names the first in `prev` through the anchor.
- **Everything held, and only that.** No cut by time or by ids, and no slice. A second tick covers the first as one record.
- **The tick is made before its own record is appended**, so it never covers itself.
- **How the spec is named.** `-` is stdin. A path, or a name that is a file under the directory, is read as a file. Any other bare name is a vertex under `.loops/vertices/`.
- **A payload erased here is withheld**, also when its bytes are still in a segment: the store's own reading decides (TK10, row 1).
- A store whose lines do not all check is handled as `emit` handles it: it warns, and the tick covers the lines that check.
- A spec that does not parse writes nothing and is refused with the spec's code. No new refusal code.

A Codex worker wrote it; the coordinator reviewed it, ran the gate and made the run above.

### The tick record: Rust for the check

Piece 6 of the seven. `rust/core/src/tick_check.rs` has the claim's run, what a checker holds open, the missing counts, the covered eraser a tick ignores, closure, the count of records taken and not covered, and `check_tick` with the model's answers in the model's order.

- **Rust runs all 126 tick vectors, none pending.** With margins-claude's Python, the claim and the check now have three implementations that agree: the model, Rust, and one written from the prose.
- **`differs` is decided on bytes** and carries both hashes, as in the model: a test uses a hash that collides on purpose.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).**
- The checker is its own module in the core, and the fixture's JSON spelling of an answer stays in the vector evaluator.
- `rust/GAPS.md`: the one boundary is the evaluator's, as before. A number past binary64's range in a fixture's subject payload or openings is refused as malformed fixture input where the model answers; no stored record can hold one.
- No CLI change here: the verbs are their own piece.

A Codex worker wrote it; the coordinator reviewed it and ran the gates.

### The tick record, piece 4: the protocol page, and two amendments from an outside reading

**The outside reading.** margins-claude wrote the tick's payload and the result reader into its Python (`sl-py`, in another repository) from the prose alone, then ran the vectors: the 11 of piece 1 agreed on the first run, and of the 72 after piece 2, one answer differed. It listed ten things the prose had left it to guess. Two were gaps in the model.

- **Amendment A: a repeated member name at any depth is refused before anything is hashed.** The model's canonical text keeps the first of two members with one name, so a `spec` or `result` with a repeat inside had a hash, and two different texts fitted one `spec-hash`. JCS does not define the text of such an object. The first reader now refuses a repeat anywhere in the payload, `tick-member-duplicate` at the repeated member's path. No stored tick could hold one, since a record's payload is refused for it already; the reader now says the same. One vector, `result-repeated-slot`, changes its answer to this code.
- **Amendment B: the result's hash under newer rules** (TK10, row 7; Kyle's to rule, built as recommended). The label inside the result's hash has its own version, and a binary cannot know a later one. For a `rules` above its own, the first reader checks `result-hash` for spelling only. The reader takes the binary's version for this.
- **`protocol/tick.md`.** The normative page: the record and its payload, the spec and the rules, what a tick claims, reading a payload, the check, reading a tick in a spec, erasure, and the vectors and laws. It says what the outside reading had to guess: `covers` is every record of the set folded, taken by the spec or not, and `withheld` every one of them not held open; the second reader's three answers as the vectors spell them; "not canonical" compares JCS texts, so member order is free; how a hash is spelled; that a path is for a person and is not to be parsed. Each refusal has one fault; the page pins no order for several.
- **A law, under `tick.canonical`.** A payload the first reader accepts has no repeated name at any depth. Two accepted payloads with one canonical text are then equal as JSON up to member order, when their numbers are finite, as a stored record's always are. Without that premise it is false: `null` and a number past binary64's range have one canonical text in the model.
- **Vectors.** 13 more, 126 in `vectors/tick/`: repeats inside `spec` and deep inside `result`; a rules 2 payload whose result hash is under another label; a supplied record the spec does not take (covered all the same); one shown erased (withheld); `"unread": "-0"` with a refitted hash.
- **Rust** follows in the same merge request, since the amendments change answers Rust already gave: the reader's two changes, and `make_tick` in the core, which `write-payload` now uses. Rust runs 80 tick vectors and counts 46 as pending, those of the claim and the check.

- **A second outside reading, of the page itself.** With the amended readers and the check, margins-claude's Python passed all 126 vectors on its first run. It then listed what `protocol/tick.md` had still left it to take from this changelog or from a vector, and the page now says each: an older tick is spared everything past the first reader; how today's comparison is spelled, with its four reasons; that the answers about the version are given only for a payload the first reader accepts; that an ignored covered eraser does not wait for missing records; when closure is unknown; what the count of records taken and not covered counts; and that outside the stored record's domain of finite numbers and well-formed text no answer is defined, the model's included. One vector it asked for is still to add: an outside record that is erased and that the spec takes, which counts.

**The order for a payload with several faults, as now built** (inside this repository only, W4). The first reader: not an object; a repeated name anywhere, found by visiting members in input order, a name before what is inside its value, array elements by index; an unknown member; the first missing member; the version; `spec` not an object; `covers`, then `withheld`; the spelling of `spec-hash`, then of `result-hash`; the spec's hash; the result's hash when the rules are the reader's or older. The result reader: for an object of fixed members, repeats, then extra members in input order, then missing ones in the shape's order (at the root, the spec's slot order); values slot by slot in the spec's order. Inside a slot: `unread` before `value` or `heads`; a grouped slot reads `groups`, `ungrouped`, `unread`; latest-by reads `unread` before `things`; a keyed thing reads `heads`, `keys`, `whole`.

**Implementer choices; they stand under Kyle's rulings of 2026-11-21 (TK7, W4, W7).**
- Amendment A moves one refusal from `tick-result-invalid` to `tick-member-duplicate`. The alternative was to say the readers take a held payload, which can hold no repeat, and leave two vectors about input that cannot occur.
- In Rust, the second reader uses the version given to the first, so a caller cannot change versions after the result hash's fit was left unjudged.
- No mutation-catalogue entry is added (W5). `model/Loops/Number.lean` gains a name for its decimal formatting step so a proof can use it; no number's text changes.

Codex workers wrote the model and the Rust; the coordinator reviewed both, ran the gates and joined the two branches.

### The tick record: Rust for the payload and the result reader

Piece 5 of the seven, taken before piece 4 because it needed nothing from the protocol page. `rust/core/src/tick.rs` has the payload, its JSON and bytes, and the two readers; `rust/core/src/result.rs` has `read_result` and the result hash over a raw member. The checks run in the model's order.

- **Rust runs 72 of the 113 tick vectors and counts 41 as pending**: those of the claim and the check, whose Rust is piece 6. The evidence page credits Rust for `tick.canonical` and `tick.record` now.
- **The seventeen refusal codes** are in the core's refusal vocabulary beside the spec's.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).**
- **The core's types hold nothing a stored record cannot hold.** The worker's first version added a variant for a number past binary64's range to the core's JSON and value types, to match the model on inputs outside the vectors. The coordinator had it taken out: the writer refuses such a number, so no tick's `spec` or `result` can hold one. The boundary stays in the vector evaluator, where it was, and `rust/GAPS.md` lists the fixture inputs on which the evaluator refuses and the model answers (an out-of-range number in a supplied payload; a repeated name inside a raw member that would be echoed). The coming Lean and Rust fuzz run leaves those out.
- No CLI change yet: `loops tick` is piece 7.

A Codex worker wrote it in three rounds; the coordinator reviewed each, brought it up to main and set the run and pending counts.

### The tick record, piece 3: the claim and the check, in the model

The third of the seven pieces, in `model/Loops/TickCheck.lean`. With it the model holds the whole tick record. Rust does not run its vectors yet.

- **The claim** (TK4, TK5). Take the forms of the covered records, read every form of a withheld id without its payload, settle and admit inside that set alone, and run the spec. Nothing follows a covered tick's `covers`.
- **Making a tick.** `covers` is every record given. `withheld` is every one of them not held open, the targets of a covered eraser among them.
- **The check** (TK7, TK9, TK10). In the order the model decides:
  1. The subject is not of kind `tick`: not a tick (`tick-kind`).
  2. Its own payload is not held: cannot check.
  3. The first reader refuses: not a tick, with its code.
  4. Newer rules: cannot check. Older rules: cannot check, with what today's rules give: the same result, another result, or why that is not available.
  5. A spec whose hash fits and that the parser refuses: cannot check. A spec not in its parsed form, or a result of the wrong shape: not a tick.
  6. A held covered eraser names a covered record that `withheld` leaves out: not a tick (`tick-withheld-erased`), whatever else is missing.
  7. Covered records not held, and payloads the tick read that are not held, are counted; if either count is not zero: cannot check, with both. A payload counts as held only when the checker's own reading shows it, so an eraser anywhere in the store takes it away.
  8. The claim is run and its bytes compared: `same`, or `differs` with both hashes.
- **The two reports** beside the answer. Closure: "no" as soon as one held covered record names a `prev` or `under` target outside `covers`; "yes" when every covered envelope is held and none does; else "unknown". Taken and not covered: the records outside `covers` that the spec's sources take from the whole store, read as `loops fold` reads it.
- **Laws.** `tick.inputs`: two holders with consistent forms, each holding what step 7 asks, get the same result bytes. `tick.withheld`: holding the payload of a withheld record changes nothing. `tick.made`: a tick checked against the holding it was made from is `same`, and stays `same` when records arrive that are outside its cover and erase nothing in it. `tick.level`: the claim over a set that contains a tick does not depend on what that tick covers. None assumes anything of the hash function beyond its width.
- **Vectors.** 41 more, 113 in `vectors/tick/`. The proposal's examples are among them: the six backups check `same`; a seventh backup leaves that `same` with one record taken and not covered, and a fresh tick is 7 and 1031; a covered `retired` record gives 5 and 615; a withheld payload gives 615 with 1 unread; a weekly tick over two nightly ticks checks `same` with no backup held; the smallest count under older rules was 0 and is 9 with 2 unread today.

**Implementer choices; they stand under Kyle's rulings of 2026-11-21 (TK7, W4, W7). Rows 1 to 4 and 6 of TK10 are built as recommended and are his to rule.**
- Two more refusal codes: `tick-kind` and `tick-withheld-erased`.
- For a tick under older rules, "not available" names its reason: today's parser does not read the spec, the spec is not in today's parsed form, the tick ignores a covered eraser, or inputs are missing (with the counts).
- The taken-and-not-covered report is given whenever a spec was read, also beside a refusal of the result's shape; closure is "unknown" until the first reader has passed.
- `differs` compares bytes, not hashes: a runtime check uses a hash that collides on purpose and still gets `differs`.
- No mutation-catalogue entry is added (W5).

A Codex worker wrote it; the coordinator reviewed it and ran the gates.

### The tick record: Rust for the spec's hash and `env.when`

Rust for piece 1. `rust/core/src/spec_hash.rs` has the rules version, the label and the hash over a raw JSON member or a parsed spec. `env.when` is a test field in `rust/core/src/where_.rs`, so `loops fold` and `loops read` take a window on it today.

A real store, in the CLI's walk tests: three backups emitted on 2026-11-21. The first states no `occurred`, a revision of it states `occurred` 2026-11-16, and a third states 2026-11-17. A window on 2026-11-21 counts 0 by `env.occurred`, 3 by `env.emitted` and 1 by `env.when`. A slot that follows `prev` counts 0 with the same window, because the backdated revision replaces the one record inside it.

- **Rust runs 11 of the 72 tick vectors and counts 61 as pending.** A tick vector is run when its `input.op` is one the evaluator implements (`hash`, `domains`, `window`). The vector test pins both numbers, so a vector cannot move between them unseen. The pending ones are the payload's, whose Rust is piece 5.
- **The evidence page credits Rust by operation.** `design/laws/laws.json` names the tick operations Rust runs, and a law is marked as checked by Rust only through vectors of those operations.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).**
- A number past binary64's range inside a raw spec member is read as `null` by the vector evaluator when it hashes the member, as the model's canonical text writes it. No stored record can hold such a number, since the writer refuses it; this keeps Rust and the model equal on a vector's input.
- `"by": "env.when"` is refused `by-invalid`, the existing code for a `by` that names no grouping key.
- No new dependency, and no entry in `rust/GAPS.md`: nothing the model states here is left unchecked by Rust.

A Codex worker wrote it; the coordinator reviewed it, brought it up to main (where the payload's vectors had arrived), set the run and pending counts, and made the evidence page's credit follow the operation.

### The tick record, piece 2: the result reader and the payload, in the model

The second of the seven pieces. Rust does not run its vectors yet.

- **The payload** (TK1 to TK5, TK9). `model/Loops/Tick.lean`: the members `rules`, `spec`, `spec-hash`, `covers`, `result`, `result-hash`, and `withheld` only when not empty. The six-backups example is a vector with its full text: 6 runs, 1027 megabytes, `spec:gs9soetl…`.
- **Two readers.** The first checks what no version of the rules changes: the members, the version's spelling, that `spec` is an object, the two id lists (sorted sets of record ids, `withheld` inside `covers` and never written empty), the spelling of both hashes, then that `spec-hash` fits the raw `spec` under the label `rules` names and `result-hash` fits the raw `result`. It parses no spec. The second is for the binary's own version: it parses the spec, requires its text to be that of its parsed form, and reads the result against the slots. For another version it answers which one; for a spec whose hash fits and that the parser refuses, it answers that it cannot read the spec.
- **The result reader.** `readResult` in `model/Loops/Result.lean` reads a claimed result against a spec's slots: the shape of each fold type, integers as decimal text, ids as record ids. It does not ask whether the parts agree. A result that says 6 runs and lists 5 ids reads; the check of piece 3 will answer `differs` for it (TK10, row 4).
- **Laws.** `tick.canonical`: what is written reads back, and whatever reads has one JCS text, reordered members included. `tick.record`: a spec reads a tick record by the payload reading and nothing more: `rules` and the hashes as text, `covers` and `withheld` as sets of ids, `spec` and `result` present with no value.
- **Vectors.** 61 more in `vectors/tick/`, 72 in all: the example, an empty cover, an empty fold, each result shape, three ticks read by a spec (the TK6 example: count 3, 2 covering the first jellyfin backup, a sum of `result` that is 0 with 3 unread), a payload under rules 2, and one file for each single fault.

**Implementer choices; they stand under Kyle's rulings of 2026-11-21 (TK7, W4, W7).**
- **Seventeen refusal codes, each with the path of what broke:** `tick-not-object`, `tick-member-duplicate`, `tick-member-unknown`, `tick-member-missing`, `tick-rules-invalid`, `tick-spec-not-object`, `tick-ids-not-array`, `tick-id-invalid`, `tick-id-duplicate`, `tick-ids-unsorted`, `tick-withheld-empty`, `tick-withheld-outside`, `tick-hash-invalid`, `tick-spec-hash-mismatch`, `tick-result-hash-mismatch`, `tick-spec-noncanonical`, `tick-result-invalid`. One code serves both id lists and both hashes; the path says which. The plan had a code for each member; the worker built the shorter set and the coordinator kept it.
- **The order for a payload with several faults** (inside this repository only, W4): not an object; a repeated member; an unknown member; the first missing member in the order `rules`, `spec`, `spec-hash`, `covers`, `result`, `result-hash`; the version; `spec` not an object; `covers`, then `withheld` (not an array, a bad id, a repeat, the order; then empty, then outside `covers`); the spelling of `spec-hash`, then of `result-hash`; the spec's hash; the result's hash. Then, for the binary's own version: the spec does not parse; its text is not its parsed form's; the result's shape, slot by slot in the spec's order.
- **A result's ids read back as held.** The codec drops what a reader held of each listed id, so the reader gives each the same value and `tick.canonical` is stated up to that.
- **The example's hashes are the vector's own.** The vector uses the model's fixed record ids, so its result hash (`res:7s24f6bt…`) is not the one in the proposal, which used the laptop's ids.
- Rows 3 to 6 of TK10 are built as recommended and are Kyle's to rule.
- No mutation-catalogue entry is added (W5).

A Codex worker wrote it; the coordinator reviewed it, ran the gates and recomputed the example's two hashes.

### The tick record, piece 1: the spec's hash and `env.when`, in the model

The plan for the tick record has seven pieces: (1) the spec's hash and the window field, (2) the result reader and the tick's payload, (3) the claim and the check, (4) the protocol text, (5) the Rust codec, (6) the Rust check, (7) the CLI verbs and the two-machine exit test. This is the first, in the model (!72). Rust for it is being built.

- **The spec's hash** (TK3, TK9). `model/Loops/SpecHash.lean`: a rules version, the label `loops/spec-hash/v<rules>:`, and `spec:` + base32hex of SHA-256 over the label and the JCS text. It is taken of a raw JSON member, so a check can verify it before parsing the spec, and of a parsed spec. The `backups` example hashes to `spec:gs9soetl…`, as the proposal computed by hand.
- **The window field** (TK8). `env.when` in a `where`: a record's `occurred` when it states one, else the instant of its `emitted`. It is always present and is read from the envelope, so also on a record held without its payload.
- **Laws.** `spec.hash`: two parsed specs have the same hash input exactly when the rules version is the same and they differ only in the order of their slots, of a `where`'s fields, or of a field's operators; such specs give the same result bytes. `spec.no-commit` and `spec.no-result`: a spec's hash input is never a record's commitment input or a result's. `window.span` and `window.heads`: what `env.when` reads, and that a window filters the heads, so a revision outside the window still replaces a record inside it.
- **Vectors.** Eleven in `vectors/tick/`: the example hash; member order giving one hash; source order, kind order, a renamed vertex or store, and the rules version each giving another; a dropped `note`; the label's bytes; and two window cases. A backfilled store counts 0 by `env.occurred`, 8 by `env.emitted` and 1 by `env.when` for the same day.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7) unless noted.**
- **A rules version is a positive whole number as decimal text**, compared as a number. This one is row 5 of TK10 and is his to rule.
- **The vectors are published with the model**, before Rust runs them. The Rust vector test and the JavaScript check each name the one directory `tick`, pin its file count and print that they do not run it; every other unknown directory still fails. Rust's exemption goes when its piece merges. The plan would have held the vectors back until then; publishing them lets margins-claude's Python start from them.
- **Reading a carried key later is not a change of rules.** A tick carries the parsed form, which drops carried keys, so earlier ticks keep their meaning. `protocol/spec.md` says so.
- **`env.when` is a test field only.** `"by": "env.when"` is refused.
- **The window's laws are named `window.span` and `window.heads`.**
- **The fuzz does not generate `env.when`.** Its field names come from a fixed list. Given by hand to the frozen JavaScript, a spec with `env.when` is refused `where-invalid`, which is what W3 means: the three-way fuzz stays on the vocabulary all three share.
- No mutation-catalogue entry is added (W5).

A Codex worker planned the seven pieces and wrote this one; the coordinator reviewed it, ran the gates and recomputed the 17 hashes of the vectors in Python.

### Ruled by Kyle, 2026-11-21: the tick record and the order of work

Each was an entry in `DECISIONS.md`, with an example. Kyle ruled for the recommendation every time. The rulings reached the coordinator through Neon, which Kyle told in its own session to send them. Kyle confirmed them in the coordinator's session the same evening, by asking for this record to be merged.
The entries as they were put are kept in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put) and [the order-of-work note](notes/2026-11-21-order-of-work.md).

**The tick record, TK1 to TK9.** The exact form is in [the note](notes/2026-11-21-tick-record.md), sections 1 to 6. Not built yet.
1. A tick is an ordinary record of kind `tick`. Its payload is the whole tick, and it has no id but its record id.
2. It carries its result and the result's hash.
3. It carries its spec as parsed, and the spec's hash. The store names in `from` select nothing in a tick.
4. `covers` lists every record that was folded, each id once. A tick among them is one record and is never opened up.
5. `withheld` lists the covered records read without a payload, and is left out when empty.
6. A later spec reads a tick by the payload reading and nothing more. A total across ticks is not in step 4.
7. A verb checks a tick and answers `same`, `differs` or `cannot check`. No store refuses a tick on import or sync.
8. The window field is `env.when`.
9. A member `rules` states the version of the rules, and the label of the spec's hash is made from it (option D). With it:
   - The rules are everything between the covered records and the result's bytes: settling, admission, what a source takes, the tests, the folds and the payload reading.
   - A new fold type or test does not raise the number. A binary that cannot parse a spec whose hash fits answers "cannot check: this binary cannot read the spec". A fault in one implementation is not a change of rules.
   - For a tick made under older rules, the check also says what today's rules give, the same result or another.

   Neon's addition for the build: the check verifies `spec-hash` over the spec's JCS text before parsing it; a spec that parses and is not in its parsed form is not a tick.

**The order of work, W1 to W6.**
- **W1.** An adoption step comes after ticks and before standing. Its pieces that touch only the CLI (`ls`, the prefix scan, the plain view, reads across stores, the shadow import) are built beside the tick work. The caching reader's law (issue #8) leaves step 4 and is stated when a caching reader is built; this amends the ruling of 2026-11-11 that it is step 4's first law. Reads across stores take the union of the named stores first.
- **W2.** A cache of which segments were checked, in the adoption step.
- **W3.** The JavaScript is frozen at what it has, as a trial for step 4: ticks are built in Lean and Rust. It keeps passing its vectors. Looked at again when ticks are done. The workbench page will use the Rust core compiled for the browser.
- **W4.** One refusal for every input, in the order the model checks; the implementer picks the order and notes it here. The protocol text and the vectors of a new step pin one fault each. The orders already pinned for records and specs stay.
- **W5.** The mutant check on every merge request stays. Step 4's mutants are added after its exit.
- **W6.** `out/` stays committed.

Open in `DECISIONS.md`: TK10, six details of the check that the plan for the build found.

### A changeset can ask for the filler: `loops emit - --fill`

Ruled on 2026-11-20 ([the cut note](notes/2026-11-15-the-cut.md), section 7) and built on 2026-11-21. Rust only: `rust/place/src/fill.rs` and the CLI. No protocol rule and no refusal code changed, and `loops emit -` without the option writes what it wrote before, byte for byte.

With `--fill`, each line gets what `loops emit` would fill in for the same statement: `of` from its `prev`, the keys its payload names, every live record of its thing in `prev`, and the heads' `about` and `occurred`. The changeset is still written whole or not at all.

A real run, on a store holding one backup of `svc:jellyfin` (`H`, about `host:nas`, occurred 2026-11-16):

```
{"kind":"backup","of":"svc:jellyfin","payload":{"mb":419}}
{"kind":"backup","of":"svc:jellyfin","occurred":"2026-11-18","payload":{"state":"ok"}}
```

| Line | `prev` | `about` | `occurred` | `sets` |
| --- | --- | --- | --- | --- |
| 1, with `--fill` | `H` | `host:nas` | 2026-11-16 | `mb` |
| 2, with `--fill` | `H`, line 1 | `host:nas` | 2026-11-18 | `state` |
| 1, without | none | none | none | none |

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7).** The full list is choice 20 of `rust/PLACE.md`.

- **The option is `--fill`, for the whole changeset.** `loops emit task --fill` is a usage error.
- **Lines of one thing chain.** A line is filled against the store and the lines prepared before it, so the second line of a thing names the first, as two emits one after another would. The alternative was two heads made by the tool.
- **How a line states none.** `"about": []` states no mentions; `"occurred": null` no occurrence; `"sets": []` sets no key; `"sets": null` is read whole. The two `null` forms mean this only with `--fill`; a plain changeset refuses them as before.
- **A stated `prev` is kept and the live records are added**, as `--prev` does on a single emit. An exact `prev`, a fork on purpose, is written with a plain changeset.
- **Labels.** A line's labelled prerequisites are prepared first, so a label in `of`, `prev`, `about` or `under` is the final id when the line is filled. A forward label works. The answer and the segment keep input order.
- **Refusals with `--fill` go line by line** after the label checks: an earlier line's field refusal can come before a later line's `prev-unknown`. Without the option the order is unchanged.
- **Only what the store holds.** The filler against a stated base (the cut note, section 8) is a separate piece and is not built.

Checked by a randomized test: 320 cases, each written as one filled changeset and as single emits one after another with the same salts and times. The 207 accepted cases gave the same ids, envelopes and signatures; the 113 refused cases wrote nothing, and the refusal code differed in six, for the reason above. Three hand-written changesets with labels match their single emits too.

Cost: the filler settles the store again for each line. On a store of 9,000 records, one `loops emit` takes 0.54 s and a filled changeset of 200 lines 3.4 s. A collector that writes thousands of lines in one changeset will want that made incremental.

A Codex worker planned and wrote it, and the coordinator reviewed both rounds and ran it against a store.

### The full mutation run after the payload change

GitLab pipeline 2190 ran the whole catalogue on main at `48037ce`. The record is in [the mutation report](model/mutate/REPORT.md). No vector and no protocol rule changed.

| Outcome | Count |
| --- | ---: |
| Rejected by the build | 282 |
| Caught by a changed vector | 20 |
| Caught by an assertion in the generator | 22 |
| Survived | 4 |

Two survivors are the known equivalent ones (`finish-json-hexdigit`, `last-spec-tests-conds`). The other two were new, and each showed a gap:

- **`record-num-hidden`: a check lost with `value` and `n` becoming ordinary fields.** The fault lets a numeric fold read a number from a record whose payload the reader does not hold. A vector caught it on 2026-11-15, through the zero `n` a withheld record then kept. Since !63 a withheld or erased record has no fields, so nothing the model builds reached the guard. No answer was wrong. The guard is now a theorem, `Record.num_hidden`, under the law `numeric.safe-whole`, and a runtime check over a record that keeps a written zero and is marked withheld.
- **`payload-whole-as-number`: a gap in the full runner.** For a mutant that built, the full runner compared the generated vectors and stopped. It never ran `lake exe run --check` or `lake exe checks`, which the per-merge runner does and which catch this fault. It now runs both, and a mutant survives only when all three pass. The new statuses are `killed-by-run-check` and `killed-by-checks`.

Twelve build rejections stood on proofs alone, with nothing that runs objecting. Ten were known. The two new ones (`result-unshown-keeps-shown`, `result-unshown-groups-kept`) change a helper that only the result laws called; a runtime check on a grouped withheld head now catches both.

Run again through the new runner, the six entries give: the two known survivors survive, `payload-whole-as-number` is killed by the checks, and the other three are rejected by the build and by the runtime checks as well.

A Codex worker wrote the change and the coordinator reviewed it. Its choices: the two status names; `numeric.safe-whole` as the law that indexes the new theorem, since it already says when a numeric slot reads a field; and an evidence summary made from the joined file of the shards, which holds no shard baselines or generator logs.

### The decisions page, and the rethink and grammar pages brought up to date

Kyle asked for both on 2026-11-21, for a colleague who reads the site and not the repository.

- **A fifth page, `out/decisions/`.** `DECISIONS.md` is its landing page; `ROADMAP.md`, `CHANGELOG.md` and each file in `notes/` are pages beside it. `node design/build.mjs` renders them with a Markdown renderer of its own, with no dependencies. A new note needs no code change.
- **The build checks what it renders.** It refuses a block it does not support, a link to a heading no page has, and a link to a repository path that does not exist; three new entries in `design/mutants.mjs` hold it to that. The link check now covers every link of every page, the rethink page's included. A link to a file that is not a page goes to the repository on GitHub.
- **So a change to any of these documents changes `out/`.** CI fails when the committed pages differ from a rebuild: run `node design/build.mjs` with every edit to the roadmap, the decisions, the changelog or a note.
- **The rethink page.** Twelve ledger entries for the rulings of 2026-11-13 to 2026-11-21 (declared keys, the anchor, edges, the payload reading, the result codec, the cut, a tick's inputs, the window, an emit against a base, step 2 closed, which choices come to Kyle), with the tick record entered as proposed. About forty passages that called something unbuilt or open are corrected, and the Resolution tab opens with what is ruled since its text was written. Passages that only TK1 to TK9 would change are left until he rules.
- **The grammar page.** The record's entry states the payload reading; `emit`, `test` and `group` are current; the tick's entry says its record is proposed.

A Codex worker wrote the fifth page and the coordinator reviewed it; the coordinator wrote the rethink and grammar changes, from a list of stale passages that a search agent drew up and that was checked passage by passage.

### `loops emit` copies `occurred`

Ruled on 2026-11-20 ([the cut note](notes/2026-11-15-the-cut.md), section 5) and built on 2026-11-21. Rust only: the filler in `rust/place/src/fill.rs` and the CLI. No protocol rule changed.

- An emit of a thing the store holds, with no `--occurred`, carries the `occurred` of the thing's heads when every head states one and all state the same text.
- `--occurred <when>` states a new occurrence, as before. `--no-occurred` states that the record has none. The two together are a usage error.
- A root and the first record of a thing copy nothing. `emit -` fills nothing in, as before.
- The case the ruling was made on, run with both binaries: an ADR emitted with `--occurred 2009-04`, then `status=accepted` with no flag. Before, the update moved the thing out of a window on 2009. Now it stays in it.

Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):
- The heads agree when their `occurred` is the same text. `2009` and `2009?` cover one span and do not agree.
- Where the heads do not agree, or one states none, the emit states none.
- Only the heads are read. A live record under them that states another `occurred` plays no part.
- An erased or withheld head supplies its `occurred`, since the envelope is held.
- The usage error reads `--no-occurred takes no --occurred beside it`.

Checked: the Rust gate (202 tests), with walks for the copy, the two flags, their conflict, a fork whose heads disagree, a first record, an erased and a withheld head, and a changeset. A Codex worker wrote it; the coordinator reviewed it and ran the gate and the ADR case.

### Ruled by Kyle, 2026-11-21

Each was an entry in `DECISIONS.md`, with an example. Kyle ruled for the recommendation every time.

- **Payload values, PV1 to PV7** ([the note](notes/2026-11-21-payload-values.md)). All seven were built the same day, in two pieces, below.
  1. The reading from a stored payload to the fields a fold reads is stated in the model, with vectors. Built the same day, below, with 3 to 7.
  2. `value` and `n` become ordinary fields (option a). Built the same day, below.
  3. `present` means the member was written, for `null`, `{}`, `[]` and `""` alike.
  4. Under `is` and `not`, a list reads as a set of its members.
  5. `at-least` and `below` on a list: neither holds. A list is never a number.
  6. A list gives one group per member under `by`. This amends the ruling of 2026-11-11 that an array is ungrouped.
  7. A list is unread to `sum`, `min` and `max`.

  Kyle's note on a slot that follows `keys`: when two writers set a list field from one base (`routes: ["svc:a"]` and `routes: ["svc:b"]`), a `where` on the field leaves the thing unread and `loops of` shows both setters, while `by` on the field puts the thing in both groups. That is how `by` already reads a scalar field with two live setters ([spec](protocol/spec.md), "A slot with `by` that follows `keys`").
  margins-claude's precision, which follows from the note with no new rule: each `where` test is asked of every live setter, and an answer they share stands. With those two setters, `routes` is `"svc:a"` is unknown (one yes, one no) and the thing is unread; `routes` is `"svc:c"` is no; `routes` present is yes. With `["svc:a", "svc:b"]` and `["svc:a"]`, `is "svc:a"` is yes and `is "svc:b"` is unknown. The payload vectors carry the mixed cases, since a port that unions the lists before testing gets only those wrong.
- **The result codec, RC1.** A result hash has its own prefix, `res:`, not `h:`. Built the same day, below. The other choices are confirmed: a listed record is its id alone; the bytes hold the slots only; slots are in JCS order; no fold type is written; the label is `loops/result-hash/v1:`. Kyle's note: the hash covers the slots without the spec, so it means something only beside its spec inside a tick; a person citing a result cites the tick's `r:` id.
- **Which choices come to Kyle, W7.** A choice comes to him when it changes a record's bytes or id, a result's bytes or hash, a refusal code, or what a spec answers. Every other implementer choice is written in this file and stands unless he objects. The 25 CLI and tooling choices that were waiting stand under this ruling.
- **The keyed-fold choices.** Kyle confirmed the 20 built choices of 2026-11-14 and 2026-11-15: `sets` in the envelope, latest-by following `keys`, the other slots following `keys`, and a keyed collect's `limit`. Among them: `unread` counts things; a test that answers no outranks one that answers unknown; a field no record of the thing sets is absent.
- **Edges, E1.** Three classes, and no change to the envelope. One target under a role is a text field. A set that one observer states whole is a list, read as a set. A relation that several writers add to, or one with facts of its own, is one thing per edge: `of: edge:<rel>:<src>:<dst>`, with `about` naming both ends. Kyle's addition: the edge's payload holds `rel`, `from`, `to` and `state` (`active` or `removed`), so that a fold reads direction and removal from fields and never from the text of `of`. The documents and the import follow this. Nothing is built. An example: `loops emit edge --of edge:routes-to:caddy:grafana --about svc:caddy --about svc:grafana rel=routes-to from=svc:caddy to=svc:grafana state=active port:=3000`.
- **Issue #10 is closed, I10,** with no strict "after" operator. An inbox keeps its own list of the ids it has seen; a reader's tick is for boundaries worth citing.

Still open in `DECISIONS.md`: W1 to W6.

### The payload reading, and lists

**Built, 2026-11-21,** on Kyle's rulings of that day (payload values, choices 1 and 3 to 7).
Until now only the Rust store decided how a stored payload became the fields a fold reads, and it skipped a member that held a list, an object or `null`. The model now states the reading, a vector family pins it, and all three implementations and the store run the same function.

What a fold reads:
- **The reading.** A held payload that is an object gives one field for each top-level member. Any other held payload gives none, so every field is absent. A number is read as binary64 and then as a whole number when it is one within ±(2^53 − 1). Nothing nested is read.
- **`present` means written.** A member that holds `null`, `{}`, `[]` or `""` is present. An object, `null`, and a list with no member an argument can name match no `is`, satisfy every `not`, have no group key and are unread to the numeric folds.
- **A list reads as the set of its members** under `is` and `not`: text, safe whole numbers and booleans. Order and repeats change nothing. A fraction, `null`, an object or a nested list inside it is not a member.
- **A list is never a number.** `at-least` and `below` do not hold on one, and it is unread to `sum`, `min` and `max`, a list of one number included.
- **Under `by`** a record joins the group of each member's key, each once, after members are turned into keys: `[80, "80"]` gives the group `80` once. This is the amendment to the ruling of 2026-11-11.
- **A slot that follows `keys`** gets no new rule. Each test is asked of each live setter's own list, and an answer they share stands; the keys of all live setters are collected, each once.

Over Kyle's example store the CLI answers as he ruled: only caddy has no `watches` recorded (it was all four services); one service watches loki (it was none); neither port bound holds for caddy's `[80, 443, 8080]`; a count by `watches` gives three groups of one with two ungrouped (it was four ungrouped); a `sum` of `ports` reads nothing.

What changed:
- **The model.** A field holds a `HeldField`: one scalar, the readable members of a list, or "written, and nothing can name it". `payloadFields` is the reading and `Stored.readInto` applies it. `Item` gains a case for a list's members and one for a written value nothing can name; `Arg.matches` asks whether a list contains the argument; `HeldField.keys` gives a list's keys.
- **The laws.** A new entry, `payload.read`: the fields depend on the payload alone, not on the envelope, the salt or `sets`; a non-object gives none; looking a name up after the reading is reading the member of that name; every list member kept is one an argument can name. `Cond.payload_present` says `present` on a held payload is exactly "the name is written". Further theorems say `is` on a list is membership, both bounds are false on a list, a list gives no number, and what a list's keys are. Thirteen new theorems in all. The index has 112 entries and 1,220 theorems.
- **One law is narrower, as the ruling requires.** "Each record has at most one key" held for a payload field by its form; a list can give several, so `By.Single` now holds for `env.kind` and `env.observer` only, and with it `By.keys_single`, `By.OneKey.of_single`, `By.accounting_single_form` and `By.count_partition_form`. `By.keys_single_nonlist` keeps the guarantee for every field that holds no list, and the general accounting and partition laws stand as they were. `Record.num_number` is restated over the new field type.
- **The vectors.** A new family, `vectors/payload/`, 18 files: a payload in, the fields out. Ten new `spec/run` files pin Kyle's examples, the two-writer list cases and the mixed cases. No file that existed changed by a byte but `vectors/README.md`, so every changed answer is in a new file and all 23 result texts and hashes stand. There are now 649 generated vectors and 661 JSON fixtures.
- **JavaScript, Rust and the store.** Each has the reading as one function and runs `vectors/payload` through it directly; `Line::record` in the store calls the Rust one. The Rust core's public `Record.fields` holds `record::Held` (`One`, `Many`, `Other`). `loops of` and `--values` show the stored JSON as before.
- **The protocol text.** [`protocol/spec.md`](protocol/spec.md) has a section "Reading a payload" and the new rules under `where`, `by` and the numeric folds, with the sentence that an envelope's empty set is absent while a payload's empty list is present.
- **The mutation catalogue** has 328 entries: 14 new, each caught by a runtime check, and 3 re-pointed.

How it was checked: the four CI gates locally (Rust 198 tests, JavaScript 7,363 vector checks); every tracked file under `vectors/` compared with main; the three-way fuzz with no mismatch, 3,000 cases on seed `payload-reading-1` and 4,000 on `coordinator-reading-1`; Kyle's example store folded by the built CLI; margins-claude's Python implementation, with a reading written without sight of the other two, passes every family (8,483 checks, and 43 of 43 on the payload family) and agrees with the branch's Rust on 148,011 fuzz checks.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):**
- A scalar reads as it did, whether or not an argument can name it. "Written, and nothing can name it" is for an object, `null`, and a list with no readable member. The first plan read an unnameable scalar that way too; margins-claude and this session changed it, so that every existing scalar answer is kept by construction.
- An empty list and a list of only unreadable members read alike. The stored JSON still tells them apart.
- The reading keeps a list's members in the order written, repeats included; the set rule is applied where a test or a grouping reads them.
- The payload family's answers carry tags (`one`, `many`, `other`; `text`, `whole`, `number`, `bool`), with numbers as text. A reader record's `"fields"` in the other families is ordinary JSON.
- The Lean type is named `HeldField`, since `Held` is the batch's type.
- A Codex worker (gpt-6.2-astra) wrote a plan, which margins-claude and this session reviewed, and then the change; this session ran the checks above.

**Found on the way, not changed here:** the fuzz generator puts lists, objects and `null` in the `spec/run` and `result` families only; the `fold`, `realm` and `scenario` families never hold one.

### `value` and `n` are ordinary fields

**Built, 2026-11-21,** on Kyle's ruling of that day (payload values, choice 2).
Every record used to have a `value`, empty text when not written, and an `n`, 0 when not written. The model's `Record` now holds `fields` and nothing else of the payload; `Record.get` is a lookup. A record without `n` is unread to a `sum`, `min` or `max` of `n`, absent to `where`, and ungrouped under `by`.
Over Kyle's example store the CLI now answers as he ruled: the smallest `n` of four backups is 9 with 2 unread (it was 0 with none); 2 of three settings have a `value` (it was 3); a count of backups by `n` leaves two ungrouped (they were in a group `"0"`).

What changed:
- **The model.** `value`, `n` and `nNumber` left `Record`; `withhold` and `erase` clear `fields`. `Sum.step`, `Max.step` and `Min.step`, which read `r.n`, now take a read function `p : Record → Option Int`; `Sum.on f`, `Max.on f` and `Min.on f` are those steps at `(·.num f)`, and `Sum.acc`, `Max.acc` and `Min.acc` are the same folds at the field `n`. `Record.num` is the only function that reads a number from a record.
- **The laws.** Fifteen statements changed, each by holding for every read function `p`: `Sum.batch`, `Sum.perm`, `Sum.fold_eq`, `Sum.repeats`, `Max.batch`, `Max.fold_some`, `Max.fold_eq`, `Max.set_decides`, `Min.batch`, `Min.neg_max`, `Min.set_decides`, the three `acc_fold` bridges and `Max.cannot_follow_prev`. None gained a premise. `Max.fold_eq` now says a record the function cannot read is skipped: `Max.fold p none xs = (xs.filterMap p).max?`. The laws used to be about a fold no spec runs; they now cover the fold a spec runs. The four helper lemmas pinned under D6 stand under their names. The law index has 111 entries and 1196 theorems, as before; two entries were reworded (`numeric.safe-whole`, `sum.repeats`).
- **The vectors.** A record's `value` and `n` are written inside `"fields"`, and only when the case states them. Of 617 generated vectors, 137 changed in the shape of their input records alone and 12 changed an expected answer: a max over heads that never supplied `n` went from 0 with 0 unread to `null` with every head unread (`fold/heads/03`, `fold/view/03`), and ten `realm` sums count such records as unread. The nine `fold/sum` and `fold/extrema` answers and all 23 result texts and hashes are byte for byte as they were. Four new `spec/run` vectors pin Kyle's examples and a written `0` and a written `""` beside them: `pv2-backups`, `pv2-settings`, `pv2-written-zero`, `pv2-written-empty`. There are now 621 generated vectors and 633 JSON fixtures.
- **JavaScript, Rust and the fuzz.** Each dropped its two special cases. The Rust core's public `Record` lost `value`, `n` and `n_number`. `Line::record` in the store reads every top-level text, number and boolean under its own name; a list, an object or `null` is still skipped (the next piece). The fixture readers in Lean and Rust refuse a flat `value` or `n` as an unknown member. The fuzz generator varies whether each of the two names is present and of what type.

How it was checked: the four CI gates locally (Rust 195 tests, JavaScript 7,205 vector checks); every changed vector compared with its old form record by record (4,094 records: each differs only by `value` and `n` leaving or moving into `fields`, and no stated value was dropped); the three-way fuzz with no mismatch, 3,000 cases on seed `payload-fields-1` and 4,000 on `coordinator-payload-1`; margins-claude's Python implementation, moved to the new reading, passes every family (8,167 checks) and agrees with the branch's Rust on 129,015 fuzz checks.

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):**
- The low-level folds take a read function and are kept, where the first plan was to delete them. margins-claude and this session agreed it: the laws are at least as strong, and no function reads a record's number but `Record.num`.
- The fixture field of the `fold/sum` and `fold/extrema` families keeps the name `n`. It is now only a name.
- Which records state a `value` or an `n` was decided from the Lean fixture source: an explicit `0` or `""` was kept (one scenario body, two group-key fixtures, the new controls), and a default that a constructor had filled in was dropped.
- The max in `fold/heads/03` and `fold/view/03` was incidental to what each vector shows, so their records were left without `n`.
- A scenario that renders a head reads the text field `value`; a missing one is reported, never read as empty.
- The mutation catalogue keeps 314 entries: 13 were re-pointed or rewritten and none retired. That took under five minutes, four of them the cheap runner (the measure asked for under W5).
- A Codex worker (gpt-6.2-astra) wrote a plan, which margins-claude and this session reviewed, and then the change; this session ran the checks above.

**Found on the way, not changed here:** the JavaScript sum is exact only to a signed 128-bit limit, while the law index calls it unbounded. No vector reaches the limit.

**Corrected:** three rows of the table in the payload values note, section 1, which had dropped a written scalar of the other type.

### A result hash is spelled `res:`

**Built, 2026-11-21,** on Kyle's ruling of that day. The hash of a canonical result is `"res:"` and then the base32hex text it had before; a record's commitment is still `h:`.
The label, the preimage and the canonical bytes did not change, so each of the 23 result vectors changed in its `expect.hash` alone, from `h:` to `res:` with the same digits. No other vector changed, and no record's bytes or id.
The model (`Loops.resultHash`), JavaScript and the Rust core write it; [`protocol/result.md`](protocol/result.md) says that the two hashes now differ in their prefix as well as in what they hash. The law `result.no-commit` is as it was: its statement is about the hashed bytes.
The horizons page's example of a result hash reads `res:7a…`.
No mutation catalogue entry needed to change: the nineteen entries on `Result.lean` still find their lines.
**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):** the tests that a result hash is no commitment now compare the digits after the prefix, since the prefixes alone would make any two differ; a Codex worker (gpt-6.2-astra) wrote the change from a brief, and this session reviewed it and ran the gate.

### `--values` gives the envelope

**Built, 2026-11-21: `--values` gives the envelope** ([issue 9](https://git.gruel.network/kaygee/strange-loops-next/-/issues/9)).
The issue came from rex-tools, a chat room over the `loops` CLI and its first outside user: a timeline needs each record's `emitted`, `observer` and `about`, and got them from a second `loops export` joined by id.
Under `--values`, each `{"id", "shown"}` entry that a collect or a latest-by following `prev` lists gains `env`, the envelope as stored, whatever its `shown`; each thing of a latest-by that follows `keys` gains `about` as `loops of` writes it, and each of its heads gains `env` ([place](rust/PLACE.md), choice 17).
Without the option nothing changes, and taking the added members out still gives the protocol's answer byte for byte. Only `rust/cli` changed.
**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):** no new flag, the envelope comes with `--values`; the member is `env`, the whole stored envelope, not three picked fields; an erased or withheld record's entry has `env` and no `payload`; a keyed thing's `about` is always written, empty when no head mentions anything, as in `loops of`; entries under a keyed thing's `keys` and `whole` gain nothing.

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

**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):**
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

**Implementer choices, confirmed by Kyle 2026-11-21, all but the first.** He ruled that day that a result hash has its own prefix, `res:`, and it was built that day.
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
**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):**
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
**Implementer choices, confirmed by Kyle 2026-11-21:**
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
**Implementer choices, confirmed by Kyle 2026-11-21** (the flags in the third stand under W7):
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
**Implementer choices, confirmed by Kyle 2026-11-21:**
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
**Implementer choices, confirmed by Kyle 2026-11-21:**
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
**Implementer choices; they stand under Kyle's ruling of 2026-11-21 (W7):**
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
- A selected record with no key is counted in `ungrouped`: a missing field, no mention under the prefix, or a value that cannot be a key (a fraction, null, an array or an object). *Amended by Kyle on 2026-11-21: a list gives one group for each of its members; see that date.* It is never dropped and never a refusal; only a payload the reader does not hold makes a key unread.
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

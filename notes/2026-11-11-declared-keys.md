# D8c: declared keys, design note

Moved here from `ROADMAP.md` on 2026-11-21, unchanged but for links. Every choice is ruled. What was built from it is in [the changelog](../CHANGELOG.md), under 2026-11-14 and 2026-11-15.

**By margins-claude, drafted 2026-11-11 for Kyle's rulings before any code.** Nothing here was built when he ruled.
Kyle ruled on 2026-11-13, and on 2026-11-14, after a consult, on how a slot that follows `keys` reads a thing's envelope (section 3). Every choice is marked **Ruled**; none is open.
The experiments and proofs are in the margins repo: `sl-py/keyfold.py`, `keyslice.py`, `keygap.py`, `keyanchor.py`, `keyslice_anchored.py`, `keyenv.py` and `lean/Convex.lean`.

**The problem.** Once an emit sets only some keys, a thing's state is the merge of its live setters, not one head's payload.
`where`, `by`, the numeric folds and the slots that follow `prev` all read one record's payload.
With anchored writers the head of a task is often the emit that last changed its title, so "count the open tasks" over heads is wrong.

**Terms.** A record *sets* key `k` when `k` is in its `sets`, or when it has no `sets` (a whole-read emit, which sets every key).
A setter of `k` is *live for `k`* when no admitted descendant of it over `prev` sets `k`.
Every record sets the envelope, whatever its `sets`. A record is live for the envelope when no admitted record of its thing is above it over `prev`: it is a head.
A record is *live* when it is live for some key, the envelope among them. So every head is live, a pure join too.
For a thing with only whole-read emits, the live records are its heads.

## 1. `sets` in the envelope

- **Holds:** a set of top-level payload key names, canonical as `about` is (UTF-16 order, each once).
- **Absent and empty** differ, as ruled: absent is read whole; empty sets nothing, a pure join.
  **Ruled:** the canonical envelope writes `"sets": []`. Every other set field leaves an empty set out, so this is the one exception.
- **Refusal:** one code, `sets-not-keys`, for a value that is not an array, a member that is not text, or an empty name (no spec could name it). It comes after `erases-not-record-id` and before `of-missing`. **Ruled:** the name, the place, and that an empty name is refused.
- **No check against the payload.** A declared key the payload lacks is unset, as ruled; a payload that is not an object unsets every declared key.
  The envelope stays checkable without its payload, which an erased record needs.
  **Ruled:** no check; a non-object payload beside a non-empty `sets` is written.
- **A root or a revision may declare anything.** A root with `sets` declares its keys like any other emit; no check compares one record's `sets` with another's.
- Refusing an undeclared payload key is a strict store's rule (step 3).

## 2. The keyed fold

**Ruled:** a second value of `follow`, not a new fold type: `{"latest-by": "of", "follow": "keys"}`.
`follow` names how supersession is read. `"prev"` reads a thing as its heads; `"keys"` reads it as its live setters.
Both readings are wanted, so a spec says which; `follow-invalid` refuses any other value.

The result, per thing, gives the live setters of each key some live record declares, the live whole-read emits, and the heads.
Here `W` and `W2` are whole-read, `B` sets `status` from `W`, and `W2` is concurrent with `B`:

```json
{"things": {"task:1": {"keys": {"status": [{"id": "r:W2", "shown": "here"}, {"id": "r:B", "shown": "withheld"}]},
                       "whole": [{"id": "r:W2", "shown": "here"}, {"id": "r:W", "shown": "here"}],
                       "heads": [{"id": "r:W2", "shown": "here"}, {"id": "r:B", "shown": "withheld"}]}},
 "unread": 0}
```

- Setters are newest first, as `latest-by` lists a fork's heads. More than one is two live setters of that key; the core declares no conflict.
- A key is unknown exactly when a live setter's `shown` is not `here`. The fold returns ids, not values, as `latest-by` does.
- `keys` has an entry for every key some live record declares, and under it all of that key's live setters, whole-read ones included: `W2` is live for `status` and `W` is not.
- `whole` lists the live whole-read emits. A key with no entry is read from them, and every one of them is live for it: a whole-read emit that some setter of that key replaced would have an entry to show it, or be replaced by a whole-read emit and so not live at all.
- `heads` lists the thing's heads, newest first: the records the envelope is read from (section 3). A pure-join head is listed here and nowhere else.
- **Ruled:** this shape; `heads` came with the envelope ruling of 2026-11-14.

## 3. How the other slots read a keyed thing

**Ruled:** `"follow": "keys"` is allowed on count, sum, min, max and collect, with or without `by`, and there the unit is the thing.
Slots without `follow`, and slots with `"follow": "prev"`, keep reading records as written, each from its own payload.
So both readings exist and a spec says which it wants.
Kyle said of an undeclared payload key (2026-11-10) that the keyed fold does not read it and a whole-record fold, a `where` test or a lens still can.
**Ruled:** a slot that follows `keys` does not read it, and the record readings do.

For a slot that follows `keys`:

- **A payload field** is read from the field's live setters, as the Terms define them: every live record that sets it, by declaring it or by being whole-read.
- **`where`:** each test is asked of each of those setters. One answer from all of them is the answer. Different answers, or a setter whose payload is not held, is unknown, and the thing is unread.
  The core compares test answers and never payload values, so agreement is not a conflict.
- **count** counts the things selected. A thing forked across whole-read emits counts once here and twice under `"follow": "prev"`.
- **sum, min, max:** a field with one live setter is read from it. With two it is unread. **Ruled:** also when both hold the same number; reading them would make the core compare values, which D8 ruled out.
- **`by` on a payload field:** the thing joins the group of each live setter's key, as a record that mentions two projects joins two groups. An unheld setter makes it unread; no setter, ungrouped.
- **collect** lists the selected things' live records, newest first, a pure-join head among them. With anchored writers this is the thing's slice.
  **Ruled 2026-11-15** (Kyle, on margins-claude's point): `limit` counts things, each listed whole, the things whose newest live records are newest; under `by`, the things in each group. A limit over records could list some of a thing's live records and not the rest, and what it listed was then not a slice of the thing: a key whose setter fell past the limit read as unset.
- **`from` and the envelope tests of `where`** choose heads, each head alone, as they do under `"follow": "prev"`. A thing is selected when one of its heads is chosen.
  The payload tests are then asked of the thing, over each field's live setters, whether or not `from` takes those records.
- **`by` on `env.kind`, `env.observer` or `env.about:<prefix>`:** the thing joins the group of each key of each chosen head, once in each group; with no key it is ungrouped. An envelope key is never unread.
- **One head:** its envelope is the thing's. **A fork:** one chosen head selects the thing; it is counted once and grouped under the keys of each chosen head. **A pure-join head:** it is the head, so its envelope is the thing's. It is how a task is moved without setting a key, and how two heads that mention different projects are settled.
- **Whole-read beside keyed:** a whole-read emit sets every key and is live for each until a descendant sets it. A later whole-read emit replaces everything under it.

The alternatives were to give `"follow": "prev"` this meaning (then no spec could ask for a fork's heads one by one), or to leave the other slots unable to read keyed things (then no spec can count open tasks once writers set keys).

**Ruled 2026-11-14: the envelope is read from the heads** (Kyle: "Rule B"; consult of 2026-11-13, `sl-py/keyenv.py` and `lean/Convex.lean` in the margins repo).
A payload key has setters. `kind`, `observer`, `about` and the times belong to a record, and nothing replaces them key by key.
So every emit sets the envelope: the envelope's live setters are the heads, and there is one notion of current, not two.
A task moves from one project to another as it does under `"follow": "prev"` (`spec/run/24-by-follow`).

What the ruling carries with it:

- **Writers:** every emit carries the thing's whole `about` (section 5).
- **The anchor rule** reaches every live record the writer holds, since every two records share the envelope (section 5).
- **The result of section 2** has a `heads` list.

The alternatives considered:

- **Every live record** (this note's first recommendation, withdrawn). A thing is selected when one of its live records is taken, and joins the group of each live record's key.
  Take a task whose every emit mentions `project:x` until one says `project:y`. It stays in x until every earlier live record is replaced: a median of 12 more emits, and never in 7 of 2,000 runs.
  If only the root mentions x, the task leaves x when some emit retires the root, usually one that sets `title`. Setting one key changes what a reading of another field says.
- **Setter by setter.** A key whose setter `from` does not take reads as absent. With a mention on the root only, `status` reads absent from the first later emit: an answer where the reader should say it does not know.

What the ruling costs: an emit no longer says only what it changes, since it restates the mentions. A changeset that leaves them out takes the thing out of its project. And concurrent emits to different keys, which are no conflict in the payload, are two heads whose envelopes are both read until a join.

## 4. The laws

| Law | Says | State |
|---|---|---|
| *witnessed* | A part reads each key's live setters as the whole does exactly when every held setter that is dead for a key has a setter of that key above it through held records. | Proved over the model and over `realm` (`witnessed_iff`, `keyedIn_part`, `keyedIn_slice`): `keyed.witnessed`, `keyed.slice`. |
| convex | A part that is convex over `prev` and holds the live setters is witnessed. Not the other way. | Proved, with no cycle over `prev` as a premise (`witnessed_of_convex`, `keyedIn_convex`, `convex_not_needed`): `keyed.convex`. |
| anchored | When every emit that sets `k` names every held live record that sets `k`, the live records alone are a witnessed part. The envelope is such a `k`, and every record sets it. | Proved, with no cycle as a premise (`anchored_slice`, `keyedIn_live`, `keyedIn_live_slice`): `keyed.anchored`. |
| `Closed` | The anchored law needs a sender that holds what the writers of its records had seen. | Proved as a premise; found by experiment (55 wrong in 35,200 when the sender is missing records, none for a convex part). Proved: the rule gives anchored names to a closed holding (`anchored_of_written`), and a holding that is not closed reads wrongly (`closed_needed`). |
| heads | A head is a live setter of a key every record sets. So *witnessed* and anchored cover the heads, and every live record is a head or named by one: the heads list the slice. With the anchor rule over payload keys only, the live records read alone can take a superseded record for a head. | Proved (`heads_slice`, `live_named_by_head`, `heads_need_env`): `keyed.heads`. |
| receiver's flag | A receiver can tell which of its own records a missing name might have replaced; the flag is sound and tight. | Proved (`flag_tight`). Step 5 uses it; not needed for D8c. |

- These were first stated over an abstract `prev` and `sets` (margins `lean/Convex.lean`). They are now stated over the model's liveness and over `realm`, behind settling and admission (`model/Laws/LiveSlice.lean`); the changelog entry "The slice laws for one keyed thing" (2026-11-14) says what the port found.
- By experiment (`keyenv.py`, three replicas, pure joins in the mix): no holding of 11,369 read a key or the heads wrongly from its live records alone, nor any through 3,172 rounds of keeping only the slice. With the anchor rule over payload keys only, 1,529 of 11,347 took a superseded record for a head. This is not yet run behind settling, admission and erasure, or with a sender that is itself missing records.
- By experiment (`keyslice.py`, 35,200 pairs with `of` stated): the answer needs no record outside the thing; the sent members must be admitted ones; a live setter's eraser is needed for the shown state, not for the answer; a sender must not send a payload its own eraser took away.
- **The slice for a spec with a keyed slot is proved** (`Spec.run_slice_keyed`, law `keyed.spec-slice`): a spec slice, and a cover of every thing the spec could select. The conjecture here was the slice of each selected thing plus what `SpecSlice` asks; that holds with one added condition, that what the part holds of each such thing is witnessed, because the spec slice brings dead members of it and each still sets its keys. A bare `prev` target reads wrongly.
- A slice law is about a slice read alone, as ruled.

## 5. The writer

**Built 2026-11-14** in `loops emit`; see [the changelog](../CHANGELOG.md), 2026-11-14, "The writer".

- **The anchor rule,** as agreed on #5: an emit that sets a key names in `prev`, beside what it builds on, every record its writer holds that sets that key and is still live.
  Every emit sets the envelope, and so does every record, so the rule names every live record of the thing the writer holds (**Ruled** 2026-11-14, with the envelope reading).
  `loops emit` fills this in on the single-record path, as it fills in `of`.
- **`about` is whole on every emit:** the head's mentions are the thing's. `loops emit` copies `about` from the heads it builds on, every mention when it builds on several. **Ruled** 2026-11-14.
  How the command line states another `about`, or none, is left to the build: `--about` replaces the copy and `--no-about` states none.
- **`emit -` fills nothing in:** a changeset states its own `prev` and its own `about`, as it states its own `of`. **Ruled.**
- **No refusal for an unanchored emit.** It reads correctly whole; only its slice is larger. **Ruled.**
- **A whole-read emit** names the same records and sets every key, so it is then the only live record.
- **Cost** (`keyenv.py`). `prev` names per record with three replicas: 1.12 plain, 2.88 anchored over payload keys only, 4.39 with the envelope counted; in long steady histories 3.75 against 6.06. For one writer over 1,000 records, 2.27 against 3.74, the longest 5 either way.
  The bound is one name per live record of the thing, so about one per key, and every emit pays it. The names are in the id. A whole-read emit brings it back to one.
- **What it buys.** Over 300 events a thing's slice is about 9 records, against 58 for the cheapest witnessed part with plain writers and 234 for the convex part.
  The heads list it: a receiver that holds a thing's heads knows every other live record by name.

## 6. Out of scope

- Strict stores, where an undeclared payload key is refused: step 3.
- Merging a slice into an older holding, and the causal context that makes it sound (`merge_reads`): step 5. It needs a per-writer sequence number or a set of seen ids, which the envelope does not have.
- The caching reader for slots over heads or live setters, and what it must keep for a record with an unknown `prev`: step 4.
- Whether two live setters conflict: a reading above the core.

**Order of work:** `sets` in the model with its refusal; liveness per key, the envelope among the keys, and `latest-by` following `keys` with `heads` in its result; vectors, JavaScript, Rust; then the other slots following `keys`, selecting by heads and reading payload fields from live setters; then the laws ported, the heads law among them (done, for one thing and for a spec); then the anchor and the copied `about` in `loops emit`. All of it is built.

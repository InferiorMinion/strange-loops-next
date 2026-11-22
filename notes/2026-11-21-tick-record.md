# Step 4: the tick record, design note

**Proposal by the coordinator, 2026-11-21, for Kyle's rulings before any code.** Nothing here is built and nothing is ruled.
The questions, each with an example, are in [DECISIONS.md](../DECISIONS.md#the-tick-record) as TK1 to TK9. This note holds the exact form behind them.
margins-claude reviewed the draft the same day. Its four additions (the rules' version, what `covers` must hold, `withheld` beside a covered eraser, a tick outliving an erasure) are in the text, and its answers are in section 7.
Neon reviewed TK1 to TK9 the same day. It agrees with eight and moved TK9 to a stated member bound into the hash; its points are in sections 1, 2, 4 and 6, and in section 9.
It rests on [the cut note](2026-11-15-the-cut.md) (ruled 2026-11-20), [the result codec](../protocol/result.md) and [the payload reading](2026-11-21-payload-values.md) (ruled 2026-11-21).

## 1. The record

A tick is a record of kind `tick`. Its envelope is an ordinary one: `observer` is whoever ran the fold, `emitted` is their now, and the record is signed.
`about`, `of`, `prev` and `occurred` are the writer's to state, as on any record. A tick written again after a late record names the earlier tick in `prev`. A record with `prev` states `of`, so the two are records of one thing: the first tick's id, or a natural key the writer chooses.

The payload is one JSON object with these members and no others:

| Member | Holds | Written |
|---|---|---|
| `spec` | the spec in its parsed form ([spec.md](../protocol/spec.md), "The parsed form") | always |
| `rules` | the version of the rules the fold ran under, as text: `"1"` (TK9, option D) | always |
| `spec-hash` | the hash of `spec` under that version, below | always |
| `covers` | the ids of the records folded: a set of record ids, sorted by UTF-16 code units, each once | always, `[]` when empty |
| `withheld` | exactly the covered ids folded without a payload, those a covered record erases among them: a set of record ids, sorted the same way, each one a member of `covers` | only when not empty |
| `result` | the canonical result object ([result.md](../protocol/result.md)): integers as decimal text, ids without `shown` | always |
| `result-hash` | the hash of `result`'s canonical bytes, `res:…` | always |

A payload is a tick's when it has this shape, `spec` parses to itself, `spec-hash` is the hash of `spec` under the label that `rules` names, `result` is a canonical result for `spec`'s slots, and `result-hash` is its hash.
The payload's bytes in the record are its JCS text, as for any payload, so two ticks with equal members are equal bytes.

## 2. The spec's hash

```
preimage  = "loops/spec-hash/v" ‖ rules ‖ ":" ‖ JCS(parsed form)        (the label in UTF-8; "loops/spec-hash/v1:" for rules "1")
spec-hash = "spec:" + base32hex(SHA-256(preimage))
```

- The parsed form keeps `vertex`, `from` and each slot as [spec.md](../protocol/spec.md) says, and drops `note` and the carried keys. It parses back to itself (`spec.parse`), so it is one text for one parsed spec.
- **The label is the version of the rules too** (TK9): of what a spec answers, and not only of how the hash is made. A change that alters any spec's answer takes the next label, so every spec's hash changes with it. The payload states the version as `rules` (option D, after Neon's review), so a check reads it and does not try labels: it answers "cannot check: made under older rules" or "made under newer rules", and "not a tick" only when the hash does not fit the label `rules` names. Under option A there is no member, a check tries the labels it knows, and a newer tick reads as a false one on an older machine.
- The label is 19 bytes while `rules` is one digit, and must not be 16, for the reason [result.md](../protocol/result.md) gives for its own label. Its 17th byte is `v`, and no JCS text starts with `v`, so no record's commitment has this preimage. It differs from the result label at the seventh byte.
- `spec:…` has the shape of a typed reference, so a writer may put it in `about`, and a `from` can then take the ticks of one spec by their envelopes.
- When a carried key (`window`, `param`, `covers`) is read one day, it enters the parsed form. A spec that does not write it keeps its hash.

## 3. What a tick claims

Take the forms of the records `covers` names. Read each one that `withheld` names without its payload, and every other one with it. Settle and admit inside that set alone, and run `spec` over it. The canonical result of that run is `result`.

- **Settling inside the set** is the cut note's section 4: a form held without its payload, with no eraser in the set, reads as withheld. An eraser acts only when `covers` names it, and then what it erases is read without a payload and stands in `withheld`. So one claim has one text. This is what `sl-py/cut.py` did: cut first, then settle inside the cut.
- **The store names in `from` select nothing.** `covers` names the records; the run applies each source's `kind`, `about` and `of` and the slots to them.
- **What `covers` must hold.** A spec's answer depends on records it does not take: an eraser of a taken record, a successor of another kind, and such a successor's targets (`spec.slice`; vector `realm/07-kind-change-supersedes`). A tick over the taken records alone is a true claim that can differ from what `loops fold` shows. So `loops tick` covers what the store holds, or a cut of it, until the Rust can compute a spec's slice.
- **Admission inside the set** is the built rule over the records given. Whether the set is a cut, or a slice of one, is the writer's care and is not part of the claim. `loops tick` chooses only such sets: everything held, the records as of a time (`downOf W (upTo W T)`), or the ids named with what is under them.
- **The inputs** are `spec-hash`, `covers` and `withheld`. The version of the rules is inside `spec-hash`. Equal inputs give equal `result` bytes.
- **A tick among the covered records** is one record. Its payload is read as any payload is; nothing follows its `covers`.

## 4. The check

`loops tick --check <id>` reads the record and answers:

- **not a tick**, with a refusal code, when the payload fails section 1, or when `covers` names a record and its eraser and `withheld` leaves the erased one out. The second needs the eraser's envelope, so only a holder of it finds it.
- **cannot check: made under older rules**, or **newer rules**, when `rules` is not the version this binary folds under (section 2). Beside "older" it says what today's rules give, the same result or another, as a report and not a verdict.
- **cannot check** when the tick's own payload is not held, or when `spec` does not parse under a version this binary knows: a later binary may have added a fold type without changing the version.
- **cannot check** when the store lacks a covered record, or lacks the payload of a covered record that `withheld` does not name. It counts each.
- **same** or **differs** otherwise, after running section 3 and comparing the result's canonical bytes with `result`.

A checker that holds a payload the tick lists under `withheld` leaves it unread.
The check goes one level: a covered tick is a record to it. Beside its answer it reports two things:

- whether `covers` is closed over `prev` and holds each `under` target. This is read from the covered envelopes alone, so it is a property of the tick and the same on every machine. Closure is not part of what a tick is: a slice is not closed on purpose.
- how many records the spec takes from the store that `covers` does not name. It counts taken records only; a late eraser or a successor of another kind needs the spec's slice.

No store refuses a tick on import or sync.

## 5. What a spec reads of a tick

Nothing new. By the payload reading:

| Member | Reads as | Tests |
|---|---|---|
| `rules`, `spec-hash`, `result-hash` | text | `is`, `not`, `present`, `by` |
| `covers`, `withheld` | a set of text | `is` (membership), `not`, `present`, `by` (one group for each id) |
| `spec`, `result` | present, with no value | `present`, `absent` |

No slot reads a number out of `result`: there is no nested lookup, and the codec writes integers as text.

## 6. Laws to state when it is built

Names are proposals.

- `tick.inputs`: equal `spec`, `covers` and `withheld` give equal result bytes. From `settle.set`, the fold laws (the set decides) and `result.exact`.
- `tick.withheld`: a holder with the payload of a record that `withheld` names, reading it without, gets what a holder without it gets. From `settle.set`.
- `tick.canonical`: a tick's payload has one text; reading it back gives the same tick.
- `spec.hash`: two parsed specs with one hash preimage are equal up to the order of their slots and of the members of a `where`, and so give the same result bytes. The model keeps slots as a list in written order and JCS sorts an object's members, so plain equality is false (Neon). The check of section 1, "`spec` parses to itself", compares JCS texts for the same reason.
- `tick.record`: what a spec reads of a tick is the payload reading of its payload (`payload.read`); no new reading.
- The base lemma (margins-claude's fourth cut MR): for any set B, an emit of a thing whose `prev` is the live records of the thing in B satisfies `Anchoring` in B with the emit added.

## 7. What margins-claude answered

- **The base lemma.** An emit against a base tick names the live records of the thing within `covers`, settled inside `covers`. It does not need `covers` closed downward.
  - Liveness inside a set is a property of envelopes, so `withheld` does not change it. A withheld setter is named like any other.
  - A record dead in the set is dead in every set that contains it (`keyed.witnessed`). So no setter of the base is left both unnamed and live.
  - A record live in the base and dead in the store is named anyway. The emit and the record that replaced it are then two heads, and the key shows as a conflict: the stale apply of the cut note's section 8, and the right answer.
  - What closure adds is meaning: when the base is `Down`, "live in the base" is what a reader at that past read. Otherwise the base is a set no one need ever have held.
  - A base that holds no record of the thing, such as a tick over ticks, answers `prev: []`. It must not expand through covered ticks. The CLI says "the base covers no record of the thing". Whether a place warns, fails or passes on the conflict is the policy Kyle left for step 3.
- **Admission inside a set that is not closed downward.** The writer's care is enough for the claim, which is a function of the set whatever the set is. It is not enough for a reader who takes the tick as a past. So closure is reported by the check (section 4) and is not required.
- **`withheld` against `Down`.** The cut laws apply unchanged: they read envelopes only. A cut with a record under `withheld` reads as a past whose reader did not hold that payload, which is `settle.set` and no new law.
- **An erasure does not reach a tick.** A result holds group keys, thing names and payload key names, and a group key under `by` on a payload field is the field's text. Erasing a covered record leaves those in every tick that covered it. The ticks are found with `where covers is <id>` and erased as records (TK2).
- **Checked from outside.** The answers of TK6 reproduce in `sl-py` from the payload reading alone, and the arithmetic of TK2, TK4 and TK5 is right. It did not rerun the store commands of section 8.

## 8. The runs behind the examples

Run on 2026-11-21 with the `loops` binary built from main at `29c6b7c`.

```
loops -C laptop init
loops -C laptop emit backup svc=jellyfin mb:=412 --about svc:jellyfin --occurred 2026-11-16
…                                                   (pihole 3, gitea 88; then the 17th: 419, 10, 95)
loops -C laptop fold backups.json                   runs 6, mb 1027, six ids
loops -C laptop export > six.jsonl
loops -C nas import six.jsonl
loops -C nas emit backup svc=pihole mb:=4 --about svc:pihole --occurred 2026-11-17
loops -C nas fold backups.json                      runs 7, mb 1031
loops -C cov import six.jsonl ; loops -C cov fold backups.json      runs 6, mb 1027, the laptop's bytes
loops -C cov erase r:kvvv… "held a path it should not"
loops -C cov fold backups.json                      runs 6, mb 615 with 1 unread, the same six ids
```

- The result hashes (`res:d8eu…`, `res:pm7t…`, `res:jv9h…`) were computed from each reading by the codec's rule: integers to decimal text, each `{"id", "shown"}` to its id, JCS, the label, SHA-256. The same code gives the hash of all 23 vectors in `vectors/result/`.
- `spec:gs9s…` is the hash of section 2 over the example spec.
- The three ticks of TK6 were written by hand with `loops emit tick spec:=… spec-hash=… covers:=[…] result:=… result-hash=…`, and read with a spec whose slots are `count`, `count` where `covers` is an id, `count` by `covers`, `count` by `spec-hash`, and `sum` of `result`.
- TK8's store is the NAS above with one more backup emitted without `--occurred`.
- TK4's third example: `loops emit retired "…" --prev r:kvvv…` in a store holding the six, then a spec with `{"count": true, "follow": "prev"}` and `{"sum": "mb", "follow": "prev"}`: 5 and 615 there, 6 and 1027 over the six alone.
- TK9's two answers are the PV2 example of this morning's `DECISIONS.md` (0 with 0 unread) and the same fold on tonight's binary (9 with 2 unread).

## 9. What Neon answered

- **TK9.** A version held only in the hash's label reads, on an older machine, as a hash that fits nothing: "not a tick" about an honest tick. A member `rules` beside a label made from it lets any binary say older or newer, and lets a spec find the ticks of one version. Taken as option D.
- **TK3 with reads across stores (W1).** A tick's check runs every source's `kind`, `about` and `of` over `covers`, and the store names select nothing. So a fold across several stores must take the union of the named stores first and run each source over that union, as [spec.md](../protocol/spec.md) says now. If it routed each source to its own store, `loops fold` and the check would disagree whenever a record in one store fits another source.
- **TK5 with TK7.** `withheld` lists what was folded without a payload, also for a spec that reads no payload. So when a covered record is erased after a tick of a `count`, the check answers "cannot check: 1 payload the tick read is not held", where the cut note says a result made of references can still be recomputed. That is stricter than it need be, and simple. A later check may answer `same` for a spec that provably reads no payload; nothing in the record stops that.
- **The `spec.hash` law** is equality up to member order (section 6).
- **An erased tick** answers "cannot check" (section 4).


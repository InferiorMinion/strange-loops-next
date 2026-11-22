# Step 4: the tick record, design note

**Ruled by Kyle on 2026-11-21: TK1 to TK9, each for its recommendation, and TK9 for option D with its three points.** Proposed by the coordinator the same day. Nothing here is built yet.
The questions as they were put, each with its example, are in [section 10](#10-the-questions-as-they-were-put). Sections 1 to 6 hold the exact form behind them.
**Ruled by Kyle on 2026-11-22: TK10, each of its seven rows for the recommendation, and TK11 for option C.** Both came from building the tick record and were built to their recommendations before he ruled; they are in [section 11](#11-the-later-questions-as-they-were-put). TK12, the exit status of the check, was ruled the same day for what was built, and is there too.
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

`loops tick --check <id>` reads the record and answers as below. It checks `spec-hash` over the JCS text of the `spec` member, under the label `rules` names, before it parses the spec (Neon). So "cannot read the spec" is answered only for a spec whose hash fits and that this binary's parser refuses. A spec that parses and whose JCS text is not that of its parsed form (a kept `note`, a left-out `fold`) is not a tick.

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
- The base lemma (margins-claude's fourth cut MR, built 2026-11-22 as `keyed.base`): an emit of a thing whose `prev` is the live records of the thing in a base B is the emit a holder of exactly B would write; when B holds what the writers of its records held, as a part closed downward does, the store with the emit keeps anchored names. As first put here it claimed this for any set B. That was wrong; see section 7.

## 7. What margins-claude answered

- **The base lemma.** An emit against a base tick names the live records of the thing within `covers`, settled inside `covers`. It does not need `covers` closed downward.
  - **Corrected 2026-11-22 by margins-claude: it does need it.** Proving the lemma showed the claim above and the second point below are wrong. The emit is right for any base. The store stays readable from its live records only when the base, for that thing, is closed downward in the store. The point below is about records of the base; what breaks is a record outside it. One record sets two keys, a second sets one of them over it, and the base is the second alone: an emit of that key names the second and not the first, which is still live for its other key and is now under the emit unnamed. The live records read alone then show the first as a setter of the emit's key, a conflict the whole does not have (`base_closed_needed`). The other points stand. The law is `keyed.base` in [laws.md](../protocol/laws.md).
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

## 10. The questions as they were put

These are the entries of `DECISIONS.md` as Kyle read them on 2026-11-21. He ruled each for its recommendation; for TK9 that is option D, with the three points under it. "Recommended", "proposal" and "not built" in the text are as written then.

A tick says: "folding exactly these records with this spec gives this result."
The cut note is ruled and the result codec is built. What the record itself holds is not designed, and these nine entries propose it.
The technical form (the hash rule, the canonical forms, the laws to state) is in [the tick record note](#1-the-record).

**The example every entry uses.** Your laptop's store holds six backup records: three services, the nights of 16 and 17 November.

| Record | Service | Night | `mb` |
| --- | --- | --- | ---: |
| `r:kvvv…` | jellyfin | 16 | 412 |
| `r:ftlf…` | pihole | 16 | 3 |
| `r:6p8c…` | gitea | 16 | 88 |
| `r:fqqv…` | jellyfin | 17 | 419 |
| `r:vm0f…` | pihole | 17 | 10 |
| `r:53eo…` | gitea | 17 | 95 |

The spec `backups` counts them, adds up `mb` and lists them:

```
{"vertex": "backups", "from": [{"store": "lab", "kind": ["backup"]}],
 "fold": {"runs": {"count": true}, "mb": {"sum": "mb"}, "list": {"collect": true}}}
```

Today `loops fold backups.json` on the laptop answers `runs` 6, `mb` 1027, and the six ids.
The canonical result of that answer hashes to `res:d8eu…`. The CLI has no verb for the hash yet, so I computed it by the codec's rule; the same rule gives the hash of all 23 result vectors.

**The tick this proposal would write for it** (ids shortened):

```
env      kind "tick", observer k:5eis… (you), emitted 2026-11-21T19:40:00Z, signed
payload  {
  "spec":        {"vertex": "backups", "from": [...], "fold": {...}},
  "spec-hash":   "spec:gs9s…",
  "covers":      ["r:53eo…", "r:6p8c…", "r:fqqv…", "r:ftlf…", "r:kvvv…", "r:vm0f…"],
  "result":      {"list": {"heads": [six ids], "unread": "0"},
                  "mb":   {"unread": "0", "value": "1027"},
                  "runs": {"unread": "0", "value": "6"}},
  "result-hash": "res:d8eu…"
}
```

It is 1,097 bytes. Each entry below is about one part of it.

### TK1: a tick is an ordinary record

**Question.** Is a tick a record like any other, of kind `tick`, with the tick as its payload? Or is it a new sort of stored thing?

**Example.** With the tick above in the store:
- `loops get r:…` shows it. `export`, `import`, `erase` and a sync treat it as they treat any record.
- You cite it by its record id, as you said on 2026-11-21.
- The NAS computes the same tick from the same six records. It writes its own record: another observer, another time, another id. The two payloads are the same bytes.

**What "ordinary" means, exactly.**
- A store takes a tick as it takes any record. Nothing checks the payload when it is written or imported. Checking is a verb ([TK7](#tk7-checking-a-tick)).
- A tick has no id but its record id. Two ticks say the same thing when their payloads are the same bytes.
- A content tick and a witness tick are the same record. In step 4 nothing in it says how its records were chosen. It claims only what it computed.
- Any record of kind `tick` whose payload has this shape is a tick. One whose payload does not is "not a tick" to the check, and still a record.

**Options.**
- **A (recommended).** An ordinary record. The payload is the whole tick.
- **B.** As A, and the payload also holds a hash of the tick itself. The design page says a content tick's id is "the hash of vertex, covers and result". It would let a spec group equal ticks by one text field. Nothing needs it yet, and any reader can compute it from the other fields, so it can be added later without touching a tick already written.
- **C.** A stored value that is not a record: no observer, no signature. Then citing, sending, syncing and erasing a tick each need new rules.

### TK2: a tick carries its result

**Question.** Does the tick hold the result, or only the result's hash?

**Example.** The NAS was sent the tick for the night of the 16th and never got the three backup records under it.
- **With the result:** `loops get` on the NAS shows `mb` 503 and the three ids. A weekly tick over the nightly ticks can be read there.
- **With the hash only:** the NAS sees `res:5itq…` and can read nothing. Only a holder of the three records learns what the tick said.

**The cost is size.** The example tick is 1,097 bytes with its result and 643 without. A result that lists ids is as long as its list.

**Options.**
- **A (recommended).** The result, and its hash beside it. The hash is what two machines compare, and a spec can test or group by it. A tick whose hash is not the hash of its result is not a tick.
- **B.** The hash only.
- **C.** The result only. The hash can be computed, but then no text field names the result.

**What A costs beyond size: a tick outlives an erasure.** A result holds more than ids. A slot with `by` on a payload field writes that field's values as its group names. You ruled on 2026-11-10 that erasure is the emergency stop, so that the content travels no further. Erasing a backup record does not reach a tick that covered it: a `by svc` result in that tick still says `jellyfin`, on every machine the tick was sent to.
- The rule of conduct: when you erase a record, erase the ticks that cover it too. They are records, so `loops erase` works on them.
- Finding them is one test: `where covers is <id>` ([TK6](#tk6-what-a-spec-reads-of-a-tick)). `loops erase` can list them when it runs.
- Option B does not avoid this. A hash of a small result can be guessed, and a tick with only a hash is of no use to a reader without the records.

Neon raised carrying the result; margins-claude and I agree with A. margins-claude raised the erasure point.

### TK3: how a tick names its spec

**Question.** How does a tick say which spec it folded, and does it carry that spec?

**Example.** In January you add a slot, `largest`, to `backups.json`. In February someone checks November's tick.
- **With only a hash:** the file on disk now hashes to something else. The spec that made the tick is gone unless someone digs it out of git. The tick cannot be checked.
- **With the spec in the tick:** the check needs the tick and the six records, and nothing else.

**The hash.** `spec-hash` is a hash of the spec in its parsed form, the form the `spec/parse` vectors already pin. So:
- Changing a `note` keeps the hash. Notes are not in the parsed form.
- Renaming the vertex, renaming the store in `from`, or reordering the list of kinds gives another hash.
- The hash names the spec as it is written down. Two specs that mean the same and are written differently have two hashes.

**Options.**
- **A (recommended).** The hash, and the spec itself. The spec here is 136 bytes. A tick whose hash is not the hash of its spec is not a tick.
- **B.** The hash only, as the design page has it. A tick can be checked only while someone keeps its spec.
- **C.** A spec becomes a record in the store, and a tick cites that record's id. That belongs with standing ("a store carries its vertices"). A does not block it.

**The store name selects nothing in a tick.** `covers` already says which records were folded. A check runs the spec's `kind`, `about` and `of` tests and its slots over those records, and never asks which store. The name stays in the hashed text, so two machines write one tick only when they hold the same spec text.

**A second choice inside this one.** Should the hash leave out the vertex's name and the store names, so that it names only what the fold does? I recommend no: one written form, already pinned, is simpler than a second form made for hashing.

### TK4: `covers` lists every record that was folded

**Question.** What does `covers` hold?

**Proposal.** The ids of every record the fold was given, each once, sorted. The result is what the spec gives when run over exactly those records and no others.
`loops tick` gives the fold what the store holds, or a cut of it: all of it, and not only the records the spec takes. In the example the two are the same six, because the store holds nothing else. The second example below shows why they can differ.

**Example: a late record.** The NAS holds the six records and a seventh, a pihole backup that reached only the NAS.
- `loops fold backups.json` on the NAS today: `runs` 7, `mb` 1031, hash `res:pm7t…`. That is the NAS's present, not the tick.
- Folding only the six that the tick covers, on the NAS: `runs` 6, `mb` 1027, hash `res:d8eu…`. The same hash as the laptop's. I ran this today by folding a store that holds only the six.

So the list is what makes the tick mean the same on both machines.

**Example: a record the spec does not take.** jellyfin's disk is replaced and you write a record of another kind over its first backup: `loops emit retired "this backup is void" --prev r:kvvv…`. A second spec, `current`, counts the backups that nothing has replaced. Run today:

| Folded | `current` | `mb` |
| --- | --- | --- |
| The store: the six backups and the `retired` record | 5 | 615 |
| Only the six records the spec takes | 6 | 1027 |

The spec takes backups only, and its answer still depends on the `retired` record. A tick whose `covers` held only the six would be a true statement about those six, and it would not be what `loops fold` shows you. So `covers` holds the `retired` record too.

**Example: a tick over ticks.** You make a tick for each night, T16 and T17, and a weekly tick over them and one note.
- The weekly tick's `covers` holds three ids: T16, T17 and the note. It does not hold the six backup records.
- T16 and T17 are folded as records. The weekly fold reads what a spec can read of a tick ([TK6](#tk6-what-a-spec-reads-of-a-tick)) and never opens them up.
- Checking the weekly tick needs those three records, not the six.
- The weekly tick is therefore not a tick over the six. It is another fold, with another result.

This is Neon's subagent example: an orchestrator's tick covers two subagent ticks and its own three records, five ids and not 98.

**What `covers` never holds.** "Everything tick X covers, and these too." It is shorter, but a check must then walk back through X, and what X names, to the start. All three agents reject it.

**How the set is chosen in step 4.** `loops tick` would take one of: every record the store holds; the records as of a time (by the ruled rule, section 2 of the cut note); or the ids you name, with what they build on. Whichever it is, the tick lists the ids outright. An empty list is allowed: "the monitor saw nothing all night" is a tick.

**The cost is size.** An id in the list is 57 bytes. Any tick made in homelab-audit lists all 9,053 ids, 516 KB, also for a spec that takes ten records. Two ways make ticks small, and neither changes the record:
- Name the records: a tick for each night over that night's records, and a weekly tick over the nightly ticks, as above.
- Cover only what the spec's answer depends on. The law that says which records those are is on main (`spec.slice`), and margins-claude's Python computes the set. The Rust does not yet. I would build it straight after step 4's exit, not in it.

**Options.**
- **A (recommended).** Every id, outright.
- **B.** Ids, and "what tick X covers": rejected above.
- **C.** A hash of the set and no ids. Tiny. Then no one can say which records were folded, a late record cannot be seen, and an emit cannot use the tick as its base.

### TK5: which covered records were read without a payload

**Question.** A covered record may be held without its payload: erased, or never sent. How does the tick say which were read that way?

**Example.** On a copy that holds the six records, I erased jellyfin's first backup (`r:kvvv…`) and folded the same six:

| | `runs` | `mb` | `list` | Hash |
| --- | --- | --- | --- | --- |
| Payload held | 6 | 1027, 0 unread | the six ids | `res:d8eu…` |
| Payload erased | 6 | 615, 1 unread | the same six ids | `res:jv9h…` |

Same spec, same six ids, two results. You ruled on 2026-11-20 that the tick must say which covered records were read open, so that one set of inputs never names two results.

**Proposal.** A list, `withheld`: the covered ids that were folded without a payload, sorted. It is left out when it is empty, so most ticks have no such member. The second holder's tick holds `"withheld": ["r:kvvv…"]`.
- The spec's hash, `covers` and `withheld` together are the tick's inputs. Equal inputs give equal results; the fold laws already say so.
- Erased and never sent are one state here, as you ruled on 2026-11-10.
- The list has one right content: exactly the covered records that were folded without a payload. When `covers` holds a record and the record that erases it, the erased one is in `withheld`. A tick that leaves it out says the same thing in other bytes, so it is not a tick, and the check says so.

**What a checker with other holdings does.**
- The tick says `r:kvvv…` was withheld, and the checker holds its payload: the checker leaves that payload unread and checks.
- The tick read all six open, and the checker lacks one payload: the answer is "cannot check" ([TK7](#tk7-checking-a-tick)). The checker can write its own tick, with `withheld` stated. That is another tick, because its inputs differ.

**Options.**
- **A (recommended).** `withheld`, left out when empty.
- **B.** A list of the records read open. It doubles nearly every tick.
- **C.** Each entry of `covers` is an object, `{"id", "open"}`. Then `covers` no longer reads as a set of ids to a spec ([TK6](#tk6-what-a-spec-reads-of-a-tick)).
- **D.** Nothing is stated. That is against the ruling.

The name is yours to change. `withheld` is the word a reading already prints.

### TK6: what a spec reads of a tick

**Question.** When a tick is itself folded (a weekly tick over nightly ticks, an orchestrator over subagents), what can the spec read of it?

**Proposal.** Nothing new. A tick is a record, and its payload is read by the rules you set on 2026-11-21:
- `spec-hash` and `result-hash` are text: `is`, `not` and `by` work on them.
- `covers` and `withheld` are lists, so they read as sets: `is` asks "does it cover this record", and `by` groups a tick under each record it covers.
- `spec` and `result` are objects: present, with no value a test or a sum can read.

**Example, run today.** I wrote three ticks by hand into the laptop's store: T16 (the three records of the 16th), T17 (the three of the 17th) and TJ (jellyfin's two). Then I folded a spec over the ticks:

| Slot | Asks | Answer today |
| --- | --- | --- |
| `count` | How many ticks | 3 |
| `count` where `covers` is `r:kvvv…` | Which cover jellyfin's first backup | 2 (T16 and TJ) |
| `count` by `covers` | How often each record is covered | jellyfin's two records: 2 each; the other four: 1 each |
| `count` by `spec-hash` | Ticks of each spec | 3 under `spec:gs9s…` |
| `sum` of `result` | A total across the ticks | 0, with 3 unread |

**What this gives.** A weekly tick can count and list the nightly ticks, find the ticks that cover a record, and find records covered twice.

**What it does not give.** It cannot add up `mb` across the nightly ticks. The number 503 sits inside `result`, and the rules read no member inside an object. It is also written as the text `"503"`, and a sum does not read text.
The weekly total today is a tick over the six records themselves: 1027.

**Options.**
- **A (recommended).** Nothing new in step 4.
- **B.** Two additions: a field name that reaches inside an object (`result.mb.value`), and a sum that reads a number written as text. Each changes what every spec answers, not only for ticks. Numbers as text is already coming with the import. I would take B up when a real spec needs a total across ticks.
- **C.** The tick repeats some numbers at its top level as JSON numbers. A sum past 2^53 does not fit in one, and someone must choose which numbers.

**A caution for B, whenever it comes.** TJ overlaps T16 and T17. A total across the three would add jellyfin's backups twice: 503 + 524 + 831 = 1858, against 1027. The `by covers` slot above is how a spec sees that overlap.

### TK7: checking a tick

**Question.** How is a tick checked, and does a store refuse a tick that fails?

**Proposal.** A verb, `loops tick --check <id>`, with three answers:

| Answer | When |
| --- | --- |
| `same` | The store holds every covered record, with a payload for each one the tick read open. The fold gives the tick's result, byte for byte. |
| `differs` | The same, and the fold gives another result. Both hashes are shown. |
| `cannot check` | Covered records are not held, or payloads the tick read open are not held. It says how many of each. |

A record that is not a tick is refused with a reason: for example its `result-hash` is not the hash of its `result`. The refusal codes are chosen when it is built, under [W4](2026-11-21-order-of-work.md#w4-the-refusal-for-input-that-is-wrong-twice).
A tick whose own payload the store does not hold, because it was erased or never arrived, answers `cannot check`, not "not a tick" (Neon).

**Example** (the verb is not built; this is what it would answer). The laptop writes the tick over the six records and the NAS has synced.
- `loops tick --check` on the NAS: `same`.
- On a machine that holds four of the six: `cannot check: 2 covered records are not held`.
- On the copy where `r:kvvv…` is erased: `cannot check: 1 payload the tick read is not held`.

**Step 4 is done when** the first of those answers `same` on a second machine. Put the other way: the NAS makes its own tick over the same six, and its payload is the laptop's, byte for byte.

**What the check does not do.**
- It does not refuse a tick at the door. `import` and a sync take a tick that would answer `differs`. A store that lacks the covered records cannot check, so a rule at the door would depend on what had arrived first, and two copies of one store could then differ. Whose ticks to believe is a question for standing (step 3).
- It checks one level. For the weekly tick it folds T16, T17 and the note. It does not check T16 itself.
- It does not say that the covered records were all there was. It reports two things beside its answer:
  - Whether `covers` holds everything its own records build on. That is read from the covered records alone, so it is the same on every machine. A tick for which it is "no" is still a tick; it is a statement about a set that no one need ever have held.
  - How many more records the spec would take from this store now: "1 record the spec takes is not covered". That is the late record of [TK4](#tk4-covers-lists-every-record-that-was-folded). It counts only records the spec takes. It misses a late `retired` record, which the spec does not take; seeing those needs the function TK4 leaves for after the exit.

**Options.**
- **A (recommended).** The verb, three answers, and no rule at the door.
- **B.** A store refuses a tick whose result it can recompute and finds different. Rejected above: the answer would depend on the order of arrival.
- **C.** No verb. To check a tick you make your own and compare.

### TK8: the name of the window field

**Question.** You ruled on 2026-11-20 that a window is a `where` test on the time that content order already uses: a record's `occurred` when it states one, and its `emitted` when it does not. The field's name was left open.

**Example, run today.** The NAS holds eight backup records. Seven state `occurred` (the 16th or the 17th). Tonight's states none. All eight were written today. "The backups of the 21st":

| Test on | Bounds 21 Nov to 22 Nov | Why |
| --- | --- | --- |
| `env.occurred` | 0 | Tonight's record states no `occurred`, so it passes neither bound |
| `env.emitted` | 8 | Every record was written today, the backfilled ones too |
| the window field | 1 | `occurred` where stated, otherwise `emitted` |

Only the third is the answer you meant.

**Options.** `env.when` (recommended), `env.at`, `env.time`, `env.span`.
The field takes the tests that `env.occurred` takes, and compares the start of the span as it does. Only the name is asked here.

### TK9: which rules a tick was folded under

**Question.** A spec's text can stay the same while its answer changes, because the rules changed. How does a tick say which rules it was folded under?

**Example.** This happened today. "The smallest number of snapshots any host has", a `min` of `n` over four backups:

| Binary | Answer |
| --- | --- |
| This morning's | 0, with 0 unread |
| Now, after your ruling PV2 | 9, with 2 unread |

The spec did not change by one byte. Had a tick been written this morning, a check tonight would answer `differs`, and its writer was honest.

**Proposal.** The spec's hash is made with a label, `loops/spec-hash/v1:`. That label is the version of the rules too. When a change alters what any spec answers, the label becomes `v2`, and every spec's hash changes with it.
- A check that meets a tick made under `v1` answers "cannot check: made under older rules". It does not answer `differs`.
- A change of that sort is one that comes to you under your ruling of 2026-11-21 (W7), so each new version is ruled, never slipped in.
- The design page already says this: "the spec hash pins the fold-vocabulary version".
- Nothing is at `v1` yet. `v1` will mean the rules as they stand when the first tick is built.

**What the proposal gets wrong: a newer tick on an older machine.** Neon found it. The laptop moves to `v2` and writes a tick. The NAS still runs `v1` and checks it.
- With the version only inside the hash, the NAS knows one label. The hash does not fit it, and a hash that fits no label it knows looks the same as a forged one. The NAS answers "not a tick" about an honest tick.
- With a member `"rules": "2"`, the NAS reads the member first and answers "cannot check: made under newer rules".

**Options.**
- **A.** The label in the spec's hash is the version. No new member. This was the recommendation until Neon's review.
- **B.** A member of its own, `"rules": "1"`, and the hash's label stays fixed. It can be read without trying each label. Nothing ties it to the hash, so the same spec has one hash under every version, and a store of results keyed by that hash would serve an old answer.
- **C.** Nothing. A tick made before a change of rules reads as a false tick after it.
- **D (recommended).** Both: the member `"rules": "1"`, and the label made from it, `loops/spec-hash/v1:`. The check reads the member, then requires the hash to fit that label, so the member cannot drift from the hash. It costs one short member.

**What D gives beyond the answer on an older machine.**
- After a change of rules, the ticks to make again are found by a spec: `where rules is "1"`. With A only the check verb knows.
- A check does not try each old label in turn.
- Equal inputs still give equal results, because the spec's hash still changes with the rules.

**Three things to settle with it.** The coordinator's view; none is built.
- **What "the rules" covers.** Everything between the covered records and the result's bytes: settling, admission, what a source takes, the tests, the folds and the payload reading. Not only the fold types. Standing (step 3) is likely to change admission, so a `2` is already in sight, beside the one Neon names ("numbers stored as text", in Being prepared).
- **What does not raise it.** A new fold type or test changes no existing spec's answer, so the number stays. An older machine then meets a spec it cannot parse under a number it knows. It must answer "cannot check: this binary cannot read the spec". "Not a tick" is kept for what no version could mend: a member missing, or a hash that does not fit what it is the hash of. A fault in one implementation is not a change of rules either: the model is the rule.
- **A change voids every earlier tick, also those it cannot touch.** A `count` by kind is not changed by how `min` reads a list, yet its ticks become "made under older rules" too. So beside that answer the check says what today's rules give: "under today's rules: the same result" or "another result". Neither is a verdict on the writer.

**Views.** margins-claude raised the question. Neon proposes D (its words: option B, keeping the versioned label). The coordinator agrees and has changed the recommendation from A to D.

### What follows from these, with no ruling asked

- **A reader's tick and a writer's base are one record used two ways.** You ruled that an emit may state a base (the cut note, section 8). `loops emit task --of task:42 status=done --base <tick>` reads the tick's `covers` and names the records of `task:42` in it that were live there. The tick needs no extra member for this.
- **An inbox's read marker is a private local file, not a tick.** All three agents agree. A tick is for a boundary worth citing: a handoff, a sign-off, a subagent's finished task.
- **A base that covers nothing of the thing.** If the tick you name as a base is a tick over ticks, it holds no record of `task:42`. The emit then names nothing, and stands beside whatever the store holds of the thing as a second head. That is honest: the base knew nothing of the thing. `loops emit` says "the base covers no record of task:42". margins-claude has the law for this ready: it holds for any set, closed or not.
- **2026-11-22, margins-claude, a correction to the point above.** "It holds for any set, closed or not" is wrong. The emit against a base is right for any set. The store keeps anchored names, so that its live records read alone as it does, only when the base holds every live record of what is under it in the store; a base closed downward does. Section 7 has the case that breaks, and the law is `keyed.base` in [laws.md](../protocol/laws.md).
- **Ticking again.** When a late record arrives, a new tick names the old one in `prev`, as any revision does. A record with `prev` states what it is of, so the two ticks are records of one thing: the first tick's id, or a name such as `tick:backups:2026-11-16`. No new rule.

**Not in this proposal:** boundaries and markers, the runner, where a tick lands (`into`), a record on the tick of how its set was chosen (the window's span, "by reference"), smaller `covers` by the slice laws, parameters, and pins.

## 11. The later questions, as they were put

These are the three entries of `DECISIONS.md` that came from building step 4, as Kyle read them. He ruled all three on 2026-11-22: TK10 for the recommended column in all seven rows, TK11 for option C, TK12 for option A.
"Recommended", "nothing here is built" and "not built" in the text are as written then. Each was built to its recommendation before the ruling, so the ruling changed no code: the check is in the changelog under 2026-11-21, and `loops emit --base` under 2026-11-22.

### TK10: seven details of the check

**Question.** The plan for the build, and margins-claude's check of the vectors, found seven places where TK1 to TK9 do not say what `loops tick --check` answers. What should it answer in each?

Nothing here is built, so each answer below is what the verb would say. The store is the one of the tick record's examples: six backups, and the laptop's tick T over them (`runs` 6, `mb` 1027). The build follows the recommended column until you rule otherwise; changing a row later changes one branch of the check and its vectors.

| | The case | Example | Recommended | The other choice |
| --- | --- | --- | --- | --- |
| 1 | **A payload erased here, with its bytes still on disk.** A store that imports an eraser does not rewrite its old segments. | The NAS imports an eraser of `r:kvvv…` after T was written. T does not cover the eraser. The payload's bytes are still in a segment, and `loops get r:kvvv…` no longer shows them. Run on today's binary with two stores: after the import, `get` answers the record without its payload, and one segment file still holds the payload's text. | `cannot check: 1 payload the tick read is not held`. The check reads what the store's own reading gives, and no more. | `same`, by reading the bytes. The check would then read what the store was told to forget. |
| 2 | **A covered eraser that the tick ignores, on a machine missing other records.** | A tick covers the six and an eraser of `r:kvvv…`, and leaves `withheld` out. The checker holds the eraser and four of the six. | Not a tick, at once (`tick-withheld-erased`). The eraser's envelope and the tick prove it; no missing record could change it. | `cannot check: 2 covered records are not held`, until every covered record has arrived. |
| 3 | **A tick made under another version of the rules.** | A tick states `"rules": "2"`. The NAS runs version 1. | The NAS checks only what no version changes: the members, the two id lists, that `spec-hash` fits the label for `2`, that `result-hash` fits `result`. Then `cannot check: made under newer rules`. It does not parse the spec or judge the result's shape. | Judge the spec and the result by today's grammar. A fold type added in version 2 would make an honest tick read as broken. |
| 4 | **A result that no fold could give.** | A tick like T whose `result` says `runs` 6 and lists five ids, with `result-hash` made from that. | With the six records held: `differs`. The reader checks a result's shape and spelling for its slots (an integer as decimal text, a list of ids) and nothing about whether its parts agree. | Not a tick. Each such test (a count equals its list's length, a listed id is covered) is one more rule that every implementation must share. |
| 5 | **How `rules` is spelled.** | `"1"`, `"2"`, `"10"` against `"01"`, `"0"`, `""` and the number `1`. | A positive whole number as decimal text with no leading zero, compared as a number. The others are not a tick. | Any text, compared as text. Then `"10"` sorts before `"2"`. |
| 6 | **A `spec` this binary's parser refuses.** | T with `"spec": 7` and a fitting hash; and T with a spec that uses a fold type from a later binary, with a fitting hash. | The first is not a tick: a spec is an object under every version. The second is `cannot check: this binary cannot read the spec`. Nonsense inside an object gets that answer too, since the check cannot tell it from a later binary's spec. | Treat every refused spec as not a tick. That is the false "not a tick" your ruling on TK9 removed. |
| 7 | **The result's hash on a tick from a later version.** The label inside the result's hash has its own version (`loops/result-hash/v1:`), and a later version of the rules may change the result's bytes and take a new one. | A tick states `"rules": "2"`, and version 2 writes results under `loops/result-hash/v2:`. The NAS runs version 1 and knows one label. | For `rules` above its own, the NAS checks that `result-hash` is spelled as a hash and does not ask whether it fits. Then `cannot check: made under newer rules`. For its own version and older ones it knows the label and requires the fit. margins-claude found this. | Require the fit under the label the binary knows. An honest version 2 tick is then "not a tick" on the NAS: the fault your ruling on TK9 removed from the spec's hash, back in the result's. |

**What is not asked.** The names of the refusal codes and their order for input that is wrong twice are the implementer's, under your rulings on TK7 and W4, and go in the changelog. So do the two reports beside the answer (whether `covers` holds every `prev` and `under` target its records name; how many more records the spec would take now) and the verb's flags.

### TK11: a write against a base that does not hold what is under it

**Question.** You ruled on 2026-11-20 that the filler takes a base, a tick or ids, and names that base's live records of the thing. What does it do when the base does not hold the records under its own?

**Why it comes up.** margins-claude proved the law for the filler (`keyed.base`, 2026-11-22) and in doing so found its earlier answer wrong: it had said any base will do. A base that leaves out a record under one of its own can leave the store reading wrongly from its live records. `loops tick` covers everything a store holds, so its ticks never do this. A tick over a chosen set of ids can, and so can ids named by hand.

**Example, run on today's binary** (the writes against a base made with `loops emit -`, since `--base` is not built). One thing, `svc:jellyfin`:

```
x   sets port 8096 and owner kyle
y   sets port 8097, over x
```

The base is `y` alone. Two things go wrong, and they are different.

*First, a conflict that does not exist.* A writer sets `port` to 8098 against the base and names `y`, the one record live in the base.

| Read from | `port` | Heads |
| --- | --- | --- |
| The whole store (x, y and the new record) | 8098 | 1 |
| The thing's live records alone (x and the new record) | in conflict: 8098 and 8096 | 2 |

`x` is still live, for `owner`, and the new record does not name it. A reader who holds only the live records has no record that puts the new one above `x`. A tick with a small `covers`, or a clone that fetched only what is live, is such a reader.

*Second, a value replaced that the writer was never shown.* Reading `y` alone shows `port` 8097 and no `owner`. The writer sets `owner` to sam against the base. The whole store then reads `owner` sam, one head, no conflict. `owner` kyle is gone and nothing showed it to the writer or says so now. This follows from what `prev` means: a record over `y` is over everything under `y`.

**Options.**

| | What the filler does | First problem | Second problem |
| --- | --- | --- | --- |
| **C (recommended)** | Names the live records of the base **together with what is under it in the store**. Here the new record names `x` and `y`. It says on standard error how many records under the base it named that the base did not hold. | Gone: the store always reads the same from its live records. margins-claude's law covers it (`anchored_emit_down`). | Still happens, and the envelope now names what was replaced. The line on standard error tells the writer. |
| A | Refuses the write when the base leaves out a record that is live under it. Needs a new refusal code. | Gone | Gone |
| B | Writes naming the bare base's live records, with a note. | Stays, for good, for every later reader of that thing's live records. | Still happens, and nothing names what was replaced. |

- For a base that holds everything under it, and for a base that is exactly a thing's live records, all three write the same record. That is every tick `loops tick` writes today. The count C prints is 0 for those.
- Under C, ids as a base means the ids with what is under them. That is margins-claude's answer and it is always sound.
- C opens nothing: a tick among the covered records stays one record (TK4). It follows `prev` only, inside one thing.
- B is worse than C in every case and is listed for completeness.

**Why C over A.** A is the only option that stops the second problem. But the second problem is a question you have already set aside for step 3 in another form: what a strict store does with a write whose base is behind what the store holds. "You replaced something you were not shown" belongs with it, and a strict store can refuse both then. Until then C never refuses, never leaves a store that reads wrongly, and tells the writer what it did. margins-claude recommends C as the default too.

**What it costs to change later.** The choice is in the filler and one line of output. No record already written changes.

### TK12: the exit status of the check

**Question.** `loops tick --check <id>` prints its answer on standard output. What is its exit status?

**Example, run on the release binary** (2026-11-21; the stores of the tick record's examples, as built):

| Where the laptop's tick is checked | Answer | Exit, as built |
| --- | --- | --- |
| The NAS, after importing the tick | `same` | 0 |
| A machine holding four of the six backups | `cannot check`: 2 records missing | 1 |
| A copy where one backup was erased afterwards | `cannot check`: 1 payload missing | 1 |
| The laptop, on a tick written by hand that claims 1028 | `differs` | 1 |

| | The status | |
| --- | --- | --- |
| **A (recommended, built)** | 0 only for `same`. `differs`, `cannot check` and "not a tick" exit 1, with the answer on standard output and nothing on standard error. | `loops tick --check <id> && deploy` is safe. A script that wants to tell `differs` from `cannot check` reads the answer. |
| B | 0 whenever the check completes, whatever it answers. | The status then says only "the verb ran". A script that forgets to read the answer goes on after `differs`. |

**What it costs to change later.** One line in the CLI and its walk tests. No record or answer changes.

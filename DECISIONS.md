# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6. TK10 to TK12 and S4 left it on 2026-11-22, AD1 to AD4, TK13 and W3b on 2026-11-23.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

Five questions wait on a ruling. SD1 to SD3 are on what a store says about its kinds. SD4 and AD5, put later the same day, are on reading and writing references after an import. One more is [being prepared](#being-prepared).

| | Question | Recommended |
| --- | --- | --- |
| [SD1](#sd1-a-store-describes-its-kinds-in-a-file-and-enforces-nothing) | Does a store describe its kinds in a plain file that the CLI uses and no read depends on? | A: `.loops/store.json` now, records at step 3 |
| [SD2](#sd2-a-write-that-contradicts-the-declared-key) | What does `emit` do when a write contradicts a kind's declared key? | A: stop before signing, and name the ways out |
| [SD3](#sd3-reference-fields-copied-into-about-and-erasure) | Does `emit` copy a declared reference field's value into `about`, where erasure cannot reach it? | C: describe reference fields, copy nothing in v1 |
| [SD4](#sd4-a-read-of-everything-that-refers-to-a-thing) | Under C, how does a reader ask what refers to a thing, across kinds? | A: `loops ls --refers-to`, reading the declared reference fields as a fold does |
| [AD5](#ad5-one-written-form-for-a-reference-set-at-the-import) | Does the import write a reference field as `kind:key`, the one form everything else uses? | Yes, where the old store proves it, in three rows; the homelab is imported afresh |

Every earlier entry is ruled.

- The tick record, TK1 to TK13: ruled and built. Step 4 is closed. The questions as they were put are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put), sections 10 and 11.
- The order of work, W1 to W6, and W3b, the look at the JavaScript after ticks: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).
- The adoption step, AD1 to AD4: ruled on 2026-11-23. AD1 and AD2 each for its recommendation with additions, AD3 for its recommendation in both rows, AD4 for rows 1 to 3 with row 4 the other way and a rule for old ids. The entries as put, and the additions, are in [the adoption note](notes/2026-11-22-adoption.md).

After the five entries: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## What a store says about its kinds

**Where these come from.** [The store descriptions note](notes/2026-11-23-store-descriptions.md) gathered what the homelab conversion and a collector prototype asked for. Neon sketched a file from it on 2026-11-23. On 2026-11-24 the coordinator, Neon and margins-claude went through the sketch in a chat room kept in a store, and checked it against the six live homelab stores (6,482 things) and the prototype's store (1,994 items). Astra then reviewed the entries from outside, as put, and found a fault in each of SD2 and SD3; SD3's recommendation changed from A to C on it. The four now agree on every point below. Three points are yours: the form (SD1), the one place the CLI would decline a write (SD2), and a consequence for erasure (SD3).

None of the three changes the record format, rewrites an existing record, or changes a law, a vector, or what a fold or a tick answers, so W7 does not send them here. One thing does change: the same shorthand command writes a different `of`, and so a record with another id, depending on the file. They are here because standing (step 3) will grow from whatever is chosen now.

"Today" below is the `loops` binary built from main on 2026-11-24. Every "today" answer was run.

## SD1: a store describes its kinds in a file and enforces nothing

**Question.** Does a store carry a description of its kinds that the CLI uses for shorthand and defaults, that no read depends on, and that refuses nothing? And is it a plain file now, or records from the first day?

**Example.** The description of the homelab coordination store, holding only what that store's records bear out:

```json
{
  "version": 1,
  "note": "Operational coordination for the homelab: what was learned operating it.",
  "kinds": {
    "deploy": {
      "note": "Rollouts: releases, host changes, migrations.",
      "key": "name",
      "key_format": "<slug>-<YYYY-MM-DD>",
      "preview": ["name", "service", "message"]
    },
    "decision": {
      "note": "Operational stances, revisable when context changes.",
      "key": "topic",
      "preview": ["topic", "message"]
    },
    "thread": {
      "note": "Open questions and design conversations in flight.",
      "key": "name",
      "lifecycle": { "field": "status", "closed": ["closed", "resolved", "done", "passed"], "active": ["open"] }
    },
    "task": {
      "note": "One implementation slice.",
      "key": "name",
      "key_format": "<arc>/<slice>",
      "refs": { "source": ["decision"] },
      "lifecycle": { "field": "status", "closed": ["done", "dropped"], "active": ["open", "in-progress"] }
    },
    "seal": { "note": "Session wrap markers.", "key": null }
  }
}
```

A kind may state eight keys. What each one does:

| Key | Today | With the file |
| --- | --- | --- |
| `key` | Every write states its key twice: `emit deploy --of deploy:<slug> name=<slug>`. | `emit deploy name=<slug>` fills in `--of deploy:<slug>`. The record is the same one: `of` is still written in full. |
| `headline` | `loops ls decision: --plain` shows "—" for all 189 decisions. The headline is read from `name`, and decisions are keyed by `topic`. | The plain views show the key field, or the field `headline` names. |
| `lifecycle` | The session hook hides closed threads with a status list copied into a jq filter. | `loops ls thread: --active` hides the 57 threads whose status is in `closed` and shows the other 68. |
| `preview` | `--fields a,b,c` on every call. | The default columns of `ls <kind>: --plain`. |
| `refs` | A reference is stated twice, as a field and as `--about`. | See [SD3](#sd3-reference-fields-copied-into-about-and-erasure). |
| `note`, `key_format`, `search` | A table in the repository's CLAUDE.md, copied by hand from the old vertices. | Printed by `loops describe`. They drive nothing. |

**Why `closed` does the hiding, and not `active`.** If `--active` showed only the statuses listed in `active`, 7 of the 125 threads would vanish from it (4 with no status, 2 `parked`, 1 `closed-bedrock-deferred`), and 6 of the 18 tasks (5 `built`, 1 `complete`). With `closed`, a thing with a missing, conflicting or undeclared status stays in view. `active` is kept as documentation. `--active` takes a prefix that names one kind, and that kind must declare a lifecycle; otherwise it is a usage error.

**The boundary.** All four agree on these. The tests they name are for the build to pass; today's binary cannot show them.

1. A reader of records never needs the file. `of` and `about` are written in full in every record.
2. With no flag that names the description, the JSON of every read is byte for byte the same with and without the file. That is a test in the suite, not a sentence. `--active` is the one such flag, and its output states the field and the list it applied, so two copies of a store that answer differently say why.
3. `headline` and `preview` change `--plain` output only.
4. With no file, every command behaves as it does today. A file that does not parse, or that holds a key this version does not know, stops the commands that would have used it and names the file: shorthand `emit`, the plain views, `--active` and `describe`. Reads in JSON go on working, so a broken file never locks a store.
5. `emit -`, `emit - --fill` and `tick` never read the file. A collector's or an import's records cannot change because the file was edited. If shorthand ever reaches a batch, that batch pins the file's hash, as the import's journal pins its flags.
6. `loops describe` prints the description, then how the store fits it. For each kind: things whose key field differs from their key, keyless roots of a keyed kind, lifecycle values in neither list, and reference values that no mention carries. It changes nothing. It is where drift shows. The last count is not a fault to repair: a reference may be left out of `about` on purpose.

**Options.**

| | The description is | Cost |
| --- | --- | --- |
| **A (recommended)** | A plain file, `.loops/store.json`, beside `.loops/vertices/`. At step 3, `loops apply` compares the file with the store's declarations and emits one record, as the rethink has it. The expectation is that these keys carry over and `fields`, `writers` and the rest come in beside them. That is step 3's to settle, not promised here. | The file is not signed, has no history, and does not travel with `export`. Two copies of a store can describe themselves differently until step 3. Point 2 is what makes that bearable: nothing a spec answers depends on it. |
| B | Records from the first day, of a reserved kind. Signed, with a history, and they travel with the store. | A reserved kind and its rules, before standing has ruled on any. Step 3 keeps or converts whatever is written now. |
| C | Nothing until step 3. | The key table in CLAUDE.md, the status lists in jq and the doubled key all stay, and the next real import happens without a description. |

**What changes for you.** Under A, one file for each store, written once from the old vertices and the CLAUDE.md table. Nothing is refused for departing from it, except as SD2 asks.

## SD2: a write that contradicts the declared key

**Question.** Once a kind declares its key field, `emit` can see two slips. Does it stop, or warn and write?

**Example, run on today's binary** in a scratch store. `deploy` is keyed by `name`.

```
$ loops emit deploy --of deploy:authentik-2026-11-24 name=authentik-2026-11-23 service=authentik "Authentik to 2026.10"
r:t6rrbeb2…                                             exit 0
$ loops emit deploy service=authentik "rolled back"
r:51lrf6e6…                                             exit 0
$ loops ls --things --plain
THING                        HEAD KINDS  RECORDS  HEADS  HEADLINE
deploy:authentik-2026-11-24  deploy      1        1      authentik-2026-11-23
r:51lrf6e6                   deploy      1        1      —
```

The first is a deploy whose name says one day and whose key says another. The second is a deploy with no key, which `ls deploy:` does not list. Both are signed and permanent.

The live homelab stores hold 37 keyless roots of keyed kinds, each a slip of the second sort. The first sort has not happened yet: in all 6,437 keyed things the key field equals the key, because the old tool derived one from the other.

**Options.**

| | `emit` | Cost |
| --- | --- | --- |
| **A (recommended)** | Stops before anything is signed. The message says what is wrong and names the way out. For a missing key: state the key field or `--of`, or `--root` for a keyless root that is meant. For a key field that disagrees with `--of`: make them agree. | The first time `loops` declines a one-record write that a store would accept. |
| B | Warns on standard error and writes, as the sketch first had it. | The warning arrives after the record. Taking the slip back means an erasure, or living with a second thing. An agent's script seldom reads standard error. |
| C | Checks nothing, as today. | The 37 become more. |

**Why A keeps "nothing is refused".** The store refuses nothing. A keyless root can still be written with `--root`, and a record whose key field is meant to differ from its key through `emit -`. A is the shorthand declining to guess. It is spelled as a usage error, with no new refusal code.

**What A leaves alone.** A revision of an existing keyless root (`--of r:<id>`). `emit -` and `--fill`. A kind with no declared key, or with `"key": null`.

**A revision by `--prev`.** A revision that states `--prev` and no `--of` is of what it replaces, so it is not a missing key. A key field stated on it is still compared with that thing's key. Today it is not: after `emit deploy --of deploy:prev-case name=prev-case`, the command `emit deploy --prev <id> name=different` leaves `deploy:prev-case` with the name `different` (found by Astra, run again by the coordinator).

## SD3: reference fields copied into `about`, and erasure

**Question.** A kind's `refs` lists its reference fields. Does `emit` copy their values into `about`, so that a fold by mention finds the record? The copy lives in the envelope, and an erasure removes only the payload.

**Example, run on today's binary.** Under the sketch, `emit asset name=laptop/t14-0193 owner=person:dana-whitfield` would write the first record below. Written by hand, then erased:

```
$ loops emit asset --of asset:laptop/t14-0193 name=laptop/t14-0193 owner=person:dana-whitfield \
    --about person:dana-whitfield "ThinkPad T14, issued 2026-11"
r:fakp4mg3…
$ loops erase r:fakp4mg3… "the owner should not have been written here"
$ loops of asset:laptop/t14-0193 --plain
other fields  unknown
about  person:dana-whitfield
$ loops export --about person:dana-whitfield
(the record's envelope, with "about":["person:dana-whitfield"])
```

The payload is gone and the name is not. A stated `--about` and an `of` already behave this way. What copying would add is that a file moves a payload value into the envelope, and the writer never typed `--about`. You ruled on 2026-11-10 that erasure is the emergency stop for something that should not have been emitted.

**Why the recommendation is C.** The entry was first put with A recommended, and a rule for dropping an old copy when a field moves. Astra found a history where that rule fails. One asset, with `runs_on` declared a reference:

```
R    sets runs_on=asset:vm/a                        about: asset:vm/a
H1   revises R, moves runs_on to asset:vm/b         about: asset:vm/b
H2   revises R beside H1, sets only status          about: asset:vm/a
     R is erased
J    joins H1 and H2, moves runs_on to asset:vm/c   about: ?
```

The field reads `asset:vm/c`. H2 still carries `asset:vm/a`. Whether that mention was copied from the field or stated by hand was recorded only in R's payload, and R is erased. A rule that keeps it leaves an old target in J and in every later envelope. A rule that drops it also drops mentions a writer stated by hand. Astra ran the history on today's binary, stating by hand the mentions the rule would have written.

This is the general case and not one missed corner. Nothing but the payload records where a mention came from, and an erasure removes the payload. The rule was rewritten four times on 2026-11-24, each time after another reader found a history it failed. What was shown: a copy never misses a readable live value (margins-claude's Python reader, 22,925 holdings with forks and late erasures). What was not: that a move removes the old copy. margins-claude then checked that too. Without an erasure it does: the rule worked out per head left no old copy in 425 things at one head. After an erasure it does not: Astra's history fails, and with the stop taken at its best an old copy stays in 48 of 388 things at one head. So the trouble is in erasure, not in forks.

**Options.**

| | `emit` | Cost |
| --- | --- | --- |
| A | Copies every `kind:key` value of a declared reference field into `about`, and drops the old copy on a move where it can tell. | A name in a declared field cannot be erased. After an erasure or across a fork an old target can stay in every later envelope. It amends a sentence you ruled on 2026-11-14 ([the changelog](CHANGELOG.md), the writer): `--about` would state only the writer's own mentions, no longer the whole `about`. |
| B | As A, but only for a field that asks for it. Every other reference field is only described. | A's costs for the fields that ask. If copying is ever taken, all four prefer this to A: a field such as `owner` is then described as a reference and stays erasable. |
| **C (recommended)** | Copies nothing in v1. `refs` describes: `loops describe` prints it, a reader that cares about roles finds a kind's reference fields from it, and `describe` counts the `kind:key` values that no mention carries. `--about` is stated by hand, as today. | The write keeps its doubled reference: `rel=relationship:<agent> --about relationship:<agent>`. A fold by mention finds only what a writer stated. |

**What C does not lose today.** The question "what runs on this VM" is already answered by the field. Run on today's binary, with three services of which two run on `asset:vm/media`: a fold with `"where": {"runs_on": {"is": "asset:vm/media"}}` under `"follow": "keys"` lists the two. After the record of one is erased it lists the other and says "1 not read: needed payload or field unavailable", which is what an erasure should do. What a mention adds is one fold of everything that mentions a thing, across kinds and fields.

And every reference value in the live stores is still in the imported `kind/key` form: in the four kinds sampled, 566 `runs_on` records in the audit store, 121 `source` records on tasks and 92 `rel` records on observations. Copying would have done nothing for them until a value was rewritten.

**What comes with C.** `--add-about` and `--drop-about` ([the note](notes/2026-11-23-store-descriptions.md#4-who-wrote-it-and-the---about-hazard), section 4), which are CLI only and need no ruling. They keep a hand mention through later revisions.

**When copying comes back.** As its own entry, once a reader needs what the field cannot give: one fold of everything that mentions a thing, across kinds. It would be for each field that asks (B), and it would come with a check that a move removes the old copy, not only that a copy never misses.

## After the import: reading and writing references

**Where these come from.** On 2026-11-24, after SD1 to SD3 were put, Kyle trialled the import on a second, private group of stores. Nothing from it is quoted here. It showed what the homelab had hidden. The import gives a mention to each reference the old tool recorded with an id, and after it nothing keeps the mentions in step with the fields: under SD3's option C a new or changed reference adds no mention, and a move leaves the old one. The same four went through that in the room. Two questions came out. SD4 is a read. AD5 changes what the import writes, and the homelab would be imported afresh.

One thing needs no ruling and should follow SD1 to SD3 directly: a store found by name on every verb ([the note](notes/2026-11-23-store-descriptions.md#3-a-store-found-by-name-on-every-verb), section 3). In the private group a large share of references name a thing held in another store of the group, so a read by reference and `describe` both have to read the group.

## SD4: a read of everything that refers to a thing

**Question.** Under SD3's option C nothing copies a reference into `about`. How does a reader then ask "what refers to this thing", across kinds and fields?

**Example, run on today's binary** in a scratch store. A service is written with its mention, then moved without one:

```
$ loops emit service --of service:jellyfin name=jellyfin runs_on=asset:vm/media --about asset:vm/media "media server"
$ loops emit service --of service:jellyfin runs_on=asset:vm/apps "moved to the apps VM"
$ loops export --about asset:vm/apps | wc -l
0
$ loops export --about asset:vm/media | wc -l
2
$ loops fold on-apps.json --plain          (where runs_on is asset:vm/apps, follow keys)
  thing  service:jellyfin  jellyfin
```

The mention still says the media VM, because a write with no `--about` copies the heads' mentions (your ruling of 2026-11-14). The field says the apps VM, and the fold on the field is right. This is not SD3's doing; it is true today.

The import mentions every reference that the old tool recorded with an id: in the six homelab stores, all 8,177 such live values. It does not mention 9 written in full without one, or the 434 bare keys of AD5's row 2 (margins-claude). After the cutover a revision inherits its thing's mentions, as the example shows. What fails is the change: a new or changed reference does not add its target unless the writer types it, and a move does not remove the old one unless the writer drops it. The homelab has reference fields in a handful of kinds. The private group has them in many more.

**Options.**

| | A reader asks | Cost |
| --- | --- | --- |
| **A (recommended)** | `loops ls --refers-to <thing>` (the spelling is open). It takes each kind's reference fields from `refs` in the description, and asks of each the test a fold's `where` asks with `is`. It lists the things for which a field answers yes. | The answer depends on the file: a field that is not declared is not read. So the output states the stores read, the fields read for each kind, and three counts beside the list. |
| B | Nothing new. A fold on the field, one spec for each kind and field, as today. | No one read across kinds. Each reader keeps its own list of reference fields. |
| C | Copying comes back for the fields that ask (SD3's option B). | SD3's costs, for those fields. |

**What A answers, exactly.** These were settled by histories Astra ran on today's binary (15 of them), and they are [the spec](protocol/spec.md) as written, not a new rule.

- **Each field answers yes, no or unknown,** as `where` does. Every live setter of the field is asked whether it holds the target. Yes when all say yes. No when all say no, or when no record sets the field. Unknown when their answers differ, or when a live setter is erased or withheld. Setters may hold different values and still agree: the lists `[a, b]` and `[a]` both say yes for `a` and no for `c`, and only `b` is unknown. Run on today's binary: one service, `runs_on` moved by two writers apart, to `asset:vm/a` and to `asset:vm/b`. A fold with `is asset:vm/a` lists nothing and says "1 not read", and so does `is asset:vm/b`. `--refers-to` says the same: it does not list a thing because one setter holds the target.
- **Across a kind's fields: any yes lists the thing.** Otherwise any unknown counts it once as not read. Otherwise it is not listed.
- **The value is matched as written:** the whole text equal to `kind:key`, or an immediate member of a list. No comma text is split, and no kind is guessed for a bare key. A thing that names itself is found like any other.
- **Across stores, the records are put together first,** and read after. Read store by store, a thing whose later record is in another store would keep its old target.
- **Three counts go with the list:** things not read; values in the fields read that are not written `kind:key`; and the fields read. A "no" is only as good as these. In the audit store 434 live values in reference fields are a bare key and would answer no.

**What A does not change.** No record, fold, tick or law. `about` stays what writers state. The two then have separate jobs: a reference field is live, has a role, and can be erased; a mention is permanent and has no role.

**In `describe`.** A reference's target is counted as held here, held in the group, or not in the group. None of these is a fault. "Not typed" is about the form alone: a readable value that is not written `kind:key`. `asset:missing` is typed, whether or not anything holds that thing. A value that could not be read is counted apart from both.

**One choice inside A.** When two stores of a group describe the same kind with different `refs`, A reads the union of the fields. Adding a store can then make a field in another store take part, so the output says which description named each field.

## AD5: one written form for a reference, set at the import

**Question.** The import keeps a reference field as the old tool wrote it, `runs_on: asset/vm/media`. Everything written since uses `asset:vm/media`: `of`, `about`, and the field form you ruled in E1. Does the import write the one form?

**Example, run on today's binary** on the live audit store, where 58 things run on the media VM:

```
$ loops fold on-media.json --plain         (where runs_on is asset:vm/media)
q  —
$ loops fold on-media-old.json --plain     (where runs_on is asset/vm/media)
(58 things listed)
```

A reader has to know to ask in the old form. Every reference value kept with a twin is in it: 13,854 facts in the six stores, none in the new form. The old form cannot be read back reliably: 5,186 of the 6,438 named things have a slash in their key, so `asset/vm/media` is `asset:vm/media` only by a rule that the first slash ends the kind. Neon confirms it was never kept on purpose. The private group is already in the new form throughout.

The old tool wrote the proof beside each value. `runs_on_ref` held the id of the fact referred to, and the import already resolves that id to a thing. In all 13,854 facts the field names the thing its twin resolves to, as the whole value, byte for byte (counted separately by Neon and by margins-claude).

**The three rows.**

| Row | Question | Recommended | The other way |
| --- | --- | --- | --- |
| 1 | A field whose twin resolves to the thing the field names | **The import writes the thing's name, `kind:key`.** Where they disagree it changes nothing and counts it. It never reads a slash as a separator. | (b) One revision for each thing after the import: no ruling needed, but the script becomes the live setter of every such field, and history keeps both forms. (c) Leave it: a reader asks for the stored old spelling by hand, SD4 finds none of these values, and they are revised later or never. |
| 2 | Five fields that hold a bare key, written without a twin by a slip in the old tool: a thread's `run` and `gap`, a finding's `run`, a baseline's `gap` and `finding` | **The group's map names the target kind for each of the five. Where that thing was imported, the import writes `kind:key` in the field and mentions the thing, as it does for a reference with a twin.** 586 values in old facts are written so. The 75 that name nothing imported stay as they are and are counted. | (b) By hand, one revision for each thing whose current value is such a key. (c) Leave them: SD4 answers no for them, no mention finds them, and `describe` counts them as not typed. |
| 3 | A field whose twin holds several ids, written as comma text | **A list**, `["asset:a","asset:b"]`, where the encoding has been reviewed and every part agrees, in the order written. | Comma text stays. `is asset:a` then never matches it, today and under SD4. |

Row 3 has no case in the homelab: outside `ref`, which AD4 took out, every twin beside its field holds one id. It matters for the private group. It also changes an answer, which is why it is a row of its own: today `is` matches comma text only as a whole, and matches a list by member ([PV4](notes/2026-11-21-payload-values.md#pv4-a-list-reads-as-a-set-under-is-and-not)). All four recommend the list, as E1 has it for a set one observer states. Under SD4's exact match, comma text with several targets can answer yes for none of them (margins-claude).

In row 2 a run's own `run` field (247 values, each naming its own thing) is left out of this repair: it is the kind's key written a second time. That is a choice about this one field. A thing may refer to itself, and SD4 reads such a value.

**The bounds, all Astra's, agreed by the four.** Field and twin agreeing is needed and is not enough.

1. **Never a kind's key field.** A `name` with a `name_ref` beside it would become a different key, and the import would have renamed a thing. The homelab has no such pair.
2. **Decided from the fact's own field and twin,** before anything is cleaned, with the group's rules for a target that is ambiguous or left out. A field is rewritten whole or not at all, and each field left alone is counted with its reason.
3. **The whole value is compared** with the resolved thing's name. A key may hold a comma, so nothing is split to find a single target.
4. **A list only by row 3.**
5. **A bare key only by row 2,** for a kind and field the map names. That a thing of that name exists is not proof: `provider=cloud` may be prose.
6. **The same rewrite in the check and in the write,** pinned in the journal under a new version. It goes into fresh stores only. A store imported before does not take it on.

**What it costs.** The six homelab stores are imported afresh, and every imported record gets a new id and a new observer key, in stores with no reference field too: a fresh import signs with one new key for the group and a new salt for each old fact. The six are cut over, so it is a second cutover: the fresh stores take the place of the live ones, which are kept aside as the first import's were. One record has been written across the six since the import, and it is written again by hand, with its `prev` taken from the new ids. This is what [the adoption note](notes/2026-11-22-adoption.md) said a later change would cost, and today is the cheap day for it. It amends AD2, under which the import keeps every payload untouched. AD4 made the first exception when it took out `ref` and the twins.

**If you rule against row 1,** nothing is rewritten. SD4 still matches `kind:key` exactly, finds none of the old values, and reports them in its count of values not typed. Reading both forms would be a different read from a fold's, and is not proposed.

## Being prepared

Nothing here needs you yet. It is listed so that you know what is coming.

**Numbers stored as text**, with the import ([W1](notes/2026-11-21-order-of-work.md#w1-an-adoption-step-before-standing-and-sharing)). [TK6](notes/2026-11-21-tick-record.md#tk6-what-a-spec-reads-of-a-tick) shows one use for it.

## Deferred by you

Not asked now. Each comes back when its step does.

- **A strict store's policy for an emit behind what the store holds:** warn, fail, or let it through as a conflict. You raised it on 2026-11-20 and left it for step 3. margins-claude's answer is recorded in [the cut note](notes/2026-11-15-the-cut.md), section 8.
- **Refusing an undeclared payload key in a strict store.** Step 3. You ruled on 2026-11-10 that refusal is progressive.
- **A top-N across groups.** 2026-11-11: "a later discussion".
- **What the old tool's `combine` and `discover` vertices did.** 2026-11-20: they wait until they are wanted.
- **The provenance export mapping,** W3C PROV or PREMIS.

## Ruled

One line for each ruling, newest first. The full text is in [CHANGELOG.md](CHANGELOG.md) under the date, or in the note named.

| Date | Ruling | Where |
| --- | --- | --- |
| 2026-11-23 | W3b: the JavaScript stays frozen, as the rule; new steps are built in Lean and Rust | Changelog; [the order-of-work note](notes/2026-11-21-order-of-work.md#w3b-the-frozen-javascript-after-ticks) |
| 2026-11-23 | AD4: references in the old payloads become mentions of what they name, through one rule for old ids; the payload is cleaned of the fields they came from | Changelog; [the adoption note](notes/2026-11-22-adoption.md#ad4-references-in-the-old-payloads) |
| 2026-11-23 | TK13: `loops tick <spec>` with no cut named covers the records the answer depends on, with everything held under them through `prev` and `under`; for a spec that reads things by key, every record of the things it touches; `--all` covers everything | Changelog; [the tick record note](notes/2026-11-21-tick-record.md#tk13-what-a-tick-covers-when-you-name-no-cut) |
| 2026-11-23 | AD3: a fact of a kind the vertex does not name, brought in with `--include-unnamed`, is a root; the old time is kept to the microsecond | Changelog; [the adoption note](notes/2026-11-22-adoption.md#ad3-two-details-of-the-import) |
| 2026-11-23 | AD1: a store name is bound in a per-user list, in an optional list kept with a group of stores, or by `--store <name>=<dir>` on the command; a name bound to nothing refuses the whole read (`store-unknown`) | Changelog; [the adoption note](notes/2026-11-22-adoption.md) |
| 2026-11-23 | AD2: the import's eight choices as recommended, and four more: no observer mention for an empty observer; the old store's row order where it keeps no arrival number; a fact without its key is a root; a kind the vertex does not name is left out unless asked for | Changelog; the adoption note |
| 2026-11-22 | Step 4 (ticks) meets its exit criterion and is closed | Changelog |
| 2026-11-22 | TK12: `loops tick --check` exits 0 only for `same` | Changelog; the tick record note, section 11 |
| 2026-11-22 | TK10, seven details of `loops tick --check`, each as recommended: an erased payload is not read from old bytes; a covered eraser the tick ignores is not a tick at once; under newer rules the check tests only what no version changes, and a result's hash only for its spelling; a result no fold could give is `differs`; `rules` is a positive whole number as text; a spec this binary cannot read is `cannot check` | Changelog; [the tick record note](notes/2026-11-21-tick-record.md#11-the-later-questions-as-they-were-put), section 11 |
| 2026-11-22 | TK11, option C: a write against a base names the base's live records together with what is under them in the store, and says how many it named that the base did not hold | Changelog; the tick record note, section 11 |
| 2026-11-21 | The tick record, TK1 to TK9: an ordinary record of kind `tick`; it carries its result and the result's hash; it carries its spec and the spec's hash; `covers` lists every record folded; `withheld` lists those read without a payload; a spec reads a tick by the payload reading; a verb checks and no store refuses a tick; the window field is `env.when`; a member `rules` bound into the spec's hash | Changelog; [the tick record note](notes/2026-11-21-tick-record.md) |
| 2026-11-21 | The order of work, W1 to W6: an adoption step after ticks, its CLI pieces beside the tick work; a cache of checked segments in that step; the JavaScript frozen as a trial for step 4; one refusal per input inside the repository, single faults in the protocol text; the mutant check stays; `out/` stays committed | Changelog; [the order-of-work note](notes/2026-11-21-order-of-work.md) |
| 2026-11-21 | Payload values, PV1 to PV7: the reading is stated in the model; `value` and `n` are ordinary fields; `present` means written; a list reads as a set under `is`, `not` and `by`; a list is never a number | Changelog; [the payload values note](notes/2026-11-21-payload-values.md) |
| 2026-11-21 | A result hash has its own prefix, `res:`; the codec's other choices confirmed | Changelog |
| 2026-11-21 | Only a choice that changes bytes or answers comes to Kyle; other implementer choices stand unless he objects | Changelog |
| 2026-11-21 | The 20 built keyed-fold choices confirmed | Changelog |
| 2026-11-21 | Edges: three classes, no change to the envelope; an edge's payload holds `rel`, `from`, `to` and `state` | Changelog |
| 2026-11-21 | Issue #10 closed with no strict "after" operator | Changelog |
| 2026-11-20 | Step 2 meets its exit criterion and is closed | Changelog |
| 2026-11-20 | A verb that answers from a store that does not check answers and is then refused `store-not-clean` | Changelog |
| 2026-11-20 | The cut: a cut is a part closed downward over `prev`; "as of T" is down; a content tick folds a set named outright; ids without `shown` in the result; a window is a `where`; `emit` copies `occurred`; the changeset path can ask for the filler; the filler takes a base | [The cut note](notes/2026-11-15-the-cut.md), sections 1 to 8 |
| 2026-11-20 | The result codec carries no tag on an integer | The cut note, section 6 |
| 2026-11-15 | `loops of` shows values; `--values`; an emit of a held thing sets the fields it names | Changelog |
| 2026-11-15 | A keyed collect's `limit` counts things, each listed whole | Changelog |
| 2026-11-14 | A slot following `keys` reads the envelope from the heads ("Rule B"); every emit carries the thing's whole `about`; the anchor names every live record | [The declared-keys note](notes/2026-11-11-declared-keys.md), sections 3 and 5 |
| 2026-11-13 | `"follow": "keys"`, its result, how the other slots read a keyed thing, the laws and the writer | The declared-keys note, sections 1 to 5 |
| 2026-11-13 | A writer refuses an `under` target it does not hold | Changelog |
| 2026-11-11 | D7: a separate, versioned result codec with integers as decimal text | Changelog |
| 2026-11-11 | `by`: its spelling, text keys, `ungrouped`, a record in several groups, `limit` per group | Changelog |
| 2026-11-11 | `follow` on count, sum, min and max; the caching reader's law is step 4's first | Changelog |
| 2026-11-11 | Envelope shapes: one new code, `envelope-not-object` | Changelog |
| 2026-11-11 | The slice law is *witnessed*; a writer's tool fills in the anchor | Changelog, under D8 |
| 2026-11-10 | D8: `of` names a record's one thing; `about` only mentions; `sets` declares keys; natural keys; no conflict declared by the core; undeclared keys refused progressively | Changelog |
| 2026-11-10 | Withheld and erased are one state to a fold | Changelog, under D8 |
| 2026-11-10 | The order: step 2 with D8, then step 4, then step 3 with step 5 | Changelog |
| 2026-11-10 | D1 to D6, as their recommendations | Changelog |
| Step 1 | R1 to R6; binary64 by Option A | Changelog |

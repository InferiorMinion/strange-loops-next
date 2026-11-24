# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6. TK10 to TK12 and S4 left it on 2026-11-22, AD1 to AD4, TK13 and W3b on 2026-11-23.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

Three questions wait on a ruling: SD1 to SD3, on what a store says about its kinds. One more is [being prepared](#being-prepared).

| | Question | Recommended |
| --- | --- | --- |
| [SD1](#sd1-a-store-describes-its-kinds-in-a-file-and-enforces-nothing) | Does a store describe its kinds in a plain file that the CLI uses and no read depends on? | A: `.loops/store.json` now, records at step 3 |
| [SD2](#sd2-a-write-that-contradicts-the-declared-key) | What does `emit` do when a write contradicts a kind's declared key? | A: stop before signing, and name the ways out |
| [SD3](#sd3-reference-fields-copied-into-about-and-erasure) | Does `emit` copy a declared reference field's value into `about`, where erasure cannot reach it? | A: copy, and have `describe` say so |

Every earlier entry is ruled.

- The tick record, TK1 to TK13: ruled and built. Step 4 is closed. The questions as they were put are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put), sections 10 and 11.
- The order of work, W1 to W6, and W3b, the look at the JavaScript after ticks: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).
- The adoption step, AD1 to AD4: ruled on 2026-11-23. AD1 and AD2 each for its recommendation with additions, AD3 for its recommendation in both rows, AD4 for rows 1 to 3 with row 4 the other way and a rule for old ids. The entries as put, and the additions, are in [the adoption note](notes/2026-11-22-adoption.md).

After the three entries: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## What a store says about its kinds

**Where these come from.** [The store descriptions note](notes/2026-11-23-store-descriptions.md) gathered what the homelab conversion and a collector prototype asked for. Neon sketched a file from it on 2026-11-23. On 2026-11-24 the coordinator, Neon and margins-claude went through the sketch in a chat room kept in a store, and checked it against the six live homelab stores (6,482 things) and the prototype's store (1,994 items). They agree on the shape. Three points in it are yours: the form (SD1), the one place the CLI would decline a write (SD2), and a consequence for erasure (SD3).

None of the three changes a record's bytes, an id, a law or a vector, so W7 does not send them here. They are here because standing (step 3) will grow from whatever is chosen now, and because SD3 amends a sentence you ruled on 2026-11-14: what `--about` states.

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

**The boundary.** All three agree on these.

1. A reader of records never needs the file. `of` and `about` are written in full in every record.
2. With no flag that names the description, the JSON of every read is byte for byte the same with and without the file. That is a test in the suite, not a sentence. `--active` is the one such flag, and its output states the field and the list it applied, so two copies of a store that answer differently say why.
3. `headline` and `preview` change `--plain` output only.
4. With no file, every command behaves as it does today. A file that does not parse, or that holds a key this version does not know, stops the commands that would have used it and names the file: shorthand `emit`, the plain views, `--active` and `describe`. Reads in JSON go on working, so a broken file never locks a store.
5. `emit -`, `emit - --fill` and `tick` never read the file. A collector's or an import's records cannot change because the file was edited. If shorthand ever reaches a batch, that batch pins the file's hash, as the import's journal pins its flags.
6. `loops describe` prints the description, then how the store fits it. For each kind: things whose key field differs from their key, keyless roots of a keyed kind, lifecycle values in neither list, and things whose reference fields are not all mentioned. It changes nothing. It is where drift shows.

**Options.**

| | The description is | Cost |
| --- | --- | --- |
| **A (recommended)** | A plain file, `.loops/store.json`, beside `.loops/vertices/`. At step 3, `loops apply` compares the file with the store's declarations and emits one record, as the rethink has it. The keys keep their names, and `fields`, `writers` and the rest come in beside them. | The file is not signed, has no history, and does not travel with `export`. Two copies of a store can describe themselves differently until step 3. Point 2 is what makes that bearable: nothing a spec answers depends on it. |
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

**What A leaves alone.** A revision of an existing keyless root (`--of r:<id>`). A revision that states `--prev` and no `--of`, which is of what it replaces. `emit -` and `--fill`. A kind with no declared key, or with `"key": null`.

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

The payload is gone and the name is not. A stated `--about` and an `of` already behave this way. What is new is that a file moves a payload value into the envelope, and the writer never typed `--about`. You ruled on 2026-11-10 that erasure is the emergency stop for something that should not have been emitted.

**The rule, if values are copied.** After a shorthand write, `about` holds every `kind:key` value of the thing's live reference fields, of the thing as the writer holds it. The field is the answer. The mention is an index, and it may hold more.

margins-claude checked the first sentence in its own Python reader: three writers on their own holdings, forks, erasures that arrive late or never, 22,925 holdings read. The mentions miss a readable live value in none of them. A planted fault misses in 8,568, so the check can fail.

- **It amends a built and ruled sentence.** Today ([the changelog](CHANGELOG.md), 2026-11-14, the writer): `--about` states the whole `about`, and with no `--about` an emit copies the union of its heads' mentions. Under A, for a kind with `refs`, `--about` and `--no-about` state the writer's own mentions, and the copied ones are added to them. With neither, each head's mentions are carried over without the ones that head's own reference fields account for, and the live values are added. A kind with no `refs` writes as it does today.
- **A move drops the old mention.** A revision that moves `runs_on` from one asset to another drops the old target and adds the new one. So does a join of two heads, one of which had moved the field. To drop a copied mention, unset the field.
- **Except after an erasure, where the write stops.** Once the record that set `owner` is erased, its value cannot be read, so no later write can tell that `person:dana-whitfield` was a copy and drop it. Moving or unsetting the field would leave the name in every later envelope, in exactly the case where someone erased to remove it. So a shorthand write stops, as in SD2, when a reference field's live setter is erased or withheld and the heads hold a mention of that field's declared kinds that no readable field accounts for. The message names the mention and the ways out: `--drop-about person:dana-whitfield`, or `--about` stating what to keep.
- **A value that is not `kind:key` gets a warning and no mention.** Every reference value in the live stores is still in the imported `kind/key` form: in the four kinds sampled, 566 `runs_on` records in the audit store, 121 `source` records on tasks and 92 `rel` records on observations. Declaring a field copies nothing for them until a value is rewritten.
- **The rule has no memory.** A thing gets its mentions on its next write after the field is declared, and `describe` counts the things not yet caught up (SD1, point 6). No verb backfills.
- **Across a fork the mentions hold more until the heads are joined.** A fold by mention of the old target still finds the thing, while the field reads the new one. None of the 6,482 things has two heads today; forks come with sharing (step 5).

**Options.**

| | `emit` | Cost |
| --- | --- | --- |
| **A (recommended)** | Copies, by the rule above. `describe` says of each `refs` field that its values are written into the envelope and outlast an erasure. | A name in a declared field cannot be erased from then on. Only `kind:key` values are copied, never free text. A field that may hold a name someone will want erased, such as `owner`, is left out of `refs`. |
| B | Copies, unless the field says not to. Such a field is still described as a reference. | One more key in the file, and a choice to make for every reference field. It needs knowing in advance which value will need erasing. |
| C | Copies nothing in v1. `refs` only describes, and `--about` is stated by hand as today. | A fold by mention finds only what a writer remembered to state. |

**What comes with A.** `--add-about` and `--drop-about` ([the note](notes/2026-11-23-store-descriptions.md#4-who-wrote-it-and-the---about-hazard), section 4), which are CLI only.

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

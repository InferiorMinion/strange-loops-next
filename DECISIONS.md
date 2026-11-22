# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [TK10](#tk10-seven-details-of-the-check) | Seven details of `loops tick --check` that the rulings leave open | As in its table; the build follows them meanwhile | Nothing yet: the check is three merge requests away |
| [TK11](#tk11-a-write-against-a-base-that-does-not-hold-what-is-under-it) | What `loops emit --base` does when the base does not hold what is under its records | Name the live records of the base together with what is under it; say how many of those the base did not hold | The filler against a base |
| [AD1](#ad1-how-a-spec-finds-the-stores-it-names) | How a spec finds the stores it names, and what a missing one answers | A per-user list of names and directories; a name bound to nothing refuses the whole read | Reads across several stores (adoption) |
| [AD2](#ad2-how-an-old-store-becomes-records) | How a fact of an old store becomes a record: its thing, its order, its times, who signs, where its origin is kept | As in its table: a keyed fact is a revision of `<kind>:<key>`; the old time is `occurred`; an import key signs | The shadow import (adoption) |

Every earlier entry is ruled (2026-11-21). TK11 is new on 2026-11-22.

- The tick record, TK1 to TK9: ruled, and being built. The questions as they were put, with their examples, are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put).
- The order of work, W1 to W6: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).

TK10 is the first entry that comes from building the tick record. More may follow: any choice that changes a tick's bytes, a refusal code or a check's answer and that the rulings do not settle.

After this: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The tick record, while it is built

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

## The adoption step

You ruled on 2026-11-21 (W1) that the adoption step's CLI pieces are built beside the tick work. A worker planned them on 2026-11-22. The pieces that are the CLI's alone are being built: `loops ls`, a scan by the start of a name, a plain-text view with relative times, a `--version`, and the cache of checked segments (W2). Two pieces need you first. AD1 is one. The other is AD2, the import of your old stores as a shadow.

### AD1: how a spec finds the stores it names

**Question.** A spec's `from` names a store for each source. Today the name is ignored. When a read may span several stores, how is a name turned into a directory, and what does a read answer when a name leads nowhere?

**Example, run on today's binary.** Two stores: `workshop` holds one task and one note, `yard` holds one task. One spec, `morning`:

```
from:  store workshop, kind task
       store yard,     kind note
fold:  items = count
```

| Run in | Today's answer | Why |
| --- | --- | --- |
| `workshop` | 2 | It counts the task and the note it holds. `yard` is never opened. |
| `yard` | 1 | The same spec, another answer: it counts its own task. |
| `workshop`, with the one store name changed to `no-such-store` | 2 | A name bound to nothing is answered from the directory all the same. |

What the answer should be is already ruled: a fold across stores takes the union of the named stores first and runs every source over it ([the tick record note](notes/2026-11-21-tick-record.md), section 9, and `protocol/spec.md`). Here that is **3**: both tasks and the note, each record once. What is open is how the names resolve.

**Options for finding a store.**

| | A name means | Cost |
| --- | --- | --- |
| **A (recommended)** | An entry in a per-user list of names and directories (`~/.config/loops/stores.json`). The spec travels between machines unchanged; each machine says where its stores are. | You bind each name once per machine. Your existing specs name stores that are bound nowhere yet. |
| B | A directory beside the current one with that name. | Renaming a directory changes what a spec reads. Stores that live apart cannot be named. |
| C | A path written in the spec. | The path is part of the spec, so its hash, and every tick of it, differs between machines. |

**Options for a name that leads nowhere.**

| | The read answers | |
| --- | --- | --- |
| **A (recommended)** | Nothing on standard output, and a refusal that names the store (`store-unknown`). | A missing store is never mistaken for an empty one. Needs the new refusal code. |
| B | What the stores that were found give, with a note. | The same spec gives another answer on a machine that lacks a store, as it does today. |

**What changes for you.** Under A and A, a spec that names one store today keeps its answer once that name is bound to its directory, and refuses until then. I would bind the names your existing vertices use before the strict rule is switched on, and list them for you first.

**Not asked.** `loops tick` and `loops tick --check` stay on the one store in the directory. A tick across stores would cover other records and is its own piece.

### AD2: how an old store becomes records

**Question.** The shadow import copies an old store into a new one, to be read beside the old tool. Each old fact becomes one signed record. Which record? The answers decide the id of every imported record, so they are yours, and they are cheap to change only before the first real import.

**What an old store holds** (the shape of yours; the values below are made up). A table of facts, each with an id, a kind, a time, an observer written as text, and a JSON payload. A vertex file says how each kind is read: `thread` by `name`, `log` collected. So three facts:

```
01ab  thread  2026-10-01 09:00  relay  {"name":"kiln","status":"open","message":"Repair kiln"}
01ac  thread  2026-10-03 16:30  relay  {"name":"kiln","status":"done"}
01ad  log     2026-10-03 16:31  relay  {"message":"fired the kiln"}
```

The old tool reads thread `kiln` as status `done`, message `Repair kiln`: a later fact of the same name overlays the fields it carries.

**Example, run on the binary with the plain view** (2026-11-22). The three facts written by hand as the recommended column below would write them (`loops emit - --fill`, with a key made for the import):

```
$ loops ls --plain
KIND    RECORDS
log     1
thread  2

$ loops ls thread: --fields name,status,message --plain
THING        HEAD KINDS  RECORDS  HEADS  name  status  message
thread:kiln  thread      2        1      kiln  done    Repair kiln
```

The second record names the first in `prev` and sets only `status`, so `message` still reads from the first. That is the old tool's answer.

**The choices.**

| | Choice | Recommended | The other way |
| --- | --- | --- | --- |
| 1 | **What a keyed fact is of.** A kind the vertex reads "by" a field. | The thing `<kind>:<value of that field>`, here `thread:kiln`. A later fact of the same name is a revision: it names what is live in `prev` and sets the fields it carries. | Every fact a root of its own. Then nothing overlays, and each read needs a spec that groups by the field. |
| 2 | **A kind the vertex collects, or does not name.** | A root, as `log` above. | A thing per kind. |
| 3 | **The order of revisions.** | The order in which the old store took the facts in (it keeps one). That is what the old tool's reading saw. | The facts' own times. Two facts with one time need a tie-break, and a backfilled fact would land in the past. |
| 4 | **The times.** | The old time becomes `occurred`. `emitted` is the time of the import. A window on `env.when` then reads the old time. | The old time as `emitted`: the record would claim to have been written then, by a key that did not exist. |
| 5 | **Who signs.** | A key made for the import. Every imported record says so in its observer, and can be told from what you write afterwards. | Your own key. Imported and new records look alike. |
| 6 | **Where the old observer and the old id are kept.** | The old observer as a mention, `observer:relay`. The old id in a file beside the import, which also makes a second run write nothing. The payload is the old payload, untouched. | The old id as a mention on each record, or both as extra payload fields. Extra fields change what a spec reads. |
| 7 | **The old ticks.** | Left out and counted. They are the old tool's own, and as kind `tick` the new check would call each "not a tick". | Imported under another kind, for the record. |
| 8 | **A fact held by two old stores.** | Imported into each new store it was in, with the same salt and time for the same old id, from the import's file. Where the fact has the same history in both stores, the two copies are then one record, and a read across the stores counts it once. Where the histories differ they are two records; I will count how often that happens in your stores before the import. | Imported once, into one store. |

**What I am not sure of, and will check before anything is built:** whether your old stores hold kinds the vertex does not name, facts whose key field is missing, and payloads the new writer refuses (a number out of range, a repeated name). The import will count each and stop on the last kind; it will not guess.

**Not asked.** The import's verb and flags, its batches, and how the copy is compared with the old tool's reading are the implementer's. The old store is never written.

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

# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6. TK10 to TK12 and S4 left it on 2026-11-22, AD1 and AD2 on 2026-11-23.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [AD3](#ad3-two-details-of-the-import) | Two details of the import that AD2 leaves open: how a kind the vertex does not name is written when you ask for it, and how fine the old time is | Such a fact is a root; the time is kept to the microsecond | The first real import. The build follows the recommended column meanwhile |

Every earlier entry is ruled.

- The tick record, TK1 to TK12: ruled and built. Step 4 is closed. The questions as they were put are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put), sections 10 and 11.
- The order of work, W1 to W6: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).
- The adoption step, AD1 and AD2: ruled on 2026-11-23, each for its recommendation with additions. The entries as put, and the additions, are in [the adoption note](notes/2026-11-22-adoption.md).

After this: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The adoption step

### AD3: two details of the import

**Question.** A worker planned the import on 2026-11-23, and its plan found two places where AD2 does not say which record is written. Both decide record ids, so they are cheap to change only before the first real import. What should each be?

The build follows the recommended column until you rule otherwise. Nothing real is imported before you do.

| | The case | Example | Recommended | The other choice |
| --- | --- | --- | --- | --- |
| 1 | **A kind the vertex does not name, brought in with the flag.** Your addition (d) leaves such facts out unless asked for. When you ask, what is each one? | The workshop's vertex no longer names `repair`. The old store still holds two `repair` facts, both with `name` kiln: status open, then status done. | **Two roots**, as row 2 of AD2 says for a kind the vertex does not read by a field. Nothing overlays: both facts can be read, and there is no thing `repair:kiln`. | Keyed by `name`, as the store's other kinds are: one thing `repair:kiln` that reads `done`. In a read across stores this is a second history of the thing beside the store that now holds the kind: the case (d) was added to prevent. |
| 2 | **How fine the old time is.** The old store keeps a time as a number of seconds with a fraction. It becomes the text of `occurred` (row 4). | The stored number `1793610000.1234567`. A number of this size can only step by about 0.24 millionths of a second. | **To the microsecond:** `2026-11-02T09:00:00.123457Z`. That is as fine as the number is, and it is what the old tool shows. | To the nanosecond: `2026-11-02T09:00:00.123456717Z`. The last three digits are an effect of how the number is stored, not a time anyone wrote. |

**Row 1, run on today's binary** (written by hand both ways, with a scratch key):

```
as roots                              keyed by name
$ loops ls --plain                    $ loops ls repair: --fields name,status --plain
KIND    RECORDS                       THING        RECORDS  HEADS  name  status
repair  2                             repair:kiln  2        1      kiln  done
$ loops ls repair: --plain
(nothing: no thing is named repair:…)
```

**What it costs to change later.** Either row changes the id of every record it touches. After a real import, changing one means a fresh shadow store.

**Not asked; the plan settled these under your rulings, and they go in the changelog when built.**
- **The import stops and does not guess** when a fact it would bring in has a key field that is empty, `null` or not text; a kind that is literally `tick`; a payload that is not an object; a repeated member name; or a number out of range. It checks the whole of every store first and writes nothing if any such fact is found. None of these occurs in the four stores of the trial.
- **A revision sets every field its payload carries**, the key field too. The example in AD2 said the second `kiln` record "sets only `status`"; it carries `name` as well and sets both. The reading is the same.
- **A shadow store is the import's alone until you cut over.** A later run brings in what the old store has gained since. It refuses a shadow that holds records it did not write, and it refuses an old store whose earlier facts have changed. So during the shadow period you write with the old tool and read with either; when you start writing with the new one, importing into that store ends.
- **How the old store is read:** a small Python script in the repository exports the facts and the vertex's kinds to two text files, and the Rust import reads those. No SQLite library is added to the binary.

## Being prepared

Nothing here needs you yet. It is listed so that you know what is coming.

**What a store says about its kinds, without enforcing it.** The old `.vertex` files of real stores hold one fold, by name, which a record's `of` now carries. The rest of what they hold describes kinds: which fields refer to which kinds, a lifecycle field and its active values, the format of a key, and what to show first. The roadmap lists declarations under standing, with admission and grants. The question being prepared is whether the descriptive half comes forward, so that a viewer and `loops ls` can use it before any store is strict.

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

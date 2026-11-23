# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6. TK10 to TK12 and S4 left it on 2026-11-22, AD1, AD2 and AD3 on 2026-11-23.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [TK13](#tk13-what-a-tick-covers-when-you-name-no-cut) | What `loops tick <spec>` covers when you name no cut: everything the store holds, as today, or only the records the answer depends on | The records the answer depends on, with what is under them; `--all` for everything | Nothing yet. The binary cannot work the set out today; building that is next, and this says what it is for |

Two more questions are [being prepared](#being-prepared).

Every earlier entry is ruled.

- The tick record, TK1 to TK12: ruled and built. Step 4 is closed. The questions as they were put are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put), sections 10 and 11.
- The order of work, W1 to W6: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).
- The adoption step, AD1 to AD3: ruled on 2026-11-23. AD1 and AD2 each for its recommendation with additions, AD3 for its recommendation in both rows. The entries as put, and the additions, are in [the adoption note](notes/2026-11-22-adoption.md).

After this: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The tick record

### TK13: what a tick covers when you name no cut

**Question.** `loops tick <spec>` with no `--as-of` and no `--cover` covers every record the store holds ([TK4](notes/2026-11-21-tick-record.md#tk4-covers-lists-every-record-that-was-folded)). When the binary can work out which records a spec's answer depends on, should that smaller set be what it covers?

**Why now.** TK4 said the list's cost is size, and that the Rust would learn to cover only what an answer depends on after step 4. Step 4 is closed. John's store makes a tick at every session, and his pilot asks for this. A tick made in a store of 9,051 records lists every id, 516 KB, also for a spec that takes ten.

**Not built.** The binary cannot work the set out. The law that says which records it holds is on main (`spec.slice`, and `keyed.spec-slice` for a slot that reads things by key). In the runs below the set is named by hand with `--cover`.

**Example, run on today's binary.** The laptop's store holds the six backups of the tick record's examples and 40 shopping notes. The spec `backups` counts the runs and sums `mb`.

| The tick | `covers` | The record | Result | `loops tick --check` |
| --- | --- | --- | --- | --- |
| `loops tick backups.json`, as today | 46 ids | 3,414 bytes | `runs` 6, `mb` 1027 | `same` |
| The six backups named with `--cover` | 6 ids | 1,134 bytes | `runs` 6, `mb` 1027, the same result hash | `same` |

A tick is itself a record, so today each tick also covers every tick before it. The third tick made in this store covered 49 ids.

**The set is not only the records the spec takes.** You void jellyfin's first backup with a record of another kind: `loops emit retired "this backup is void" --prev r:saav…`. The spec `current` counts the backups that nothing has replaced. `loops fold current.json` answers `current` 5, `mb` 615.

| The tick | `covers` | Result | `loops tick --check` |
| --- | --- | --- | --- |
| The six backups named | 6 ids | `current` 6, `mb` 1027 | `same` |
| The six and the `retired` record | 7 ids | `current` 5, `mb` 615 | `same` |

The first tick is a true claim about its six records, and it is not what `loops fold` shows. The set the law names holds the `retired` record, and what erases a taken record, so a tick over it answers as the fold does. Working that set out is what the binary would learn; naming ids by hand does not give it.

| | The case | Recommended | The other choice |
| --- | --- | --- | --- |
| 1 | **What `loops tick <spec>` covers when you name no cut.** | **The records the spec's answer depends on.** `--all` covers everything the store holds. The tick is small, it says which records the answer came from, and its result is what `loops fold` shows at that moment. | Everything held, as today, and a flag asks for the small set. Nothing changes for a tick made without the flag. A tick stays as large as its store unless you remember the flag. |
| 2 | **Whether the set also holds everything under it.** The law needs one step down `prev` from a taken record. `--as-of` and `--cover` today add every held record that the covered ones build on, all the way down. | **Everything under it, as the two cuts do.** The check then reports `closure` `yes` for every tick the binary writes. It adds ids only where a taken record builds on a history of another kind. | The law's set and nothing more. The smallest tick. The check can then report `closure` `no` for a tick the binary wrote itself. |

**What a smaller tick means for a write against it.** `loops emit --base <tick>` names what is live among the tick's covered records. Run today against the tick over the six backups, for a thing the spec does not take:

```
$ loops emit plant --of plant:fern water=daily --base r:…
the base covers no record of plant:fern
$ loops of plant:fern --plain
records  2
heads  2
  water  conflict (2): daily | weekly
```

The write says it had read nothing of the fern, which is true of that tick, and the fern then has two heads. With today's default the same tick covers the fern and the write is a revision. So under row 1's recommendation a tick is a base for the things its spec takes, and `--all` makes the wider one.

**What it costs to change later.** Little. A tick lists its ids outright, so every tick already written means what it meant, whichever way the default goes. Only ticks written afterwards differ.

**Not asked.** How the set is worked out, and the flag's name. A worker plans the build first. A slot that reads things by key needs more than the set above (each thing's live records); if the plan finds a choice there that changes what a tick answers, it comes back to you.

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

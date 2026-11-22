# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [TK10](#tk10-six-details-of-the-check) | Six details of `loops tick --check` that the rulings leave open | As in its table; the build follows them meanwhile | Nothing yet: the check is three merge requests away |

Every earlier entry is ruled (2026-11-21).

- The tick record, TK1 to TK9: ruled, and being built. The questions as they were put, with their examples, are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put).
- The order of work, W1 to W6: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).

TK10 is the first entry that comes from building the tick record. More may follow: any choice that changes a tick's bytes, a refusal code or a check's answer and that the rulings do not settle.

After this: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The tick record, while it is built

### TK10: six details of the check

**Question.** The plan for the build found six places where TK1 to TK9 do not say what `loops tick --check` answers. What should it answer in each?

Nothing here is built, so each answer below is what the verb would say. The store is the one of the tick record's examples: six backups, and the laptop's tick T over them (`runs` 6, `mb` 1027). The build follows the recommended column until you rule otherwise; changing a row later changes one branch of the check and its vectors.

| | The case | Example | Recommended | The other choice |
| --- | --- | --- | --- | --- |
| 1 | **A payload erased here, with its bytes still on disk.** A store that imports an eraser does not rewrite its old segments. | The NAS imports an eraser of `r:kvvv…` after T was written. T does not cover the eraser. The payload's bytes are still in a segment, and `loops get r:kvvv…` no longer shows them. Run on today's binary with two stores: after the import, `get` answers the record without its payload, and one segment file still holds the payload's text. | `cannot check: 1 payload the tick read is not held`. The check reads what the store's own reading gives, and no more. | `same`, by reading the bytes. The check would then read what the store was told to forget. |
| 2 | **A covered eraser that the tick ignores, on a machine missing other records.** | A tick covers the six and an eraser of `r:kvvv…`, and leaves `withheld` out. The checker holds the eraser and four of the six. | Not a tick, at once (`tick-withheld-erased`). The eraser's envelope and the tick prove it; no missing record could change it. | `cannot check: 2 covered records are not held`, until every covered record has arrived. |
| 3 | **A tick made under another version of the rules.** | A tick states `"rules": "2"`. The NAS runs version 1. | The NAS checks only what no version changes: the members, the two id lists, that `spec-hash` fits the label for `2`, that `result-hash` fits `result`. Then `cannot check: made under newer rules`. It does not parse the spec or judge the result's shape. | Judge the spec and the result by today's grammar. A fold type added in version 2 would make an honest tick read as broken. |
| 4 | **A result that no fold could give.** | A tick like T whose `result` says `runs` 6 and lists five ids, with `result-hash` made from that. | With the six records held: `differs`. The reader checks a result's shape and spelling for its slots (an integer as decimal text, a list of ids) and nothing about whether its parts agree. | Not a tick. Each such test (a count equals its list's length, a listed id is covered) is one more rule that every implementation must share. |
| 5 | **How `rules` is spelled.** | `"1"`, `"2"`, `"10"` against `"01"`, `"0"`, `""` and the number `1`. | A positive whole number as decimal text with no leading zero, compared as a number. The others are not a tick. | Any text, compared as text. Then `"10"` sorts before `"2"`. |
| 6 | **A `spec` this binary's parser refuses.** | T with `"spec": 7` and a fitting hash; and T with a spec that uses a fold type from a later binary, with a fitting hash. | The first is not a tick: a spec is an object under every version. The second is `cannot check: this binary cannot read the spec`. Nonsense inside an object gets that answer too, since the check cannot tell it from a later binary's spec. | Treat every refused spec as not a tick. That is the false "not a tick" your ruling on TK9 removed. |

**What is not asked.** The names of the refusal codes and their order for input that is wrong twice are the implementer's, under your rulings on TK7 and W4, and go in the changelog. So do the two reports beside the answer (whether `covers` holds every `prev` and `under` target its records name; how many more records the spec would take now) and the verb's flags.

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

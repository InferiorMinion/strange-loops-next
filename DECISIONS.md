# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 have left it: PV1 to PV7, RC1, W7, E1 and I10, then TK1 to TK9 and W1 to W6. TK10 to TK12 and S4 left it on 2026-11-22, AD1, AD2, AD3 and TK13 on 2026-11-23.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.
## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [AD4](#ad4-the-old-ref-field) | The old tool reads a payload field `ref` as a list of references and gathers them across a thing's revisions; the import keeps it as plain text. Should its references become mentions? | Typed references become mentions on their record, and a revision carries its thing's earlier ones forward; paths and old ids stay in the payload | Using the homelab shadows for anything that follows references. The shadows exist and read as the old tool does in every other field |

Two more questions are [being prepared](#being-prepared).

Every other entry is ruled.

- The tick record, TK1 to TK13: ruled and built. Step 4 is closed. The questions as they were put are in [the tick record note](notes/2026-11-21-tick-record.md#10-the-questions-as-they-were-put), sections 10 and 11.
- The order of work, W1 to W6: ruled. The entries as put are in [the order-of-work note](notes/2026-11-21-order-of-work.md).
- The adoption step, AD1 to AD3: ruled on 2026-11-23. AD1 and AD2 each for its recommendation with additions, AD3 for its recommendation in both rows. The entries as put, and the additions, are in [the adoption note](notes/2026-11-22-adoption.md).

After this: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The adoption step

### AD4: the old `ref` field

**Question.** The first real import, of the six homelab stores on 2026-11-23, read as the old tool does in every field but one. The old tool treats a payload field named `ref` as reserved: a comma-separated list of references, taken out of the payload and gathered across all of a thing's revisions. The import keeps the payload untouched, as AD2 says, so `ref` stays plain text and only the latest one is read. Should the references in `ref` become mentions (`about`)?

**How much it touches.** In the six stores, 894 of 11,072 facts carry `ref`, with 933 references. 464 are typed references such as `thread:<name>` or `decision:<topic>`: the same form the import gives a thing's name, so each names a thing in the shadow. 457 are file paths such as `docs/<file>.md`, and 12 are old fact ids. 478 things have references; for 53 of them the old tool shows more than the latest `ref`. The platform stores have no `ref` at all.

**Example, run on both tools today.** An invented old store with one task, written with the old tool twice: first `name=kiln status=open ref=thread:firing` by potter, then `name=kiln status=done ref=decision:glaze,docs/kiln.md` by clerk.

```
$ sl read workshop.vertex --json --all          (the old tool; one row, shortened)
payload  {"name": "kiln", "status": "done"}
refs     ["decision:glaze", "docs/kiln.md", "thread:firing"]

$ loops of task:kiln --plain                      (exported and imported with today's tools)
  name    kiln
  ref     decision:glaze,docs/kiln.md
  status  done
about     observer:clerk

$ loops fold mentions.json --plain                (records that mention thread:firing)
records  0
```

| | The case | Recommended | The other choice |
| --- | --- | --- | --- |
| 1 | **A typed reference in `ref`** (`thread:firing`). | **It also becomes a mention on its record.** `about` gets `thread:firing` beside `observer:potter`. A spec with `"about": "thread:firing"` finds every record that referred to the thread, which the old tool counted as the thread's inbound references. | Payload only, as imported today. Finding what refers to the thread needs a payload test on the whole text of `ref`, which cannot match one reference out of several. |
| 2 | **References of a thing's earlier revisions.** | **A revision carries them forward:** its `about` holds its own references and those of the records it replaces, but not their observers. Written by hand that way today, `loops of task:kiln` shows `about decision:glaze observer:clerk thread:firing`, as the old tool's `refs` did, and both records count as mentioning the thread. It is what `loops emit` does for a revision today: run without `--about`, it copied the head's `thread:firing`. | Each record mentions only its own references. The thing shows the latest fact's references; all of them are a read over its records. |
| 3 | **A file path or an old fact id in `ref`** (`docs/kiln.md`). | **It stays in the payload only.** A mention must be `prefix:value`, and the old stores never gave paths a prefix. | Paths become mentions under a new prefix such as `path:docs/kiln.md`, and old ids become the new record's id through the import's journal. Every reader would then have to know that prefix. |
| 4 | **The `ref` field itself.** | **It stays in the payload**, untouched, as AD2 ruled. The mentions are added beside it. | Take it out of the payload once its references are mentions, as the old tool did. The payload is then no longer the old payload. |

**What it costs to change later.** Rows 1, 2 and 4 change the bytes and id of every record they touch: 894 facts and the revisions after them. The six shadows written today follow the other column. Importing them again into fresh shadows takes about 3 seconds; nothing reads them yet.

**Not asked.** The comparison tool learns to compare the old tool's `refs` with a thing's mentions once this is ruled. The facts without their key field (37 here) are roots under AD2 (c); the old tool does not show them at all, and the comparison counts them apart.

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

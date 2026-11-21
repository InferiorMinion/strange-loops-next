# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 (PV1 to PV7, RC1, W7, E1, I10) have left it.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.

## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [W1](#w1-an-adoption-step-before-standing-and-sharing) | An adoption step before standing and sharing | Yes, after ticks, with its CLI pieces built beside the tick work | The order of work |
| [W2](#w2-remember-which-segments-were-checked) | Remember which segments were checked | Yes, in the adoption step | Read speed on big stores |
| [W3](#w3-stop-extending-the-javascript-implementation) | Stop extending the JavaScript implementation | Yes, as a trial for step 4 | How step 4 is built |
| [W4](#w4-the-refusal-for-input-that-is-wrong-twice) | The refusal for input that is wrong twice | One answer in the repository; the protocol text pins single faults only | How step 4 is built |
| [W5](#w5-the-mutant-check-on-every-merge-request) | The mutant check on every merge request, and new mutants for step 4 | Keep (measured: under five minutes on the largest model change); add step 4's mutants after its exit | Nothing |
| [W6](#w6-generated-pages-in-the-repository) | Generated pages in the repository | Keep | Nothing |

None of these blocks the build that is under way (payload values, and the result hash's prefix). W3, W4 and W5 are wanted before the tick record is built.
Each entry now ends with Neon's comments on it (2026-11-21) and what the coordinator makes of them.

After these: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## Order of work, and how we work

### W1: an adoption step before standing and sharing

**Question.** Should a step that makes the new `loops` your daily tool come before standing and sharing?

**Example: a Monday morning.** With the old tool you typed `sl read project` and saw each item's name and status. With the new `loops` today:

- A read needs a JSON spec written by hand for each question (`.loops/vertices/project.json`).
- There is no `ls`. Nothing lists the things or kinds in a store without a spec.
- Nothing finds things by the start of a name. "Every thing that starts with `task:`" cannot be asked without a spec.
- Every answer is JSON.
- The shell finds the old Python `loops` first. rex-tools reads the text of `loops --help` to learn which one it got.
- Your 11,037 facts in six stores are not in it.

So the only daily users of the new tool are agents.

**What the step would build.**
- An import of the old stores as a shadow: the old tool stays in charge, and the copy is read beside it to compare.
- `ls`, a scan by name prefix, relative times, and a plain-text view.
- Reads across several stores. A spec names a store in `from` today, and the name is ignored: a spec naming `homelab-audit` and `relay` was answered from whatever store was in the directory. In the old setup homelab-audit, homelab-coord and relay are three stores, and `generate_changelog.py` went around the CLI to combine them. This is the one piece Neon would not give up; it needs no change to the protocol.
- The cache of [W2](#w2-remember-which-segments-were-checked).

**Options.**

| | Order | When you would be using it daily |
| --- | --- | --- |
| A | Payload values, the tick record and a first tick, adoption, then standing with sharing | After ticks |
| **A, in parallel (recommended)** | As A, and the adoption pieces that touch only the CLI (`ls`, the prefix scan, the plain view, reads across stores, the shadow import) are built beside the tick work | Piece by piece, from now |
| B | Your order of 2026-11-10 as it stands: ticks, then standing with sharing. The adoption pieces stay "later work". | After the two largest steps are built, with no daily use to test them |
| C | Adoption first, then ticks | Soonest. Ticks, which Recall and handoffs between agents need, wait. |

I lean to A because the tick work is in motion: the laws are in the model and the codec is built. C is a fair choice too; the import needs nothing from ticks.
Neon and margins-claude both support an adoption step before standing.

**Neon's comments, and what I make of them.**
- *A or C, never B. If A, keep step 4 tight to its exit and let the CLI-only adoption work run in parallel.* I agree, and it is now the recommended row. The CLI pieces touch nothing in the model, a Codex worker can build each, and their choices are implementer choices under your ruling of 2026-11-21. The tick proposal is mine to write and does not wait on them.
- *Do not let issue #8 gate adoption.* Issue #8 is the law for a reader that caches. You ruled on 2026-11-11 that it is step 4's first law. No caching reader is being built, since the read path is fast enough without one. I agree with Neon: the law should be stated when a caching reader is built, and should not hold up step 4's exit. That changes your ruling of 2026-11-11, so it is part of this entry: **say whether issue #8 leaves step 4.**
- *The shadow import should write a few large segments, not one per record.* Agreed. It is how the import would be built anyway (`loops import` or `emit -`).

**Done when** `loops` is first on `PATH` for daily work.

### W2: remember which segments were checked

**Question.** May a read skip the signature checks on a segment that this user's own `loops` has already checked?

**Example.** homelab-audit has 9,053 facts. Once imported, every `loops` command checks every signature before it answers: about half a second each time (measured: `loops check` over 9,000 records takes 0.53 s).
A shell prompt or a chat hook that reads on every prompt pays that every time. Writing 2,000 records one emit at a time takes 145 s, because each emit checks all that came before.

**The proposal** (Neon's design; I think it is sound):
- A file per user, outside the store (under `~/.cache/loops/`), lists the hashes of segments that were checked whole and found clean.
- Every read still hashes each segment's bytes, so a segment changed in place is caught. Only the signature checks are skipped.
- A segment with a refused line is never listed, so `store-not-clean` still fires every time.
- `loops check` ignores the file and checks everything.
- The file is named for the version of the checking rules that wrote it. A newer, stricter `loops` does not trust what an older one checked.
- The file is never inside `.loops/`. A store that came from a clone or a shared directory could otherwise bring its own "already checked" list.

**Options.**
- **Yes, in the adoption step (recommended).** That is when the first large store exists.
- Yes, now.
- No: every read checks everything, as today.

**What it buys.** Neon estimates a read of 9,000 records goes from about 0.6 s to under 0.2 s. That is an estimate, not a measurement.

**Neon's comment.** My first version of this entry left out the bullet about the version of the checking rules. Neon is right that it is required; it is added above.

### W3: stop extending the JavaScript implementation

**Question.** Each protocol piece is built three times today: in Lean (the model), in JavaScript, and in Rust, and a fuzz run compares all three. Should the pieces from ticks on be built in Lean and Rust only?

**Example: the tick record.** The last piece built the present way was the result codec: `model/Loops/Result.lean`, `interop/js/result.mjs` and `rust/core/src/result.rs`, then the fuzz generator taught about it.

| | The tick record is built in | Checked by |
| --- | --- | --- |
| A, as now | Lean, JavaScript, Rust | The three-way fuzz; margins-claude's Python from outside |
| **B, freeze (recommended as a trial)** | Lean, Rust | A two-way fuzz; margins-claude's Python from outside |
| C, retire JavaScript now | Lean, Rust | The same, and the existing layers lose their third implementation too |

Under B the JavaScript stays for everything it has today (records, folds, specs, the result codec) and is kept passing its vectors, the payload values change included. It does not grow ticks, standing or sync.

**Views.** Neon proposes B. margins-claude is against retiring it (C): JavaScript is what the in-repo fuzz compares, and its own Python lives in another repository and is not in CI. It has no objection to B as a trial for step 4, with the risk below. I recommend B for step 4 only, looked at again when ticks are done.

**What B risks.** When Lean and Rust disagree, no third implementation says which is wrong; the model wins by definition.

**Neon's comments, and what I make of them.**
- *The first version of this entry overstated the risk.* It said a misreading shared by Lean and Rust would go unseen because the same sessions write both. Neon is right: the same sessions write the JavaScript too, so it never guarded against that. The implementation written apart from us, from the prose, is margins-claude's Python, and it stays under every option. I have taken the sentence out.
- *Updating JavaScript once for payload values, then freezing at step 4, is a clean trial.* Agreed; that is what B says.
- *If B holds, the workbench page should use the Rust core compiled for the browser, not the JavaScript.* Agreed. The "workbench page" row in the roadmap changes when you rule B.

**What I do not have.** A count of faults that only the JavaScript side caught; margins-claude has none either. If you want it before ruling, I can go through the fuzz reports for it.

### W4: the refusal for input that is wrong twice

**Question.** When an input has two faults, must every implementation give the same one refusal, and who chooses which?

**Example.** This slot has two faults, an empty field name and an unknown `follow`:

```
{"sum": "", "follow": "next"}      refused: follow-invalid at fold.total.follow
{"sum": ""}                        refused: fold-argument at fold.total.sum
```

The protocol pins that the first answer is `follow-invalid` and never `fold-argument`. All three implementations give exactly that. Neon counts about 70 lines of protocol text that pin such orders, and several of the orders came to you for a ruling.
The same will come up for ticks: a tick that names a record nobody holds, and whose hash also does not match.

**Options.**
- **A.** As now: one answer, and the order is brought to you.
- **B (Neon).** For new steps, pin the refusal for a single fault only. Input that is wrong twice is "refused with one of the codes that apply".
- **C (recommended, with Neon's guardrail below).** One answer for every input, in the order the model checks. The implementer picks that order and notes it in the changelog. It does not come to you.

**Why not B.** The fuzz compares answers for equality today. Under B it would need to know every code that applies to an input, which is more machinery than following the model's order. Or it accepts "both refused" and no longer sees an implementation that refuses for the wrong reason.

**Neon's comment, and what I make of it.** Neon accepts C and adds a guardrail: keep the exact order as a rule inside this repository, between Lean and Rust, and do not write tables of tie-breaks into `protocol/*.md` that bind a reader written from the prose. I agree, and would rule it this way for new steps:
- The protocol text names each refusal and the one fault that causes it.
- The vectors for a new step hold inputs with one fault each.
- For an input with several faults, the protocol says only that it is refused with one of the codes that apply. Inside the repository the model's order decides which, and the fuzz checks Lean and Rust for the same answer.

The cost: two readers written from the prose may name different codes for an input that is wrong twice. Both refuse it. The orders already pinned for records and specs stay as they are.

### W5: the mutant check on every merge request

**What it is.** The mutation catalogue holds 314 deliberate faults, each tied to an exact line of the model. Every merge request runs a quick check that each one still finds its line, and that a sample of 33 are caught. The full run is separate and on request.

**Example.** Making `value` and `n` ordinary fields changes about 70 places in the model. Some of the 314 faults point at those lines.
- **With the check (today):** the same merge request must re-point them, or the pipeline is red.
- **Without it (Neon):** the merge request is smaller. The next full run reports those faults as not applying, and someone re-points them in a batch, later, away from the change that moved them.

**Options.** Keep it, or move it to the full run. **Recommended: keep it.**

**Measured, 2026-11-21.** The `value` and `n` change was the test: it touched 218 files and the model's record itself. 13 of the 314 faults needed work (8 re-pointed, 5 rewritten, none retired), and it took under five minutes, four of them the runner. So the check cost almost nothing on the largest model change so far, and it caught the 13 in the merge request that moved them. I no longer have low confidence: keep it.

**Neon's comment, and what I make of it.** Neon agrees, and adds: write no new mutants for step 4 until ticks have survived real use. So far every piece has added its mutants in the same merge request (nineteen for the result codec). I agree in part: while the tick record's shape is still moving, mutants tied to its lines would be re-pointed again and again. I would add step 4's mutants once its exit is met, a content tick recomputed to the same hash on two machines, and not wait longer than that. **Say whether step 4's mutants wait for its exit.**

### W6: generated pages in the repository

**Question.** Neon proposes that the generated HTML under `out/` stop being committed and be built in CI.

**Example.** A merge request that changes one law shows the Lean change and, below it, the regenerated `out/` pages. A reviewer scrolls past generated HTML.

**What stands in the way.** Your site's sync script (`projects.gruel.world/scripts/sync.sh`) refuses to copy output that is not committed: "so every hosted revision is a commit you can check out and regenerate".

**Options.** Keep committing `out/`, or change the site's sync to build it. **Recommended: keep.** The rule in the sync script is a good one and the cost is noise in a diff.

**Neon's comment.** Keep, and mark `out/` as generated in `.gitattributes` so that a merge request folds those files away. That costs nothing and suits either ruling, so it is done (2026-11-21).
The other half of Neon's point, counts kept by hand in the roadmap, is done: the roadmap no longer holds them.

## Being prepared

Nothing here needs you yet. It is listed so that you know what is coming.

**The tick record.** The cut note is ruled; what a tick record holds is not designed. The coordinator is writing the proposal, with an example for each choice. A preview of its first question, using Neon's example:

- Subagent 1 emits 40 records and closes with a tick, T1, that covers them. Subagent 2 emits 55 and closes with T2. The orchestrator writes 3 records of its own and closes with T3.
- **Agreed among the three agents:** T3's `covers` lists five ids: T1, T2 and its own three records. It does not list 98 records. T1 and T2 are folded as records, each with its own result, and never opened up. Checking T3 needs T1 and T2, not the 95 records under them.
- **What follows from that, and will be yours to rule:**
  - T3 is then not the same as a tick over all 98 records. It is a different fold with a different hash.
  - A count in T3 counts what T3 covers: five records, two of them ticks. It never reaches the 95. A total across the children exists only when T3 adds up a number that T1 and T2 each carry in their results. If T1 and T2 both covered one record, that record is in both numbers, and the total has it twice. (Neon asked for this to be said exactly; the first wording blurred counting ticks with adding up their results.)
  - A tick record carries its result, not only the result's hash. Then someone who holds T1 and not the 40 records under it can still read T1 and fold it into T3. (Neon's second point; it will be a stated choice in the proposal.)
  - A reader's tick ("this is what I had read when I decided") and a writer's base (the cut note, section 8) are one kind of record used two ways.
  - An inbox's read marker is a private local file, not a tick.

Also coming with it: the name of the window field, how `covers` is written, and the input key that says which covered records were read open.

**Numbers stored as text**, with the import ([W1](#w1-an-adoption-step-before-standing-and-sharing)).

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

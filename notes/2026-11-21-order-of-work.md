# The order of work, and how we work: W1 to W6

**Put to Kyle in `DECISIONS.md` on 2026-11-21 and ruled the same day, each for its recommendation.** The entries are kept here as they were put, with the ruling at the head of each. "Recommended" in the text is what was recommended then.
The ruling is recorded in [the changelog](../CHANGELOG.md) under 2026-11-21. W7 (which choices come to Kyle) was ruled earlier that day and is in the changelog too.

## W1: an adoption step before standing and sharing

**Ruled by Kyle, 2026-11-21:** option A, in parallel. The adoption step comes after ticks and before standing, and its pieces that touch only the CLI are built beside the tick work. The caching reader's law (issue #8) leaves step 4: it is stated when a caching reader is built.

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
- *Reads across stores must take the union of the named stores first, then run each source over it* (from its review of the tick record). Agreed, and it is what `protocol/spec.md` says now. A tick's check runs every source over `covers` and ignores the store names ([TK3](2026-11-21-tick-record.md#tk3-how-a-tick-names-its-spec)). If a fold routed each source to its own store, `loops fold` and the check would disagree whenever a record in one store fits another source.

**Done when** `loops` is first on `PATH` for daily work.

## W2: remember which segments were checked

**Ruled by Kyle, 2026-11-21:** yes, in the adoption step.

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

## W3: stop extending the JavaScript implementation

**Ruled by Kyle, 2026-11-21:** option B, as a trial for step 4. The JavaScript keeps what it has and passes its vectors; it does not grow ticks. It is looked at again when ticks are done.

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

## W4: the refusal for input that is wrong twice

**Ruled by Kyle, 2026-11-21:** option C with Neon's guardrail. The protocol text and the vectors of a new step pin one fault each; for an input with several, the model's order decides inside this repository and is noted in the changelog.

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

## W5: the mutant check on every merge request

**Ruled by Kyle, 2026-11-21:** keep the check on every merge request. Step 4's mutants are added after its exit.

**What it is.** The mutation catalogue holds 314 deliberate faults, each tied to an exact line of the model. Every merge request runs a quick check that each one still finds its line, and that a sample of 33 are caught. The full run is separate and on request.

**Example.** Making `value` and `n` ordinary fields changes about 70 places in the model. Some of the 314 faults point at those lines.
- **With the check (today):** the same merge request must re-point them, or the pipeline is red.
- **Without it (Neon):** the merge request is smaller. The next full run reports those faults as not applying, and someone re-points them in a batch, later, away from the change that moved them.

**Options.** Keep it, or move it to the full run. **Recommended: keep it.**

**Measured, 2026-11-21.** The `value` and `n` change was the test: it touched 218 files and the model's record itself. 13 of the 314 faults needed work (8 re-pointed, 5 rewritten, none retired), and it took under five minutes, four of them the runner. So the check cost almost nothing on the largest model change so far, and it caught the 13 in the merge request that moved them. I no longer have low confidence: keep it.

**Neon's comment, and what I make of it.** Neon agrees, and adds: write no new mutants for step 4 until ticks have survived real use. So far every piece has added its mutants in the same merge request (nineteen for the result codec). I agree in part: while the tick record's shape is still moving, mutants tied to its lines would be re-pointed again and again. I would add step 4's mutants once its exit is met, a content tick recomputed to the same hash on two machines, and not wait longer than that. **Say whether step 4's mutants wait for its exit.**

## W6: generated pages in the repository

**Ruled by Kyle, 2026-11-21:** keep committing `out/`.

**Question.** Neon proposes that the generated HTML under `out/` stop being committed and be built in CI.

**Example.** A merge request that changes one law shows the Lean change and, below it, the regenerated `out/` pages. A reviewer scrolls past generated HTML.

**What stands in the way.** Your site's sync script (`projects.gruel.world/scripts/sync.sh`) refuses to copy output that is not committed: "so every hosted revision is a commit you can check out and regenerate".

**Options.** Keep committing `out/`, or change the site's sync to build it. **Recommended: keep.** The rule in the sync script is a good one and the cost is noise in a diff.

**Neon's comment.** Keep, and mark `out/` as generated in `.gitattributes` so that a merge request folds those files away. That costs nothing and suits either ruling, so it is done (2026-11-21).
The other half of Neon's point, counts kept by hand in the roadmap, is done: the roadmap no longer holds them.

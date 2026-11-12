# Roadmap

## State

The tree has three implementations: Lean, JavaScript, and Rust.
Step 1 applies the six settled rulings before records exist.
All three implement the settled decisions D1–D6, and D8a and D8b.
All three refuse every envelope shape by name (issue #1) and let a label stand in `under`.
All three writers refuse an `under` target they do not hold (`under-unknown`).
All three let count, sum, min and max follow `prev` (issue #3), and group count, sum, min, max and collect with `by` (issue #2).
The law index has 92 entries, 829 theorems, and 6 boundary witnesses.
See [the law index](design/laws/laws.json) and [proof and boundary data](design/laws/).
These proofs apply only under their stated premises.

The tree has 536 JSON fixtures: 524 generated fixtures and 12 standard fixtures.
Lean replays 515 generated fixtures; 9 occurred fixtures use separate checks.
Rust checks every fixture, six JCS byte pairs, 4,000 binary64 cases, and 128 SUPERCOP signing cases with ed25519-dalek.
Rust passes 156 tests; JavaScript passes 6,438 vector checks and 36 regression and law tests.
Both also check that `spec/run` answers write their members, group and thing keys included, in the model's order; JavaScript checks `fold/latest-by` answers' thing keys too.
JavaScript refuses a held record's invalid `emitted` or `occurred` before it answers, as the Lean and Rust readers do.
The fuzz suite passes 56 tests.
See [vectors](vectors/README.md), [Rust boundaries](rust/GAPS.md), and [the mutation report](model/mutate/REPORT.md).
These checks provide bounded evidence, not complete JCS or cryptographic correctness.
They do not certify browser accessibility, external links, or official ASD dictionary compliance.

Input numbers round once to binary64.
A number past binary64's range is refused at the writer's number check.
The writer accepts values with magnitude at most 2^53 − 1, including fractions.
Numeric folds read safe whole values only.
Raw constructors and theorem premises retain separate domains.

The gates pass locally on Node 25.9, and CI runs every gate on Node 22.
The `under-1` campaign, over the generator after `under-unknown`, checks 5,600 cases, 800 per family, in 90.70 seconds.
It reports 3,776 matches, 1,477 protocol refusals, 347 shared decoder refusals, and no mismatch.
Four wrong ports of the check in the JavaScript witness each give mismatches on the same seed: 60, 24, 24 and 25 in 800 batch cases.
The `issue-7-1`, `by-collect-1`, `by-shape-1`, `by-1`, `shape-follow-1`, `shape-under-2` and `follow-2` campaigns, each over an earlier generator, found no mismatch either.
Decoder refusals are not agreement; for record and batch inputs JavaScript must also report them as malformed.
The generator carries issue #1's shapes and labels in `under`, followed count, sum, min and max slots, grouped slots, keys written as number text (issue #7), an unknown key beside an envelope that is not an object, and `under` targets held or not, so the same seeds now generate different inputs; earlier campaigns' counts are historical.
The `d8ab-1` and `d1-d6-1` campaigns found no mismatch.
The 42 duplicate-base lens mismatches of the `step1-3` campaign are the class D1 settles.
`interop/fuzz/summarize.py` derives these counts from tracked summaries.
See [the campaign report](interop/fuzz/REPORT.md) for the scripts and current and historical results.
The CI seed `ci-1` checks 28 three-way cases without mismatches.

The mutation catalogue has 240 entries, including six for the D1 and D2 definitions, nine for D8's `of`, nine for `follow` on count, sum, min and max, ten for issue #1's shapes and labels in `under`, seven for `by`, and four for `under-unknown`.
Ten earlier entries are retargeted at `of` and the one thing it names, two at the `follow` grammar, and four at the shape stage and labels in `under`.
It retires the redundant positive-offset guard under R4 and the entry for the deleted `stripZeros` family.
The last full run covers the 210 entries before issue #1 and `follow`. After the merge, the 25 entries the two changes add or retarget, run alone with `--only`, give 17 build rejections, three vector differences and five generator assertions, and no survivor.
The seven `by` entries, rerun alone on the tree merged with issue #1, give six build rejections and one vector difference, and no survivor.
The four `under-unknown` entries, run alone, give three vector differences and one generator assertion, and no survivor.
The full run reports 165 build rejections, 28 vector differences, 15 generator assertion exceptions, and 2 survivors.
The runner exits 1 for assertion exceptions; no timeout or infrastructure failure occurs.
Of the nine D8 entries, six fail the build, two change batch vectors, and one stops at a generator assertion.
The four D6 helper survivors fail the build at their lemmas; the two scoped-equivalence survivors remain.
All-envelope shape checks now belong to each library.
Six deliberate per-envelope faults failed the respective vector layers in Step 1; that probe is not repeated here.
The cheap runner kills 33 of 33 mutants: each compiles and then fails runtime checks.
Build rejections are not semantic proof kills.
Survival does not prove equivalence.
See [the mutation report](model/mutate/REPORT.md) for outcomes, restoration checks, and limits.

## Step 2: the first product slice

`loops-place` and the `loops` CLI emit, read and fold for one person, signed, over `.loops/`.
Stored lines match the model's record vectors byte for byte; 14 tests drive the place and the binary.
The layout, the CLI and their refusal codes are a working proposal, not protocol.
[`rust/PLACE.md`](rust/PLACE.md) lists what is implemented and the nine choices made on the way.
Two of them need the model: what a place is has no law, and a payload reaches a fold only as `value`, `n` and top-level fields.

**Built, 2026-11-11: `"follow": "prev"` on count, sum, min and max (issue #3).**
Such a slot folds the whole-reader heads, each thing's heads with each id once, which are the heads latest-by and a followed collect read.
It selects as latest-by does, and its result keeps the shape of the same slot without `follow`.
Every head of a forked thing counts; count counts an erased or withheld head by its envelope, and sum, min and max count it unread.
Any other `follow` value is refused with `follow-invalid`, at the check that already refused it on collect; that check now accepts `"prev"` on every fold type.
The law is `spec.follow`: the answer depends only on the set, through each thing's heads (`heads.set`), not `acc.set`. `heads.why` still holds.
A caching reader for these slots is not built. A record with an unknown `prev` can supersede a head and later be refused when its target arrives, so a reader that caches must keep what such a record supersedes. Holding such records back instead would refuse the slices the anchor rule allows, whose live setters travel without their ancestors. Kyle, 2026-11-11: this is step 4's first law.
**Implementer choices, confirmed by Kyle 2026-11-11:**
- The law's name, `spec.follow`, and its place in the spec table.
- `follow` is a flag on each accumulating fold type, as on collect, not a slot-level field.
- `"follow": "prev"` with `limit` on count is refused with `limit-invalid`, as before: `limit` stays collect's.

**Built, 2026-11-11: `by` on count, sum, min, max and collect (issue #2).**
`by` groups a slot by `env.kind`, `env.observer`, the members of `about` under a typed-reference prefix (`env.about:<prefix>`), or a payload field.
It is allowed on count, sum, min, max and collect, following `prev` or not; on latest-by, or naming nothing to group by, it is `by-invalid` at `fold.<name>.by`, checked after `limit` and before the fold's argument.
The result is `{"groups": {<key>: <result>}, "unread": n, "ungrouped": m}`, each `<result>` the slot's own shape.
Settling, admission and the whole-reader heads come first, once, over everything held; each group is then the slot without `by` or `where` over the records selected into it.
The laws are `by.set`, `by.after-admission`, `by.accounting` and `by.partition`; `spec.slice` covers grouped slots without a new premise.
Ordering or top-N across groups, buckets and windows, `by` on latest-by, and keyed things (D8c) are not built.
**Kyle's choices, confirmed 2026-11-11:**
- The spelling `env.about:<prefix>`: it reuses `where`'s existing rule that `env.` names the envelope, so a bare name such as `about:project` is a payload field, and no second name is reserved.
- Keys are text, so a text value and a number or boolean value that render alike share a group: the text `"1"` and the number 1, the text `"true"` and `true`.
- A selected record with no key is counted in `ungrouped`: a missing field, no mention under the prefix, or a value that cannot be a key (a fraction, null, an array or an object). It is never dropped and never a refusal; only a payload the reader does not hold makes a key unread.
- A record mentioning several things under the prefix counts in each.
- `limit` applies per group; a top-N across groups is a later discussion (the earlier strange-loops had one, in a weather vertex example).
- `by` is allowed on collect that follows `prev`, and refused only on latest-by.
**Implementer choices, confirmed by Kyle 2026-11-11:**
- Groups are written in UTF-16 order of their keys, the order canonical JSON and `thingsOf` use.
- The four law names, and `by` checked before the fold's argument as the issue proposed.

## Decisions for the human

### Settled: Step-1 rulings R1–R6

- **R1:** Every byte string uses lowercase, unpadded base32hex.
  Readers require canonical lengths and zero pad bits.
  Proved id text order equals digest order.
- **R2:** The id preimage remains the JCS object containing `commit` and `env`.
- **R3:** Signed bytes remain `loops/record/v1:` followed by the id text, encoded as UTF-8.
- **R4:** Instants use UTC `Z`, seconds 00 through 59, and optional fractions of one through nine digits.
  A fraction ends in a nonzero digit.
  The actual parser has proved spelling uniqueness and parse-render recovery.
- **R5:** Typed references retain lowercase prefixes; sets retain UTF-16 code-unit sorting.
- **R6:** Raw envelope readers return `envelope-field-unknown` before checking values, including `kind-empty`.
  The payload remains the extension point.

See [record rules](protocol/record.md) and [the settled ledger](design/rethink/ledger.json).

### Settled: D1–D6

The human settles each decision as its written recommendation.

- **D1:** A public lens reader refuses a base that repeats a section id, with `base-id-duplicate`.
  It checks the base before the text, so this refusal precedes every text refusal.
  The raw core keeps first-lookup behavior.
- **D2:** Text is Unicode scalar values.
  A lone surrogate is refused with `string-not-unicode` before hashing, and nothing is substituted.
  The rule covers values, member names, envelopes, and batches.
  The refusal follows `envelope-field-unknown` and precedes `kind-empty`.
  A payload text receives it only when the text is JSON by syntax.
  A fixture that holds a lone surrogate anywhere else, ignored metadata included, is malformed input in all three.
- **D3:** A number past binary64's range is refused by the writer's number check, with `number-out-of-range`.
  That check stays last in the writer's order.
  The Rust core stays finite; its input adapter carries the overflow to that check and to the spec parser.
  Reader records still require finite numbers.
- **D4:** Signing fixtures require a canonical seed of exactly 32 bytes, read only after the record is written.
  No adapter pads or truncates. This is a fixture rule, not a record rule.
- **D5:** A record input's first envelope and first payload decide its refusal, as in Lean.
  Once they are written, a later variant that names another record is malformed input.
  So is a later payload variant that is refused.
- **D6:** Four helper lemmas are proved and pinned.
  They are `Sum.on_step_missing`, `Max.acc_readable`, `Min.acc_readable`, and `Spec.mem_select`.

D2 and D3 left the code name, stage, and precedence to the human.
Kyle confirmed the landed choices on 2026-11-10:
the name `string-not-unicode` and its stage after raw shape, the reuse of `number-out-of-range`, the same overflow handling for spec arguments, and D1 refusing duplicates only.
D1's name `base-id-duplicate`, D5's refused-payload fault, and the Rust stand-in are implementer choices.
See [record rules](protocol/record.md), [the lens contract](protocol/lens.md), and [Rust boundaries](rust/GAPS.md).
D7 below remains open and unchanged.

### Settled: binary64 standards and writer range

The human closes the standards choice with Option A.
Rust uses native shortest binary64 digits, ECMAScript notation, and exact midpoint tie correction.
The writer refuses every magnitude above 2^53 − 1, including fractional values.
Canonical formatting covers finite binary64 independently of writer acceptance.
A Lean roundtrip proof and a shortest proof do not yet exist; neither blocks this choice.
See [Rust numeric boundaries](rust/GAPS.md) and [record rules](protocol/record.md).

### D7: Canonical result encoding

Kyle chose the recommendation below on 2026-11-11. It is not built, and changes no current behavior until it is; step 4's ticks need it.
The [open design](design/rethink/sections/16-open.html) prefers JCS for results.
Exact, unbounded `Sum` outputs include `9007199254740993` in `phase2-exact-output`.
Neither current tests nor the model define a canonical result hash codec.
**Readings:** Define a separate exact-integer canonical JSON extension, or use typed decimal integer strings inside JCS.
Raw JCS numeric rounding cannot preserve every `Int`.
**Effects:** These choices produce different bytes, hashes, and schemas.
**Recommendation:** Define a separate versioned result codec using typed decimal integer strings under JCS.
Never round or cap current model outputs.

### D8: Items, mentions and declared keys

Kyle ruled this on 2026-11-10.
D8a and D8b are built across the model, vectors, JavaScript and Rust; D8c is not built.
It changes the settled envelope, so it lands as R1–R6 and D1–D6 did: model, vectors, JavaScript, Rust, then protocol and pages.
**Finding:** Two apps built on the product slice hit one wall on 2026-11-10.
The orchestration dry run is in [Scenarios](design/rethink/sections/13-scenarios.html), step 8; margins-claude's `work` app is described in its notes on merge request !12.
`about` does two jobs: "this record is an emit of this thing" and "this record mentions that thing".
A task about its plan became a rival head of the plan.
A record about something else was not a thing under its own id.
A closing record about the 23 ids it covered became the winner of each.
Restating a whole payload on every change left a writer no basis to merge a fork field by field; the fork showed as two heads, and one writer's merge dropped a change.
The earlier systems (`tasked`, `ticked`) keyed a fold by a field and re-emitted only the changed key.

The choice has three parts. D8a and D8b go together; D8c can follow.

- **D8a, an item field.** One single-valued envelope field, `of`, says which thing a record is an emit of.
  Only a root, a record with no `of` and no `prev`, leaves it out; it is an emit of the thing its own id names.
  Every record with `prev` states `of`. A tool may fill it in; the protocol never infers it.
  `of` is read literally: a reader never resolves it through another record, so a record's thing is a fact of its own envelope.
  An id in `of` names a root; a writer holding a record that is not a root refuses to name it.
  `latest-by` runs over this field; a thing's heads are records of it.
  Every `prev` target must be an emit of the same thing, so `prev` never crosses things.
  One thing replacing another is a new thing that mentions the old one, plus an emit on the old one that closes it: two records, not atomic.
- **D8b, `about` is mention only.** `about` never makes a head.
  `where` and `collect` still read it, so "everything said about X" stays a fold.
  A link needs no new primitive: it is a thing whose `about` names both ends.
- **D8c, declared keys.** An envelope field, `sets`, lists the payload keys an emit sets.
  A keyed fold holds, for each key, the setters no descendant setter of that key replaces, over `prev`.
  Different keys from one head merge; one key set twice from one head leaves two live setters, and the fold reports both.
  A key is unknown exactly when a live setter's payload is not held, as `where` answers.
  Withheld and erased are one state to the fold: an envelope without its payload.
  They differ only in what comes next, which the arrival laws carry: a withheld payload may arrive, an erased one must be dropped.
  Either way a later emit that sets the key makes it known again.
  A declared key absent from the payload is unset, and a payload key that is not declared is not read.
  An absent declaration and an empty one differ: absent means the record is read whole, as today; empty means it sets nothing, which is a pure join of two heads.
  A whole-read emit sets every key, named or not, and stays needed until a later whole-read emit replaces it.

**Readings:** Put identity and keys in the envelope, or in the payload.
In the payload, an erased or withheld emit can be placed only by `prev`, and a slice or a sync cannot be computed from envelopes.
For declared keys, margins-claude's experiment (`sl-py/keyfold.py`) found no wrong answer in 240,000 key answers, and none in 320,000 more with whole-read emits mixed into the chains.
Within one thing, its envelopes plus its live setters' payloads sufficed, and none could be dropped.
The experiment takes the item as given, one thing at a time, with no erasing records and no admission.
That identity belongs in the envelope (D8a) is argued, not measured, and the composition with `thing.slice` is untested.
Without declared keys, a reader needs every payload in the chain, or answers wrongly when an unheld emit is taken to set nothing.
**Effects:** Two new envelope fields enter the id; a record without them keeps its bytes and id.
Records already about a thing stop making heads of it, so the `latest-by` vectors and the heads and slice laws are restated.
`prev-unrelated` is restated over the item field.
A record is an emit of one thing only; today one record can head several.
An erased emit still shows which keys it changed, though not to what.
Kyle accepted this on 2026-11-10: erasure is the emergency stop for something that should not have been emitted, and its job is that the content travels no further.
`kind`, `about` and the times already survive erasure; secrets do not go in envelopes, key names included.
The core starts to read a payload's top-level keys by name.
`thing.slice` as proved covers whole-record folds only; the keyed fold needs its own slice law, argued and not yet stated in Lean.
**Order:** Take D8a and D8b now, in the envelope, before any store is kept.
Take D8c as a second fold family beside whole-record `latest-by`, once D8a's shape is settled and its slice law is stated.
Kyle, 2026-11-11: D8c starts with a design note, before any code. Once an emit can set only some keys, a thing's current state is the merge of its live setters, not one head's payload, and `where`, `follow` and `by` read a head's payload today. The note says how they read a keyed thing, the spec grammar and result of the keyed fold, the writer's anchor rule and the witnessed slice law. margins-claude drafts it.
**Amended by Kyle, 2026-11-10,** on margins-claude's finding in issue #4 (`sl-py/keyslice.py`): D8a first let a record with `prev` leave `of` out and take its thing from what it replaces.
That made a record's thing depend on another record being held, and a partial copy then answered wrongly: 10,276 of 33,634 cases under one slice rule and 412 under the other, against none in 35,200 with `of` stated.
The CLI's "a revision without `--about` is about what it replaces" stays as a writer's convenience only.
The same experiment found that a replaced member sent without the path to what replaced it reads as live, and proposed that what is sent of a thing be convex over `prev`.
Issue #5 corrects the proposal: convexity is sufficient but not exact, and on a steady history the convex part is nearly the whole history.
The exact law is *witnessed*: every held setter of a key that is dead for that key has a setter of that key above it through held records (`witnessed_iff`, in margins-claude's `lean/Convex.lean`).
Either law is about a slice read alone; a slice merged into an older holding is step 5's question.
margins-claude will state the law over the model once D8a is modelled.
Kyle agreed on 2026-11-11 (issue #5): the model states *witnessed*, with convex as a corollary, and a writer's tool fills in the anchor, as it fills in `of`: an emit that sets a key names in `prev` every record its writer holds that sets that key and is still a live setter of some key. Then the live setters alone are a slice. A slice law is about a slice read alone; merging slices is step 5's question.
**Kyle's choices, 2026-11-10,** following margins-claude's reasoning on merge request !13:
- **Names:** the item field is `of` and the declaration is `sets`. A record *of* the task, against a record *about* it; `sets`, like `erases`, is a verb for what the record does.
- **Natural keys:** `of` takes a record id or a natural key, as `about` does.
  A writer can name a thing before holding any record of it, and two writers who start the same named thing join.
  An unnamed thing is rooted at its first record, and a second branch without `prev` joins it by that id.
  In an open store anyone can emit of a name and become a head; that is left to standing, where grants per thing belong.
- **Same value from two setters:** the core declares no conflict.
  The keyed fold reports each key's live setters, newest first, as whole-record `latest-by` gives a winner and the others.
  Whether two of them conflict is a reading made above the core, so the core never defines equality on payload values.
  A reading should not treat agreement as a conflict.
**Kyle, 2026-11-10, on undeclared payload keys:** refusal is progressive.
A writer refuses a payload key it did not declare only in a store declared strict.
Elsewhere the key is written, the keyed fold does not read it, and a whole-record fold, a `where` test or a lens still can.
Making it count takes a new emit that declares it, since the declaration is in the id.

**Built, 2026-11-10: D8a and D8b.**
The model gives each record one thing, `of` or its own id, and makes `about` a mention only.
Its laws are restated over the thing, and `thing.one`, `thing.mentions` and `record.of` join the index.
The vectors are regenerated; JavaScript and Rust agree on every one, and the `d8ab-1` campaign finds no mismatch.
D8c, declared keys, is not built, and the keyed fold's slice law is not stated.
**Kyle's choices, confirmed 2026-11-10:**
- **Spec:** the spec says `"latest-by": "of"`. `from.of` selects a thing's records; `from.about` is a mention test.
- **Filling in `of`:** only the single-record `emit --prev` path fills `of` in, from the record it replaces.
  `emit -` fills nothing in: a changeset states `of` itself.
- **Unheld target:** the CLI refuses `prev-unknown` itself when a revision's target is not held, since it has nothing to fill `of` in from.
  On that path only, this precedes the writer's other checks.
- **Export:** `export --about` is a mention filter and `export --of` filters by thing.
- **Verb:** the verb that shows a thing is `loops of <thing>`, renamed from `loops about <thing>`.
**Implementer choices, confirmed by Kyle 2026-11-11** (there are no stores yet beyond iteration tests, so the missing compatibility path costs nothing):
- The codes `of-not-typed`, `of-missing` and `of-not-root`, and their places: `of-not-typed` right after `about-not-typed`, `of-missing` last of the envelope checks, and `of-not-root` between `prev-unknown` and `prev-unrelated`, all three across the whole batch.
- `"latest-by": "about"` is refused with `fold-argument`; there is no compatibility path.
- A reader polices neither `of-missing` nor roots. A held record with `prev` and no `of` is of its own id, so its `prev` crosses things and admission refuses it.
- `from.about` and `from.of` in one source mean both.
- A held entry in a batch vector is `{"id", "of"?, "prev"}`.

### D8c: declared keys, design note

**Draft by margins-claude, 2026-11-11, for Kyle's rulings before any code.** Nothing here is built.
Every choice that is Kyle's is marked **Open**; each gives a recommendation and the alternatives.
The experiments and proofs are in the margins repo: `sl-py/keyfold.py`, `keyslice.py`, `keygap.py`, `keyanchor.py`, `keyslice_anchored.py` and `lean/Convex.lean`.

**The problem.** Once an emit sets only some keys, a thing's state is the merge of its live setters, not one head's payload.
`where`, `by`, the numeric folds and the slots that follow `prev` all read one record's payload.
With anchored writers the head of a task is often the emit that last changed its title, so "count the open tasks" over heads is wrong.

**Terms.** A record *sets* key `k` when `k` is in its `sets`, or when it has no `sets` (a whole-read emit, which sets every key).
A setter of `k` is *live for `k`* when no admitted descendant of it over `prev` sets `k`.
A record is *live* when it is live for some key.
For a thing with only whole-read emits, the live records are its heads.

#### 1. `sets` in the envelope

- **Holds:** a set of top-level payload key names, canonical as `about` is (UTF-16 order, each once).
- **Absent and empty** differ, as ruled: absent is read whole; empty sets nothing, a pure join.
  **Open:** the canonical envelope writes `"sets": []`. Every other set field leaves an empty set out, so this is the one exception; the alternative is another way to write a join.
- **Refusal:** one code, `sets-not-keys`, for a value that is not an array, a member that is not text, or an empty name (no spec could name it). It comes after `erases-not-record-id` and before `of-missing`. **Open:** the name and the place.
- **No check against the payload.** A declared key the payload lacks is unset, as ruled; a payload that is not an object unsets every declared key.
  The envelope stays checkable without its payload, which an erased record needs.
  **Open:** the alternative refuses a non-object payload beside a non-empty `sets`.
- **A root or a revision may declare anything.** A root with `sets` declares its keys like any other emit; no check compares one record's `sets` with another's.
- Refusing an undeclared payload key is a strict store's rule (step 3).

#### 2. The keyed fold

**Recommendation:** a second value of `follow`, not a new fold type: `{"latest-by": "of", "follow": "keys"}`.
`follow` already names how supersession is read. `"prev"` reads a thing as its heads; `"keys"` reads it as its live setters.
Nothing built changes meaning, and `follow-invalid` already refuses any other value.
**Open:** the alternative is a new fold type beside `latest-by`; the word `keys` is also open.

The result, per thing, gives each declared key's live setters and the live whole-read emits:

```json
{"things": {"task:1": {"keys": {"status": [{"id": "r:B", "shown": "here"}],
                                "title":  [{"id": "r:A", "shown": "withheld"}]},
                       "whole": []}},
 "unread": 0}
```

- Setters are newest first, as `latest-by` lists a fork's heads. More than one is two live setters of that key; the core declares no conflict.
- A key is unknown exactly when a live setter's `shown` is not `here`. The fold returns ids, not values, as `latest-by` does.
- `whole` lists the live whole-read emits. A key no listed setter declares is read from them.
- **Open:** this shape.

#### 3. How the other slots read a keyed thing

**Recommendation:** `"follow": "keys"` is allowed on count, sum, min, max and collect, with or without `by`, and there the unit is the thing.
Slots without `follow`, and slots with `"follow": "prev"`, keep reading records as written, each from its own payload.
So both readings exist, a spec says which it wants, and an undeclared payload key stays readable by the record readings, as ruled.

For a slot that follows `keys`:

- **A payload field** is read from the field's live setters: the live records that declare it, or when none does, the live whole-read emits.
- **`where`:** each test is asked of each of those setters. One answer from all of them is the answer. Different answers, or a setter whose payload is not held, is unknown, and the thing is unread.
  The core compares test answers and never payload values, so agreement is not a conflict.
- **count** counts the things selected. A thing forked across whole-read emits counts once here and twice under `"follow": "prev"`.
- **sum, min, max:** a field with one live setter is read from it. With two it is unread. **Open:** two setters holding the same number could be read; that makes the core compare values, which D8 ruled out.
- **`by` on a payload field:** the thing joins the group of each live setter's key, as a record that mentions two projects joins two groups. An unheld setter makes it unread; no setter, ungrouped.
- **collect** lists the selected things' live records, newest first. With anchored writers this is the thing's slice.
- **`from` and envelope tests:** a thing is selected when one of its live records is taken; its fields are then read from all its live setters. **Open:** the alternative selects setters one by one, and then a key whose setter is not taken reads as absent.
- **Whole-read beside keyed:** a whole-read emit sets every key and is live for each until a descendant sets it. A later whole-read emit replaces everything under it.

The alternatives to the whole recommendation: change what `"follow": "prev"` means (it changes built answers for forks), or leave the existing slots unable to read keyed things (then no spec can count open tasks once writers set keys).

#### 4. The laws

| Law | Says | State |
|---|---|---|
| *witnessed* | A part reads each key's live setters as the whole does exactly when every held setter that is dead for a key has a setter of that key above it through held records. | Proved abstractly (`witnessed_iff`). To port. |
| convex | A part that is convex over `prev` and holds the live setters is witnessed. Not the other way. | Proved (`witnessed_of_convex`, `Named`). To port as the corollary. |
| anchored | When every emit that sets `k` names every held live record that sets `k`, the live records alone are a witnessed part. | Proved (`anchored_slice`; `slice_reads` from the rule itself). To port. |
| `Closed` | The anchored law needs a sender that holds what the writers of its records had seen. | Proved as a premise; found by experiment (55 wrong in 35,200 when the sender is missing records, none for a convex part). To port as a premise with its boundary witness. |
| receiver's flag | A receiver can tell which of its own records a missing name might have replaced; the flag is sound and tight. | Proved (`flag_tight`). Step 5 uses it; not needed for D8c. |

- These are stated over an abstract `prev` and `sets`. The model needs `sets` on the record, liveness per key, and the laws restated over `realm`, so behind settling and admission.
- By experiment (`keyslice.py`, 35,200 pairs with `of` stated): the answer needs no record outside the thing; the sent members must be admitted ones; a live setter's eraser is needed for the shown state, not for the answer; a sender must not send a payload its own eraser took away.
- **`spec.slice` over keyed things is not proved.** For a slot that follows `keys`, "current" becomes "live for a key", and the successors that matter are the setters of the same key. I expect the law to be: the slice of each selected thing, plus what `SpecSlice` already asks of records read as written. That is a conjecture until it is tested and stated.
- A slice law is about a slice read alone, as ruled.

#### 5. The writer

- **The anchor rule,** as agreed on #5: an emit that sets a key names in `prev`, beside what it builds on, every record its writer holds that sets that key and is still live.
  `loops emit` fills this in on the single-record path, as it fills in `of`.
- **`emit -` fills nothing in:** a changeset states its own `prev`, as it states its own `of`. **Open:** the alternative fills anchors in for a changeset too.
- **No refusal for an unanchored emit.** It reads correctly whole; only its slice is larger. **Open:** a writer could refuse it (`prev-not-anchored`), since it holds what it would need to check.
- **A whole-read emit shares every key,** so it names every live record of the thing it holds, and is then the only live record.
- **Cost.** `prev` names per record: 1.13 plain against 3.10 anchored with three replicas; 2.4 for one writer over 1,000 records, longest 5. The bound is about one name per key of the thing.
  The names are in the id. A root that stays live for one key is named by every later setter of its other keys; restating that key lets it go.
- **What it buys.** Over 300 events a thing's slice is about 8 records anchored, 59 for the cheapest witnessed part with plain writers, and 233 for the convex part.

#### 6. Out of scope

- Strict stores, where an undeclared payload key is refused: step 3.
- Merging a slice into an older holding, and the causal context that makes it sound (`merge_reads`): step 5. It needs a per-writer sequence number or a set of seen ids, which the envelope does not have.
- The caching reader for slots over heads or live setters, and what it must keep for a record with an unknown `prev`: step 4.
- Whether two live setters conflict: a reading above the core.

**Order of work, if the recommendations stand:** `sets` in the model with its refusal; liveness per key and `latest-by` following `keys`; vectors, JavaScript, Rust; then the other slots following `keys`; then the laws ported; then the anchor in `loops emit`.

### Issue #1: envelope shape refusals, and a label in `under`

**Built, 2026-11-11.** Every envelope shape has a named refusal, as `protocol/spec.md` has for specs.
An envelope that is not an object is refused with a new code, `envelope-not-object`.
A required field left out, or any field of the wrong JSON type, is refused with that field's existing code, in that field's place.
The model reads such a field as empty text, which every field's own check refuses (`unreadable_refused`).
The same change brings the batch up to the settled ledger entry "A label may stand in `under`": a draft may name another by label in `about`, `of`, `prev` or `under`, never in `erases`, and labels in `under` take part in cycles.
**Kyle's choice, confirmed 2026-11-11:** `loops emit -` reads a changeset line's envelope fields through the core's reader, as `Read.envOf` does.
A field left out or of the wrong JSON type gets the field's own code, in the writer's order across the whole changeset, so another line's label refusal outranks it.
`draft-shape` stays for a line's own shape only: a line that is not a JSON object, or a label that is not text; an unknown member stays `draft-field-unknown` and a repeated one `member-duplicate`.
The CLI fills in `observer` and `emitted` only when a line leaves them out; one given mistyped gets its code, and `observer-unbound` now judges only a typed observer.
A line's salt is read as the core reads one, so one that is not base32hex text is `salt-not-16-bytes` in its place.
**The reviewer's direction, confirmed by Kyle 2026-11-11 when approving !21:**
- One new code, `envelope-not-object`; every other shape fault folds into an existing code, extending "A malformed value uses the field's existing refusal code".
**Implementer choices, confirmed by Kyle 2026-11-11 when approving !21** (an untyped CLI observer is `observer-not-typed`, as above):
- `envelope-not-object` is a shape check across every envelope: after `envelope-field-unknown`, before `string-not-unicode` and every value.
  So a later record variant that is not an object refuses the input, as an unknown key does, though the first envelope decides value refusals (D5).
- A batch draft object without `env` has an envelope that is not an object.
- `null` is a value of the wrong type, not a field left out: `"occurred": null` is `occurred-not-span`.
- A set that is not an array is refused with its code; so is a set with a member that is not text, and a label beside such a member is still a label, so labels are checked first.
- A salt left out, not text, or not canonical base32hex is `salt-not-16-bytes`, in that check's place.
- A batch draft that is not an object, an unknown draft member, a label that is not text, a draft without a payload, and a repeated envelope member name stay malformed input, with no code.

**Implementer choice, confirmed by Kyle 2026-11-11.**
- In JavaScript, a repeated envelope or draft member is seen only from raw text, in the fuzz adapter; the value APIs (`resolveBatch`, `writeRecord`, `seal`, `writeText`) cannot see one (`interop/js/README.md`).

### `under` targets

**Kyle, 2026-11-13:** "Let's take under targets now."
This replaces the choice deferred on 2026-11-11, that a writer checks no `under` target beyond its shape.
Kyle took margins-claude's view: a writer refuses an `under` target it does not hold, beside `prev-unknown`.
**Built, 2026-11-13.** A new writer refusal, `under-unknown`: a record's `under` must name a record the writer holds, in the batch or in the store.
It runs across the whole batch, after `prev-unknown` and before `of-not-root` and `prev-unrelated`, so no draft order changes the refusal.
A batch member counts, named by its label or by its id.
A stored record counts whatever its shown state: the writer holds an erased or withheld record's envelope.
Nothing else about the target is checked. Whether it is a grant, of what kind and by whom, is still standing's question (step 3).
Readers do not refuse on it, as they do not for `prev-unknown`.
The batch writer and the CLI's single-record path check it against the same store lookup; there the CLI's own `prev-unknown` still comes first.
`batch/67`–`75` pin it: an unheld target refused, a held root and a held revision accepted, a batch member's raw id and an erased target accepted, and the order beside `prev-unknown`, `of-not-root`, `prev-unrelated` and a writer refusal.
No existing vector changed meaning: every earlier batch vector's `under` names a batch member by label.
The CLI walk pins an erased and a withheld stored target.

### Retained boundaries

These boundaries trace existing domains and premises; they do not create new protocol choices.
See [Rust boundaries](rust/GAPS.md), [laws](protocol/laws.md), and [vectors](vectors/README.md).

- `Addressed`, `Distinct`, `Forms`, `EnvOnly`, and `Acc.comm` remain caller obligations where laws require them.
- Raw Count and Sum retain repeats; unchecked fold state and insertion retain their wider domains.
- Spec admission examines raw members before discarding accepted unused metadata; refusal precedence remains significant.
- Source routing, store retrieval, and external services are outside the pure evaluator.
- Reader IDs use bounded 32-byte digests; writer links remain text until validation.
  This boundary does not prove SHA correctness or the complete text-to-hash bridge.
- Raw integers and exact output sums remain unbounded; binary64 normalization applies to numeric input.
- Raw duplicate object names use stable UTF-16 sorting and first-equal-key rendering; validators inspect all raw members.
- Parsed time types do not expose every raw Lean moment; typed references use ASCII rules distinct from fixture digest decoding.
- Scenario checks cover supplied arrivals, not all permutations.
- Raw lens laws require their validity and success premises, including distinct base IDs and matching ordered IDs where stated.
  The public reader checks distinct base IDs; it does not check that base IDs are nonempty.
- Batch meaning uniqueness requires acyclicity and meaning premises; it does not establish resolver correctness.
- Raw extrema fuzz compares held inputs with readable safe-whole `n`.
  This declared scope is not a bug; broader inputs require a different fixture contract.

Future public adapters can validate premises or expose raw operations with caller obligations.
Validation costs explicit error design; raw access demands greater caller responsibility.
Recommend validation at future public adapters, while keeping the existing raw core and premises.
Do not implicitly normalize the core.

Review corrections narrow proof scope and mark examples as prototype or proposed.
They also correct privacy, index, hash, signature-prefix, and obsolete Rust claims.
These mechanical corrections do not add protocol choices.

## Build sequence

Steps 0 and 1 are done. Step 2's first slice is merged. Steps 2–6 are proposals.
**Order, as Kyle set it on 2026-11-10:** finish step 2 with D8, then step 4 (ticks), then step 3 (standing) together with step 5 (sharing).
Standing matters once records cross between people: sharing through git repositories across teams, and over HTTP.
Everything wanted before that runs on one machine under one trust root.
Retiring still follows ticks, as the Trust walk found.
The step numbers below are names, not the order.
Each step extends the model laws, computes protocol vectors, then implements Rust.

| Step | Builds | Done when |
| --- | --- | --- |
| 0, done | The Lean spike: Count and LatestBy, one lens combinator, 23 vectors | The first laws are proved |
| 1, done | Protocol and vectors for layer 0: canonical bytes, salt and commitment, id, signature | The model and the JS writer agree on every vector; Rust checks all fixture families and external standards |
| 2 | Emit, read, fold for one person: JSONL segment, JSON spec, the two fold families, `"follow": "prev"`, `by`, a signing key, signing by default. `of` names a record's one thing, an id or a natural key; `about` only mentions. D8a and D8b are built; D8c is not. | It does the old CLI's workable job, signed |
| 3 | Standing: declarations, admission, stances, grants, presence key, retire through; then name tables and `resolve-by` | A strict store rejects visibly; a steward's changeset takes effect |
| 4 | Ticks: content and witness, boundaries, markers | A content tick recomputes to the same hash on two machines |
| 5 | Sharing: the git layout, then segment-list sync | Two clones converge |
| 6 | Index and search, SQLite segments, collectors, lenses, `serve` | Something is slow, or a surface needs it |

**Why Rust.** Small writers can use JSON in any language.
Phones and browser extensions need embedded readers, and Rust runs there without a server.
Exhaustive enums expose missing cases, and Lean's closed types map directly to enums.
This is design rationale; no bindings exist.

A reader reaches the core in three proposed ways. All three speak the same JSON shapes.

- **Local server:** a browser, LSP or MCP client speaks JSON-RPC to `loops serve`, which reads `.loops/` on disk.
  This suits a laptop and keeps keys out of the browser.
- **WebAssembly:** `loops-wasm` runs inside the page or extension, over browser storage or files the reader picks.
  This suits an annotation extension, static hosting and an offline web app.
- **Native bindings:** `loops-ffi`, through UniFFI, runs inside a Swift or Kotlin app, over the app's own storage.
  This suits a phone.

### Standards to stand on

The record rows are settled; see R1–R6 and [record rules](protocol/record.md).
Index, keys and provenance export are proposals.

| Concern | Choice |
| --- | --- |
| `emitted` | Canonical UTC instant, with at most nine fractional digits |
| `occurred` | A span: dates, instants and intervals from ISO 8601 and EDTF Level 1; the grammar is closed in `protocol/record.md` |
| Canonical bytes | RFC 8785 (JCS): ECMAScript-exact numbers, no NaN |
| Hash | SHA-256 |
| Id text | Typed prefix plus lowercase unpadded base32hex (`r:`, `st:`, `sl:`, …); algorithm changes go through the version label |
| Signatures | RFC 8032 (Ed25519) over `loops/record/v1:` + id |
| Specs | JSON (RFC 8259), defined by [`protocol/spec.md`](protocol/spec.md); declaration and config shapes are proposals |
| Index, search | SQLite with FTS5 through `rusqlite`, bundled; the CLI only, never the core |
| Keys | The OS keychain behind a `Signer` trait, with a file fallback |
| Provenance export | W3C PROV or PREMIS mapping, undecided |

## Next work, by value

Costs are S, M, or L for relative effort, not schedules.

| Work | Value | Cost | Dependencies and blocking choices |
| --- | --- | --- | --- |
| Integrate a workbench page | Readers build scenarios and try vector presets | M: UI, validation, browser tests | Use the JS interpreter and scenario-vector presets; version localStorage drafts; never treat them as canonical storage |
| Define result encoding and Lean encoding vectors | Reproducible result bytes and hashes without losing exact integers | L: codec, schemas, and cross-runtime vectors | Human decision D7; preserve unbounded model outputs |
| Turn horizons into Lean scenario vectors | Tests concrete compositions and exposes missing premises | L: model and scenario work | Start with minimal compositions of existing core; mark new boundary, tick, and service premises unimplemented before proof claims |
| Prove binary64 roundtrip, then shortest | Strengthens assurance beyond sampled standard cases | L: formal number reasoning | Existing canonical formatter; neither proof blocks the settled standard choice |
| Bridge source and fold records to hashes | Connects text, record bytes, IDs, and fold outputs | L: byte-level model and evidence | Defined source and writer boundaries; do not assume SHA correctness from bounded IDs |
| Model the place and the payload bridge | Gives the product slice laws: segments as a set, line to reader record | M: model and vectors | The choices in `rust/PLACE.md`; a second reader of the layout |
| Build standing, routing, key management, and sync | Supports more than one writer | L: architecture and security work | Step 3; explicit service contracts and key decisions; safe-wrapper versus raw access choice |
| Expand malformed-input fuzz and standards coverage | Finds more boundary and adapter differences | M: generators, triage, and regression pins | The settled D1–D5 boundaries; explicit contract before broadening raw extrema |
| Complete browser, accessibility, and link validation | Checks the reader experience and published references | M: browser and accessibility checks | Workbench and generated pages; validate anchors and external links separately |

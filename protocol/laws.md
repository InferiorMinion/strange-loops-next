# Laws

The tables name theorems in `model/Laws/` or mark guarantees supplied by construction.
Lean checks the listed proofs under their premises.
The model computes protocol-vector answers; external standards supply separate fixtures.
JS and Rust check all fixture families, including those standards.

## Id digests (`model/Laws/Id.lean`, `model/Laws/IdText.lean`, `model/Laws/IdSource.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `id.digest` | `idDigest_length`, `idDigest_byte`, `idDigest_roundtrip`, `idDigest_bytes_roundtrip`, `idText_injective`, `bytes_bounded`, `bytes_idDigest`, `idOf_eq_idText`, `Gen.sha256_length`, `Gen.sha256_bounded`, `Gen.sha256_idOf_eq_idText`, `Gen.Read.recordId_bounded` | An id's digest has exactly 32 bytes. Reading these bytes gives the same id. Two ids have the same text only when they are equal. Reading any 32 bytes gives an id below 2^256. Writing that id back gives the same 32 bytes. | (proved) |

## Canonical byte and instant text (`model/Laws/Base32.lean`, `model/Laws/IdText.lean`, `model/Laws/Instant.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `base32.canonical` | `base32_injective`, `unbase32_base32`, `unbase32_eq_some_iff`, `byteText_eq_true_iff` | The decoder accepts exactly the encoder image and recovers its bytes. The fixed-width reader also requires the stated byte count. No hash premise is needed. | `byte-text/*` |
| `id.text-order` | `idText_order` | Actual String lexical order on id text equals numeric order. Both inputs have type RecordId, which bounds them below 2^256. | `byte-text/id-32`, runtime id-order pins |
| `instant.unique` | `instant_moment_unique` | If the actual instant reader accepts both Strings with the same moment, those Strings are equal. | `occurred/04`, `record/r4-emitted-0`, `record/r4-emitted-2`, `record/r4-emitted-5`, `record/r4-offset-0` |
| `instant.round-trip` | `instantParts_render`, `instant_parse_render` | If the actual instant parser returns numeric fields, rendering those fields recovers the original text. | `occurred/04`, runtime parse/render pins |

The model keeps every record id and reference below 2^256.
The id reader returns only ids below 2^256.

The model's SHA-256 returns 32 bytes.
With the model's SHA-256, an id written from bytes equals the id written from the number.

## Folds (`model/Laws/Fold.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| incremental = batch | `Count.batch`, `LatestBy.batch` | folding `xs` then `ys` is folding `xs ++ ys` | every fold vector |
| `count.perm` | `Count.perm` | arrival order doesn't change a count | `fold/count/01`, `02` |
| `count.repeats` | `Count.repeats` | a repeated delivery is counted again, which is why the read path hands each id over once | `fold/count/03` |
| `latest-by.set` | `LatestBy.set_decides` | the state depends only on the set of records folded | `fold/latest-by/*` |
| `latest-by.order` | `LatestBy.winner_newer` | The winner is a head, newer than every other head. Head membership follows `prev`; content order compares span, `emitted`, then id. | `fold/latest-by/02`, `05`, `06`, `12`, `13` |
| content order | `Newer.irrefl`, `Newer.trans`, `Newer.total`, `Newer.congr`, `id_order` (`Laws/Order.lean`) | Records order by span, then `emitted`, then id digest bytes. Without `occurred`, the span is an instant at `emitted`. Distinct ids have strict total order. Time spelling does not affect this order. | `fold/latest-by/05`, `06`, `12`, `13` |
| `latest-by.repeats` | `LatestBy.repeats` | re-delivering records changes nothing | `fold/latest-by/08` |
| `latest-by.converge` | `LatestBy.converge` | two places syncing in either direction reach the same state | `fold/latest-by/09` |

| `latest.set`, `latest.repeats` | `Latest.set_decides`, `Latest.perm`, `Latest.repeats` | the input set decides Latest's whole-reader heads and winner | `fold/latest/*` |
| `heads.whole-view` | `mem_wholeHeads`, `wholeHeads_sorted`, `wholeHeads_distinct` | every head belongs to some thing; heads occur once per id in content order | `fold/heads/gap15-partial-and-anchor`, `fold/view/gap15-partial-and-anchor`, `fold/latest/gap16-newest-union-head` |
| `latest.newest` | `Latest.winner_newest`, `Latest.others_ordered` | Latest chooses the newest whole-reader head; its other heads stay in content order | `fold/latest/01-chain`, `fold/latest/gap16-newest-union-head` |
| `latest.one-thing` | `Latest.oneThing` | Latest equals global LatestBy when every record is of the same thing | `fold/latest/01-chain` |
| `numeric.safe-whole` | `Record.num_safe`, `Record.num_number`, `Sum.on_step_missing`, `Max.acc_readable`, `Min.acc_readable` | numeric slots read only safe-whole values; nonwhole values count as unread. An unreadable field leaves `Sum.on`'s sum as it was. `Max.acc` and `Min.acc` read a record exactly when its `n` is a held safe whole number. | `spec/run/gap14-field-types` |
| `sum.perm` | `Sum.perm` | arrival order doesn't change a sum | `fold/sum/01`, `02` |
| `sum.repeats` | `Sum.repeats` | a repeated delivery is added again, which is why the read path hands each id over once | `fold/sum/03` |
| `extrema.set` | `Max.set_decides`, `Min.set_decides` | Min and Max depend only on the set of records | `fold/extrema/*` |

Incremental = batch holds for every step fold by construction. The law with
teeth is `latest-by.set`: it is what lets late arrivals, re-sent segments and
two-way syncs agree. It needs content addressing (`Addressed`): within a
store, an id names one record.

Numeric slots read safe-whole binary64 values within ±(2^53 − 1).
Integral decimals and exponents, including `1.0` and `1e2`, normalize to integers.
Payloads retain nonwhole binary64 numbers, but numeric slots count them as unread.
Addition uses exact integers because binary64 addition is not associative.

The model, JS and Rust accumulate sums with unbounded integers (BigInt).
Writers accept finite binary64 values within ±(2^53 − 1), whole or not; numeric readers additionally require whole values.
A future bounded implementation must refuse overflow with `sum-overflow`; it must never wrap.

## Two families, views and honest results (`model/Laws/Families.lean`)

With distinct input ids, every fold depends only on the set of records it receives. **Accumulating** folds (Count, Sum, Min, Max) have a step that commutes.
Folds **over heads** follow `prev`, keep LatestBy's state and compute their answer from it. A new record folds onto a commuting accumulator's cached state.
Over-head proofs use cached LatestBy state, not merely a cached answer.

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `input.distinct` | `Distinct.addressed`, `distinct_dedup`, `mem_dedup` | The read path hands each id over once. `dedup` leaves each id once. Where an id names one record, it keeps the same records. | (the read path) |
| `input.union` | `dedup_union` | the union doesn't depend on which place is read first | (the read path) |
| `acc.set` | `Acc.set_decides` | any step that commutes gives the same state for the same set | every count, sum, extrema vector |
| `acc.late` | `Acc.late` | a late record folds onto the cached state | (follows from `acc.set`) |
| `heads.set` | `OverHeads.set_decides` | any answer worked out from the heads depends only on the set | `fold/heads/*` |
| `heads.late` | `OverHeads.late` | a late record folds onto the cached heads of the records folded; a reader folds admitted records, and admission can retract (`admit.late`) | (follows from `heads.set`) |
| `heads.why` | `Max.cannot_follow_prev` | Max's scalar state cannot maintain the maximum over changing heads | `fold/heads/01`, `02` |
| `view.no-resurrection` | `LatestBy.view`, `LatestBy.map_fold`, `LatestBy.view_head_ids`, `LatestBy.no_false_resurrection` | A view keeps each thing's head ids. A record superseded within a thing is never current there. Any map that keeps each record's id, `prev` and place in content order keeps the heads, each mapped. | `fold/view/*` |
| `view.refines` | `Where.eval_view` | a view only loses information: a `where` answer stays or becomes unknown | `fold/select/01` |
| `result.unread` | `LatestBy.maxHeads_view`, `Acc.select_unread`, `Count.select_unread`, `latestOf_unread`, `collectOf_unread`, `Count.followed_unread` | A fold does not guess. It counts the records whose test is unknown and the selected records it cannot read. | `fold/view/04`, `fold/select/*`, `spec/run/02`, `spec/run/03` |
| `result.full` | `Acc.select_full` | when every test answers and every record is readable to the fold, nothing is unread and the result is the plain fold | `fold/select/03` |
| `select.set` | `Acc.select_set_decides` | what is read and what is unread depend only on the set | `fold/select/*` |

Count can read every selected record.
Latest-by counts each head once when its payload tests are unknown, after the source and envelope tests select it.
Collect counts each unknown record once, before its limit.
It counts heads when it follows replacements, and all records otherwise.
Count, sum, min and max count each unknown chosen head once when they follow replacements, as latest-by does.
Max over heads counts each head whose `n` field lacks a readable safe whole value.
Numeric folds count hidden, missing, nonnumeric, nonwhole and unsafe whole values as unread.

`view.no-resurrection` holds because `prev`, ids and times are on the
envelope, which every reader holds. A `where` test on the envelope always
answers; one on a missing payload answers unknown, and the result counts it.

**Proof policy.** The model and its vectors are required at every step.
`the set decides` (in each family) and `view.no-resurrection` define what the
system means and must be proved. Other laws may be listed here as
conjectures, checked by vectors and fuzzing until proved; `lake build
--wfail` keeps `sorry` out of the build.

## Things and what a reader holds (`model/Laws/Things.lean`, `model/Laws/Slice.lean`, `model/Laws/LiveSlice.lean`)

`prev` is succession within a thing. Before evaluating a vertex, the reader settles place-held forms to one record per id.
It applies admission to `prev`, then computes per-thing states over admitted records:

* **Settle.** Settling keeps a held payload unless a held eraser names its id.
  The read-order law requires consistent forms. A record in the set whose
  `erases` names an id erases every form of it, whatever that record's kind.
  A form that arrives erased with no such record behind it counts as
  withheld.
* **Admit.** The reader refuses a record when a held `prev` target is of a different thing (`prev-unrelated`).
  It also refuses the record when the store forbids replacement of a held target (`prev-not-permitted`).
  Unknown targets do not cause refusal. The rule reads only envelopes;
  step 2 allows everything, and step 3 supplies standing.
  A reader does not check `of-missing` or roots: a held record with `prev` and no `of` is of its own id.
* **Per thing.** A record is of exactly one thing: what `of` names, read as written, or without `of` its own id.
  A reader never resolves `of` through another record.
  A root, with no `of` and no `prev`, starts the thing named by its id; unrelated roots never fork each other.
  A revision names the root's id in `of`.
  Each thing's state is LatestBy over the records of it.
  `about` only mentions things and never makes a head.

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `settle.set` | `settle_eq`, `settle_distinct` | what a reader keeps depends only on which forms its places hold, one record per id | `realm/01`–`03` |
| `settle.keeps` | `settle_keeps` | a payload any place holds is kept unless erased | `realm/01`, `02` |
| `settle.erased` | `settle_erased` | once a record naming it in `erases` is held, every form of its target is erased | `realm/03`, `09` |
| `admit.set` | `admit_same`, `refusalOf_none` | admission depends only on the set; a record is refused exactly when it isn't entitled to its `prev` | `realm/04` |
| `admit.no-cross` | `admit_no_cross`, `admit_same_thing` | Each held `prev` target is of its admitted successor's thing and permits replacement, so `prev` never crosses things. Readers admit unknown targets because they cannot judge them. | `realm/04`, `08`, `10`, `11` |
| `admit.late` | `perThing_cannot_follow_admit` | Admission isn't monotone in arrival. A record admitted while its `prev` target is unknown is refused when the target arrives of another thing, and the records it superseded are current again. No step from the admitted records' per-thing state follows that, so a caching reader keeps what such a revision supersedes. | (arrival, not a set: none) |
| `admit.grows` | `admit_append_unnamed`, `stateIn_late_unnamed` | When no held record names the arriving record in `prev`, every earlier admission stands, and each thing's state is the cached one, stepped by the arrival if it is admitted and of that thing. | (arrival, not a set: none) |
| `thing.one` | `isOf_iff`, `isOf_unique`, `stateIn_heads_thing`, `heads_one_thing` | A record is of exactly one thing, what `of` names or else its own id. A thing's heads are records of it, so a record heads at most one thing. | `realm/06`, `11`, `fold/latest-by/15` |
| `thing.local` | `stateIn_local`, `perThing_set_decides` | For a fixed admitted set, a thing's state depends only on its records. The set decides per thing. | `realm/04`–`08`, `fold/latest-by/*` |
| `thing.mentions` | `stateIn_about`, `thingsOf_about` | Changing only what records mention leaves the things and each thing's state: the same `prev` bookkeeping and the same heads, each with its new `about`. | `realm/05`, `10`, `fold/latest-by/14`, `spec/run/05`, `15` |
| `thing.slice` | `stateIn_slice`, `slice_needs_erasers`, `slice_needs_targets` | A reader holding part of the set gets a thing's state exactly when its part is a slice. A slice holds every form of the records of the thing, every record that erases one of them, and every record one of them names in `prev`. Only the envelopes of the last two are read. Neither addition can be dropped. | (a part of a set: none yet) |
| `keyed.witnessed` | `witnessed_iff`, `keyedIn_part`, `liveK_part` | A reader holding part of a thing's records reads each record it holds as live for a key, or not, as a reader of all of them does, exactly when the part is witnessed for the key: every held setter the whole reads as dead has a setter of the key above it through held records. The envelope is one of the keys. A part that is witnessed for every key and holds the thing's live records gives its whole keyed state. Holding less never makes a live record read as dead. | (a part of a set: none yet) |
| `keyed.convex` | `witnessed_of_convex`, `keyedIn_convex`, `convex_not_needed` | A part that holds every record between two of its own, and the thing's live records, is witnessed, so it gives the keyed state. It holds more than is needed: a part can be witnessed and not convex. | (a part of a set: none yet) |
| `keyed.anchored` | `anchored_slice`, `anchoredKeys_slice`, `keyedIn_live`, `anchored_of_written`, `closed_needed` | When every record names every live record under it, the live records of a thing, read alone, give every key's live setters and the thing's whole keyed state. Writers that name every live record they hold leave such names in a holding that is closed: one that holds what the writers of its records held. A holder missing a record can read wrongly from its own live records. The rule over payload keys alone gives the payload keys. | (a part of a set: none yet) |
| `keyed.heads` | `heads_slice`, `live_named_by_head`, `heads_need_env` | A head is live for the envelope, which every record sets. So with anchored names the live records read alone give the thing's heads, and every live record is a head or is named by one: the heads list the slice. With the rule over payload keys alone, the live records can take a superseded record for a head. | (a part of a set: none yet) |
| `keyed.slice` | `keyedIn_slice`, `keyedIn_live_slice`, `entitled_fewer`, `keyedIn_thing_slice`, `target_needs_witness`, `keyed_slice_needs_erasers` | After settling and admission, a part of the forms held gives a thing's keyed state when it holds a form of every live record of the thing, its records of the thing are witnessed, and for each record of the thing it holds at all it holds every form and every record that erases it, and the whole admits it. With anchored names that is the forms of the live records and what erases them. Nothing is owed for `prev` targets: a record the whole admits, any part admits. A target held without its witness still sets its keys and can read as live, so `thing.slice`'s targets are not added bare; a whole `thing.slice` gives the keyed state too. Without the eraser a part shows a payload the whole has erased. | (a part of a set: none yet) |
| `view.no-resurrection` (things) | `admit_view`, `stateIn_view`, `stateIn_no_false_resurrection` | With an envelope-only admission rule, a view admits the same records and keeps each thing's heads in view form. A superseded record never becomes current within that thing. | `fold/view/*`, `spec/run/03` |

Writing checks `prev` too: you can only replace what you hold. `emit` refuses an unheld `prev` target outside the batch (`prev-unknown`).
It refuses an unheld `under` target the same way (`under-unknown`); whether the target is a grant is standing's question.
It refuses an `of` naming a held record that is not a root (`of-not-root`).
It also refuses a `prev` target of a different thing (`prev-unrelated`).

## The spec (`model/Laws/Spec.lean`, `model/Laws/Live.lean`, `model/Laws/KeyedSlots.lean`, `model/Laws/Parse.lean`)

[`spec.md`](spec.md) is the grammar: the keys, what each test may compare
against, and the refusals in the order they are checked. These are its laws.

`where` has six operators: `is`, `not`, `present`, `absent`, `at-least`, `below`.
It has three answers.
A map holds when every test holds. An `env.<field>` test reads the written envelope and always answers.
A payload-field test answers unknown when the reader lacks the payload.
`env.of` reads `of` as written, so a root, of its own id, has no `env.of`.
`about`, `prev`, `under` and `erases` are sets; `of` and `under` have at most one member.
`is` tests membership, `not` tests nonmembership, and `present` tests whether the set holds anything. Time tests read `emitted` and `occurred` by meaning.
`is` asks for equal spans, regardless of spelling.
`at-least` and `below` compare starts; absent `occurred` passes neither.

`from` chooses by thing or by mention: `"of": "t"` takes the records of `t`.
A root is of its own id's thing, so `"of": "r:X"` takes X and its revisions.
`"about": "t"` takes the records whose `about` names `t`, as written.
A source with both takes the records that pass both.

Accumulating slots select, then fold.
A numeric fold (sum, min, max) counts a selected record as unread when its payload is not held.
It also counts the record as unread when its field is not a safe whole number.
Text is not a number.
Slots over heads read each thing's admitted state before selection.
These slots are latest-by and collect with `"follow": "prev"`.
`from` and envelope tests choose heads; payload tests then filter them. Selecting first
would bring back a record whose successor is of another kind, no longer
matches, or is withheld. Collect without `follow` takes every selected
record, superseded or not.

The parser gives exactly the specs that fit (`Spec.Fits`).
A fitting spec has a vertex, at least one source and distinct slot names.
Names and kinds are nonempty.
Each `where` names fields once, supplies tests, and never repeats an operator.
Each argument fits layer 0's field rules. Every spec that fits has a
written form that parses back to it, so an accepted spec's parsed form
parses back to itself. A spec with a repeated key is refused, at the path
the search in `Json.repeatAt` gives. These say what the parser gives, not
which JSON it accepts: a stricter parser would satisfy them too. Which JSON
is accepted, the order of the checks, and each refusal's code and path are
pinned by the vectors rather than proved.

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `where.complements` | `Op.not_is`, `Op.absent_present`, `Op.atLeast_below`, `Op.atLeast_below_time` | `not` complements `is`; `absent` complements `present`. With a held safe whole value and a numeric bound, exactly one of `at-least` and `below` holds. Present times obey the corresponding time law. | `spec/run/06`, `07` |
| `where.typed` | `Op.ordered_like_only`, `parseOp_ok_iff`, `parseArg_ok_iff`, `parseBound_ok_iff` | `at-least` and `below` compare like numeric or time values only. A test parses exactly when its argument fits its field (`Op.Fits`). Rendering preserves the written test. | `spec/run/07`, `spec/parse/22`–`37` |
| `where.fields` | `Cond.text_is`, `Cond.about_is`, `Cond.of_is`, `Cond.id_is`, `Cond.set_present`, `Cond.emitted_is`, `Cond.emitted_atLeast`, `Cond.emitted_below` | `kind` and `observer` read as text. `about`, `of`, `prev`, `under` and `erases` test members (`is`) and nonemptiness (`present`); `of` as written, absent on a root. `emitted` reads as its own instant. `occurred` is `where.time`'s | `spec/run/08`, `09` |
| `where.time` | `Cond.occurred_is`, `Cond.occurred_atLeast`, `Cond.occurred_below`, `Cond.occurred_missing`, `Cond.time_meaning`, `Op.time_arg_meaning` | Envelope-time tests read meaning, not record or argument spelling. `is` compares spans. `at-least` and `below` compare starts. A record without `occurred` passes neither bound. | `spec/run/06`, `08` |
| `where.total` | `Cond.eval_here`, `Cond.eval_envelope` | a test answers for every record whose payload is here, and an envelope test for every record, withheld or not | every `spec/run` vector |
| `where.set` | `Cond.all_perm` | the tests of a `where` map can come in any order | (by construction) |
| `view.refines` (where) | `Cond.eval_view`, `Cond.all_view`, `Tri.and_refines` | under a view a test, and a `where` map, keeps its answer or becomes unknown | `spec/run/02` |
| `spec.select` | `Spec.select_view`, `Source.takes_view`, `Spec.mem_select` | selection reads only envelopes, so a view selects the same records. `Spec.select` holds exactly the input records the spec takes; `Spec.run` does not call it. | `spec/run/*` |
| `spec.from-thing` | `Source.takes_of`, `Source.takes_about` | `from`'s `of` takes exactly the records of the thing, a root by its own id. `from`'s `about` is a mention test: it takes exactly the records whose `about` names the thing, as written. | `spec/run/09`, `14` |
| `spec.set` | `Slot.set_decides`, `Spec.set_decides`, `keyedValue_set_decides`, `Slot.keyedGrouped_set_decides` | Every slot of every spec gives the same value for the same distinct-id set, with each id consumed once, the slots that follow `keys` among them (`keyed.fold`, `keyed.slots`). | `spec/run/*` |
| `spec.follow` | `wholeHeads_overHeads`, `wholeHeads_set_decides`, `wholeHeads_answer_set_decides`, `Acc.followed_set_decides`, `mem_accPool_follow`, `accPool_follow_heads`, `select_as_latest`, `Count.followed_unread` | Count, sum, min and max that follow `prev` fold the whole-reader heads: every head of every thing, each id once, and no superseded record. The heads are each thing's LatestBy heads (`heads.set`), so any answer worked out from them depends only on the set held, and the accumulating form needs no commuting step (`acc.set`). They choose heads as latest-by does, and count each chosen head whose payload tests are unknown once. | `spec/run/16`–`20` |
| `keyed.live` | `mem_upward`, `mem_above`, `above_congr`, `liveFor_iff`, `liveEnv_iff`, `live_iff`, `mem_stateIn_heads_iff`, `head_live` | A record is above another when its `prev` names it, or names a record above it, through the records held: exactly what `upward` reaches, and only which records are held decides it. A setter of a key is live for it when nothing above it sets the key. Live for the envelope, which every record sets, is a head: nothing held names it, so a thing's heads are exactly its records live for the envelope, and every head is live. A record is live exactly when it is a head or live for some key, a key it does not declare among them when it is read whole. | `spec/run/31`–`37`, `41`, `scenario/keys-01-settled-setter` |
| `keyed.fold` | `keyedIn_keys`, `keyedIn_setters`, `keyedIn_whole`, `keyedIn_heads`, `pureJoin_only_heads`, `keyedIn_whole_read`, `keyedIn_set_decides`, `keyedOf_set_decides` | A latest-by that follows `keys` gives each thing an entry for every key some live record declares and no other, each holding exactly that key's live setters; its live records read whole; and its heads. A pure join is in neither of the first two. For a thing whose records are all read whole there is no key, and the records read whole are its heads. The keyed state and the slot depend only on the set held. | `spec/run/31`–`38`, `41` |
| `keyed.slots` | `thingEv_yes`, `thingEv_unchosen`, `keyedValue_count`, `Slot.keys_count`, `soleSetter_eq`, `soleSetter_two`, `soleSetter_unset`, `Acc.overThings_value`, `Acc.overThings_unread`, `keyedValue_collect`, `keyedValue_collect_unread` | Count, sum, min, max and collect that follow `keys` read things, selected as `keyed.select` selects them. Count counts each selected thing once, however many of its heads are chosen. Sum, min and max read the field from its one live setter: with none, or two or more (the same number or not), or one the fold cannot read, the thing is unread. Collect lists exactly the selected things' live records. A thing whose tests are unknown is unread, and each thing at most once. | `spec/run/42`–`44`, `47`, `48`, `50` |
| `keyed.by` | `By.thingKeys_payload`, `By.thingKeys_envelope`, `By.thingKeys_unheld`, `By.thingKeys_unset`, `Slot.keyedGrouped_eq`, `Slot.keyedGrouped_envelope_unread`, `By.keyedAccounting` | A slot with `by` that follows `keys` groups things. A payload key is read from each live setter of the field, and one that is not held makes it unread; with no setter the thing is ungrouped. An envelope key is read from each chosen head and is never unread, so a later head moves a thing between groups and a forked thing is in the group of each of its chosen heads, once. Every selected thing, or one whose tests are unknown, is exactly one of grouped, ungrouped or unread. | `spec/run/45`–`48`, `50` |
| `keyed.whole-read` | `Slot.keys_as_prev`, `keyedValue_as_prev`, `Acc.select_as_things`, `collectOf_as_things`, `whole_liveFor_iff`, `whole_live_iff`, `whole_setters`, `whole_thingTest`, `whole_thingEv`, `whole_thingKeys` | When every record is read whole and every thing has one head, a count, sum, min, max or collect that follows `keys` gives what the same slot following `prev` gives, grouped or not: a thing's live records are its one head, each field's live setter is that head, and its envelope is that head's. | `spec/run/49` |
| `keyed.select` | `keyedOf_things`, `keyedOf_unread`, `Cond.onThing_agree`, `Cond.onThing_differ`, `Cond.onThing_unheld`, `Cond.onThing_unset`, `liveFor_undeclared` | Heads choose, each alone: a thing is kept when `from` and the envelope tests choose one of its heads and its payload tests hold of the thing, and it is kept with its whole keyed state. A payload test is asked of every live setter of its field: one answer from all of them is the answer; two that differ, or one whose payload is not held, make it unknown; with no setter the field is absent. A record with `sets` is no setter of a key it does not declare. A chosen thing whose tests are unknown is unread once. | `spec/run/37`–`40`, `scenario/keys-01-settled-setter` |
| `by.set` | `Slot.grouped_set_decides`, `Slot.plain_set_decides`, `groupKeys_same`, `Slot.pool_set_decides` | The group a record joins is a function of that record alone, and each group is the slot without `by` run over the whole held set, choosing only its own records. So each group's answer depends only on the set: through `acc.set` without `follow` and `heads.set` with it. The keys, the unread and the ungrouped are counts of the same set. | `spec/run/21`–`25` |
| `by.after-admission` | `Slot.grouped_eq` | Settling, admission and the whole-reader heads are worked out once, over everything held, before any group; a group never settles, admits or finds heads on its own. Each group is the unmodified slot over all of it with its selection narrowed to the group. | `spec/run/23`, `24`, `scenario/by-01-erased-in-another-group` |
| `by.accounting` | `By.accounting`, `By.memberships`, `By.accounting_single`, `By.accounting_single_form`, `groupKeys_nonempty` | Every record the slot selects, or whose `where` is unknown, is exactly one of grouped (it has a key), ungrouped (its key is readably absent) or unread (its `where` is unknown, or its key can't be read). The groups hold each grouped record once per key, so the group sizes add up to keys of selected records: memberships, not records. When every selected record has at most one key they add up to the grouped records: always for every `by` but `env.about:<prefix>`, and for that one over records mentioning one thing under the prefix. A group exists only where something was selected into it. | `spec/run/21`, `22`, `24`, `25` |
| `by.partition` | `By.count_partition`, `By.count_partition_form`, `By.count_memberships` | A grouped count's group values add up to the keys of the records it selects. When every selected record has at most one key (always for every `by` but `env.about:<prefix>`), its groups, `ungrouped` and `unread` add up to the value and unread of the same count without `by`. | `spec/run/21`, `23`, `24` |
| `spec.after-supersession` | `latestOf_heads`, `collectOf_heads`, `Spec.view_state` | A slot over heads returns only heads retained in at least one thing, selected by its source, and passing every test. If the store's replacement rule reads only envelopes, a view keeps each thing's heads. Each head then shows only what the view shows. | `spec/run/03`, `04`, `08` |
| `spec.slice` | `Spec.run_slice`, `followed_slice`, `Slot.grouped_slice`, `spec_slice_needs_successors`, `spec_slice_needs_their_targets` | A reader holding part of the set gets a spec's whole answer when its part is a slice for the spec. A spec slice holds every form of the records the spec takes, every record that erases one of them, every record one of them names in `prev`, every record that names one of them in `prev`, and every record such a successor names in `prev`. Only the taken records are read whole; the envelopes of the rest are enough. A slot with `by` needs nothing more: a group's key reads only a taken record. Neither of the last two can be dropped. A slot that follows `keys`, of any fold type, is outside this law: it reads live setters the spec does not take. The slice for one keyed thing is `keyed.slice`; the slice for a spec with a keyed slot is not stated. | (a part of a set: none yet) |
| `spec.parse` | `Spec.ofJson_fits`, `Spec.ofJson_toJson`, `Spec.reparse`, `Spec.parsed_iff`, `Spec.ofJson_repeat`, `Json.repeatAt_none` | The parser gives exactly fitting specs, each with a source. Parsed forms parse back to themselves. Repeated keys are refused at `Json.repeatAt`'s path. Specs without repeats never receive that refusal. | `spec/parse/*` |

`heads.why` still holds with `spec.follow`.
A followed max is not a scalar accumulator that a late record steps: each answer is worked out again from the per-thing LatestBy state, which is the over-heads family's state.
Its law is `heads.set`'s. An incremental reader caches each thing's LatestBy state, onto which `heads.late` folds a late record per thing; a cached maximum alone would not do.
No law yet states the whole-view slot incrementally, and `admit.late` still bounds what a caching reader may drop.

## A batch with labels (`model/Laws/Batch.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `batch.meaning` | `Meaning.unique` | An acyclic batch has at most one meaning. A label may stand in `about`, `of`, `prev` or `under`, and every one of them counts toward acyclicity. This does not prove resolver correctness. | `batch/01`–`03`, `26`, `32`, `33`, `65`, `66`, `68`–`70` |
| `batch.refuse` | (by construction) | Duplicate, unknown and cyclic labels are refused, a label in `under` among them. So are drafts refused by `write`, unheld `prev` targets, unheld `under` targets, an `of` naming a held record that is not a root, and `prev` targets of another thing. | `batch/10`–`17`, `27`–`31`, `34`–`38`, `67`, `71`–`75` |
| `batch.refuse-order` | `envelopeKeysProblem_set`, `draftsShapeProblem_perm`, `firstRefusal_set`, `writeProblem_codes` | Across the batch: shape, then labels, then writing, then `prev-unknown`, then `under-unknown`, then `of-not-root`, then `prev-unrelated`. Raw shape checks every envelope's keys, then that every envelope is an object, before any draft value, label or salt is decoded. Shape depends only on which envelopes there are, and writing's priority only on the refusal set; stage order is by construction, pinned by raw-input rotations. | `batch/15`, `16`, `18`–`22`, `38`, `40`–`64`, `71`–`75` |

`gen` checks batch vectors across draft rotations and reversed `prev` sets.
Accepted variants must give the same records, one per draft, satisfying `Meaning`.
Refused variants must give the same code. What isn't proved yet is that `resolveBatch` returns the meaning
`Meaning.unique` speaks of; `gen` checks it on the vectors.

## Lens: keyed sections (`model/Laws/Lens.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `lens.escaping` | `escaping`, `unescape_keeps` | any body survives when its section id is nonempty; reading removes only an escape backslash | `lens/section/02`, `10` |
| `lens.round-trip` | `parse_render`, `parse_render_iff` | rendering roundtrips exactly when section ids are nonempty and distinct | `lens/section/01` |
| `lens.no-op` | `no_op` | the unedited render gives no outcomes when section ids are nonempty and distinct | `lens/section/03` |
| `lens.put-get` | `put_get`, `put_get_text` | With distinct base ids, an edit keeping exactly the base's sections in the same order applies exactly, up to normal form. | `lens/section/04`, `10` |
| `lens.nothing-dropped` | `nothing_dropped`, `absence_reported` | every section is unchanged, accepted or reported; every dropped one is reported | `lens/section/05`, `06` |
| `lens.absence-not-edit` | `absence_not_edit` | a dropped section leaves that section in the state as it was; the other sections' accepted edits still apply | `lens/section/06` |
| normal form | `normalize_idem` | normalizing twice is normalizing once | every lens vector |
| `lens.refuse` | `parse_valid` | malformed text, empty section ids and repeated section ids are refused | `lens/section/07`–`09` |
| `lens.base-distinct` | `baseProblem_none`, `read_base_first`, `read_ok`, `read_put_get` | The public reader refuses a base that repeats an id with `base-id-duplicate`, before it reads the text and whatever the text says. What it accepts is the raw edit over distinct base ids, so put-get needs no promise about the base from the caller. | `lens/section/d1-duplicate-base-id`, `lens/section/d1-duplicate-base-unedited`, `lens/section/d1-duplicate-base-apart`, `lens/section/d1-duplicate-base-before-text`, `lens/section/d1-duplicate-base-empty-text` |

`lookup` takes the first section with an id; that raw behavior is unchanged and unchecked.
The public boundary is `read`: it checks the base, then the text.

## Record (`model/Laws/Record.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `record.erasure` | `erase_keeps_id`, `erase_idem` | forgetting the opening never changes the id | `record/08`, every `erased_line` |
| `record.commit` | `opening_checks` | a record's own salt and payload open its commitment | every stored record vector |
| `record.sets` | `id_sets`, `canonSet_eq`, `setsJson_eq` | reordering or repeating `about`, `prev`, `erases` or `sets` never changes the id | `record/02`, `03`, `11`, `15` |
| `record.keys` | `Env.problem_sets`, `Env.problem_sets_not_keys`, `Env.problem_of_missing_sets`, `toJson_sets`, `sets_empty_ne_absent`, `Read.envOf_sets_mistyped` | `sets` holds key names, and absent and empty differ. The canonical envelope holds `sets` only as stated: left out when absent, written when stated, `"sets": []` included, so an envelope with an empty `sets` is not the envelope without it. Every key an accepted envelope declares is non-empty text; `sets-not-keys` speaks only when every member of `erases` is a record id, and `of-missing` only when every declared key is a key name. A `sets` that is not an array, or has a member that is not text, reads as an unreadable member and is refused. Pinned by the vectors, not proved: the bytes and ids, that nothing compares `sets` with the payload, and that a batch ranks `sets-not-keys` after `erases-not-record-id` and before `of-missing` and replaces no label in it. | `record/15`–`19`, `record/45`–`47`, `record/shape-sets-text`, `record/shape-sets-member-number`, `record/shape-sets-empty-name`, `record/shape-erases-before-sets`, `record/shape-sets-before-of-missing`, `batch/76`–`82` |
| `record.of` | `Env.problem_of` | `of` holds at most one typed reference and is left out when absent, so a record without it keeps its bytes and id. Only a root leaves it out: an envelope the writer accepts with `prev` states `of`. | `record/14`, `41`, `44`, `batch/27` |
| `record.shape` | `unreadable_refused`, `Env.problem_unreadable`, `Written.no_unreadable`, `Read.envOf_missing`, `Read.envOf_mistyped`, `Read.envOf_set_mistyped`, `Read.envOf_sets_mistyped`, `Read.salt_length`, `writeProblem_salt`, `envelopesShapeProblem_perm` | Every envelope shape has a named refusal. Proved: the reader reads a required field left out, or a field of the wrong JSON type, as empty text, and an envelope holding it is refused, so no accepted or stored record holds it (it can be hashed on the way to a refusal); a salt that is not base32hex text of 16 bytes is not 16 bytes, which a good envelope's writer refuses; raw shape depends only on which envelopes there are. Pinned by the vectors, not proved: an envelope that is not an object refuses with `envelope-not-object`, after any unknown key and before every value; which code a missing or mistyped field gets, and its place in the order; that the reader's decoder is `Read.envOf` over the object's members. A repeated envelope member name is malformed input. | `record/shape-not-object-number`, `record/shape-not-object-after-written`, `record/shape-not-object-before-text`, `record/shape-missing-kind`, `record/shape-kind-number`, `record/shape-of-not-missing`, `record/shape-observer-before-emitted`, `record/shape-salt-number`, `record/r6-nonobject-unknown-first`, `record/r6-nonobject-unknown-last`, `record/shape-about-null`, `record/shape-salt-null`, `record/shape-empty-sets`, `record/shape-sets-null`, `batch/40`–`64`, `batch/77`–`82` |
| `record.member-order` | `canon_member_order` | reordering an object's members never changes its canonical bytes | `record/04` |
| UTF-16 order | `lexLt_trans`, `lexLt_total`, `utf16_inj` | member and set order is a strict total order on distinct strings | every record vector |
| `record.unicode` | `ofUtf16?_eq_some_iff`, `ofUtf16?_utf16`, `ofUtf16?_lone_high`, `ofUtf16?_lone_high_end`, `ofUtf16?_lone_low` | UTF-16 code units read as text exactly when some text writes them. A string's own code units read back as that string, so no string the model canonicalizes or hashes holds a lone surrogate. A high surrogate without a low one after it, and a low one without a high one before it, are no text. | `record/d2-lone-high`, `record/d2-lone-low`, `record/d2-low-then-high`, `record/d2-high-then-escape`, `record/d2-member-name`, `record/d2-paired-escape` |
| `record.refuse` | (by construction) | Bad envelopes, salt lengths, repeated members, nonfinite and out-of-range numbers receive named refusals. Raw unknown envelope keys refuse before `kind-empty`, and an envelope that is not an object after them (`record.shape`). A payload text that is JSON but holds a lone surrogate refuses with `string-not-unicode`, after raw shape and before every value check. A record input's first envelope and first payload decide its refusal. | `record/20`–`34`, `record/45`–`47`, `record/r6-unknown-field`, `record/r6-unknown-before-kind`, `record/d2-before-kind`, `record/d2-before-member-duplicate`, `record/d2-shape-first`, `record/d5-refused-then-written`, `record/d5-differently-refused-0`, `record/d5-differently-refused-1`, `record/d5-first-pair` |
| `write.checked` | `writeProblem_none`, `Written` | `write` succeeds exactly when every check passes, and what it returns carries the proof; a batch writes through it | `record/*`, `batch/15`, `17` |
| `write.member-order` | `Written.commit_member_order` | a written payload's commitment doesn't depend on the order of its members: `write` established the distinct names `record.member-order` needs | `record/04` |
| `write.overflow` | `Number.finite_of_inRange`, `writeProblem_nonfinite`, `writeProblem_range_last` | A number past binary64's range is nonfinite, and no nonfinite number is in the writer's range. Under a good envelope and salt the writer refuses it with `number-out-of-range`. That refusal is the last: it speaks only when the envelope, the salt and the member names pass. | `record/d3-overflow`, `record/d3-overflow-value`, `record/d3-member-duplicate-first`, `record/d3-salt-first`, `record/d3-kind-first`, `record/d3-shape-first`, `batch/23`–`25`, `spec/parse/d3-overflow-bound`, `spec/parse/d3-overflow-limit` |

## Occurred (`model/Laws/Time.lean`)

| Law | Theorem | Says | Vectors |
|---|---|---|---|
| `occurred.span` | `occurred_point`, `occurred_open`, `occurred_interval`, `point_le` | Each accepted point gives its grammar-defined span, alone, with `..`, or paired with a point starting no earlier. A span never ends before its start. | `occurred/01`–`06` |
| `occurred.qualifier` | `occurred_qualified`, `occurred_qualified_start`, `occurred_qualified_end` | a qualifier never changes a span, on a date alone or on either end of an interval | `occurred/03`, `05` |
| `occurred.moment` | `instant_canonical`, `point_canonical`, `moment_lt_iff_scaled` | A parsed moment has whole seconds and digits without trailing zeros. For stripped fractions and a scale covering both, list comparison equals exact decimal comparison. | `occurred/04` |
| `occurred.order` | `Span.before_irrefl`, `Span.before_trans`, `Span.before_total` | Spans have strict total order by start, then end, with an open end last. Stores using that order agree. | `occurred/08` |
| `occurred.refuse` | (by construction) | a text the reader doesn't accept is refused with `occurred-not-span`, never guessed at | `occurred/07`, `record/34` |

Which texts are points is the definition itself (`dateDays?`, `instant?`),
pinned by the vectors rather than by a theorem about the grammar. The
acceptance and decimal-order proofs are adapted from Gemini's review.

The laws hold for any hash function. The vectors use SHA-256 and Ed25519 reference code in `model/Gen/`.
Before writing, that code checks FIPS 180-4 and RFC 8032 values.

## What these laws don't promise

The laws about folds, views and specs are each about a given set of
records. None says the set is complete, current, trusted or well dated:

* **Absence isn't visible.** A reader that lacks a successor can see its predecessor as current.
  It cannot count records it does not know exist. The laws promise convergence, not freshness.
* **Torn reads.** A reader must not see half a changeset.
  The planned place closes each changeset through atomic rename.
  Readers ignore a partial last line.
* **Time is the writer's word.** Content order breaks ties by claimed
  times, so a skewed clock picks a fork's winner. Every head of a fork is
  still shown.
* **Trust is step 3.** Fold laws do not check signatures or standing.
  They require an admitted record set in which each id names one record.
* **A checkpoint is a set.** "Since" means segments not yet seen, not an
  offset, or a late segment is missed.

## Simplifications in the model

* The model stores each id as a natural number.
  The digest laws require each id to be less than 2^256.
  This bound applies to a record's id and each id that it references.
  The protocol supplies the bound through its 32-byte SHA-256 digests.
  A number is not a hash of the record's envelope, so nothing in the model keeps a record's `prev` from naming itself or a record above it.
  The keyed slice laws that need `prev` to have no cycle state it as a premise (`NoCycle`); a real id gives it, since it is a hash of an envelope that holds its `prev`.
  The model does not derive fold records from stored records.
  No theorem connects a fold record's times to the checks on a stored record's envelope.
* The payload is `value`, `n` and named `fields`.
* Fold records carry time text and its parsed values (`Stamp`, `Claim`).
  Their other envelope fields are direct inputs, not fields parsed from an envelope.
* Text is a list of lines; splitting a file into lines is outside the model.

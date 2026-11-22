# The tick record

## The record and its payload

A tick is an ordinary signed record of kind `tick`. Its record id identifies it.
Its opening holds one JSON object with these members.

| Member | Value |
|---|---|
| `rules` | The evaluation rules version, as text. |
| `spec` | The spec in its parsed form. |
| `spec-hash` | The identity of that spec under those rules. |
| `covers` | The array of record ids in the set folded. The empty array is allowed. |
| `withheld` | The covered ids read without their payloads. Write this member only when the array is nonempty. |
| `result` | The canonical result object, with every slot of the spec. An empty fold gives `{}`. |
| `result-hash` | The hash of that result. |

The payload bytes are the UTF-8 encoding of the object's JCS text. Two writers using the same rules, parsed spec, covered set and withheld set, with consistent forms and all required openings, write identical payload bytes. Each signs its own record with its own observer, emitted time and salt. Those can change the record id.

## The spec and the rules

A version is positive decimal text matching `[1-9][0-9]*`. Its length is unbounded. Compare versions numerically. The current version is `"1"`.

The parsed spec carries the vertex, sources and fold and drops carried keys such as `note`. Hash the supplied `spec` member's UTF-8 JCS bytes under the label `loops/spec-hash/v<rules>:`. The stated version always determines this label, including on a reader that cannot interpret the spec. [The spec's hash](spec.md#the-specs-hash)

The rules cover settling, admission, source selection, tests, folds, payload reading and result encoding. A change to an existing parsed spec's answer raises the version for every spec. An added fold, test or newly read key that preserves all existing answers can keep the version. Earlier ticks carry the parsed form that omitted the key. Correcting an implementation to match the rules keeps the version.

The result codec versions its hash label independently. The current label is `loops/result-hash/v1:`. For its own and older rules versions, a reader requires the result hash to fit the raw result under the known label. For newer rules it requires a well-spelled result hash and leaves its fit unjudged. [The result's hash](result.md#the-hash)

## What a tick claims

The writer covers **every record in the set it folds**, including records the spec does not take. Each id appears once. Within that set, settle the forms. Every covered id whose settled record lacks an opening belongs to `withheld`. This includes an erased record and a target of a covered eraser.

To run the claim, take the forms of covered ids. Mask the payload of every form of every withheld id. Settle and admit inside this set alone, then run the spec's sources and slots. Store names remain in the spec's identity. The covered set supplies the input records. Encode the run with the [canonical result codec](result.md).

A covered tick contributes one record. Its payload fields can be read. Its own covered records enter the run only if the outer tick covers them.

## Reading a payload

The common-member reader takes the reader's rules version and the original payload. Preserve object member names, including repeats, when decoding the input. Names must be distinct at every depth, including inside an array or an opaque spec or result. The signed record supplies the ordinary JCS number and Unicode domain: every number is finite binary64 and every string is well-formed Unicode. Within that domain, one JCS text determines the scalar tokens, the array positions and the named values of every object. Outside it no answer is defined. A number past binary64's range or a lone surrogate inside `spec` or `result` cannot be in a stored record, and what any implementation answers for one, the model included, is not a rule. [Record bytes](record.md#canonical-bytes)

Both id arrays are strictly increasing by canonical record id, with every withheld id in `covers`. A spec hash starts with `spec:`, a result hash with `res:`. Each prefix is followed by the canonical lowercase unpadded base32hex encoding of 32 bytes, exactly 52 characters with zero unused bits, as for the digits of a record id.

The common reader requires an object for `spec` and verifies its raw-member hash independently of parsing. It applies the result-hash rule above to the raw `result`. It leaves the spec vocabulary and result shape for the version-specific reader. In the `members` vectors a successful answer gives `rules`, `spec`, `result`, `covers` and `withheld`. An absent withheld member gives `withheld: []` in this answer. The answer omits both hashes.

Besides a structural refusal, the version-specific reader has three answers in the vectors.

| Answer | Meaning |
|---|---|
| `{"payload": …, "jcs": …}` | Under its own rules the reader accepts the parsed spec and the result's shape and spelling. The payload is the written object and `jcs` is its canonical text. |
| `{"other-rules": "<version>"}` | The stated version differs from the reader's version. |
| `{"cannot-read": "spec"}` | The spec is an object with a fitting hash under the reader's version, and its parser refuses it. |

For a spec that parses, compare the JCS text of the supplied member with the JCS text of its parsed form. Object member order is free. Keeping a dropped `note` or omitting an empty `fold` changes this text. The result reader uses the spec's slots. It accepts a well-shaped claim even when counts and lists disagree. [Reading a result](result.md)

A structural refusal carries `refused` and `path`. Paths are for people. Members are joined with `.` and array indices are written `[i]`, starting at zero. A group, thing or other member name can itself contain a dot. Paths have no machine parsing grammar. For several faults the reader refuses with one applicable code.

| Code | Fault | Path |
|---|---|---|
| `tick-not-object` | The payload is not an object. | Empty string |
| `tick-member-duplicate` | An object repeats a member name. | Repeated member, at any depth |
| `tick-member-unknown` | The payload has an unrecognized member. | Member name |
| `tick-member-missing` | A required payload member is absent. | Missing name |
| `tick-rules-invalid` | The version is outside the positive-decimal text grammar. | `rules` |
| `tick-spec-not-object` | The spec member is not an object. | `spec` |
| `tick-ids-not-array` | A supplied id list is not an array. | `covers` or `withheld` |
| `tick-id-invalid` | An id-list entry is not a canonical record id. | List and index |
| `tick-id-duplicate` | An id occurs twice in an id list. | List name |
| `tick-ids-unsorted` | The distinct ids in a list are out of order. | List name |
| `tick-withheld-empty` | The withheld member is present with zero ids. | `withheld` |
| `tick-withheld-outside` | A withheld id is outside the cover. | `withheld` |
| `tick-hash-invalid` | A hash has the wrong spelling for its member. | Hash member |
| `tick-spec-hash-mismatch` | The spec hash does not fit the supplied spec under the stated label. | `spec-hash` |
| `tick-result-hash-mismatch` | A result hash whose fit is required does not fit the supplied result. | `result-hash` |
| `tick-spec-noncanonical` | A readable spec differs in JCS text from its parsed form. | `spec` |
| `tick-result-invalid` | A member or value fails the slot codec, including a noncanonical decimal such as `"-0"` at `unread`. | Offending or missing position, rooted at `result` |

## The check

Import and sync apply ordinary record verification. Neither requires a tick check. The check takes the subject kind, its locally held opening, the checker's whole holding and its rules version.

| Answer | Evidence |
|---|---|
| `not a tick` | A subject of another kind gives `tick-kind` at `kind`. |
| `not a tick` | A structural payload fault gives the reader's code and path. |
| `not a tick` | Under current rules a held covered eraser naming a covered target omitted from `withheld` gives `tick-withheld-erased` at `withheld`. Its envelope is sufficient evidence on a partial holding: this answer does not wait for the records that are missing. |
| `cannot check` | The reason is `the tick's payload is not held`, `this binary cannot read the spec`, `made under older rules`, `made under newer rules`, or `missing inputs`. An erased opening and one that never arrived are both unheld. |
| `same` | The replay's canonical result bytes equal the claimed result bytes. |
| `differs` | The bytes differ. The answer carries `claimed-hash` and `computed-hash`. |

Missing inputs give two counts, `missing-records` and `missing-openings`. Count each covered id with no form as a missing record. Among the remaining covered ids outside `withheld`, count each id without local access to its opening as a missing opening. Access requires a form with its payload and no eraser of that id anywhere in the checker's holding. An eraser outside the cover can remove access while the bytes are still on disk. Both counts apply even to a count-only spec.

The answers about the version are given only for a payload the common reader accepts. A payload whose `spec-hash` does not fit is not a tick, whatever version it states.

For an older tick nothing past the common reader makes it "not a tick". Report `made under older rules`, and beside it today's comparison, `today` in the vectors. It compares today's run with the older raw result bytes, and the older result's shape is not read. It is one of:

| `today` | When |
|---|---|
| `"the same result"` | Today's parser reads the spec in its parsed form, the inputs are held, and the bytes are equal. |
| `"another result"` | The same, and the bytes differ. |
| `{"not-available": "this binary cannot read the spec"}` | Today's parser refuses the spec. |
| `{"not-available": "spec is not in today's parsed form"}` | The spec parses and its JCS text is not that of its parsed form. |
| `{"not-available": "tick-withheld-erased"}` | A held covered eraser names a covered target that `withheld` leaves out. This reason is the refusal's code; the other three are prose. |
| `{"not-available": "missing inputs", "missing-records": n, "missing-openings": m}` | Covered records or required openings are not held. |

A newer tick receives no such comparison.

Two reports accompany the verdict.

* **Closure** is `no` when a held covered envelope names a `prev` or `under` target outside the cover. It is `yes` when all covered envelopes are held and all such targets are covered. Otherwise it is `unknown`. It is `unknown` for every refusal of the common reader, also when `covers` itself is sound: the cover is used only from an accepted payload.
* **Taken and not covered** (`taken-not-covered` in the vectors) counts ids outside the cover that the spec's sources take from the whole local holding after settling and admission. Sources read envelopes, so a record whose payload is withheld or erased counts when a source takes it, and a record refused at admission does not. The count is available once the checker has a spec that is its own parsed form, including for an older-rules diagnostic. Otherwise it is unavailable: a spec that parses and is not in its parsed form gives none.

Closure and the outside count leave the verdict unchanged. Agreement concerns the stated set and visibility. It establishes neither completeness of the store nor what a fresh fold over a larger holding would give.

## Reading a tick in a spec

An open tick uses the ordinary payload reading.

| Member | Field reading |
|---|---|
| `rules`, `spec-hash`, `result-hash` | Text. Equality, presence and grouping read the text. |
| `covers`, `withheld` | Text members. An equality test asks for membership. Grouping contributes once to each id's group. An omitted `withheld` is absent. |
| `spec`, `result` | Present with no scalar value. Presence tests can see the field. A numeric fold cannot reach a nested slot. |

Two nightly ticks each cover three different backups. A third tick covers the two jellyfin backups, one from each night. A spec over these ticks counts three ticks. Grouping by `covers` gives two for each jellyfin backup and one for each other backup. Testing membership of the first jellyfin id counts two ticks. All three share one spec hash. Summing `result` gives zero with three unread.

## Erasure

Erasing a covered record leaves the tick and the copy of the result in its payload intact. To find affected ticks, select kind `tick` and test `"where": {"covers": {"is": "<record id>"}}`. Erase each tick that needs erasure by naming its own record id, as for any other record.

## Vectors and laws

In `vectors/tick/`, `hash` and `domains` pin spec identity and domain separation. `window` runs window tests. `write-payload` writes from supplied forms. `payload` and `payload-text` read a payload, with the latter preserving its original textual member names. `members` runs the common reader. `fields` projects one opening, and `read-ticks` runs a spec over tick records. `make-and-check` makes and replays a tick, and `check-tick` checks a supplied subject.

The indexed laws are `tick.inputs`, `tick.withheld`, `tick.canonical`, `tick.record`, `tick.made` and `tick.level`. Spec identity uses `spec.hash`, `spec.no-commit` and `spec.no-result`. [Law statements and proof names](laws.md) · [Vector operations](../vectors/README.md#tick)

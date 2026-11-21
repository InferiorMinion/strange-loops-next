# Vectors

`model/` writes the protocol fixture families:
`fold/`, `lens/`, `record/`, `byte-text/`, `spec/`, `batch/`, `realm/`, `occurred/`, `scenario/` and `result/`.
Run `lake exe gen ../vectors` from `model/`; never edit these fixtures by hand.
CI regenerates them and fails on any difference.
`standard/` holds external standards data; see its README.
JS and the Rust workspace check all fixture families and standards.

Every file has the same envelope:

```json
{
  "vector": "02-fork",
  "layer": "fold/latest-by",
  "laws": ["latest-by.set", "latest-by.order"],
  "source": "model",
  "input": { },
  "expect": { }
}
```

## `fold/count`, `fold/latest-by`, `fold/latest`, `fold/sum`, `fold/extrema`

`input.orders` lists arrival orders of the same records.
Fold each order separately; each must produce `expect`.
`count.repeats` has one order.

* Count: `expect` is the number.
* LatestBy: each thing has `heads` newest first, a `winner` and `others`.
  Nonempty `others` shows a fork.
  A record is of exactly one thing: its `of`, read as written, or without `of` its own id.
  `about` never makes a head.
* Latest: the `winner` and `others` of the combined, distinct-id per-thing heads.
* Sum: the number.
* Extrema: `min` and `max`, each `null` when no input had a readable `n`.

Each record's `about` is a set of the things it mentions.
A record may carry `of`, the one thing it is an emit of; a record without it is of its own id.
It carries `emitted` and may carry `occurred`, as defined in `protocol/record.md`.
Content order compares span start, span end, `emitted`, then id digest bytes.
A record without `occurred` has an instant span at its `emitted`.
Newest first reverses this whole order.
Ids use layer 0's encoding, but these fixture digests are made up.
The optional `fields` object holds the written primitive payload fields, including
`value` and `n`. Omitting the object gives a held record with no fields.
A top-level `value` or `n` on a reader record is an unknown member.
The raw sum and extrema fixtures read `fields.n` through the safe-whole reader.
An omitted or unreadable field contributes no value. The accumulated integer is exact.
The envelope supplies the observer and may name erased ids in `erases`.
An optional `sets` is the envelope's, kept when the payload is not: a reader record of any family may carry it, and only a slot that follows `keys` reads it.
A hidden payload uses `shown` (`withheld` or `erased`) and omits `fields`.

## `fold/heads`

Answers come from LatestBy's heads.
`expect.max.value` is the largest readable safe whole `n`, or `null` if none exists.
`expect.max.unread` counts every head with an unreadable `n`, not just hidden payloads.
`expect["limit-2"]` lists the two newest heads.

## `fold/view`

Some records arrive without payloads.
`expect.heads` lists heads newest first, with each record's `shown` form.
With consistent forms, these are the same heads as the full reader's.
`expect.max` follows `fold/heads`.

## `fold/select`

`input.where` uses spec syntax and names envelope fields `env.<field>`.
`gen` checks that it gives the test the model runs.
An envelope test always answers.
A payload test answers unknown when the reader lacks the payload.
`expect.count` and `expect.sum` give `value` over selected, readable records and an `unread` count.
A covered record is unread if its test is unknown or its selected fold cannot read it.
Count needs no payload.
Sum, min and max require a held payload and a safe whole field value.
Full-result equality requires both answering tests and fold-readable covered records.

## `spec/parse`

`input.spec` is a vertex's JSON spec.
A repeated-key spec also supplies `input.spec_text`; read that text without discarding repeats.
`expect.parsed` gives the parsed form, which parses back to itself.
Alternatively, `expect.refused` gives the code and `path`.
`protocol/spec.md` defines the normative grammar, parsed form and refusal order.
Several vectors break multiple rules; refusal order decides the expected code and path.
A path has no array index, so `119` and `120` repeat a different key in each of two elements of `from`: the refusal is at the first element's repeat.

## `spec/run`

`input.spec` and `input.orders` follow the fold fixtures.
`expect` names each `fold` slot.
Count, sum, min and max return `value` and `unread`.
Latest-by returns each thing's current records in `things`.
Collect returns records newest first in `heads`.
Each record carries its held form; `unread` counts records whose payload tests cannot answer.
Count each unread id once per slot.
An erased record's eraser is among the records, so a reader that settles them first keeps the erased form; `gen` checks this.

Latest-by, and collect, count, sum, min and max with `"follow": "prev"`, read heads.
Supersession runs per thing over the admitted set before selection.
`from` and envelope tests choose heads; payload tests then filter them.
`from.of` takes the records of a thing; `from.about` takes the records whose `about` names it, as written.
`env.of` tests `of` as written, so a record without `of` holds nothing there.
Collect, count, sum, min and max without `follow` take every selected record, superseded or not.
With it, every head of a forked thing counts; count counts an erased or withheld head by its envelope, and sum, min and max count it unread (`spec/run/16`–`18`).
A slot with `by` gives `{"groups": {<key>: <result>}, "unread": n, "ungrouped": m}`, each `<result>` shaped as the slot without `by` (`spec/run/21`–`25`).
A latest-by with `"follow": "keys"` gives `{"things": {<thing>: {"keys": {<key>: [...]}, "whole": [...], "heads": [...]}}, "unread": n}` (`spec/run/31`–`41`).
A record's optional `sets` lists the keys it sets: without it the record is read whole, and `"sets": []` sets none.
`keys` lists each key some live record declares, with its live setters; `whole` the live records read whole; `heads` the heads; each newest first.
`from` and envelope tests choose heads; each payload test is asked of every live setter of its field, and a thing whose tests are unknown is unread once.
`scenario/keys-01-settled-setter` reads a setter withheld, erased without its eraser and erased.
Count, sum, min, max and collect with `"follow": "keys"` keep the shape of the slot without `follow`, and read things (`spec/run/42`–`51`): count counts the selected things, sum, min and max read the field from its one live setter, and collect lists the selected things' live records, its `limit` counting things, each listed whole (`spec/run/51`); a thing whose tests are unknown, or whose field or key cannot be read, is unread once.
With `by`, a payload key is read from the field's live setters and an envelope key from the chosen heads (`spec/run/45`, `46`).
Settling, admission and heads come first, over every record; each group is the slot without `by` or `where` over the records selected into it, so a collect's `limit` is per group.
A `where` that answers unknown, or a payload key on a payload not held, adds to the top-level `unread`; a key readably absent adds to `ungrouped`; a record with two keys is in two groups.
A whole number key is written in decimal and a boolean as `true` or `false`, so text `"1"` and the number 1 share group `1`.
`scenario/by-01-erased-in-another-group` settles forms: an eraser in another group still erases.
Collect counts unread candidates before applying its limit.

## `result`

`input.spec` and `input.orders` follow `spec/run`; sixteen inputs are `spec/run`'s own, under the same names, and the `r` cases are new.
`expect.canonical` is the vertex's whole result as canonical text, and `expect.hash` its hash, as [`protocol/result.md`](../protocol/result.md) defines them.
Every arrival order gives both.
The text lists ids without `shown`, writes each integer as a decimal string, and is JCS, so its slots are in UTF-16 order.
`r01` sums past −2^53, `r02` sums to `"0"`, `r03` reads nothing: zero, `null`, an empty list and an empty map.
`r04` has a grouped slot whose `ungrouped` and `unread` differ.
`r05`, `r06` and `r07` hold one result three ways, a record's payload held, withheld and erased: their texts and hashes are equal and their readings are not; `gen` checks both.

## `batch`

`input.drafts` is one changeset.
Each draft has an optional `label` (`@name`), an envelope, a `salt` and a payload.
`about`, `of`, `prev` and `under` may use labels; `erases` may not.
`sets` holds key names, never labels: a key that starts with `@` is written as given (`batch/76`).
`input.held` lists store-held records: each `id`, its `of` when it has one, and its `prev`.
Writers require every `prev` and `under` target in the batch or held store state.
A batch member counts by its label or its id; a `held` entry carries no shown state, so an erased or withheld record is held like any other.
Readers can admit unknown targets because they cannot judge them.

`expect.records` gives ids and canonical envelopes in draft order, with labels replaced by ids.
Alternatively, `expect.refused` gives the first refusal.
Shape checks run first across every raw envelope, before any draft value: `envelope-field-unknown`, then `envelope-not-object`.
A draft object without `env` has an envelope that is not an object.
A field left out or of the wrong JSON type, and a `salt` that is not base32hex text, are refused by the writer with the field's own code.
A payload number past binary64's range is a writer refusal, `number-out-of-range`, ranked with the others.
Label checks follow: `label-duplicate`, `label-unknown`, then `label-cycle`.
Next come writer codes from `kind-empty`, in `protocol/record.md` order across every draft.
Finally, `prev-unknown`, `under-unknown`, `of-not-root` and `prev-unrelated` run, in that order, across the whole batch.
`under-unknown` refuses an `under` naming a record the writer holds neither in the batch nor in `held`; nothing else about the target is checked.
`of-not-root` refuses an `of` naming a record the writer holds, in the batch or `held`, that is not a root: one with `of` or `prev`.
`prev-unrelated` refuses a `prev` naming a held record of another thing.
Every draft order and `prev` order gives the same records or refusal.
Meaning uniqueness states at most one satisfying label assignment; it does not prove resolver correctness.

## `realm`

`input.forms` lists all forms held across places.
Consistent forms share one record per id.
A held eraser can suppress a target's payload; a bare erased copy cannot suppress a held payload.
`expect.settled` gives one record per id and its held form.
`expect.refused` gives admission refusals and codes: `prev-unrelated` when a held `prev` target is of another thing.
`expect.things` gives per-thing states, as in `fold/latest-by`.
`expect.sum` gives Sum over admitted records.
Every rotation must give the same answer when forms are consistent.
Write `settled` and `refused` in ascending id digest-byte order.
Canonical id text has the same order.
This order does not change head content order.

## `scenario`

A scenario gives steps over one spec.
`input.records` supplies full records.
Each `input.steps` entry names place-held `forms`: an id and its held form.
Its `orders` lists the rotations checked by `gen`.
Each `expect.steps` entry gives `settled`, `admitted`, the requested thing's heads and winner, and spec `slots`.
These fields follow `realm` and `spec/run`.
Consistent forms give the same answer in every rotation.
Write `settled` and `admitted` in ascending id digest-byte order.

`input.lens.base` holds base sections; `input.lens.text` holds edited lines.
`expect.lens` holds rendered base, outcomes and applied sections.
The scenario adapter requires a text field named `value` on a rendered head and splits it at LF, keeping all parts, including empty parts.
It leaves CR unchanged.
The lens itself does not split line elements.
See [`protocol/lens.md`](../protocol/lens.md).

`01-decision` supplies the grammar page's walk.
Two people write, fork, merge and erase a decision.
A reader holds less; a lens edit returns as a record.

## `occurred`

`input.occurred` lists time texts.
`expect.spans` gives each span's UTC `start` and `end`, or `refused`, in input order.
An open span has `end: null`.
Instants use canonical UTC text as specified in `protocol/record.md`.
A calendar span ending after year 9999 can have a five-digit end year.
In `08-order`, `expect["oldest-first"]` lists texts ordered by span, per `protocol/record.md`.

## `lens/section`

`input.base` supplies base sections; `input.text` supplies edited lines.
A base that repeats an id gives `{"refused": "base-id-duplicate"}`, whatever the text is.
Otherwise `expect` is either `{"parsed": null, "refused": true}` or the parsed document and its normal form.
It also gives `accepted`, `unknown` and `absent` outcomes, plus the applied base.
Render roundtrip requires nonempty, distinct ids.
Put-get additionally requires distinct base ids and the exact same ordered id list in the edit.

## `record`

`input.envs` gives envelope variants with the same id.
`input.payloads` gives payload variants with the same commitment.
`salt` and the signing key's `seed` use base32hex.
`expect` pins canonical envelope and payload, commitment, id preimage, id, signing key, signature and both stored lines.
The two stored lines show forms before and after erasure.
The signed message uses the exact prefix `loops/record/v1:`.

`sets` is canonical as `about` is, except that an empty one is written `"sets": []`.
`record/16` and `17` are the same envelope, salt and payload with an empty `sets` and without one: two records, two ids.
`record/18` declares a key its payload lacks and `19` declares keys beside a payload that is not an object; both are written.
Every stored record vector's `erased_line` keeps the envelope, `sets` included.
A refused vector's `expect.refused` names the shape, text or writer refusal from `protocol/record.md`.
The `shape-*` vectors give envelopes as written: one that is not an object (`envelope-not-object`), a required field left out, or a field of the wrong JSON type, which gets that field's own code.
A `salt` left out, not text, or not canonical base32hex refuses with `salt-not-16-bytes`.
`payload_text` preserves repeated member names in JSON text.
Use a parser that detects repeated names.
`payload_text` also carries a lone surrogate as a `\u` escape, which a vector cannot hold otherwise.
Such text refuses with `string-not-unicode` after raw shape and before every value check.
A `payload_text` that is not JSON is malformed input, not a refusal.
A number such as `1e999`, in `payloads` or in `payload_text`, is past binary64's range.
It refuses with `number-out-of-range` where the writer checks numbers.

The first envelope and the first payload decide the answer.
If writing them is refused, `expect.refused` is that refusal, whatever the other variants are.
Once they are written, every other variant must name the same record and every other payload must be writable.
An input that breaks this is malformed: no implementation answers it.
`seed` is read only once the record is written.
It is exactly 32 bytes; a seed of another length is malformed input, never padded or cut.
This seed rule belongs to these fixtures, not to the record protocol.
Writers cap every number's magnitude at 2^53 − 1, whole or not.
Numeric readers separately require safe whole values.
JS and Rust sums use unbounded integers; future bounded ports must refuse overflow.

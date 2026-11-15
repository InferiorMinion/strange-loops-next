# Layer 0: the record

A record is an observation. Records are immutable, the only write is
append, and a store is a set of records. This page is normative; the laws
it relies on are in [`laws.md`](laws.md) and the bytes are pinned by the
vectors in `vectors/record/`.

## Envelope

| Field | Required | Holds |
|---|---|---|
| `kind` | yes | a label, non-empty |
| `observer` | yes | a typed reference (`prefix:value`), usually a lineage id `sl:…` |
| `emitted` | yes | the observer's now: a canonical UTC instant |
| `occurred` | no | the claimed past: a span, below |
| `about` | no | a set of typed references: the things it mentions |
| `of` | no | a typed reference: the one thing it is an emit of |
| `prev` | no | a set of record ids this one replaces |
| `under` | no | the record id of the grant it acts on |
| `erases` | no | a set of record ids whose payloads this one erases |
| `sets` | no | a set of key names: the top-level payload keys this one sets |

* A **typed reference** is `prefix:value`.
  The prefix starts with a lower-case letter and holds lower-case letters, digits and hyphens.
  The value is not empty.
* A **record id** is `r:` and 52 canonical base32hex characters.
* **`emitted`** is `YYYY-MM-DDTHH:MM:SS`, an optional fraction, then `Z`.
  Use upper-case `T` and `Z`, a real calendar day, and a year from 0000 to 9999.
  Hours range from 00 to 23, and minutes and seconds from 00 to 59.
  A fraction is `.` followed by 1 to 9 digits, with a nonzero last digit.
  Numeric offsets are refused. A writer can put its local offset in the payload.
* **`occurred`** is a span of time. Its grammar is below.
* **Sets** (`about`, `prev`, `erases`, `sets`) have one canonical form: sorted by UTF-16 code
  units, repeats removed, and left out when empty.
  `sets` is the one exception: it is left out only when absent, and an empty one is written `"sets": []`.
* **`of`** holds one typed reference, a record id or a natural key such as `task:42`.
  Like `occurred` and `under`, it is left out when absent, so a record without it keeps its bytes and id.
  Only a root leaves it out: a root has no `of` and no `prev`.
  A record with `prev` states `of` (`of-missing`).
* **`sets`** names the top-level payload keys a record sets.
  A **key name** is non-empty text.
  Absent and empty differ: a record without `sets` is read whole, and one with `"sets": []` sets nothing, a pure join.
  They are different records, with different ids.
  Nothing compares `sets` with the payload: a declared key the payload lacks is written, and so is a payload that is not an object beside a non-empty `sets`.
  The envelope stays checkable without its payload.
  A root or a revision may declare any keys; nothing compares one record's `sets` with another's.
  A key that starts with `@` is a key, not a label.
  A latest-by that follows `keys` reads it ([`spec.md`](spec.md)).

The envelope reader refuses fields outside this table.
Fields the core does not read belong in the payload; otherwise the id would silently omit them.

The envelope reader uses these refusal codes: `envelope-field-unknown`, `envelope-not-object`,
`kind-empty`, `observer-not-typed`, `emitted-not-rfc3339`, `occurred-not-span`,
`about-not-typed`, `of-not-typed`, `prev-not-record-id`, `under-not-record-id`,
`erases-not-record-id`, `sets-not-keys`, `of-missing`.
These are the only codes for an envelope.
It checks them in that order.
`of-missing` comes last: it reads `prev` and `of` together, after each has its own check.
Raw shape comes first: inspect all envelope keys before decoding any envelope value, label or salt.
An unknown key gives one top-level `envelope-field-unknown` refusal, including when another envelope is not an object or has malformed values.
Then an envelope that is not an object gives `envelope-not-object`, whatever the other envelopes hold.
A required field left out, or any field whose value has the wrong JSON type, is refused with that field's own code, in that field's place:

| Field | Left out | Wrong JSON type |
|---|---|---|
| `kind` | `kind-empty` | `kind-empty` (not text) |
| `observer` | `observer-not-typed` | `observer-not-typed` (not text) |
| `emitted` | `emitted-not-rfc3339` | `emitted-not-rfc3339` (not text) |
| `occurred` | allowed | `occurred-not-span` (not text) |
| `about` | allowed | `about-not-typed` (not an array, or a member not text) |
| `of` | allowed | `of-not-typed` (not text) |
| `prev` | allowed | `prev-not-record-id` (not an array, or a member not text) |
| `under` | allowed | `under-not-record-id` (not text) |
| `erases` | allowed | `erases-not-record-id` (not an array, or a member not text) |
| `sets` | allowed | `sets-not-keys` (not an array, or a member not text) |

`sets-not-keys` also refuses a member that is the empty string, which names no key.
`null` is a value of the wrong type, not a field left out.
So `{"observer": 7}` is refused with `kind-empty`, and `{"kind": "note", "observer": 7, "emitted": 7}` with `observer-not-typed`.
Text comes second: every string and member name is Unicode scalar values.
A lone UTF-16 surrogate, in a value or a member name, is refused with `string-not-unicode`.
This check follows raw shape and precedes every check from `kind-empty` on, including the salt.
For a batch, the stages are shape, then labels, then writing, then `prev-unknown`, then `under-unknown`, then `of-not-root`, then `prev-unrelated`.
A batch given as values that can hold a lone surrogate checks text after shape and before labels.
The typed writer starts at `kind-empty`; its envelope type cannot contain unknown fields.
Several equivalent envelope variants remain valid when every variant passes shape checks.

The reader settles forms before admission checks `prev`.
Admission refuses a record when a held `prev` target is of a different thing (`prev-unrelated`).
A reader does not check `of-missing` or roots on what it holds.
A record with `prev` and no `of` is of its own id, so a held `prev` target is of another thing and admission refuses the record.
A held record applies `erases` even when admission refuses its `prev`.
The reader applies `erases` without reading the eraser's payload.
This rule does not establish signature validity or authority.
A lone erased form settles as withheld unless a held record names its id in `erases`.
The reader keeps its envelope and does not restore its payload.
Write settled, refused, and admitted record arrays in ascending id order.
Compare ids by unsigned digest bytes, first byte first.
Canonical id text has the same order as digest bytes (law `id.text-order`).
This array order does not change head content order.

Any record may erase, whatever its kind.
The envelope field alone carries the meaning.
Every reader applies it without needing the eraser's payload. A revision can erase what it
replaces (`of: r:X, prev: [r:X], erases: [r:X]`).
Targets don't go in `about`: `about: [r:X]` only mentions X.

Writing checks the envelope, then salt length (`salt-not-16-bytes`) and repeated payload members (`member-duplicate`).
A salt left out, of another JSON type, or not canonical base32hex text is not 16 bytes, and is refused the same way.
It then checks nonfinite or out-of-range payload numbers (`number-out-of-range`).
Every number must have magnitude at most 2^53 − 1, whole or not.
A number spelling past binary64's range is such a number: it stays a number until this last check.
No reader refuses it earlier or under another name.
A batch writes each record through the same checks.
A draft in a batch may name another draft by its label in `about`, `of`, `prev` or `under`; never in `erases`, and `sets` holds keys, not labels.
A label in `under` takes part in cycles as the others do, so a grant and a record acting under it can be written together.
A draft's envelope that is not an object, or a draft without one, is refused with `envelope-not-object` in the shape stage.
A draft that is not an object, an unknown draft member, a label that is not text and a draft without a payload are malformed input, with no refusal code.
The shape stage reads the envelopes of every draft that is an object and keeps priority: a batch is malformed input only when no envelope refuses there.
The writer then checks what each record names, across the whole batch.
A `prev` target must be in the batch or the store (`prev-unknown`).
An `under` target must be in the batch or the store too (`under-unknown`).
A record of the batch counts, named by its label or by its id.
A stored record counts whatever its shown state: the writer holds an erased or withheld record's envelope.
Nothing else about an `under` target is checked: whether it names a grant, of what kind and by whom, is standing's question.
An `of` that names a record the writer holds, in the batch or the store, must name a root (`of-not-root`).
A `prev` target must be of the record's thing (`prev-unrelated`).
A reader checks only the last, against the records it holds, as admission above.
No reader refuses an unheld `prev` or `under` target.

The whole order, for a record given as an envelope, a salt and a payload:
`envelope-field-unknown`, `envelope-not-object`, `string-not-unicode`, the envelope codes from `kind-empty`,
`salt-not-16-bytes`, `member-duplicate`, `number-out-of-range`.
A repeated envelope member name has no code: it is malformed input.

## Things

A record is an emit of exactly one thing.
That thing is what `of` names, read as written, or, without `of`, the record's own id.
A root starts the thing named by its id; a revision of it states that id in `of`.
A natural key in `of` names a thing before any record of it is held.
Two writers who emit of the same key emit of one thing.
A reader never resolves `of` through another record, so a record's thing is a fact of its own envelope.
`latest-by`, per-thing state, admission and whole-view heads read this thing.

`about` names the things a record mentions.
It never makes a head and never says what a record is an emit of.
`where` and `collect` read it, so everything said about a thing is still a fold.
A link between two things is a record whose `about` names both.

## Occurred

`occurred` is the observer's claim about when something happened: a span,
possibly coarse. It is one of:

| Form | Example | Span |
|---|---|---|
| a calendar date: `YYYY`, `YYYY-MM` or `YYYY-MM-DD` | `2009`, `2009-07`, `2009-07-14` | the whole year, month or day |
| a date with its last digits unspecified: `YYYX`, `YYXX`, `YYYY-XX`, `YYYY-MM-XX` or `YYYY-XX-XX`, and no other shape | `201X`, `20XX`, `2004-XX`, `1985-04-XX`, `1985-XX-XX` | the decade (`YYYX`), century (`YYXX`), year (`YYYY-XX`, `YYYY-XX-XX`) or month (`YYYY-MM-XX`) |
| either, then one qualifier | `2009?` (uncertain), `2009~` (approximate), `2009%` (both) | the date's own span |
| an instant, as `emitted` writes one | `2026-10-17T09:14:03Z` | a span of no length |
| `a/b`, two of those | `2009-07/2011-03`, `2026-10-17T09:14:03Z/2026-10-17T09:18:13Z` | from the start of `a` to the end of `b` |
| `a/..` | `2019/..` | from the start of `a`, with no end yet |

* Calendar dates are UTC days of the proleptic Gregorian calendar, years
  0000 to 9999. A date's span starts at `00:00:00Z` on its first day.
  It ends at `00:00:00Z` on the day after its last.
  `2009` runs from `2009-01-01T00:00:00Z` to `2010-01-01T00:00:00Z`, where `2010` starts.
  Unspecified digits follow EDTF Level 1, in exactly the five shapes above.
* A qualifier is part of the claim and stays in the text; it never changes
  the span. Instants take no qualifier.
* In `a/b`, `b` may not start before `a` does. The span ends where `b` ends, even when that is before `a` ends.
  `2009/2009-06` runs from the start of 2009 to the end of June 2009.
* The text retains the precision and qualifiers of the claim.
  For example, `2009` and `2009-01/2009-12` are different claims.
* **Order.** Spans order by start, then end. A span of no length comes before a longer one with the same start.
  A span with no end comes after any with an end. Content order compares a record's span, then its `emitted`, then its id.
  A record without `occurred` is an instant at its `emitted`.
  Ids compare as their 32 unsigned digest bytes, first byte first.
  Canonical id text has the same order (law `id.text-order`).
  Newest first reverses the whole order, so a later span, then a later
  `emitted`, then a greater digest comes first.
* **Moments.** An instant's moment is its whole seconds in UTC, then its fraction.
  Compare fractions exactly. Each instant in `occurred` follows the same
  canonical rules as `emitted`, whether alone or in an interval.

The forms come from ISO 8601 and EDTF Level 1 (ISO 8601-2).
This page gives the whole grammar.
It excludes durations, seasons, open or unknown starts, unknown ends and years past 9999.
Instead of a duration, write the end instant: `start/end`. Anything else is
refused with `occurred-not-span`. Accepting more later is free, since
every record valid now stays valid with the same id; accepting less would
invalidate records already stored.

## Canonical bytes

RFC 8785 (JCS): members sorted by UTF-16 code units, minimal string escapes,
ECMAScript number formatting, no whitespace. Object member names must be distinct at every depth.
The writer refuses repeated names (`member-duplicate`).
It does not select the first or last value; parsers disagree on that choice.

Strings and member names are sequences of Unicode scalar values, as RFC 8785 requires.
A lone surrogate names no scalar value, so it has no canonical bytes and is never hashed.
The writer refuses it with `string-not-unicode`; it substitutes nothing, U+FFFD included.
This holds for a payload given as text, where a lone surrogate is written as a `\u` escape,
and for one given as values in a language whose strings can hold one.
It covers member names, every envelope field, and what a batch holds.
A payload text is refused this way only when it is JSON by its syntax.
Text that is not JSON is malformed input and has no refusal code, wherever a surrogate stands in it.
Of JSON text, a lone surrogate is refused before a repeated member name: text is decoded, then validated.
A paired escape, such as `\ud83d\ude00`, is the character it names.

A payload can contain RFC 8259 number spellings, including fractions and exponents.
JCS reads each number as an IEEE-754 binary64 value.
The writer accepts finite values only within ±(2^53 − 1), including fractions.
Numeric folds have a separate safe-whole rule in [`spec.md`](spec.md).

Number spellings round once to binary64 before canonicalization.
For example, `9007199254740993` becomes `9007199254740992`; `9007199254740991.1` becomes `9007199254740991`.
The first is refused as out of range; the second is accepted after rounding.
Numbers needing exact decimal or large-integer precision belong in strings.
Numeric readers additionally require whole values; within-range fractions remain writable but unread by numeric folds.

The model also permits raw integers, before binary64 ingestion.
Its writer refuses raw integers outside ±(2^53 − 1).
This prevents unchecked exact values from bypassing binary64 semantics.
These raw integers are not distinct wire values.
Canon's finite number domain is distinct from the writer's magnitude cap.

A spelling such as `1e999` is past binary64's range.
The writer's number check refuses it with `number-out-of-range`, in that check's usual place:
after the envelope, the salt and repeated member names (law `write.overflow`).
A spec refuses it where it refuses any number out of range.
An implementation whose number type is finite keeps it finite.
Its input adapter carries the overflow to that check; it does not refuse earlier or under another name.
A reader's own records must hold finite numbers: there an overflowing spelling is malformed input.
The model's checked writer also refuses out-of-range raw numbers.
Historical ingestion discussions do not override the magnitude cap.

## Commitment, id, signature

```
commit = "h:" + base32hex(SHA-256(salt ‖ JCS(payload)))
id     = "r:" + base32hex(SHA-256(JCS({"commit": commit, "env": env})))
signed = "loops/record/v1:" + id            (UTF-8)
sig    = { "key": "k:" + base32hex(public key), "ed25519": base32hex(Ed25519(signed)) }
```

* `base32hex` is RFC 4648 section 7 in lower case without `=` padding.
  Its alphabet is `0123456789abcdefghijklmnopqrstuv`.
  All byte strings written as text use this encoding, including salts.
  Readers refuse upper case, characters outside the alphabet, wrong lengths and nonzero final pad bits.
  A 16-byte value has 26 characters and 2 pad bits.
  A 32-byte value has 52 characters and 4 pad bits.
  A 64-byte value has 103 characters and 3 pad bits.
  A malformed value uses the field's existing refusal code, as does a field left out or of the wrong JSON type.
* The **salt** is 16 random bytes; it stops anyone confirming a guess about a
  payload from its commitment.
* The **payload** is any JSON value.
* The id covers the commitment, not the payload, so **erasing** the opening
  never changes the id. The signature covers the id, so it survives erasure
  too.

## Stored lines

A store holds each record as one canonical JSON line:

```
{"env":{…},"id":"r:…","payload":…,"salt":"…","sig":{"ed25519":"…","key":"k:…"}}
```

After erasure the opening is replaced by the commitment:

```
{"commit":"h:…","env":{…},"id":"r:…","sig":{"ed25519":"…","key":"k:…"}}
```

## Future work and implemented boundaries

* Windows arrive with ticks and hold records whose span starts in them.
  The start is included and the end excluded.
  A `where` test on `env.occurred` compares starts too.
  A record without `occurred` passes neither bound; a window places it at its `emitted`.
* Key bindings, presence keys and admission arrive with standing (step 3).

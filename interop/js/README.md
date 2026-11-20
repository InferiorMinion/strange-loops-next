# JavaScript port

`loops.mjs` implements the record writer and raw envelope shape checks.
`batch.mjs` checks every draft's envelope shape before labels or values.
The vector harness calls these library boundaries; it does not define the key policy.

## Shape

Raw shape is an unknown key in any envelope (`envelope-field-unknown`), then any envelope that is not an object (`envelope-not-object`).
A draft object without an `env` has an envelope that is not an object.
`readEnv` then reads a field left out or of the wrong JSON type as empty text, so the writer refuses it with the field's own code, as the model does.
`readSalt` reads a salt that is not base32hex text as no bytes, which refuses as `salt-not-16-bytes`.
A batch draft that is not an object, has a label that is not text or has no payload is malformed input: it throws.
A label may stand in `about`, `of`, `prev` or `under`; never in `erases`.
`sets` holds key names; a key that starts with `@` is not a label.

## Things

A record is of one thing: its `of`, read as written, or without `of` its own id (`fold.mjs`, `thingOf`).
Heads, admission (`realm.mjs`), the writer's `prev` checks (`batch.mjs`) and a spec's `latest-by` and `from.of` all read that.
`about` only mentions; `from.about` and `env.about` test it as written, and it never makes a head.
A latest-by that follows `keys` reads each thing by its live setters (`fold.mjs`, `above`, `liveFor`, `live`, `keyedOf`), over the records of the thing that were admitted.
`from` and the envelope tests choose heads; `thingAnswer` (`spec.mjs`) asks each payload test of every live setter of its field.
Count, sum, min, max and collect that follow `keys` read those things (`keyedThings`, `overThings`): a field from its one live setter, and with `by` a payload key from the field's live setters and an envelope key from the chosen heads (`thingKeys`).

## Raw text

A JavaScript object cannot retain repeated JSON members.
The caller must retain every repeated `env` member at the raw-text boundary.
The fuzz adapter re-reads raw text and passes those occurrences to the library shape check.
This rule has adapter tests, but protocol vectors do not cover it.
This handling does not add a duplicate-member refusal for envelopes.

A repeated member in a draft or an envelope, two `env` members included, is malformed input in all three, after raw shape.
JavaScript sees it only where it holds the raw text: the fuzz adapter (`interop/fuzz/evaluate.mjs`) checks it there, for batch and record inputs.
The value APIs `resolveBatch`, `writeRecord`, `seal` and `writeText` take JavaScript values and have no raw path: given a value parsed by `JSON.parse`, they see only the last of a repeated member and may accept the input.
The vector checkers (`check.mjs`, `check-batch.mjs`) use these value APIs; no vector carries a repeated envelope or draft member, since a vector's JSON can't be relied on to keep one.

## Record inputs

`writeRecord` reads a record input as the vectors give one.
Its first envelope and first payload decide the answer.
Once those are written, every other variant must be writable and name the same record.
An input that breaks this throws an error that is not a refusal code.
The seed is read last and is exactly 32 bytes.
A caller that kept each payload's raw text passes it as `payloadTexts`, so a repeated member is seen.

## Reading held records

The readers (`runSpec`, `settle` and `realm`, the folds) check every held record's times before they answer, as the Lean and Rust readers do when they decode one (`checkTimes`, `fold.mjs`).
`emitted` must be text that is a canonical UTC instant, and `occurred`, when present, text in its grammar.
The first record in the list that fails, `emitted` before `occurred`, throws the writer's code, `emitted-not-rfc3339` or `occurred-not-span`.
The error's `path` names the field in the list given, as Lean's decoder does, such as `[1].occurred`; `settle` and `realm` name the forms as given.
A record is refused whether or not the reader would order it.
These readers check no other field of a held record.

A spec's slots are answered in the order the spec writes them, as in the model.
An object's own enumeration puts integer-like names first, so the parsed `fold` and the answer `runSpec` builds keep the written order beside their members, read by `slotNames`.
`slotsText` writes the slots in that order, and the keys of `groups`, of `things` and of a keyed thing's `keys` in UTF-16 code-unit order (`jsonText`).
A spec given as a value has only its enumeration order; read it from text with `parseSpecText` to keep the written one.
`check-spec.mjs` checks the member order of `spec/run` answers, and `check-fold.mjs` that of `fold/latest-by` answers.

## The canonical result

`result.mjs` writes a `runSpec` answer as the canonical result of [`protocol/result.md`](../../protocol/result.md), and hashes it.
It reads the answer's shape from its member names and writes ids alone, without `shown`.
An integer becomes its decimal text through `BigInt`: a `Number` is taken only while it is a safe integer, and a `BigInt` sum past 2^53 keeps every digit; no integer goes through `JSON.stringify`.
`canonical` then writes the JCS text, so member order is its sort, which is UTF-16 code-unit order.
`check-result.mjs` checks every `result` vector in every arrival order, and reads each expected text back: JCS of itself, no JSON number in it, an integer text at `value`, `unread` and `ungrouped` alone.

## Text

`seal`, `writeText`, `canonical` and `resolveBatch` refuse a lone surrogate with `string-not-unicode`.
The check follows the raw key check and precedes every value check.
`readVector` refuses a vector that holds one anywhere, ignored members included.
`lens.edit` is the public lens boundary; it refuses a base that repeats an id first.

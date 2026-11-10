# JavaScript port

`loops.mjs` implements the record writer and raw envelope shape checks.
`batch.mjs` checks every draft's envelope keys before labels or values.
The vector harness calls these library boundaries; it does not define the key policy.

## Things

A record is of one thing: its `of`, read as written, or without `of` its own id (`fold.mjs`, `thingOf`).
Heads, admission (`realm.mjs`), the writer's `prev` checks (`batch.mjs`) and a spec's `latest-by` and `from.of` all read that.
`about` only mentions; `from.about` and `env.about` test it as written, and it never makes a head.

## Raw text

A JavaScript object cannot retain repeated JSON members.
The caller must retain every repeated `env` member at the raw-text boundary.
The fuzz adapter re-reads raw text and passes those occurrences to the library shape check.
This rule has adapter tests, but protocol vectors do not cover it.
This handling does not add a duplicate-member refusal for envelopes.

## Record inputs

`writeRecord` reads a record input as the vectors give one.
Its first envelope and first payload decide the answer.
Once those are written, every other variant must be writable and name the same record.
An input that breaks this throws an error that is not a refusal code.
The seed is read last and is exactly 32 bytes.
A caller that kept each payload's raw text passes it as `payloadTexts`, so a repeated member is seen.

## Text

`seal`, `writeText`, `canonical` and `resolveBatch` refuse a lone surrogate with `string-not-unicode`.
The check follows the raw key check and precedes every value check.
`readVector` refuses a vector that holds one anywhere, ignored members included.
`lens.edit` is the public lens boundary; it refuses a base that repeats an id first.

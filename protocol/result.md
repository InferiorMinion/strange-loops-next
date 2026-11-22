# The canonical result

A vertex's result is what its slots hold after a run over the records a reader holds.
A reader prints a reading of it: each listed record with what the reader holds of it, and integers as JSON numbers.
The canonical result is the bytes that stand for the result, and its hash names it.
This page is normative: it is the codec `resultText` and `resultHash` (`model/Loops/Result.lean`) implement, version 1.
The vectors in `vectors/result/` pin it, and [`laws.md`](laws.md) states what is proved of it.
It changes no reading: the CLI and the `spec/run` vectors still print readings.

## What is written

The canonical result is one JSON object.
Its members are the spec's slots, every one, each under its name.
There is no member for the vertex, the spec or the records.

Each slot's value has one of these shapes.
The slot's fold type and `follow` decide which, and `by` puts it in groups.

| Slot | Value |
|---|---|
| `count`, `sum`, `min`, `max` | `{"value": <integer or null>, "unread": <integer>}` |
| `collect` | `{"heads": [<id>, …], "unread": <integer>}` |
| `latest-by` following `prev` | `{"things": {<thing>: [<id>, …]}, "unread": <integer>}` |
| `latest-by` following `keys` | `{"things": {<thing>: {"heads": [<id>, …], "keys": {<key>: [<id>, …]}, "whole": [<id>, …]}}, "unread": <integer>}` |
| a slot with `by` | `{"groups": {<group key>: <value>}, "ungrouped": <integer>, "unread": <integer>}` |

`count`, `sum`, `min`, `max` and `collect` keep their shape when they follow `prev` or `keys`.
In a slot with `by`, each group's value is what the slot holds without `by`.
Each member holds what [`spec.md`](spec.md) says the slot holds: the same things, keys and groups, the same ids in the same order, the same counts.
A latest-by that follows `keys` lists under `keys` each key a live record declares, under `whole` the live whole-read emits, and under `heads` the heads.

## Integers

An integer is a JSON string of its decimal digits.

* The digits are `0` to `9`, the most significant first.
* Zero is `"0"`. No other integer starts with `0`.
* A negative integer has `-` before its digits. Nothing else has: no `+`, and never `-0`.
* Nothing else is in the string: no space, fraction, exponent or other sign.

Each integer has exactly one text, and a reader of the codec accepts no other.
An integer is never a JSON number, so it is never rounded to binary64 and never capped.
A `sum` past 2^53 keeps every digit: two records holding 9007199254740991 and 2 sum to `"9007199254740993"`.
The writer's range bounds what a record holds, not what a slot computes.

An integer stands at `value`, `unread` and `ungrouped`, and only there.
`value` is `null` for a `min` or `max` that read nothing.
A `count` or a `sum` that read nothing is `"0"`.
These members hold nothing else, and no other member holds an integer.
So the position alone tells an integer from text, and no tag is written.
A thing, a group key and a payload key are text, even where they read as a number: the group key of the number 1 is the text `"1"`.

## Ids

A listed record is written as its id text alone, `r:` and base32hex ([`record.md`](record.md)).
What the reader holds of it, `shown`, is not written.
So a reader that holds a record's payload, one that has it withheld, and one that had it erased write the same bytes wherever they list the same ids.
A value read from a payload can still differ between them: a payload test, a `sum`, a `min` or `max`, a `by` on a payload field.

A list keeps the slot's order: newest first in content order.
The order is part of the result.

## Bytes

The bytes are the JCS text ([RFC 8785](https://www.rfc-editor.org/rfc/rfc8785)) of that object, in UTF-8.

* JCS writes every object's members in the order of the UTF-16 code units of their names.
  So the slots are in that order, not the spec's.
  The fixed members are in it too: `unread`, `value`; `heads`, `unread`; `things`, `unread`; `groups`, `ungrouped`, `unread`; and in a keyed thing `heads`, `keys`, `whole`.
* A map by thing, group key or payload key is already in that order in every reading, so JCS keeps the order the slot writes.
* Arrays are not reordered.
* There is no whitespace, and strings are escaped as JCS escapes them.

Every value in the object is a string, `null`, an array or an object, so JCS's rule for numbers is never used.
A spec names each slot once, so no member name repeats.

## Reading a claimed result

The reader takes a parsed spec and a JSON value. It requires exactly the spec's named slots and the members of each slot's shape above. Every object has distinct member names. Thing names, group keys and payload keys are text. Every listed id must have the canonical spelling of a 32-byte record id. The reader preserves array order and multiplicity. It reconstructs slots in spec order and assigns each listed id the model's `here` value for the omitted `shown`.

All integers use the decimal spelling above. Counts, `unread` and `ungrouped` must be nonnegative. Only a minimum or maximum accepts `null` at `value`. These checks apply inside groups as well. Reading does not reconcile counts with lists, check membership in a covered set, enforce a slot's limit, or test the order, uniqueness or agreement of listed ids. A claim with such a disagreement still reads. Comparing it with a run is a separate operation.

## The hash

```
preimage = "loops/result-hash/v1:" ‖ bytes        (the label in UTF-8)
hash     = "res:" + base32hex(SHA-256(preimage))
```

The label names the codec and its version, as `loops/record/v1:` frames the id a signature covers.
A codec that writes other bytes for some result takes another label.
The hash covers the whole result, every slot; there is no hash for one slot.

The label is 21 bytes, and it must not be 16.
A record's commitment is SHA-256 of a 16-byte salt and then its payload's JCS bytes ([`record.md`](record.md)).
The two also differ in their prefix. A result hash starts with `res:` and a commitment starts with `h:`.
With a 16-byte label, a record whose salt is the label and whose payload is the canonical result would commit to the result's own preimage, and the two would be one digest.
With this label, the preimage's 17th byte is `h`, and no JCS text starts with `h`: it starts with `n`, `t`, `f`, `"`, `[`, `{`, `-` or a digit.
So no split of the preimage after 16 bytes leaves a JSON value, and no commitment has a result hash's preimage (`resultPreimage_ne_commit`).
A later label must keep this.

## Outside the codec

* The reading: `{"id", "shown"}` entries and JSON numbers, as the CLI prints and `spec/run` pins.
  A reading is not a source for the canonical result. It writes an integer as a JSON number, and a reader that takes JSON numbers as binary64 loses digits past 2^53: the reading of `phase2-exact-output` holds 9007199254740993. The canonical result is computed from the run, which holds every digit.
* `--values`, which prints payload values beside ids. It is a view of the CLI. No canonical result holds a payload value, a fraction or a boolean; if one ever must, a position would hold text or a number, and the codec would need a tag and layer 0's number rule.
* A tick, and whatever it records beside a result. The codec turns a result into bytes and a hash, and nothing more.

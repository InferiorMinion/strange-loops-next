# The spec

A vertex's spec is JSON.
`from` chooses records by their envelopes.
`fold` names slots, each with one fold type and an optional `where`. This page is normative: it is the grammar `Spec.ofJson`
(`model/Loops/Spec.lean`) implements. What the slots then hold is in
[`laws.md`](laws.md), and the vectors in `vectors/spec/` pin both.
A vertex's whole result as canonical bytes, and its hash, are in [`result.md`](result.md).

## The spec

| Key | Required | Holds |
|---|---|---|
| `vertex` | yes | the vertex's name: text, not empty |
| `from` | yes | an array of at least one source |
| `fold` | no | an object of slots by name; without it, the spec has none |
| `note`, `ticks`, `order`, `resolve`, `window`, `boundary`, `param`, `covers`, `into`, `bundle` | no | any value: carried, not yet read |

Any other key is refused. So is a key repeated in any object anywhere in the
spec, as layer 0 refuses one in a payload.

## Sources

| Key | Required | Holds |
|---|---|---|
| `store` | yes | the store, as the realm names it: text, not empty |
| `kind` | no | the kinds taken: an array of at least one, each text that isn't empty |
| `about` | no | a thing mentioned: a typed reference (`prefix:value`) |
| `of` | no | the thing taken: a typed reference |

A source checks kind, mention and thing.
A supplied `kind` must include the record's kind.
A supplied `about` must be in the record's `about`, as written.
A supplied `of` must be the record's thing.
A source with both takes the records that are of its `of` and mention its `about`.
The spec takes a record when any of its sources does.

For direct spec evaluation, the caller resolves store names outside the runner.
The runner receives records and does not look up stores.
Each source tests only `kind`, `about` and `of` on those records.
The direct spec and scenario vectors do not test store resolution, missing stores, or per-source routing.

A record's thing is what its `of` names, or without `of` its own id ([`record.md`](record.md)).
Thus `"of": "r:X"` takes the root X and every record of X.
(`env.of` in `where` reads the field as written, which is absent on X.)
`"about": "r:X"` takes the records that mention X, and not X itself unless it mentions itself.

## Slots

A slot's name is text that isn't empty. A slot is an object with exactly one
fold type:

| Fold type | Argument |
|---|---|
| `count` | `true` |
| `sum`, `min`, `max` | the payload field it reads: text, not empty |
| `latest-by` | `"of"`; any other argument, `"about"` included, is refused |
| `collect` | `true` |

and optionally:

| Key | Holds | On |
|---|---|---|
| `follow` | `"prev"` or `"keys"` | any; always written on `latest-by` |
| `limit` | a whole number, at least 1 | `collect` |
| `by` | what to group by, below | `count`, `sum`, `min`, `max`, `collect`, following or not |
| `where` | a `where` map, below | any |
| `note` | any value: carried, not read | any |

Any other key is refused.

Settle forms before admission, then run heads and all slots over admitted records.
For each slot, count each unread record id once, or for a slot that follows `keys` each unread thing once.
For `collect`, count unread candidates before the limit.
Apply the limit only to the ordered list of matching records; for a `collect` that follows `keys`, the limit counts things, each listed whole (below).

A record belongs to exactly one thing, its `of` or its own id.
`about` plays no part in heads.
Compute heads separately for each thing, using only its records.
For a whole-reader view, combine these heads and keep each id once.
`latest` selects the newest head in this combined view as its winner.
It returns the other heads in content order.
It does not use global succession state.

Without `follow`, `count`, `sum`, `min`, `max` and `collect` read every record `from` takes, superseded or not.
With `"follow": "prev"` they read only the current records: the heads of the whole-reader view above, which are the heads `latest-by` reads.
Their result keeps its shape: `count`, `sum`, `min` and `max` still give a `value` and `unread`.
A slot over heads selects as `latest-by` does.
`from` and the envelope tests choose heads; the payload tests then filter them.
A chosen head whose payload tests answer unknown is unread, once.
Every head of a forked thing is current, so each one counts.
An erased or withheld head is still current, and only its envelope is held.
`count` counts it, since counting reads no payload; a payload test on it answers unknown, and the head is unread.
`sum`, `min` and `max` cannot read its field, so it is unread to them.

## Following `keys`

`"follow": "keys"` reads each thing by its live setters, where `"follow": "prev"` reads it by its heads.
On `latest-by` it gives each thing's keyed state; on `count`, `sum`, `min`, `max` and `collect` the unit is the thing (below).
Slots without `follow`, and slots that follow `prev`, read records as written, each from its own payload.

A record *sets* key `k` when `k` is in its `sets`, or when it has no `sets`: a record read whole sets every key.
`"sets": []` sets no key; such a record is a pure join.
A record is *above* another when its `prev` names it, or names a record above it, through the admitted records of their thing the reader holds.
A `prev` naming a record the reader does not hold reaches nothing.
A setter of `k` is *live for `k`* when no record above it sets `k`.
Every record sets the envelope, so a record is live for the envelope when nothing is above it: it is a head.
A record is *live* when it is live for some key, the envelope among them.
So every head is live, and a record read whole is live until a record read whole is above it.
An erased or withheld record keeps its envelope, `sets` included: it still sets its keys, and only its `shown` changes.
Liveness is worked out where heads are: after settling and admission, per thing, over everything held, before any slot selects.

The slot holds `{"things": {<thing>: {"keys": {<key>: [...]}, "whole": [...], "heads": [...]}}, "unread": n}`.
Each list holds `{"id", "shown"}` entries, newest first in content order.
`keys` has an entry for every key some live record of the thing declares in its `sets`, and under it every live setter of that key, records read whole included.
`whole` lists the live records read whole. A key with no entry is read from them.
`heads` lists the thing's heads, as `"follow": "prev"` gives them; a pure-join head is listed there and nowhere else.
`keys` is written in UTF-16 code-unit order of its key names.
More than one setter under a key is two live setters of it; the core declares no conflict and compares no values.
For a thing whose records are all read whole, `keys` is empty and `whole` and `heads` are its heads.

`from` and the envelope tests choose heads, each head alone, as under `"follow": "prev"`.
A thing is selected when one of its heads is chosen.
The payload tests are then asked of the thing: each test on a payload field is asked of every live setter of that field, whether or not `from` takes it.
A record with `sets` is asked only about the keys it declares; a payload key it does not declare is not read.
When every live setter gives the same answer, that is the test's answer.
When they give different answers, or one of them is erased or withheld, the test answers unknown.
When no record of the thing sets the field, it is absent, as a missing field in a held payload is: `is`, `present`, `at-least` and `below` answer no, and `not` and `absent` answer yes.
The tests then combine as a `where` map does.
A selected thing whose tests answer yes is in `things` with its whole keyed state, every head listed whether chosen or not.
One whose tests answer unknown adds one to `unread`.

### Count, sum, min, max and collect following `keys`

These slots select things as `latest-by` following `keys` does, and keep the shape of the same slot without `follow`.
A selected thing whose tests answer unknown is unread, once; a thing is counted in `unread` at most once in a slot, whatever makes it unread.

- `count` counts the selected things whose tests answer yes. A thing forked across records read whole counts once here, and once per head under `"follow": "prev"`.
- `sum`, `min` and `max` read the field from its live setters, as a payload test does. With one live setter the field is read from it, by the rule below for a record's field. With none, or with two or more, it is unread, also when two hold the same number: no values are compared. So is a thing whose one live setter is erased or withheld, or holds no safe whole number in the field, as a record would be.
- `collect` lists the live records of the selected things, newest first in content order, a pure-join head among them, each with what is shown of it. `limit` counts things, not records: the list holds every live record of the first `limit` things and no record of the rest, so a thing is listed whole or not at all. The things are taken in the order of each one's newest live record, newest first in content order, the order of the list itself; content order ends in the record id, so no two things tie. The records of the things taken keep their place in the one newest-first list, so they are not grouped thing by thing, and the result keeps its shape. A thing whose tests are unknown is counted in `unread` before the limit and takes no place under it.

A slot that follows `keys` reads a payload field only from the records that set it, so it never reads a payload key a record with `sets` does not declare.

## `by`

`by` groups a slot's records by a key read from each record alone; following `keys`, it groups things (below).
It is text, spelled as `where` spells fields: `env.` names the envelope, and any other name is a payload field.

| `by` | A record's keys |
|---|---|
| `env.kind` | its kind |
| `env.observer` | its observer, the whole typed reference |
| `env.about:<prefix>` | each member of `about` that starts `<prefix>:`, without it (`project:loops` gives `loops`) |
| a payload field | the field's value, when it is text, a whole number or a boolean |

The prefix follows the typed-reference grammar: a lower-case letter, then lower-case letters, digits and hyphens.
A payload field's name is one `where` accepts, so the bare name `about:project` is a payload field.
A key is text. Text stays as written, a whole number is written in decimal (`-3`, `100`), and a boolean is `true` or `false`.
So the text `"1"` and the number 1 share the group `1`.
Any other value, such as `1.5`, has no key.

A slot with `by` holds `{"groups": {<key>: <result>}, "unread": n, "ungrouped": m}`.
Each `<result>` is what the slot holds without `by`, with its own `unread` (for example a `sum` over an unreadable value).
Groups are written in UTF-16 code-unit order of their keys, the order canonical JSON gives object members.

Grouping comes after settling, admission and, for a slot that follows `prev`, the heads of the whole-reader view.
These are worked out once, over everything held. A group never settles, admits or finds heads on its own.
Then the slot selects: `from` and `where`, as without `by`.
Then each selected record joins a group for each of its keys.
Each group is the slot without `by` or `where`, run over the records selected into it.
So `limit` on `collect` applies within each group, and a `collect` that follows `prev` lists in each group the current records selected into it, newest first.
A top-N across groups is not part of `by`.

- A record whose `where` answers no is in no group.
- A record whose `where` answers unknown adds to the top-level `unread`. So does a selected record whose key cannot be read: a payload field on a payload the reader does not hold.
- A selected record whose key is readably absent adds to `ungrouped`: it has no member with the prefix, or its payload lacks the field or holds a value with no key.
- A record with several keys, such as one mentioning two projects, is in each of those groups, once in each.

A group exists only where some record was selected into it, so there are no empty groups.
Every selected record (every record whose `where` answers yes or unknown) is grouped, ungrouped or unread, exactly one of these.
The groups hold each grouped record once per key, so the group sizes add up to memberships, not records.
A collect's `limit` then shortens each group's `heads`, so its lists can hold fewer than the memberships.
When every selected record has at most one key, the counts of a grouped `count`, its `ungrouped` and its `unread` add up to the `value` and `unread` of the same count without `by`.
Every `by` but `env.about:<prefix>` gives at most one key; `env.about:<prefix>` does too over records that each mention one thing under the prefix.

A slot with `by` that follows `keys` groups the things it selects, and its result has the same shape.
Settling, admission, the heads and liveness come first, over everything held.

- **A payload field:** the thing's keys are the keys of the field's live setters, each once. A live setter that is erased or withheld makes the key unread, and the thing adds one to `unread`. With no live setter, or none whose value has a key, the thing is ungrouped.
- **`env.kind`, `env.observer` or `env.about:<prefix>`:** the thing's keys are the keys of each head `from` and the envelope tests chose, each once. The envelope is read from the heads, so a later head moves a thing from one group to another, and a pure-join head settles two heads that mention different things. Such a key is never unread; with no key the thing is ungrouped.
- Each group is the slot without `by` over the things in it, so a forked thing is in each of its groups once, and a collect's `limit` counts the things in each group, each listed whole in each group it is in.

Every selected thing (every thing whose tests answer yes or unknown) is grouped, ungrouped or unread, exactly one of these.

Sum, min, and max read only safe whole values from the named payload field.
The names `value` and `n` follow the same rules as any payload field. In a held
payload an omitted field is absent to `where`, ungrouped under `by`, and unread
by a numeric slot. Written empty text and written zero are both present.
Read each number using the layer 0 binary64 rule first.
For each selected record, count an unavailable payload, missing field, or other value as unread.
Do not convert text or booleans to numbers.
The safe input range does not limit the sum accumulator.

## `where`

A `where` map is an object of at least one field, each naming an object of
at least one test: an operator and its argument. An envelope field starts `env.` and names `kind`, `observer`, `about`, `of`, `prev`, `under`, `erases`, `emitted` or `occurred`.
`sets` is not among them: no test reads it.
A payload field has a nonempty name that does not start `env.`.

| Operator | Argument |
|---|---|
| `is`, `not` | a value the field could hold, below |
| `present`, `absent` | `true` |
| `at-least`, `below` | a whole number on a payload field, a time on `env.emitted` or `env.occurred`; refused on any other field |

A value is what layer 0 would accept in the field, since nothing else can
match:

| Field | Value |
|---|---|
| `env.kind` | text, not empty |
| `env.observer`, `env.about`, `env.of` | a typed reference |
| `env.prev`, `env.under`, `env.erases` | a record id |
| `env.emitted`, `env.occurred` | a time in `occurred`'s grammar ([`record.md`](record.md)) |
| a payload field | text, a whole number, or a boolean |

[`laws.md`](laws.md) defines test meaning.
A map holds when every test holds.
A set field holds a value when a member equals that value.
`env.of` and `env.under` hold at most one value; on a root, `env.of` holds none.
Time tests read spans, not spellings.

For payload tests, read JSON numbers using the layer 0 binary64 rule.
A whole value has no fractional part and lies within ±(2^53 − 1), inclusive.
This rule applies to `is`, `not`, `at-least`, and `below`.
Thus `1.0` and `1e2` are whole values, but `1.5` and ±2^53 are not.

A `where` map answers no if any test answers no.
Otherwise, it answers unknown if any test answers unknown.
Otherwise, it answers yes.
Test order does not change the answer.
A missing field in a held payload is known absent.
For that field, `is`, `present`, `at-least`, and `below` answer no.
`not` and `absent` answer yes.
A payload field test answers unknown when the reader does not hold the payload.

## Refusals

A spec that breaks a rule is refused with a code and the path of what broke
it. A path joins keys from the spec root to the offending member with `.`.
An array adds nothing, so every source's path starts `from`.
Paths locate a refusal; they aren't unique, since a key may itself hold a
`.` (`fold.c.where.env.kind`).

Checks run in this order, and the first that fails is the refusal:

1. **A repeated key**, anywhere: `member-duplicate`, at the first repeat
   found depth first in member order. A member's value is searched before
   the members after it, and an array's elements in order, so
   `{"a": 1, "b": {"c": 1, "c": 2}, "a": 2}` is refused at `b.c`.
   Check each member's key before searching its value.
   If the key repeats, refuse at its path without searching its value.
2. **An object**: `spec-not-object`, at the empty path.
3. **Known keys**: `unknown-key`, at the first unknown key as written.
4. **`vertex`**: `vertex-missing` when it is left out, `vertex-invalid` when
   it isn't text or is empty; at `vertex`.
5. **`from`**: `from-missing` when it is absent or an empty array; `from-invalid`
   at `from` when it isn't an array. Check each source in order.
   Refuse a nonobject (`from-invalid` at `from`) or an unknown key (`unknown-key` at `from.<key>`).
   Then check `store`, `kind`, `about` and `of`, regardless of written order.
   These use `from-invalid` at `from.store`, `from.kind`, `from.about` and `from.of`.
6. **`fold`**: `fold-invalid` at `fold` when it isn't an object. Then each
   slot in the order written, at `fold.<name>`:
   1. its name is empty, or it isn't an object: `fold-invalid`;
   2. an unknown key: `unknown-key` at `fold.<name>.<key>`;
   3. not exactly one fold type: `fold-type`;
   4. `follow`: `follow-missing` on latest-by without it, `follow-invalid`
      when it isn't `"prev"` or `"keys"`; at `fold.<name>.follow`.
      This check stays after the fold type and before `limit` and the
      argument, so `{"sum": "", "follow": "next"}` is refused at `follow`,
      and `{"sum": "", "follow": "keys"}` at the argument;
   5. `limit`: `limit-invalid` when it isn't a whole number of at least 1,
      or the fold type isn't collect; at `fold.<name>.limit`;
   6. `by`: `by-invalid` at `fold.<name>.by` when it isn't text, is empty,
      starts `env.` but is none of `env.kind`, `env.observer` and
      `env.about:<prefix>`, has an empty or malformed prefix, or is written
      on latest-by. This check stays after `limit` and before the argument,
      so `{"sum": "", "by": "env.of"}` is refused at `by`;
   7. the fold type's argument: `fold-argument` at `fold.<name>.<type>`;
   8. `where`, below.
7. **`where`**: `where-invalid` throughout. At `fold.<name>.where` when it
   isn't an object or is empty. Check fields in written order, at `fold.<name>.where.<field>`.
   Refuse an invalid field name or tests that are not a nonempty object.
   Then check tests in written order, at that same path.
   Refuse an unknown operator or an argument that does not fit.

These are the only codes.

## The parsed form

`spec/parse` vectors show accepted specs in parsed form.
They keep `vertex` and `from`.
Each source has `store`, then supplied `kind`, `about` and `of`.
Each `fold` slot keeps its fold type, its `follow` as written, `"prev"` or `"keys"`, supplied `limit`, `by` as written and written `where`.
Latest-by always writes `follow`.
Times keep their spelling. Carried keys and `note` are dropped. An accepted spec's parsed form parses back to itself.
The parser gives exactly the specs that fit (`spec.parse` in [`laws.md`](laws.md)). That is a law
about what the parser gives; which JSON it accepts is this page, pinned by
the vectors.

# Payload values, design note

Moved here from `ROADMAP.md` on 2026-11-21, unchanged but for links. **Ruled by Kyle on 2026-11-21: every one of the seven choices, each for the recommendation.** The rulings and his note on a list with two live setters are in [the changelog](../CHANGELOG.md) under that date. Nothing is built yet.

**Draft by margins-claude, 2026-11-21, for Kyle's rulings before any tick is built.** Nothing here is built.
Every choice that is Kyle's is marked **Open**, with a recommendation and the alternatives.
The coordinator asked for it: each choice changes what some spec answers, so it changes result hashes, and ticks will pin those.
Everything was checked at main `15c9422`; the runs and counts are in the margins repo, `notes/2026-11-21-payload-values.md`.

**Summary.**

1. **The model never says how a stored payload becomes the record a fold reads.** That step exists only in Rust (`rust/place/src/line.rs`, `Line::record`). No vector, law, JS or sl-py code checks it. Both problems below live in that step. I recommend stating it in the model, with vectors.
2. **`value` and `n` should be ordinary fields.** Today every record has them, as `""` and `0`, and a `min` of `n` over records with no `n` answers 0 with nothing unread.
3. **A list should read as a set of its members** under `is`, `not` and `by`, and never as a number. Today a field holding a list, an object or `null` reads as absent.
4. **No result holds a payload value**, so the result codec does not change. Hashes change only for specs that test or group by the fields named in section 2 or 3.

Each numbered section ends with an Open choice and my recommendation.

## 1. What happens today

Four records in a scratch store, folded by `loops` built from `15c9422`:

| Record | Payload |
|---|---|
| `t:1` | `name: "a"` |
| `t:2` | `name: "b"`, `value: 7`, `n: "5"` |
| `t:3` | `name: "c"`, `routes: ["svc:a","svc:b"]`, `meta: {"x":1}`, `gone: null`, `ratio: 1.5` |
| `t:4` | `name: "d"`, `value: "hello"`, `n: 4` |

| Slot | Answer | What a reader would expect |
|---|---|---|
| count where `value` present | 4 | 2 (or 1, if a number is no `value`) |
| count where `value` is `""` | 3 | 0 |
| count where `n` present | 4 | 2 |
| count where `n` is 0 | 3 | 0 |
| `min` of `n` | 0, unread 0 | 4, unread 3 |
| `max` of `m` (no record has it) | null, unread 4 | as answered |
| count by `n` | group `"0"`: 3, group `"4"`: 1 | group `"4"`: 1, ungrouped 3 |
| count by `value` | group `""`: 3, group `"hello"`: 1 | group `"hello"`: 1, ungrouped 3 |
| count where `routes` present | 0 | 1 |
| count where `routes` absent | 4 | 3 |
| count where `routes` is `"svc:a"` | 0 | 1, if a list reads as a set |
| count by `routes` | ungrouped 4 | groups `svc:a` and `svc:b`, if a list reads as a set |
| count where `meta` present, `gone` present | 0, 0 | open (section 4) |
| count where `ratio` present | 1 | as answered |
| `sum` of `ratio` | 0, unread 4 | as answered |

`loops of t:3` shows every one of these values, so the store keeps them and the fold does not read them. The `min` row is the one that gives a wrong number with no sign of it: `n` reads differently from a field of any other name (`max` of `m` over the same records is null with 4 unread), and the text `"5"` under `n` reads as the number 0.

Where it comes from: the model's `Record` (`model/Loops/Record.lean`) has `value : String`, `n : Int := 0`, `nNumber` and `fields : List (String × Val)`, with `Val` one of text, whole number, number or boolean, and `Record.get` says "`value` and `n` always exist". That record is the shape the vectors are written in. `Stored` (`model/Loops/Stored.lean`) holds the payload as any JSON value. Nothing in the model maps one to the other. `Line::record` does: it copies a text `value` and a numeric `n`, drops a `value` or `n` of another type, puts text, numbers and booleans in `fields`, and drops every other member.

## 2. State the reading in the model

Whatever is ruled below, I recommend the step from a stored payload to a record's fields is defined in the model and pinned by vectors (a `vectors/payload/` set: a payload in, the fields out), so that JS, Rust and sl-py are held to one answer. Today this is the one place in the read path where an implementation decides alone, and `rust/PLACE.md` choice 6 already lists it as "to settle with the model". The rule is short:

- A payload that is an object gives one field per member. Any other payload gives none.
- A number that is whole and within ±(2^53 − 1) reads as a whole number, as `where` already says.

Under the split in use, the coordinator generates the `payload` vectors, and I look over the Lean definition of the reading, since it is the one definition all four implementations are held to.

**Choice 1, ruled 2026-11-21 for the recommendation.** State the reading in the model, with vectors. *Recommended: yes.*

## 3. `value` and `n`

Two rulings already point one way. Under the keyed fold, "`value` and `n` … are payload keys like the others … a field no record sets is absent even when it is `value`" (D8c, in the changelog under 2026-11-14). And "a missing field in a held payload is known absent" (`protocol/spec.md`). The plain fold breaks both for these two names.

In Kyle's old stores no fact has a key named `value` or `n` (0 of 11,037 facts in six stores). So nothing real depends on the special case; it is the test fixture showing through.

- **(a) Ordinary fields.** `Record` keeps `fields` only; `value`, `n` and `nNumber` go, and `Record.get` is a lookup. A record without `n` is unread to a `sum`, `min` or `max` of `n`, as it is for any other field. Vectors write `value` and `n` inside `fields` when a case needs them.
- **(b) Keep the structure, make both optional.** The same answers as (a); the vector format changes less, and the model still names two fields no protocol text gives a meaning.
- **(c) Leave it and document it.** No work. The `min` row above stays, and a writer who uses the name `n` or `value` for something else gets a silent wrong answer.

Cost of (a), by counting and not by trying it: about 70 places in `model/` name `r.n`, `r.value` or `nNumber`; the fixture folds (`Sum.step` and its kin read `r.n` directly) become the field-reading folds that already exist beside them; every vector that holds a record is regenerated (about 150 of 633 files hold `"value"` or `"n"` members); JS, Rust and sl-py drop two special cases each. No law's statement should change, since the laws speak of `Record.num`, not of `n`. The answers that change are exactly the rows of the table above.

**Choice 2, ruled 2026-11-21 for the recommendation.** *Recommended: (a).* It is the larger of the two changes in this note and it is mechanical; it is cheapest now, before ticks pin result hashes over real stores.

## 4. A member is present when it is written

Today a member holding a list, an object or `null` reads as absent, so `where routes absent` answers yes for the record that has routes. A fraction is already handled the other way: `ratio` is present, has no group key, and is unread to `sum`.

I recommend one rule for every kind of value: **`present` asks whether the held payload has the member.** A value no argument can name (an object, `null`, a fraction) is present, matches no `is`, satisfies every `not`, has no group key, and is unread to the numeric folds. That is what a fraction does now.

The alternative is that `null` and an empty list read as absent, on the grounds that a writer means "nothing here". the envelope already took the other side for the envelope ("`null` is a value of the wrong type, not a field left out"), and under the keyed fold `loops of` shows `gone: null` as the key's value. A reader who wants "no value" can test `is` against what it wants.

**Choice 3, ruled 2026-11-21 for the recommendation.** `present` means written, for `null`, `{}`, `[]` and `""` alike. *Recommended: yes.*

## 5. Lists

**What the old data holds.**

No fact in the old stores has a list a writer wrote, because the old CLI could only write text. Several values under one name appear in two other forms (homelab-audit, 9,053 facts):

| Form | Count | Examples |
|---|---|---|
| Numbered keys, `watches_1_ref` … `watches_N_ref` | 594 groups in 547 facts; 94 groups have 8 or more members | `watches`, `subject`, `used_by`, `provider` |
| Text joined by commas | 1,798 fields in 883 facts, over 48 keys | `lan_dns_a`, `observed_values`, `declared_tags` |

There are 13,893 `*_ref` fields in that store (the coordinator's 13,595 counts slightly differently), and 2,923 facts carry more than one. Of the reference values, 13,452 are bare names with no kind prefix: the key's name carries the role (`runs_on_ref`, `used_by_ref`), and the value is the target.

So the data asks for two different things, and they barely overlap (2 of the comma-joined fields are references):

- **A set one observer saw at once** (the DNS answers, the tags, the options). One writer, one observation, written whole. A list value fits this exactly.
- **Several targets under one role** (`watches`). A list value keeps the role, which `about` cannot: `about` is a flat set of typed references and has no place for "watches" as against "runs on". `about` still earns its place beside it, because it is in the envelope and reads when the payload is withheld or erased.

**How a list reads.**

- **`where`.** A list holds its members, as `env.about` does. `is` holds when a member equals the argument; `not` is its complement. Order and repeats in the list change nothing. The argument grammar does not change, and no operator is added.
- **`at-least` and `below`.** No set field takes these today (`protocol/spec.md` refuses them on every envelope field but the two times), so there is nothing to follow. Two readings:
  - *Some member satisfies it.* The list `[1, 10]` against the bound 5 then satisfies both, so a reader can no longer take one as the other's complement. The law `where.complements` is stated for a single whole value and still holds as written.
  - *A list is not a number.* Neither holds, as for text. This is the same rule as for `sum`, `min` and `max` below.
- **`by`.** A record joins the group of each distinct member that has a key, as it does under `env.about:<prefix>`. A list with no such member is ungrouped. **This amends a ruling**: the ruling on `by` says a record whose value is "a fraction, null, an array or an object" is ungrouped. A record in several groups is counted in each, so group sums can exceed the ungrouped total, as they already can under `about`.
- **`sum`, `min`, `max`.** A list is not a number: the record is unread, as for text. Summing the members would make one record count several times in a fold that reads one number per record.
- **The keyed fold.** No change. A key's value is whatever its live setter wrote, the whole list. The core never compares payload values and no result holds one (both already ruled), so the canonical result and its codec are untouched.
- **Members.** Text, a whole number and a boolean read as members. A fraction, `null`, a nested list or an object inside a list is a member no argument can name: it matches nothing and has no key. Nested values stay unread; this note proposes no path syntax.

**What a list is not.**

A list is one value, set whole by one emit. Under the keyed fold, two writers who each add a member from the same base leave two live setters, which is a conflict and not a union. A set that several writers grow belongs in one record per member (as rex-tools #8 asks for room membership), where the store's own union does the merging. The note that goes upstream should say this where it introduces lists, because the review that prompted this proposed lists as the edge mechanism.

**What the `*_ref` fields become.**

The split is Neon's (its take of 2026-11-21, section 2), and the counts from homelab-audit bear it out. It needs no change to the envelope.

| Class | Becomes | In homelab-audit |
|---|---|---|
| One target under a role (`runs_on_ref`) | A text field, as now. Two writers who disagree leave a real conflict | 11,418 fields (82%) |
| A set one observer states whole (`watches_1_ref` … `watches_N_ref`) | A list, read as a set | 2,473 fields in 594 groups |
| A set several writers grow, or an edge with its own attributes | One thing per edge (`of: edge:<rel>:<src>:<dst>`, `about` naming both ends, removal as a state) | none found |

Most reference values are bare names with the role in the key, so bringing the old data over means choosing a kind prefix for each role. That is the import's question.

**Choice 4, ruled 2026-11-21 for the recommendation.** Under `is` and `not`, a list reads as a set of its members. *Recommended: yes.*

**Choice 5, ruled 2026-11-21 for the recommendation.** `at-least` and `below` on a list: some member, or neither holds. *Recommended: neither holds.* "A list is never a number" is one rule for the comparisons and the numeric folds alike (the coordinator's point, and I agree).

**Choice 6, ruled 2026-11-21 for the recommendation.** A list gives one group per member under `by`, amending the ruling on `by` of 2026-11-11. *Recommended: yes.* Without it the `watches` case needs `about` with the role as the prefix, which is a workaround of the kind the numbered keys were.

**Choice 7, ruled 2026-11-21 for the recommendation.** A list is unread to the numeric folds. *Recommended: yes.*

## 6. In the model

A sketch, for whoever builds it, with no nested type:

```lean
inductive Held where
  | one (v : Val)          -- text, whole number, number, boolean
  | many (vs : List Val)   -- a list's readable members
  | other                  -- an object, null: written, and nothing can name it

structure Record where
  ...
  fields : List (String × Held) := []
```

`Item` gains a third case for a written value nothing can name, so `present` stays "holds an item" and `Arg.matches` already answers no for it. `Field.read` on a payload field gives the one value, the members, or that case (also for an empty list). `By.keys` on a payload field gives the members' keys, each once, through the `canonSet` that `about` uses.

What moves:

- **Laws.** The statements of the `where` and `by` laws are over lists of items and lists of keys already, because `about` needs that; I expect new cases in proofs and no changed statement. I have not tried it.
- **Vectors.** A new `payload` set; new `spec/run` cases for a list under `where` and `by`, an object, `null` and an empty list; the regeneration from section 3.
- **Implementations.** `Line::record` in Rust becomes a port of the model's reading. JS and sl-py gain it.
- **Mutation catalogue.** Regenerated, as after any model change.
- **Result hashes.** A spec's hash changes only if it tests or groups by `value` or `n` over records lacking them, reads `n` in a numeric fold over such records, or tests or groups by a member holding a list, an object or `null`. No store that matters holds such results yet.

## 7. Not proposed, and noted

- **Numbers written as text.** `loops emit count=7` writes the text `"7"`; `count:=7` writes the number. The old stores are all text: 4,792 fields in homelab-audit are whole numbers written as text (`entity_count`, `drift_count`) and 895 are decimals. Brought over as they are, a `sum` of `entity_count` reads nothing, with every record unread. That is a question for the import and the CLI, not for the model, and it should be decided before the replay of the old stores.
- **Paths into objects** (`meta.x`), list operators beyond membership (length, all-of), and ordering by a list. None is needed by the data above.
- **A way for the CLI to write a list** shorter than `routes:=["a","b"]`.

## How this was checked

- The runs in section 1: a scratch store and `loops` built from `15c9422`, in my worktree.
- The counts in sections 3, 5 and 7: the six SQLite stores under `~/code/gruel.network/.loops/data`, opened read-only. Only counts and key names were taken.
- Not done: the change itself in Lean, so every cost in sections 3 and 6 is an estimate from reading and counting.

## The examples the rulings were made on

By the coordinator, 2026-11-21. These were the entries PV1 to PV7 in `DECISIONS.md`; Kyle ruled each for the recommendation.
Every "today" answer was run against `loops` built from main `9460a15`. Each "with the change" answer is what the build must give, so these are its first acceptance cases.

All seven use one small homelab store. Each row is one emit:

| Thing | Fields |
| --- | --- |
| `svc:grafana` | `runs_on: "host:nuc1"`, `watches: ["svc:prometheus", "svc:loki"]` |
| `svc:prometheus` | `runs_on: "host:nuc1"`, `watches: ["svc:node-exporter"]` |
| `svc:loki` | `runs_on: "host:nuc2"`, `watches: []`, `notes: null`, `labels: {"tier": "infra"}` |
| `svc:caddy` | `runs_on: "host:nuc2"`, `ports: [80, 443, 8080]`, and no `watches` |
| `backup:nuc1` | `host: "nuc1"`, `n: 14` |
| `backup:nuc2` | `host: "nuc2"`, `n: 9` |
| `backup:nas` | `host: "nas"`, `status: "failed"` |
| `backup:pi` | `host: "pi"`, `snapshots: 3` |
| `setting:dns` | `value: "1.1.1.1"` |
| `setting:ntp` | `value: "pool.ntp.org"` |
| `setting:proxy` | `enabled: false` |

`loops of svc:loki` shows every one of loki's fields, so the store keeps them all. The questions are about what a fold reads.

### PV1: state the reading in the model

**Question.** Should the model say how a stored payload becomes the fields a fold reads, with vectors that every implementation must match?

**Example.** `svc:loki` holds `notes: null`. Ask "does loki have notes?" and today the answer is no.
The rule that gave that answer, "a `null` reads as if the field were not there", is written in one Rust function (`Line::record` in `rust/place/src/line.rs`).
The model does not state it. No vector tests it. The JavaScript implementation and margins-claude's Python one never see a stored payload.
If rex-tools' Python, or a browser reader, read the payload its own way and answered yes, no check would notice.
Every other step between a stored line and an answer is pinned by vectors.

**Options.**
- **Yes (recommended).** A definition in Lean and a `vectors/payload/` family: a payload in, the fields out.
- **No.** It stays the Rust store's own choice, as [`rust/PLACE.md`](../rust/PLACE.md) choice 6 lists it today.

**If yes.** A small piece on its own. PV2 to PV7 decide what the definition says.

### PV2: `value` and `n` become ordinary fields

**Question.** Today every record has a `value` and an `n` whether or not anyone wrote them: `value` is empty text and `n` is 0. Should they be fields like any other?

**Example.** "What is the smallest number of snapshots any host has?" over the four backups, as a `min` of `n`:

| | Answer | Reads as |
| --- | --- | --- |
| Today | 0, with 0 unread | "Some host has no snapshots, and I read every record." |
| With the change | 9, with 2 unread | "The smallest I could read is 9. Two records had no `n`." |

`backup:nas` and `backup:pi` have no `n`. Today the fold makes up a 0 for each and says nothing.
Ask the same of a field with any other name and the fold is honest today: a `min` of `snapshots` answers 3 with 3 unread.

The same happens with `value`. "How many settings have a value?" answers 3 today. `setting:proxy` has none; with the change the answer is 2.
A count of backups by `n` puts nas and pi in a group named `"0"` today. With the change they are ungrouped.

**Options.**
- **(a) Ordinary fields (recommended).** The two names lose their meaning in the model. A record without `n` is unread to a `min` of `n`.
- **(b) Keep the two names in the model and make them optional.** The same answers as (a). Fewer vector files change, and the model still names two fields that no protocol text gives a meaning.
- **(c) Leave it and document it.** Anyone who uses the name `n` or `value` for something of their own gets a wrong number with no sign of it.

**If (a).** The largest piece here: about 70 places in the model and about 150 of the 629 vector files. It is mechanical.
None of your old stores uses either name as a key (0 of 11,037 facts), so nothing real depends on today's reading.

### PV3: `present` means written

**Question.** Today a field that holds a list, an object or `null` reads as absent. Should `present` mean "the payload has this field", whatever it holds?

**Example.** "Which services have nothing recorded about what they watch?", asked as `watches` absent:

| | Answer |
| --- | --- |
| Today | All four, grafana included, which watches two services |
| With the change | `svc:caddy` only |
| The alternative: `null` and `[]` read as absent | `svc:caddy` and `svc:loki` |

Loki's `watches` is `[]`: someone looked and recorded that loki watches nothing. Caddy has no `watches` field: nobody recorded anything.
The recommendation keeps those two apart. The alternative makes them one answer.
The same goes for `notes: null` on loki: with the change, `notes` is present.

**What the recommendation costs.** "Which services watch nothing?" then has no single test. Caddy answers to `watches` absent; loki's empty list is present and matches no `is`. A writer who wants that question simple leaves the field out.

**Options.**
- **Yes (recommended):** present means written, for `null`, `{}`, `[]` and `""` alike. A fraction already works this way today.
- **No:** `null` and an empty list read as absent, on the grounds that the writer meant "nothing here".

### PV4: a list reads as a set under `is` and `not`

**Question.** Should `is` hold when some member of a list equals the argument?

**Example.** "Which services watch loki?", asked as `watches` is `"svc:loki"`:

| | `is "svc:loki"` | `not "svc:loki"` |
| --- | --- | --- |
| Today | 0 services | 4 services |
| With the change | 1 (`svc:grafana`) | 3 |

Today a list can be stored and shown and no fold can ask about it. That is why the old stores have numbered keys, `watches_1_ref` to `watches_N_ref`: 594 such groups in homelab-audit.

**Options.**
- **Yes (recommended).** Order and repeats in the list change nothing. No new operator.
- **No.** A list matches nothing, and several values under one name stay numbered keys.

### PV5: `at-least` and `below` on a list

**Question.** What do the two number tests answer for a list of numbers?

**Example.** Caddy has `ports: [80, 443, 8080]`. Ask "which services have a port below 1024?" and "which services have a port at 1024 or above?"

| Reading | below 1024 | at-least 1024 |
| --- | --- | --- |
| Some member passes | caddy: yes | caddy: yes |
| A list is never a number (recommended) | caddy: no | caddy: no |
| Today | no | no |

Under "some member passes" caddy answers yes to both. The two tests stop being opposites, so a reader who counts one and subtracts from the total gets the wrong number.
Under the recommendation, "does anything listen on a low port?" cannot be asked of a list. It needs one record per port, or a field such as `lowest_port`.

**Options.** Some member, or neither holds. **Recommended: neither holds.** It is the same rule as PV7: a list is never a number.

### PV6: a list gives one group per member under `by`

**Question.** Should a count by a list field put the record in one group for each member?

**Example.** "Count the services by what they watch":

| | Answer |
| --- | --- |
| Today | Ungrouped: 4. No groups. |
| With the change | `svc:loki`: 1, `svc:node-exporter`: 1, `svc:prometheus`: 1. Ungrouped: 2 (loki's empty list, and caddy). |

Grafana is counted in two groups, so the groups can add up to more than the records. That already happens with `by` on `env.about:<prefix>`, where a record that mentions two projects counts in each (your ruling of 2026-11-11).

**This amends a ruling of yours.** On 2026-11-11 you ruled that a value that cannot be a key, "a fraction, null, an array or an object", is ungrouped. An array would leave that list.

**Options.**
- **Yes (recommended).**
- **No.** The ruling stands. Grouping by several targets then needs `about` with the role as its prefix, or numbered keys.

### PV7: a list is unread to `sum`, `min` and `max`

**Question.** What does a numeric fold read from a list of numbers?

**Example.** A `sum` of `ports` over the four services:

| Reading | Answer |
| --- | --- |
| Unread (recommended), and today | 0, with 4 unread |
| Sum the members | 8,603, with 3 unread |

For ports the sum means nothing. For something like `disk_gb: [500, 500, 2000]` the sum of members, 3,000, is what a person would want; under the recommendation the writer states `disk_total_gb`, or writes one record per disk.
A numeric fold reads one number from each record. Summing members would count one record several times.

**Options.** Unread, or read the members. **Recommended: unread.**

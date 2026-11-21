# Payload values, design note

Moved here from `ROADMAP.md` on 2026-11-21, unchanged but for links. Its seven choices are open: they are in [DECISIONS.md](../DECISIONS.md) as PV1 to PV7, each with an example.

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

**Open choice 1.** State the reading in the model, with vectors. *Recommended: yes.*

## 3. `value` and `n`

Two rulings already point one way. Under the keyed fold, "`value` and `n` … are payload keys like the others … a field no record sets is absent even when it is `value`" (D8c, in the changelog under 2026-11-14). And "a missing field in a held payload is known absent" (`protocol/spec.md`). The plain fold breaks both for these two names.

In Kyle's old stores no fact has a key named `value` or `n` (0 of 11,037 facts in six stores). So nothing real depends on the special case; it is the test fixture showing through.

- **(a) Ordinary fields.** `Record` keeps `fields` only; `value`, `n` and `nNumber` go, and `Record.get` is a lookup. A record without `n` is unread to a `sum`, `min` or `max` of `n`, as it is for any other field. Vectors write `value` and `n` inside `fields` when a case needs them.
- **(b) Keep the structure, make both optional.** The same answers as (a); the vector format changes less, and the model still names two fields no protocol text gives a meaning.
- **(c) Leave it and document it.** No work. The `min` row above stays, and a writer who uses the name `n` or `value` for something else gets a silent wrong answer.

Cost of (a), by counting and not by trying it: about 70 places in `model/` name `r.n`, `r.value` or `nNumber`; the fixture folds (`Sum.step` and its kin read `r.n` directly) become the field-reading folds that already exist beside them; every vector that holds a record is regenerated (about 150 of 633 files hold `"value"` or `"n"` members); JS, Rust and sl-py drop two special cases each. No law's statement should change, since the laws speak of `Record.num`, not of `n`. The answers that change are exactly the rows of the table above.

**Open choice 2.** *Recommended: (a).* It is the larger of the two changes in this note and it is mechanical; it is cheapest now, before ticks pin result hashes over real stores.

## 4. A member is present when it is written

Today a member holding a list, an object or `null` reads as absent, so `where routes absent` answers yes for the record that has routes. A fraction is already handled the other way: `ratio` is present, has no group key, and is unread to `sum`.

I recommend one rule for every kind of value: **`present` asks whether the held payload has the member.** A value no argument can name (an object, `null`, a fraction) is present, matches no `is`, satisfies every `not`, has no group key, and is unread to the numeric folds. That is what a fraction does now.

The alternative is that `null` and an empty list read as absent, on the grounds that a writer means "nothing here". the envelope already took the other side for the envelope ("`null` is a value of the wrong type, not a field left out"), and under the keyed fold `loops of` shows `gone: null` as the key's value. A reader who wants "no value" can test `is` against what it wants.

**Open choice 3.** `present` means written, for `null`, `{}`, `[]` and `""` alike. *Recommended: yes.*

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

**Open choice 4.** Under `is` and `not`, a list reads as a set of its members. *Recommended: yes.*

**Open choice 5.** `at-least` and `below` on a list: some member, or neither holds. *Recommended: neither holds.* "A list is never a number" is one rule for the comparisons and the numeric folds alike (the coordinator's point, and I agree).

**Open choice 6.** A list gives one group per member under `by`, amending the ruling on `by` of 2026-11-11. *Recommended: yes.* Without it the `watches` case needs `about` with the role as the prefix, which is a workaround of the kind the numbered keys were.

**Open choice 7.** A list is unread to the numeric folds. *Recommended: yes.*

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

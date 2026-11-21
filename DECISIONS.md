# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.

## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [PV1](#pv1-state-the-reading-in-the-model) | State in the model how a stored payload is read | Yes | Any tick |
| [PV2](#pv2-value-and-n-become-ordinary-fields) | `value` and `n` become ordinary fields | Yes | Any tick |
| [PV3](#pv3-present-means-written) | `present` means the field was written | Yes | Any tick |
| [PV4](#pv4-a-list-reads-as-a-set-under-is-and-not) | A list reads as a set under `is` and `not` | Yes | Any tick |
| [PV5](#pv5-at-least-and-below-on-a-list) | `at-least` and `below` on a list | Neither holds | Any tick |
| [PV6](#pv6-a-list-gives-one-group-per-member-under-by) | A list gives one group per member under `by` | Yes | Any tick |
| [PV7](#pv7-a-list-is-unread-to-sum-min-and-max) | A list is unread to `sum`, `min` and `max` | Yes | Any tick |
| [RC1](#rc1-the-result-codecs-choices) | The result hash's prefix, and five choices to confirm | Its own prefix | Any tick |
| [W1](#w1-an-adoption-step-before-standing-and-sharing) | An adoption step before standing and sharing | Yes, after ticks | The order of work |
| [W2](#w2-remember-which-segments-were-checked) | Remember which segments were checked | Yes, in the adoption step | Read speed on big stores |
| [W3](#w3-stop-extending-the-javascript-implementation) | Stop extending the JavaScript implementation | Yes, as a trial for step 4 | How step 4 is built |
| [W4](#w4-the-refusal-for-input-that-is-wrong-twice) | The refusal for input that is wrong twice | One answer, the implementer picks the order | How step 4 is built |
| [W5](#w5-the-mutant-check-on-every-merge-request) | The mutant check on every merge request | Keep, and measure it on PV2 | Nothing |
| [W6](#w6-generated-pages-in-the-repository) | Generated pages in the repository | Keep | Nothing |
| [W7](#w7-which-choices-come-to-you) | Which choices come to you | Only those that change bytes or answers | 53 unconfirmed choices |
| [E1](#e1-edges-three-classes-and-no-change-to-the-envelope) | Edges: three classes, no change to the envelope | Yes | The import |
| [I10](#i10-close-issue-10) | Close issue #10 | Yes | Nothing |

After these: [choices to confirm](#to-confirm), [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled).

## Payload values

From margins-claude's note, [notes/2026-11-21-payload-values.md](notes/2026-11-21-payload-values.md). margins-claude and the coordinator recommend the same answer on each.
Each choice changes what some spec answers, so it changes result hashes. That is why they come before any tick is built.

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
- **No.** It stays the Rust store's own choice, as [`rust/PLACE.md`](rust/PLACE.md) choice 6 lists it today.

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

### What the note says that is not a choice

- A list is one value, written whole by one emit. Two writers who each add a member from the same base leave a conflict, not a union. So a list is not how a growing relation is kept: see [E1](#e1-edges-three-classes-and-no-change-to-the-envelope).
- The result codec does not change. A hash moves only for a spec that tests or groups by these fields.
- Your old stores hold numbers as text (4,792 whole numbers in homelab-audit). Brought over unchanged, a `sum` of `entity_count` would read nothing. That is a question for the import; it comes back to you with examples when the import is designed.

## The result codec

### RC1: the result codec's choices

The codec (D7) is built: [protocol/result.md](protocol/result.md). It turns a vertex's result into fixed bytes and a hash. Once a tick holds such a hash, changing any of this changes every tick, so these are confirmed first.

**Example.** A vertex over two tasks gives this canonical text (cut short), and this hash:

```
{"blocked":{"unread":"0","value":"1"},"current":{"things":{"task:1":["r:…0110"],"task:2":["r:…011g"]},"unread":"0"}, …}
h:igb5pksk2doe9kbgaauequeiasc9iu5tjuuufhhfnivif0bq37o0
```

**The one real question: the prefix.** A result hash is spelled `h:`. A record's commitment to its payload is also spelled `h:`.
Suppose a note says "see `h:igb5…`". Nothing in the text says whether that is a tick's result or a record's commitment. Every other id says what it is: `r:` is a record, `k:` is a key.
The two can never be equal (law `result.no-commit`), so nothing breaks. The cost is to a person or a tool reading one out of context.

- **Its own prefix (recommended).** The name is yours; `res:` would do. It costs one constant in each implementation and the 23 result vectors regenerated. It is free now and expensive after the first tick.
- **Keep `h:`.** One spelling for "a hash of content".

**Five choices to confirm.** I would keep each:

| Choice | What it means in the example |
| --- | --- |
| A listed record is its id alone | `"r:…0110"`, not `{"id": "r:…0110"}` |
| The bytes hold the slots only | No vertex name and no version inside. The version is in the label that is hashed first, `loops/result-hash/v1:`. Two vertices with the same slots and the same answers have the same hash. |
| Slots are in sorted order | `blocked` comes before `current` whatever order the spec wrote them |
| No fold type is written | A count of 1 and a sum of 1 write the same JSON. The bytes name a result only together with its spec. |
| The label text | `loops/result-hash/v1:` |

## Order of work, and how we work

### W1: an adoption step before standing and sharing

**Question.** Should a step that makes the new `loops` your daily tool come before standing and sharing?

**Example: a Monday morning.** With the old tool you typed `sl read project` and saw each item's name and status. With the new `loops` today:

- A read needs a JSON spec written by hand for each question (`.loops/vertices/project.json`).
- There is no `ls`. Nothing lists the things or kinds in a store without a spec.
- Nothing finds things by the start of a name. "Every thing that starts with `task:`" cannot be asked without a spec.
- Every answer is JSON.
- The shell finds the old Python `loops` first. rex-tools reads the text of `loops --help` to learn which one it got.
- Your 11,037 facts in six stores are not in it.

So the only daily users of the new tool are agents.

**What the step would build.**
- An import of the old stores as a shadow: the old tool stays in charge, and the copy is read beside it to compare.
- `ls`, a scan by name prefix, relative times, and a plain-text view.
- Reads across several stores. A spec names a store in `from` today, and the name is ignored: a spec naming `homelab-audit` and `relay` was answered from whatever store was in the directory. In the old setup homelab-audit, homelab-coord and relay are three stores, and `generate_changelog.py` went around the CLI to combine them. This is the one piece Neon would not give up; it needs no change to the protocol.
- The cache of [W2](#w2-remember-which-segments-were-checked).

**Options.**

| | Order | When you would be using it daily |
| --- | --- | --- |
| **A (recommended)** | Payload values, the tick record and a first tick, adoption, then standing with sharing | After ticks |
| B | Your order of 2026-11-10 as it stands: ticks, then standing with sharing. The adoption pieces stay "later work". | After the two largest steps are built, with no daily use to test them |
| C | Adoption first, then ticks | Soonest. Ticks, which Recall and handoffs between agents need, wait. |

I lean to A because the tick work is in motion: the laws are in the model and the codec is built. C is a fair choice too; the import needs nothing from ticks, and it does need PV1 to PV7 ruled first, as ticks do.
Neon and margins-claude both support an adoption step before standing.

**Done when** `loops` is first on `PATH` for daily work.

### W2: remember which segments were checked

**Question.** May a read skip the signature checks on a segment that this user's own `loops` has already checked?

**Example.** homelab-audit has 9,053 facts. Once imported, every `loops` command checks every signature before it answers: about half a second each time (measured: `loops check` over 9,000 records takes 0.53 s).
A shell prompt or a chat hook that reads on every prompt pays that every time. Writing 2,000 records one emit at a time takes 145 s, because each emit checks all that came before.

**The proposal** (Neon's design; I think it is sound):
- A file per user, outside the store (under `~/.cache/loops/`), lists the hashes of segments that were checked whole and found clean.
- Every read still hashes each segment's bytes, so a segment changed in place is caught. Only the signature checks are skipped.
- A segment with a refused line is never listed, so `store-not-clean` still fires every time.
- `loops check` ignores the file and checks everything.
- The file is never inside `.loops/`. A store that came from a clone or a shared directory could otherwise bring its own "already checked" list.

**Options.**
- **Yes, in the adoption step (recommended).** That is when the first large store exists.
- Yes, now.
- No: every read checks everything, as today.

**What it buys.** Neon estimates a read of 9,000 records goes from about 0.6 s to under 0.2 s. That is an estimate, not a measurement.

### W3: stop extending the JavaScript implementation

**Question.** Each protocol piece is built three times today: in Lean (the model), in JavaScript, and in Rust, and a fuzz run compares all three. Should the pieces from ticks on be built in Lean and Rust only?

**Example: the tick record.** The last piece built the present way was the result codec: `model/Loops/Result.lean`, `interop/js/result.mjs` and `rust/core/src/result.rs`, then the fuzz generator taught about it.

| | The tick record is built in | Checked by |
| --- | --- | --- |
| A, as now | Lean, JavaScript, Rust | The three-way fuzz; margins-claude's Python from outside |
| **B, freeze (recommended as a trial)** | Lean, Rust | A two-way fuzz; margins-claude's Python from outside |
| C, retire JavaScript now | Lean, Rust | The same, and the existing layers lose their third implementation too |

Under B the JavaScript stays for everything it has today (records, folds, specs, the result codec) and is kept passing its vectors, the payload values change included. It does not grow ticks, standing or sync.

**Views.** Neon proposes B. margins-claude is against retiring it (C): JavaScript is what the in-repo fuzz compares, and its own Python lives in another repository and is not in CI. It has no objection to B as a trial for step 4, with the risk below. I recommend B for step 4 only, looked at again when ticks are done.

**What B risks.** When Lean and Rust disagree, no third implementation says which is wrong; the model wins by definition. A misreading shared by the Lean side and the Rust port goes unseen, and the same sessions write both.

**What I do not have.** A count of faults that only the JavaScript side caught; margins-claude has none either. If you want it before ruling, I can go through the fuzz reports for it.

### W4: the refusal for input that is wrong twice

**Question.** When an input has two faults, must every implementation give the same one refusal, and who chooses which?

**Example.** This slot has two faults, an empty field name and an unknown `follow`:

```
{"sum": "", "follow": "next"}      refused: follow-invalid at fold.total.follow
{"sum": ""}                        refused: fold-argument at fold.total.sum
```

The protocol pins that the first answer is `follow-invalid` and never `fold-argument`. All three implementations give exactly that. Neon counts about 70 lines of protocol text that pin such orders, and several of the orders came to you for a ruling.
The same will come up for ticks: a tick that names a record nobody holds, and whose hash also does not match.

**Options.**
- **A.** As now: one answer, and the order is brought to you.
- **B (Neon).** For new steps, pin the refusal for a single fault only. Input that is wrong twice is "refused with one of the codes that apply".
- **C (recommended).** One answer for every input, in the order the model checks. The implementer picks that order and notes it in the changelog. It does not come to you.

**Why not B.** The fuzz compares answers for equality today. Under B it would need to know every code that applies to an input, which is more machinery than following the model's order. Or it accepts "both refused" and no longer sees an implementation that refuses for the wrong reason.

### W5: the mutant check on every merge request

**What it is.** The mutation catalogue holds 314 deliberate faults, each tied to an exact line of the model. Every merge request runs a quick check that each one still finds its line, and that a sample of 33 are caught. The full run is separate and on request.

**Example.** PV2 changes about 70 places in the model. Some of the 314 faults point at those lines.
- **With the check (today):** the same merge request must re-point them, or the pipeline is red.
- **Without it (Neon):** the merge request is smaller. The next full run reports those faults as not applying, and someone re-points them in a batch, later, away from the change that moved them.

**Options.** Keep it, or move it to the full run. **Recommended: keep it through PV2 and measure.** I will report what the re-pointing cost in that merge request. If it is more than a small part of the work, the check moves. My confidence here is low either way.

### W6: generated pages in the repository

**Question.** Neon proposes that the generated HTML under `out/` stop being committed and be built in CI.

**Example.** A merge request that changes one law shows the Lean change and, below it, the regenerated `out/` pages. A reviewer scrolls past generated HTML.

**What stands in the way.** Your site's sync script (`projects.gruel.world/scripts/sync.sh`) refuses to copy output that is not committed: "so every hosted revision is a commit you can check out and regenerate".

**Options.** Keep committing `out/`, or change the site's sync to build it. **Recommended: keep.** The rule in the sync script is a good one and the cost is noise in a diff.
The other half of Neon's point, counts kept by hand in the roadmap, is done: the roadmap no longer holds them.

### W7: which choices come to you

**Question.** 53 implementer choices in the changelog still say "for Kyle to confirm", some since 2026-11-14. Should all such choices come to you?

**Example.** Two of the 53:
- "The flags are `--values` and `--whole`." This is CLI wording. [`rust/PLACE.md`](rust/PLACE.md) calls the CLI a working proposal, and changing it later is a rename.
- "`unread` counts things, not records." This changes what a spec answers, so it changes a result hash, and a tick will pin it.

**Proposal.**
1. A choice comes to this file, with an example, when it changes a record's bytes or id, a result's bytes or hash, a refusal code, or what a spec answers.
2. Every other choice (a flag's name, a law's name, a file layout, how a test is built, the shape of a CLI-only view) is written in the changelog as an implementer choice and stands unless you object.

**Options.**
- **Yes (recommended).** Of the 53, 25 then stand as they are, and 28 remain [to confirm](#to-confirm).
- **No.** All 53 wait for you, as now.

### E1: edges: three classes, and no change to the envelope

**Question.** Neon's first review proposed typed edges. What should a reference from one thing to another be?

**The answer the three of us agree on** (the split is Neon's, in its second take):

| Class | Example | Kept as | In homelab-audit |
| --- | --- | --- | --- |
| One target under a role | Grafana runs on one host | A text field: `runs_on: "host:nuc1"`. Two writers who disagree leave a conflict, which is right. | 11,418 fields (82%) |
| A set one observer states whole | What a collector saw grafana watching | A list, read as a set ([PV4](#pv4-a-list-reads-as-a-set-under-is-and-not), [PV6](#pv6-a-list-gives-one-group-per-member-under-by)) | 2,473 fields in 594 numbered groups |
| A relation several writers add to, or one with facts of its own | Caddy routes to grafana on port 3000 | One thing per edge | None found. rex-tools' room membership is one. |

**Example of the third class.**

```
loops emit edge --of edge:routes-to:caddy:grafana --about svc:caddy --about svc:grafana port:=3000
```

Two people who add caddy→grafana and caddy→loki from the same base write two different things, so nothing conflicts. Removing an edge is a new state of that edge. `--about` names both ends, so "everything that touches caddy" stays one read.

**Why a list is not enough.** One writer sets `routes: ["svc:grafana"]`; another, from the same base, sets `routes: ["svc:loki"]`. The store holds two rival values of `routes`, not a caddy that routes to both.

**Why not a role on each mention in `about`.** It changes the envelope, and so every id. It also does not fix the case above, because an emit copies `about` whole from the heads.

**Options.**
- **Yes (recommended):** these three classes, and no change to the envelope.
- **No:** design typed edges in the envelope.

**If yes.** Nothing is built now. It decides what the import turns the old `*_ref` fields into, and what the documents tell a writer.

### I10: close issue #10

**Question.** [Issue #10](https://git.gruel.network/kaygee/strange-loops-next/-/issues/10), from rex-tools, asks for a strict "after" test on `emitted`, for an inbox's read marker.

**Example.** A reader marks everything up to 9:05:00.123 as read. Its next query, "`emitted` at-least 9:05:00.123", returns the last read message again. A strict "after" would fix that.
It would not fix this: a message stamped 9:04 that reaches the store at 9:10, through a slow clock or a git merge, is never unread. `emitted` is each writer's own clock.

rex-tools has since moved to a local list of the ids it has seen, which has neither problem. The session that filed the issue wrote that it is fine to close.

**Options.**
- **Close it, with no new operator (recommended).** An inbox keeps its own list of what it has seen, as rex-tools now does. A reader's tick is kept for boundaries worth citing, such as a handoff; it is not an inbox's read marker.
- Add a strict operator.

## To confirm

The 53 implementer choices that are built and unconfirmed. The full text of each is in [CHANGELOG.md](CHANGELOG.md) under the date given.
If you take [W7](#w7-which-choices-come-to-you), only the first table needs you.

### Choices that change what a spec answers

| Where | Date | Count | The ones I would read |
| --- | --- | --- | --- |
| `sets` in the envelope | 2026-11-14 | 3 | `sets` holds key names, never labels: a name that starts with `@` in a changeset is written as given. |
| Latest-by following `keys` | 2026-11-14 | 8 | The three in the example below. |
| Count, sum, min, max and collect following `keys` | 2026-11-14 | 6 | A field with no live setter is unread to a sum. Collect writes one newest-first list across things, not a list per thing. |
| A keyed collect's `limit` | 2026-11-15 | 3 | The things kept are those whose newest live record is newest. |
| The result codec | 2026-11-20 | 8 | [RC1](#rc1-the-result-codecs-choices) above. |

**Example for the keyed choices.** `task:7` has `team: "infra"`. On your laptop you set `status` to `open`. On another machine, from the same base, `status` is set to `done`. The two stores merge, and `status` now has two live setters that disagree.

- **`unread` counts things.** "Count the tasks whose status is open" answers with `task:7` unread, once, though two records are involved.
- **A no outranks an unknown.** "Count the tasks whose status is open and whose team is web": the status test cannot be answered, and the team test answers no. `task:7` is left out and is not counted as unread. However the status is settled, it is not a web task.
- **A field nobody set is absent.** `task:7` has no `due`. "`due` is absent" answers yes, and so does "`due` is not Friday".

### Choices in the CLI and the tooling

| Where | Date | Count | The ones I would read |
| --- | --- | --- | --- |
| The flags for `sets` | 2026-11-14 | 1 | `--sets <key>` and `--sets-nothing`. |
| The writer in `loops emit` | 2026-11-14 | 7 | No flag turns the anchor off; `emit -` is the path that fills nothing in. What was filled in is not printed. |
| Values beside ids | 2026-11-15 | 9 | An emit of a held thing that names no field sets nothing: `loops emit task --of task:1 --about project:y` moves the task and keeps every value. The first record of a thing sets every key it names. |
| `--values` gives the envelope | 2026-11-21 | 5 | No new flag. The member is `env`, the whole stored envelope. |
| The Rust read path | 2026-11-21 | 3 | Two new public names in the core, `live::LiveState` and `Spec::run_admitted`. |

## Being prepared

Nothing here needs you yet. It is listed so that you know what is coming.

**The tick record.** The cut note is ruled; what a tick record holds is not designed. The coordinator is writing the proposal, with an example for each choice. A preview of its first question, using Neon's example:

- Subagent 1 emits 40 records and closes with a tick, T1, that covers them. Subagent 2 emits 55 and closes with T2. The orchestrator writes 3 records of its own and closes with T3.
- **Agreed among the three agents:** T3's `covers` lists five ids: T1, T2 and its own three records. It does not list 98 records. T1 and T2 are folded as records, each with its own result, and never opened up. Checking T3 needs T1 and T2, not the 95 records under them.
- **What follows from that, and will be yours to rule:**
  - T3 is then not the same as a tick over all 98 records. It is a different fold with a different hash.
  - If T1 and T2 both covered one record, a count taken through T3 counts it twice.
  - A reader's tick ("this is what I had read when I decided") and a writer's base (the cut note, section 8) are one kind of record used two ways.
  - An inbox's read marker is a private local file, not a tick.

Also coming with it: the name of the window field, how `covers` is written, and the input key that says which covered records were read open.

**Numbers stored as text**, with the import ([W1](#w1-an-adoption-step-before-standing-and-sharing)).

## Deferred by you

Not asked now. Each comes back when its step does.

- **A strict store's policy for an emit behind what the store holds:** warn, fail, or let it through as a conflict. You raised it on 2026-11-20 and left it for step 3. margins-claude's answer is recorded in [the cut note](notes/2026-11-15-the-cut.md), section 8.
- **Refusing an undeclared payload key in a strict store.** Step 3. You ruled on 2026-11-10 that refusal is progressive.
- **A top-N across groups.** 2026-11-11: "a later discussion".
- **What the old tool's `combine` and `discover` vertices did.** 2026-11-20: they wait until they are wanted.
- **The provenance export mapping,** W3C PROV or PREMIS.

## Ruled

One line for each ruling, newest first. The full text is in [CHANGELOG.md](CHANGELOG.md) under the date, or in the note named.

| Date | Ruling | Where |
| --- | --- | --- |
| 2026-11-20 | Step 2 meets its exit criterion and is closed | Changelog |
| 2026-11-20 | A verb that answers from a store that does not check answers and is then refused `store-not-clean` | Changelog |
| 2026-11-20 | The cut: a cut is a part closed downward over `prev`; "as of T" is down; a content tick folds a set named outright; ids without `shown` in the result; a window is a `where`; `emit` copies `occurred`; the changeset path can ask for the filler; the filler takes a base | [The cut note](notes/2026-11-15-the-cut.md), sections 1 to 8 |
| 2026-11-20 | The result codec carries no tag on an integer | The cut note, section 6 |
| 2026-11-15 | `loops of` shows values; `--values`; an emit of a held thing sets the fields it names | Changelog |
| 2026-11-15 | A keyed collect's `limit` counts things, each listed whole | Changelog |
| 2026-11-14 | A slot following `keys` reads the envelope from the heads ("Rule B"); every emit carries the thing's whole `about`; the anchor names every live record | [The declared-keys note](notes/2026-11-11-declared-keys.md), sections 3 and 5 |
| 2026-11-13 | `"follow": "keys"`, its result, how the other slots read a keyed thing, the laws and the writer | The declared-keys note, sections 1 to 5 |
| 2026-11-13 | A writer refuses an `under` target it does not hold | Changelog |
| 2026-11-11 | D7: a separate, versioned result codec with integers as decimal text | Changelog |
| 2026-11-11 | `by`: its spelling, text keys, `ungrouped`, a record in several groups, `limit` per group | Changelog |
| 2026-11-11 | `follow` on count, sum, min and max; the caching reader's law is step 4's first | Changelog |
| 2026-11-11 | Envelope shapes: one new code, `envelope-not-object` | Changelog |
| 2026-11-11 | The slice law is *witnessed*; a writer's tool fills in the anchor | Changelog, under D8 |
| 2026-11-10 | D8: `of` names a record's one thing; `about` only mentions; `sets` declares keys; natural keys; no conflict declared by the core; undeclared keys refused progressively | Changelog |
| 2026-11-10 | Withheld and erased are one state to a fold | Changelog, under D8 |
| 2026-11-10 | The order: step 2 with D8, then step 4, then step 3 with step 5 | Changelog |
| 2026-11-10 | D1 to D6, as their recommendations | Changelog |
| Step 1 | R1 to R6; binary64 by Option A | Changelog |

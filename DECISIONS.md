# Decisions

Everything that waits on Kyle, in one place. Each entry has the question, an example, the options and a recommendation.

- An entry leaves this file when it is ruled. The entries ruled on 2026-11-21 (PV1 to PV7, RC1, W7, E1, I10) have left it.
- The ruling is written in [CHANGELOG.md](CHANGELOG.md) under its date, and one line is added to [Ruled](#ruled) at the end of this file.
- What is next once these are ruled is in [ROADMAP.md](ROADMAP.md).
- "Today" in an example means the `loops` binary built from main on 2026-11-21. Every "today" answer below was run, not predicted.

## At a glance

| Id | Question | Recommended | What waits on it |
| --- | --- | --- | --- |
| [TK1](#tk1-a-tick-is-an-ordinary-record) | Is a tick an ordinary record of kind `tick`? | Yes; its payload is the whole tick, and it has no id but its record id | The tick record |
| [TK2](#tk2-a-tick-carries-its-result) | Does a tick carry its result, or only the result's hash? | Both | The tick record |
| [TK3](#tk3-how-a-tick-names-its-spec) | How a tick names its spec | By a hash of the spec as parsed, and it carries the spec too | The tick record |
| [TK4](#tk4-covers-lists-every-record-that-was-folded) | What `covers` lists | Every record that was folded, each id once; a tick among them is one record | The tick record |
| [TK5](#tk5-which-covered-records-were-read-without-a-payload) | How a tick says which covered records it read without a payload | A list, `withheld`, left out when empty | The tick record |
| [TK6](#tk6-what-a-spec-reads-of-a-tick) | What a later spec reads of a tick | Nothing new: the payload rules of 2026-11-21. A total across ticks is not in step 4 | The tick record |
| [TK7](#tk7-checking-a-tick) | How a tick is checked | A verb with three answers; a store admits a tick without checking it | The first tick, and step 4's exit |
| [TK8](#tk8-the-name-of-the-window-field) | The name of the window field | `env.when` | The window field |
| [TK9](#tk9-which-rules-a-tick-was-folded-under) | How a tick says which rules it was folded under | The label inside the spec's hash is the version of the rules; it goes to `v2` when what a spec answers changes | The tick record |
| [W1](#w1-an-adoption-step-before-standing-and-sharing) | An adoption step before standing and sharing | Yes, after ticks, with its CLI pieces built beside the tick work | The order of work |
| [W2](#w2-remember-which-segments-were-checked) | Remember which segments were checked | Yes, in the adoption step | Read speed on big stores |
| [W3](#w3-stop-extending-the-javascript-implementation) | Stop extending the JavaScript implementation | Yes, as a trial for step 4 | How step 4 is built |
| [W4](#w4-the-refusal-for-input-that-is-wrong-twice) | The refusal for input that is wrong twice | One answer in the repository; the protocol text pins single faults only | How step 4 is built |
| [W5](#w5-the-mutant-check-on-every-merge-request) | The mutant check on every merge request, and new mutants for step 4 | Keep (measured: under five minutes on the largest model change); add step 4's mutants after its exit | Nothing |
| [W6](#w6-generated-pages-in-the-repository) | Generated pages in the repository | Keep | Nothing |

The tick record is the next thing to build, and TK1 to TK9 are its proposal (2026-11-21). margins-claude reviewed it; its four additions are in the entries, and TK9 is one of them. W3, W4 and W5 are wanted with them, before it is built.
Each W entry ends with Neon's comments on it (2026-11-21) and what the coordinator makes of them.

After the tick record and the W entries: [what is being prepared](#being-prepared), [what you deferred](#deferred-by-you) and [what is ruled](#ruled). Nothing waits to be confirmed: under your ruling of 2026-11-21 an implementer choice comes here only when it changes bytes or answers.

## The tick record

A tick says: "folding exactly these records with this spec gives this result."
The cut note is ruled and the result codec is built. What the record itself holds is not designed, and these nine entries propose it.
The technical form (the hash rule, the canonical forms, the laws to state) is in [the tick record note](notes/2026-11-21-tick-record.md).

**The example every entry uses.** Your laptop's store holds six backup records: three services, the nights of 16 and 17 November.

| Record | Service | Night | `mb` |
| --- | --- | --- | ---: |
| `r:kvvv…` | jellyfin | 16 | 412 |
| `r:ftlf…` | pihole | 16 | 3 |
| `r:6p8c…` | gitea | 16 | 88 |
| `r:fqqv…` | jellyfin | 17 | 419 |
| `r:vm0f…` | pihole | 17 | 10 |
| `r:53eo…` | gitea | 17 | 95 |

The spec `backups` counts them, adds up `mb` and lists them:

```
{"vertex": "backups", "from": [{"store": "lab", "kind": ["backup"]}],
 "fold": {"runs": {"count": true}, "mb": {"sum": "mb"}, "list": {"collect": true}}}
```

Today `loops fold backups.json` on the laptop answers `runs` 6, `mb` 1027, and the six ids.
The canonical result of that answer hashes to `res:d8eu…`. The CLI has no verb for the hash yet, so I computed it by the codec's rule; the same rule gives the hash of all 23 result vectors.

**The tick this proposal would write for it** (ids shortened):

```
env      kind "tick", observer k:5eis… (you), emitted 2026-11-21T19:40:00Z, signed
payload  {
  "spec":        {"vertex": "backups", "from": [...], "fold": {...}},
  "spec-hash":   "spec:gs9s…",
  "covers":      ["r:53eo…", "r:6p8c…", "r:fqqv…", "r:ftlf…", "r:kvvv…", "r:vm0f…"],
  "result":      {"list": {"heads": [six ids], "unread": "0"},
                  "mb":   {"unread": "0", "value": "1027"},
                  "runs": {"unread": "0", "value": "6"}},
  "result-hash": "res:d8eu…"
}
```

It is 1,097 bytes. Each entry below is about one part of it.

### TK1: a tick is an ordinary record

**Question.** Is a tick a record like any other, of kind `tick`, with the tick as its payload? Or is it a new sort of stored thing?

**Example.** With the tick above in the store:
- `loops get r:…` shows it. `export`, `import`, `erase` and a sync treat it as they treat any record.
- You cite it by its record id, as you said on 2026-11-21.
- The NAS computes the same tick from the same six records. It writes its own record: another observer, another time, another id. The two payloads are the same bytes.

**What "ordinary" means, exactly.**
- A store takes a tick as it takes any record. Nothing checks the payload when it is written or imported. Checking is a verb ([TK7](#tk7-checking-a-tick)).
- A tick has no id but its record id. Two ticks say the same thing when their payloads are the same bytes.
- A content tick and a witness tick are the same record. In step 4 nothing in it says how its records were chosen. It claims only what it computed.
- Any record of kind `tick` whose payload has this shape is a tick. One whose payload does not is "not a tick" to the check, and still a record.

**Options.**
- **A (recommended).** An ordinary record. The payload is the whole tick.
- **B.** As A, and the payload also holds a hash of the tick itself. The design page says a content tick's id is "the hash of vertex, covers and result". It would let a spec group equal ticks by one text field. Nothing needs it yet, and any reader can compute it from the other fields, so it can be added later without touching a tick already written.
- **C.** A stored value that is not a record: no observer, no signature. Then citing, sending, syncing and erasing a tick each need new rules.

### TK2: a tick carries its result

**Question.** Does the tick hold the result, or only the result's hash?

**Example.** The NAS was sent the tick for the night of the 16th and never got the three backup records under it.
- **With the result:** `loops get` on the NAS shows `mb` 503 and the three ids. A weekly tick over the nightly ticks can be read there.
- **With the hash only:** the NAS sees `res:5itq…` and can read nothing. Only a holder of the three records learns what the tick said.

**The cost is size.** The example tick is 1,097 bytes with its result and 643 without. A result that lists ids is as long as its list.

**Options.**
- **A (recommended).** The result, and its hash beside it. The hash is what two machines compare, and a spec can test or group by it. A tick whose hash is not the hash of its result is not a tick.
- **B.** The hash only.
- **C.** The result only. The hash can be computed, but then no text field names the result.

**What A costs beyond size: a tick outlives an erasure.** A result holds more than ids. A slot with `by` on a payload field writes that field's values as its group names. You ruled on 2026-11-10 that erasure is the emergency stop, so that the content travels no further. Erasing a backup record does not reach a tick that covered it: a `by svc` result in that tick still says `jellyfin`, on every machine the tick was sent to.
- The rule of conduct: when you erase a record, erase the ticks that cover it too. They are records, so `loops erase` works on them.
- Finding them is one test: `where covers is <id>` ([TK6](#tk6-what-a-spec-reads-of-a-tick)). `loops erase` can list them when it runs.
- Option B does not avoid this. A hash of a small result can be guessed, and a tick with only a hash is of no use to a reader without the records.

Neon raised carrying the result; margins-claude and I agree with A. margins-claude raised the erasure point.

### TK3: how a tick names its spec

**Question.** How does a tick say which spec it folded, and does it carry that spec?

**Example.** In January you add a slot, `largest`, to `backups.json`. In February someone checks November's tick.
- **With only a hash:** the file on disk now hashes to something else. The spec that made the tick is gone unless someone digs it out of git. The tick cannot be checked.
- **With the spec in the tick:** the check needs the tick and the six records, and nothing else.

**The hash.** `spec-hash` is a hash of the spec in its parsed form, the form the `spec/parse` vectors already pin. So:
- Changing a `note` keeps the hash. Notes are not in the parsed form.
- Renaming the vertex, renaming the store in `from`, or reordering the list of kinds gives another hash.
- The hash names the spec as it is written down. Two specs that mean the same and are written differently have two hashes.

**Options.**
- **A (recommended).** The hash, and the spec itself. The spec here is 136 bytes. A tick whose hash is not the hash of its spec is not a tick.
- **B.** The hash only, as the design page has it. A tick can be checked only while someone keeps its spec.
- **C.** A spec becomes a record in the store, and a tick cites that record's id. That belongs with standing ("a store carries its vertices"). A does not block it.

**The store name selects nothing in a tick.** `covers` already says which records were folded. A check runs the spec's `kind`, `about` and `of` tests and its slots over those records, and never asks which store. The name stays in the hashed text, so two machines write one tick only when they hold the same spec text.

**A second choice inside this one.** Should the hash leave out the vertex's name and the store names, so that it names only what the fold does? I recommend no: one written form, already pinned, is simpler than a second form made for hashing.

### TK4: `covers` lists every record that was folded

**Question.** What does `covers` hold?

**Proposal.** The ids of every record the fold was given, each once, sorted. The result is what the spec gives when run over exactly those records and no others.
`loops tick` gives the fold what the store holds, or a cut of it: all of it, and not only the records the spec takes. In the example the two are the same six, because the store holds nothing else. The second example below shows why they can differ.

**Example: a late record.** The NAS holds the six records and a seventh, a pihole backup that reached only the NAS.
- `loops fold backups.json` on the NAS today: `runs` 7, `mb` 1031, hash `res:pm7t…`. That is the NAS's present, not the tick.
- Folding only the six that the tick covers, on the NAS: `runs` 6, `mb` 1027, hash `res:d8eu…`. The same hash as the laptop's. I ran this today by folding a store that holds only the six.

So the list is what makes the tick mean the same on both machines.

**Example: a record the spec does not take.** jellyfin's disk is replaced and you write a record of another kind over its first backup: `loops emit retired "this backup is void" --prev r:kvvv…`. A second spec, `current`, counts the backups that nothing has replaced. Run today:

| Folded | `current` | `mb` |
| --- | --- | --- |
| The store: the six backups and the `retired` record | 5 | 615 |
| Only the six records the spec takes | 6 | 1027 |

The spec takes backups only, and its answer still depends on the `retired` record. A tick whose `covers` held only the six would be a true statement about those six, and it would not be what `loops fold` shows you. So `covers` holds the `retired` record too.

**Example: a tick over ticks.** You make a tick for each night, T16 and T17, and a weekly tick over them and one note.
- The weekly tick's `covers` holds three ids: T16, T17 and the note. It does not hold the six backup records.
- T16 and T17 are folded as records. The weekly fold reads what a spec can read of a tick ([TK6](#tk6-what-a-spec-reads-of-a-tick)) and never opens them up.
- Checking the weekly tick needs those three records, not the six.
- The weekly tick is therefore not a tick over the six. It is another fold, with another result.

This is Neon's subagent example: an orchestrator's tick covers two subagent ticks and its own three records, five ids and not 98.

**What `covers` never holds.** "Everything tick X covers, and these too." It is shorter, but a check must then walk back through X, and what X names, to the start. All three agents reject it.

**How the set is chosen in step 4.** `loops tick` would take one of: every record the store holds; the records as of a time (by the ruled rule, section 2 of the cut note); or the ids you name, with what they build on. Whichever it is, the tick lists the ids outright. An empty list is allowed: "the monitor saw nothing all night" is a tick.

**The cost is size.** An id in the list is 57 bytes. Any tick made in homelab-audit lists all 9,053 ids, 516 KB, also for a spec that takes ten records. Two ways make ticks small, and neither changes the record:
- Name the records: a tick for each night over that night's records, and a weekly tick over the nightly ticks, as above.
- Cover only what the spec's answer depends on. The law that says which records those are is on main (`spec.slice`), and margins-claude's Python computes the set. The Rust does not yet. I would build it straight after step 4's exit, not in it.

**Options.**
- **A (recommended).** Every id, outright.
- **B.** Ids, and "what tick X covers": rejected above.
- **C.** A hash of the set and no ids. Tiny. Then no one can say which records were folded, a late record cannot be seen, and an emit cannot use the tick as its base.

### TK5: which covered records were read without a payload

**Question.** A covered record may be held without its payload: erased, or never sent. How does the tick say which were read that way?

**Example.** On a copy that holds the six records, I erased jellyfin's first backup (`r:kvvv…`) and folded the same six:

| | `runs` | `mb` | `list` | Hash |
| --- | --- | --- | --- | --- |
| Payload held | 6 | 1027, 0 unread | the six ids | `res:d8eu…` |
| Payload erased | 6 | 615, 1 unread | the same six ids | `res:jv9h…` |

Same spec, same six ids, two results. You ruled on 2026-11-20 that the tick must say which covered records were read open, so that one set of inputs never names two results.

**Proposal.** A list, `withheld`: the covered ids that were folded without a payload, sorted. It is left out when it is empty, so most ticks have no such member. The second holder's tick holds `"withheld": ["r:kvvv…"]`.
- The spec's hash, `covers` and `withheld` together are the tick's inputs. Equal inputs give equal results; the fold laws already say so.
- Erased and never sent are one state here, as you ruled on 2026-11-10.
- The list has one right content: exactly the covered records that were folded without a payload. When `covers` holds a record and the record that erases it, the erased one is in `withheld`. A tick that leaves it out says the same thing in other bytes, so it is not a tick, and the check says so.

**What a checker with other holdings does.**
- The tick says `r:kvvv…` was withheld, and the checker holds its payload: the checker leaves that payload unread and checks.
- The tick read all six open, and the checker lacks one payload: the answer is "cannot check" ([TK7](#tk7-checking-a-tick)). The checker can write its own tick, with `withheld` stated. That is another tick, because its inputs differ.

**Options.**
- **A (recommended).** `withheld`, left out when empty.
- **B.** A list of the records read open. It doubles nearly every tick.
- **C.** Each entry of `covers` is an object, `{"id", "open"}`. Then `covers` no longer reads as a set of ids to a spec ([TK6](#tk6-what-a-spec-reads-of-a-tick)).
- **D.** Nothing is stated. That is against the ruling.

The name is yours to change. `withheld` is the word a reading already prints.

### TK6: what a spec reads of a tick

**Question.** When a tick is itself folded (a weekly tick over nightly ticks, an orchestrator over subagents), what can the spec read of it?

**Proposal.** Nothing new. A tick is a record, and its payload is read by the rules you set on 2026-11-21:
- `spec-hash` and `result-hash` are text: `is`, `not` and `by` work on them.
- `covers` and `withheld` are lists, so they read as sets: `is` asks "does it cover this record", and `by` groups a tick under each record it covers.
- `spec` and `result` are objects: present, with no value a test or a sum can read.

**Example, run today.** I wrote three ticks by hand into the laptop's store: T16 (the three records of the 16th), T17 (the three of the 17th) and TJ (jellyfin's two). Then I folded a spec over the ticks:

| Slot | Asks | Answer today |
| --- | --- | --- |
| `count` | How many ticks | 3 |
| `count` where `covers` is `r:kvvv…` | Which cover jellyfin's first backup | 2 (T16 and TJ) |
| `count` by `covers` | How often each record is covered | jellyfin's two records: 2 each; the other four: 1 each |
| `count` by `spec-hash` | Ticks of each spec | 3 under `spec:gs9s…` |
| `sum` of `result` | A total across the ticks | 0, with 3 unread |

**What this gives.** A weekly tick can count and list the nightly ticks, find the ticks that cover a record, and find records covered twice.

**What it does not give.** It cannot add up `mb` across the nightly ticks. The number 503 sits inside `result`, and the rules read no member inside an object. It is also written as the text `"503"`, and a sum does not read text.
The weekly total today is a tick over the six records themselves: 1027.

**Options.**
- **A (recommended).** Nothing new in step 4.
- **B.** Two additions: a field name that reaches inside an object (`result.mb.value`), and a sum that reads a number written as text. Each changes what every spec answers, not only for ticks. Numbers as text is already coming with the import. I would take B up when a real spec needs a total across ticks.
- **C.** The tick repeats some numbers at its top level as JSON numbers. A sum past 2^53 does not fit in one, and someone must choose which numbers.

**A caution for B, whenever it comes.** TJ overlaps T16 and T17. A total across the three would add jellyfin's backups twice: 503 + 524 + 831 = 1858, against 1027. The `by covers` slot above is how a spec sees that overlap.

### TK7: checking a tick

**Question.** How is a tick checked, and does a store refuse a tick that fails?

**Proposal.** A verb, `loops tick --check <id>`, with three answers:

| Answer | When |
| --- | --- |
| `same` | The store holds every covered record, with a payload for each one the tick read open. The fold gives the tick's result, byte for byte. |
| `differs` | The same, and the fold gives another result. Both hashes are shown. |
| `cannot check` | Covered records are not held, or payloads the tick read open are not held. It says how many of each. |

A record that is not a tick is refused with a reason: for example its `result-hash` is not the hash of its `result`. The refusal codes are chosen when it is built, under [W4](#w4-the-refusal-for-input-that-is-wrong-twice).

**Example** (the verb is not built; this is what it would answer). The laptop writes the tick over the six records and the NAS has synced.
- `loops tick --check` on the NAS: `same`.
- On a machine that holds four of the six: `cannot check: 2 covered records are not held`.
- On the copy where `r:kvvv…` is erased: `cannot check: 1 payload the tick read is not held`.

**Step 4 is done when** the first of those answers `same` on a second machine. Put the other way: the NAS makes its own tick over the same six, and its payload is the laptop's, byte for byte.

**What the check does not do.**
- It does not refuse a tick at the door. `import` and a sync take a tick that would answer `differs`. A store that lacks the covered records cannot check, so a rule at the door would depend on what had arrived first, and two copies of one store could then differ. Whose ticks to believe is a question for standing (step 3).
- It checks one level. For the weekly tick it folds T16, T17 and the note. It does not check T16 itself.
- It does not say that the covered records were all there was. It reports two things beside its answer:
  - Whether `covers` holds everything its own records build on. That is read from the covered records alone, so it is the same on every machine. A tick for which it is "no" is still a tick; it is a statement about a set that no one need ever have held.
  - How many more records the spec would take from this store now: "1 record the spec takes is not covered". That is the late record of [TK4](#tk4-covers-lists-every-record-that-was-folded). It counts only records the spec takes. It misses a late `retired` record, which the spec does not take; seeing those needs the function TK4 leaves for after the exit.

**Options.**
- **A (recommended).** The verb, three answers, and no rule at the door.
- **B.** A store refuses a tick whose result it can recompute and finds different. Rejected above: the answer would depend on the order of arrival.
- **C.** No verb. To check a tick you make your own and compare.

### TK8: the name of the window field

**Question.** You ruled on 2026-11-20 that a window is a `where` test on the time that content order already uses: a record's `occurred` when it states one, and its `emitted` when it does not. The field's name was left open.

**Example, run today.** The NAS holds eight backup records. Seven state `occurred` (the 16th or the 17th). Tonight's states none. All eight were written today. "The backups of the 21st":

| Test on | Bounds 21 Nov to 22 Nov | Why |
| --- | --- | --- |
| `env.occurred` | 0 | Tonight's record states no `occurred`, so it passes neither bound |
| `env.emitted` | 8 | Every record was written today, the backfilled ones too |
| the window field | 1 | `occurred` where stated, otherwise `emitted` |

Only the third is the answer you meant.

**Options.** `env.when` (recommended), `env.at`, `env.time`, `env.span`.
The field takes the tests that `env.occurred` takes, and compares the start of the span as it does. Only the name is asked here.

### TK9: which rules a tick was folded under

**Question.** A spec's text can stay the same while its answer changes, because the rules changed. How does a tick say which rules it was folded under?

**Example.** This happened today. "The smallest number of snapshots any host has", a `min` of `n` over four backups:

| Binary | Answer |
| --- | --- |
| This morning's | 0, with 0 unread |
| Now, after your ruling PV2 | 9, with 2 unread |

The spec did not change by one byte. Had a tick been written this morning, a check tonight would answer `differs`, and its writer was honest.

**Proposal.** The spec's hash is made with a label, `loops/spec-hash/v1:`. That label is the version of the rules too. When a change alters what any spec answers, the label becomes `v2`, and every spec's hash changes with it.
- A check that meets a tick made under `v1` answers "cannot check: made under older rules". It does not answer `differs`.
- A change of that sort is one that comes to you under your ruling of 2026-11-21 (W7), so each new version is ruled, never slipped in.
- The design page already says this: "the spec hash pins the fold-vocabulary version".
- Nothing is at `v1` yet. `v1` will mean the rules as they stand when the first tick is built.

**Options.**
- **A (recommended).** The label in the spec's hash is the version. No new member.
- **B.** A member of its own, `"rules": "1"`. It can be read without trying each label. It is one more member to keep true.
- **C.** Nothing. A tick made before a change of rules reads as a false tick after it.

margins-claude raised this.

### What follows from these, with no ruling asked

- **A reader's tick and a writer's base are one record used two ways.** You ruled that an emit may state a base (the cut note, section 8). `loops emit task --of task:42 status=done --base <tick>` reads the tick's `covers` and names the records of `task:42` in it that were live there. The tick needs no extra member for this.
- **An inbox's read marker is a private local file, not a tick.** All three agents agree. A tick is for a boundary worth citing: a handoff, a sign-off, a subagent's finished task.
- **A base that covers nothing of the thing.** If the tick you name as a base is a tick over ticks, it holds no record of `task:42`. The emit then names nothing, and stands beside whatever the store holds of the thing as a second head. That is honest: the base knew nothing of the thing. `loops emit` says "the base covers no record of task:42". margins-claude has the law for this ready: it holds for any set, closed or not.
- **Ticking again.** When a late record arrives, a new tick names the old one in `prev`, as any revision does. A record with `prev` states what it is of, so the two ticks are records of one thing: the first tick's id, or a name such as `tick:backups:2026-11-16`. No new rule.

**Not in this proposal:** boundaries and markers, the runner, where a tick lands (`into`), a record on the tick of how its set was chosen (the window's span, "by reference"), smaller `covers` by the slice laws, parameters, and pins.

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
| A | Payload values, the tick record and a first tick, adoption, then standing with sharing | After ticks |
| **A, in parallel (recommended)** | As A, and the adoption pieces that touch only the CLI (`ls`, the prefix scan, the plain view, reads across stores, the shadow import) are built beside the tick work | Piece by piece, from now |
| B | Your order of 2026-11-10 as it stands: ticks, then standing with sharing. The adoption pieces stay "later work". | After the two largest steps are built, with no daily use to test them |
| C | Adoption first, then ticks | Soonest. Ticks, which Recall and handoffs between agents need, wait. |

I lean to A because the tick work is in motion: the laws are in the model and the codec is built. C is a fair choice too; the import needs nothing from ticks.
Neon and margins-claude both support an adoption step before standing.

**Neon's comments, and what I make of them.**
- *A or C, never B. If A, keep step 4 tight to its exit and let the CLI-only adoption work run in parallel.* I agree, and it is now the recommended row. The CLI pieces touch nothing in the model, a Codex worker can build each, and their choices are implementer choices under your ruling of 2026-11-21. The tick proposal is mine to write and does not wait on them.
- *Do not let issue #8 gate adoption.* Issue #8 is the law for a reader that caches. You ruled on 2026-11-11 that it is step 4's first law. No caching reader is being built, since the read path is fast enough without one. I agree with Neon: the law should be stated when a caching reader is built, and should not hold up step 4's exit. That changes your ruling of 2026-11-11, so it is part of this entry: **say whether issue #8 leaves step 4.**
- *The shadow import should write a few large segments, not one per record.* Agreed. It is how the import would be built anyway (`loops import` or `emit -`).

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
- The file is named for the version of the checking rules that wrote it. A newer, stricter `loops` does not trust what an older one checked.
- The file is never inside `.loops/`. A store that came from a clone or a shared directory could otherwise bring its own "already checked" list.

**Options.**
- **Yes, in the adoption step (recommended).** That is when the first large store exists.
- Yes, now.
- No: every read checks everything, as today.

**What it buys.** Neon estimates a read of 9,000 records goes from about 0.6 s to under 0.2 s. That is an estimate, not a measurement.

**Neon's comment.** My first version of this entry left out the bullet about the version of the checking rules. Neon is right that it is required; it is added above.

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

**What B risks.** When Lean and Rust disagree, no third implementation says which is wrong; the model wins by definition.

**Neon's comments, and what I make of them.**
- *The first version of this entry overstated the risk.* It said a misreading shared by Lean and Rust would go unseen because the same sessions write both. Neon is right: the same sessions write the JavaScript too, so it never guarded against that. The implementation written apart from us, from the prose, is margins-claude's Python, and it stays under every option. I have taken the sentence out.
- *Updating JavaScript once for payload values, then freezing at step 4, is a clean trial.* Agreed; that is what B says.
- *If B holds, the workbench page should use the Rust core compiled for the browser, not the JavaScript.* Agreed. The "workbench page" row in the roadmap changes when you rule B.

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
- **C (recommended, with Neon's guardrail below).** One answer for every input, in the order the model checks. The implementer picks that order and notes it in the changelog. It does not come to you.

**Why not B.** The fuzz compares answers for equality today. Under B it would need to know every code that applies to an input, which is more machinery than following the model's order. Or it accepts "both refused" and no longer sees an implementation that refuses for the wrong reason.

**Neon's comment, and what I make of it.** Neon accepts C and adds a guardrail: keep the exact order as a rule inside this repository, between Lean and Rust, and do not write tables of tie-breaks into `protocol/*.md` that bind a reader written from the prose. I agree, and would rule it this way for new steps:
- The protocol text names each refusal and the one fault that causes it.
- The vectors for a new step hold inputs with one fault each.
- For an input with several faults, the protocol says only that it is refused with one of the codes that apply. Inside the repository the model's order decides which, and the fuzz checks Lean and Rust for the same answer.

The cost: two readers written from the prose may name different codes for an input that is wrong twice. Both refuse it. The orders already pinned for records and specs stay as they are.

### W5: the mutant check on every merge request

**What it is.** The mutation catalogue holds 314 deliberate faults, each tied to an exact line of the model. Every merge request runs a quick check that each one still finds its line, and that a sample of 33 are caught. The full run is separate and on request.

**Example.** Making `value` and `n` ordinary fields changes about 70 places in the model. Some of the 314 faults point at those lines.
- **With the check (today):** the same merge request must re-point them, or the pipeline is red.
- **Without it (Neon):** the merge request is smaller. The next full run reports those faults as not applying, and someone re-points them in a batch, later, away from the change that moved them.

**Options.** Keep it, or move it to the full run. **Recommended: keep it.**

**Measured, 2026-11-21.** The `value` and `n` change was the test: it touched 218 files and the model's record itself. 13 of the 314 faults needed work (8 re-pointed, 5 rewritten, none retired), and it took under five minutes, four of them the runner. So the check cost almost nothing on the largest model change so far, and it caught the 13 in the merge request that moved them. I no longer have low confidence: keep it.

**Neon's comment, and what I make of it.** Neon agrees, and adds: write no new mutants for step 4 until ticks have survived real use. So far every piece has added its mutants in the same merge request (nineteen for the result codec). I agree in part: while the tick record's shape is still moving, mutants tied to its lines would be re-pointed again and again. I would add step 4's mutants once its exit is met, a content tick recomputed to the same hash on two machines, and not wait longer than that. **Say whether step 4's mutants wait for its exit.**

### W6: generated pages in the repository

**Question.** Neon proposes that the generated HTML under `out/` stop being committed and be built in CI.

**Example.** A merge request that changes one law shows the Lean change and, below it, the regenerated `out/` pages. A reviewer scrolls past generated HTML.

**What stands in the way.** Your site's sync script (`projects.gruel.world/scripts/sync.sh`) refuses to copy output that is not committed: "so every hosted revision is a commit you can check out and regenerate".

**Options.** Keep committing `out/`, or change the site's sync to build it. **Recommended: keep.** The rule in the sync script is a good one and the cost is noise in a diff.

**Neon's comment.** Keep, and mark `out/` as generated in `.gitattributes` so that a merge request folds those files away. That costs nothing and suits either ruling, so it is done (2026-11-21).
The other half of Neon's point, counts kept by hand in the roadmap, is done: the roadmap no longer holds them.

## Being prepared

Nothing here needs you yet. It is listed so that you know what is coming.

**The tick record** is no longer here: it is proposed above as [TK1 to TK9](#the-tick-record).

**Numbers stored as text**, with the import ([W1](#w1-an-adoption-step-before-standing-and-sharing)). [TK6](#tk6-what-a-spec-reads-of-a-tick) shows one use for it.

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
| 2026-11-21 | Payload values, PV1 to PV7: the reading is stated in the model; `value` and `n` are ordinary fields; `present` means written; a list reads as a set under `is`, `not` and `by`; a list is never a number | Changelog; [the payload values note](notes/2026-11-21-payload-values.md) |
| 2026-11-21 | A result hash has its own prefix, `res:`; the codec's other choices confirmed | Changelog |
| 2026-11-21 | Only a choice that changes bytes or answers comes to Kyle; other implementer choices stand unless he objects | Changelog |
| 2026-11-21 | The 20 built keyed-fold choices confirmed | Changelog |
| 2026-11-21 | Edges: three classes, no change to the envelope; an edge's payload holds `rel`, `from`, `to` and `state` | Changelog |
| 2026-11-21 | Issue #10 closed with no strict "after" operator | Changelog |
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

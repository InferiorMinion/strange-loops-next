# Store descriptions: what the first two conversions asked for

**Written 2026-11-23, at Kyle's request, as the opening input for the question being prepared in [DECISIONS.md](../DECISIONS.md#being-prepared): what a store says about its kinds, without enforcing it, and what of the old `.vertex` and `.loop` files comes forward.** Nothing here is ruled or put to Kyle. (On 2026-11-24 the questions of sections 2 and 5 were put, as SD1 to SD3 in DECISIONS.md.)

It draws on two pieces of work done that day.

- **The homelab repository's conversion** ([the changelog](../CHANGELOG.md), 2026-11-23). Its agent instructions, a skill and an operations page write with the new tool. Its changelog generator, its session-start hook and the SRE agent's history search read the live stores, about 1,860 records in the three stores they use. The coordinator did the work and wrote this note.
- **A collector prototype**, built from nothing by Kyle and Neon. It curates 1,994 items out of 2.77 million from an outside source into one fresh store, in one segment. A fold of it takes 0.12 s, and a fold across it and the homelab coordination store 0.215 s. A tick with no cut named (TK13) covers 83 of the 1,994. It met almost the same frictions. Its findings are folded in below, marked "(Kyle and Neon)".

## 1. What moved from the store to the command

The old tool kept much of what it knew in the store's `.vertex` file: the store's name, which field keys each kind, which fields are references, who writes there. The new tool keeps none of it, by design: a record's `of` names its thing, and a reader needs no vertex. The cost moved to the writer. One write from the repository's instructions, before and after:

```
sl emit <store> observation topic=<topic> message="..." rel=relationship/<agent>          (80 characters)

loops -C ~/.local/share/loops/<group>/<store> emit observation --of observation:<topic> \
  topic=<topic> message="..." --about observer:<me> --about relationship:<agent>         (166 characters)
```

The 86 more characters say four things the vertex used to say.

| What | The old tool | The new tool | Section |
| --- | --- | --- | --- |
| Which field keys the kind | The vertex's fold | `--of observation:<topic>`, and the key again as `topic=<topic>` | 2 |
| Where the store is | A name | A path, in every instruction | 3 |
| Who writes | The store's observer | An `observer:<me>` mention, by convention | 4 |
| What it refers to, and as what | `rel=`, a field that names its role | A mention, which names no role | 5 |

## 2. What a kind is: its key, its references, what to show, its lifecycle

**What happened.** Every write states its key twice, in `--of` and as a field, and nothing checks that the two agree: `--of deploy:$slug name=$slug`. A slip makes a second thing, silently. The repository's CLAUDE.md now carries a table, copied by hand from the old vertices, of each kind's key field: `topic` for decisions, observations and feedback, `trigger` for intentions, `agent` for relationships, `name` for most others, none for seals and change logs. That table is a description of the store's kinds, written as prose.

It is not the only one. Each reader keeps its own:
- **The session-start hook** hides a thread whose `status` is `closed`, `resolved`, `done` or `passed`. That is the old vertices' `lifecycle`, copied into a jq filter.
- **`estate.py`** keeps an 869-line `data/estate-schema.toml`: which fields are references, which field names mean the same field, the values a field may take, and units. It is the largest description of the stores in the repository, and no store holds it.
- **The prototype** needed the same facts on the reading side: the key, the references, which fields `ls` and a preview show first, and the lifecycle (Kyle and Neon).

**Proposed** (Kyle and Neon; the conversion agrees). A store describes each kind: its key field, its reference fields and the kinds they name, the fields to show first, and a lifecycle field with its values. The description only describes. Before standing, nothing is refused for breaking it. The writer uses it as shorthand: `loops emit deploy name=<slug>` fills in `--of deploy:<slug>`, and the reference fields add mentions (section 5). `of` is still written in full in the envelope, so a reader of the records never needs the description. `loops ls` and a viewer take their defaults from it.

**Open for the discussion.**
- Where the description lives: a file in `.loops/`, or records in the store. A record is signed, travels with the store once stores are shared (step 5), and has a history. A file is simpler and has none of these.
- Whether the writer warns when a stated field and the key disagree.
- How it meets standing's declarations (step 3). One reading: this is the descriptive half of a declaration, and admission is the other half. A strict store would then check writes against the same text.

## 3. A store found by name, on every verb

**What happened.** Since AD1, `read` and `fold` find the stores a spec names, through `--store`, a group's `stores.json` or the user's list. Every other verb takes only `-C <dir>`: `emit`, `ls`, `of`, `get`, `tick`, `check`, `export`. So every instruction, skill and script carries a full path. The changelog generator grew `--stores DIR`, and the hook reads `GRUEL_LOOPS_STORES`, so that tests can point each one elsewhere.

**Proposed** (Kyle and Neon). Every verb takes a store name and resolves it through the same lists. A name bound to nothing is refused with the existing `store-unknown`, so no new refusal code is needed, and no record changes.

**Open.** How it is spelled. If `-C` takes a name too, a name and a relative directory of the same spelling are ambiguous. A flag of its own avoids that.

**Settled on 2026-11-24, to be built with SD1 to SD4** (Kyle asked for it in the same pass; the coordinator, Neon, margins-claude and Astra agreed the form in the room). Under W7 it needs no ruling: no record changes and no refusal code is new.

- **Spelling:** `--in <name>`, before the verb, as `-C` is: `loops --in audit ls asset:`. `-C` keeps meaning a directory, and `--store name=dir` keeps meaning a binding.
- **What it moves:** only the store the verb works on. The lists are looked for, and relative paths are read, from where the command started or from `-C`, as without it. So every name in one command resolves from one list. The first proposal was "`--in` is `-C` with the resolved directory". margins-claude ran that on the homelab group: the list that resolved one store was then not found from inside it, and a relative spec path was read from the wrong place.
- **Resolution:** as `read` and `fold` resolve a name today, with nothing added. The user's list; over it, the first `stores.json` found at the store's root or its parent (outside a store, the starting directory only); over that, `--store`. A root's list hides its parent's whole, even when empty. A name bound to nothing, or to a directory that holds no `.loops` itself, is `store-unknown`: a named store is never found by walking up, as `-C` does. All of these are Astra's, from 16 checks on today's binary.
- **`.` in a spec** stays as AD1 has it: the store the verb works on, unless a list or `--store` binds `.` elsewhere.
- **`--store name=dir` is taken before the verb too,** read from where the command started or from `-C`, so that the override works the same on every verb (Neon).
- **A write through `--in`** (`emit`, `erase`, `tick`, `import`) says on standard error, before it writes, the name asked for, the store's directory and which list bound it. Standard output is unchanged. It then writes to the store it opened, without looking the name up again. This shows a wrong store; it does not prevent one. A write is not refused because two lists bind the name differently: a local list over the user's is how a test group works.
- **More than one store** is read only by `ls --refers-to` and `describe`. There `--in` may repeat, or `--group` reads every store that the local `stores.json` names: that list alone, never the user's, which may span groups. With no local list, `--group` is a usage error. With an empty one it reads no store and says so; it never falls back to the user's list. Every member is opened before anything is read, and one that cannot be opened fails the command, so a part is never given as the whole. Two names for one store count once. `--group` with `--in` is a usage error. Every other verb refuses a repeated `--in` and `--group` before it reads or writes anything; `tick` is among them, since it writes.
- **Not taken:** `init`, `whoami` and `shadow-import` refuse `--in` and `--group`; they do not ignore them.
- **A test:** for a verb with no file operand and no other store named, `--in X` gives the same bytes on standard output as `-C` with the directory X resolves to (margins-claude). `--in` reads AD1's `stores.json` and never a store's description, so SD1's test is untouched, and no vector changes.

## 4. Who wrote it, and the `--about` hazard

**What happened.** Every agent on the workbench signs with the one workbench key, so `observer` in the envelope is the same on every record. The conversion names the author with a mention, `--about observer:<name>`, as the import did for the old observers. Two things then go wrong.

- A stated `--about` replaces every earlier mention of the thing. An agent that names only itself on a revision drops all the thing's other mentions.
- With no `--about`, the writer copies the head's mentions, the previous author's among them. An agent that revises a thing Kyle wrote, and states none, writes a record that still reads as Kyle's.

The prototype met the first (Kyle and Neon). A collector's refresh of an item's tags would have wiped links a person had added by hand. The fix was to split the kinds so the two never write the same thing: an `item` the collector owns, and an `annotation` a person writes.

**Proposed** (Kyle and Neon), in two parts.

- **No second author field.** `observer` is the author field ([record rules](../protocol/record.md)). What is missing is a key for each agent, and that works today. `LOOPS_KEY_FILE` chooses the key a process signs with, so each agent can have its own, and `observer` then tells them apart. Naming a key, so that a reader shows an agent's name and not `k:…`, waits on name tables. Lineages and grants come with standing (step 3). Until then, a short list of which key is which agent, kept beside the stores, would serve. The imported records keep their `observer:` mentions, so history from before the switch reads as it does now.
- **`--add-about <thing>` and `--drop-about <thing>`** on a revision. They start from the copied mentions and change only the ones named. They are CLI only. A record written this way is one a person can already write by stating every mention. The slice laws are about what `prev` names, not about mentions (`Anchored` in `model/Laws/LiveSlice.lean`), so no law changes. Once authors are keys, the hazard is left with real mentions alone, and these flags cover it.

## 5. Roles on references

**What happened.** The old tool wrote a reference in two ways: a field with a role and its `_ref` twin (`owner=person/potter`, plus `owner_ref` holding the old fact's id), and a list with no roles in `ref`. AD4 made both into mentions and took the `_ref` fields and `ref` out of the imported payloads. The role field (`owner: person/potter`) was kept, so role and target are still there as text, in the old `kind/key` form. The mention carries the target without the role.

`estate.py` and `atlas_materialize.py` are the two readers that care about roles. They find edges by the `_ref` suffix, read against their schema file, and the imported records no longer have that suffix. That is the real reason they stay on the archive. Moving them means finding edges by the role fields the schema names, and turning `kind/key` into `kind:key`. A new path to the store is not enough.

**Proposed** (Kyle and Neon). `about` stays a flat set. E1 (2026-11-21) already says that one target under a role is a text field: `runs_on=asset:vm/media`. The kind's description (section 2) lists its reference fields, and the writer adds each one's value to `about`. A fold of what mentions `asset:vm/media` then finds the record, and a reader that wants the role reads the field. A relation with facts of its own stays an edge thing, as E1 rules.

**Open.** When a revision changes `runs_on` from one asset to another, the shorthand should drop the old target from `about` and add the new one, or the mentions only ever grow. That is the same work as `--drop-about` in section 4.

## 6. One time to sort by

**What happened.** The import gave every record the import's `emitted`, with the old time in `occurred`. New writes have only `emitted`. The changelog generator, the hook and the history search each sort by "`occurred`, else `emitted`": three copies of the rule, in two languages.

**Proposed** (Kyle and Neon). Reads show a computed `when`, `occurred` or else `emitted`, in `--values`, `ls --fields` and `of`. It is never stored in a record.

**Open.**
- `occurred` is a span: a date, a month, an instant or an interval ([record rules](../protocol/record.md)). Sorting by it needs a rule for spans, probably their start.
- A `when` that only a read's display shows changes nothing W7 sends to Kyle. One that a fold can use, as the field of a `latest-by`, changes what a spec answers and its result's hash, so that would come to him.

## 7. Writing in batches, and what replaced `.loop`

**What happened.** The prototype has no `.loop` file. A script reads the outside source and pipes JSONL into `loops emit -`, and that worked cleanly, except for revisions. Plain `emit -` writes each line as given, so a revision there must state its own `prev`. `emit - --fill` works out `prev` and the copied mentions, but its cost grows with the square of the batch. The prototype splits new items (`emit -`) from revisions (`emit - --fill`, in chunks) (Kyle and Neon).

Measured on main's binary in a fresh store: N new things with plain `emit -`, then one revision of each with `emit - --fill`.

| N | `emit -` | `emit - --fill` |
| --- | --- | --- |
| 250 | 0.01 s | 0.11 s |
| 500 | 0.02 s | 0.41 s |
| 1,000 | 0.05 s | 1.57 s |
| 2,000 | 0.07 s | 6.67 s |

Each doubling of N takes about four times as long. The cause is in `fill::changeset_with_base` (`rust/place/src/fill.rs`). For each row it builds `things::realm` over everything the store holds and every row prepared so far, and then uses only that row's thing. With a base, `fill_base` does the same for each row, over the selection.

**Proposed** (Kyle and Neon). Keep each thing's live state through the batch, updated as each row is prepared. Then `script | loops emit - --fill` is a whole collector's sink at any size.

**Conditions.**
- `realm` settles erasures before admission, and an erasure may name a record of another thing. The state kept for a thing must see erasures from the whole batch and store.
- The output must be byte for byte what it is today. The changeset tests and the import's byte equality with `emit - --fill` check that.

The change is in place code and the CLI only.

## 8. What the old setup hid

The conversion turned up faults the old setup had covered. None of them is about descriptions. They are here because a shared or checked store would have caught each one.

- **Two readings of one store disagreed.** The old changelog generator and the SRE agent read the SQLite tables directly, and saw 25 facts of kinds no vertex names that `sl read` never showed. The import counted them, and the changelog was the first reader to miss one.
- **Failures were silent.** The old session hook's warnings were lost in a subshell. Its read of the monitor store on another host had always exited 127, because `sl` is not on a non-interactive SSH's `PATH`. Both are fixed.
- **Git did more than it seemed to.** The SQLite files, committed, gave crude sharing between machines, an off-site copy, and data CI could check. The live stores are now outside git. That ended binary merge conflicts and clones that drift apart, and it lost all three. The changelog's check moved from CI to a pre-commit hook, Kyle's Mac stays on the old tool, and the live stores have no backup ([ROADMAP](../ROADMAP.md), row 5). Sharing is step 5.
- **The import's strictness has a cost.** Its journal pins `--include-unnamed`, so the 25 facts could not be added later: that takes a fresh import, with every id changed. It is the right default. It is also a reason to settle descriptions before the next real import, not after.

## 9. What works well

- **`loops of <thing>`** shows a thing's values, the record that set each one, its mentions, its heads and its count of records. It was the first thing to reach for whenever a reading was in doubt.
- **Refusals say what is wrong.** Run in the repository, `loops` answers "no .loops/ at or above" and does not mistake the old `.loops/` for a store. A name bound to nothing refuses the read. `--version` let each script tell the new binary from the old while both answered to `loops`.
- **The import was safe to repeat.** A dry run, a second run that writes nothing, and a journal that pins its flags made a 3-second re-import something to trust.
- **Every reader is a fold spec and a little sorting**, and none needs a vertex. The readers needed three shapes: `collect` for every record, `collect` with `follow: prev` for each thing's heads as written, and `latest-by: of` with `follow: keys` for each thing's values. They are in [the spec](../protocol/spec.md), not in `--help`, and each was learned from rex-tools' specs. A few lines in `--help` would spare the next adopter that.
- **Speed is no problem at these sizes.** With no index or salience, readers fold whole stores: 0.1 s for the changelog, 0.35 s for the history search over three stores, about 1 s for the hook, 0.12 s for the prototype. What is lost is a shared sense of what matters most. Each reader now ranks by time, on its own.

## 10. What each proposal touches

Under W7 (2026-11-21), a choice comes to Kyle only if it changes a record's bytes or id, a result's bytes or hash, a refusal code, or what a spec answers.

| Proposal | Section | What it changes | Comes to Kyle under W7 |
| --- | --- | --- | --- |
| Kind descriptions: key, references, what to show, lifecycle | 2 | Store metadata and the CLI | Not by W7: no record changes. Its form is worth his word, since standing's declarations may grow from it |
| Store names on every verb | 3 | The CLI | No: it reuses `store-unknown` |
| A key for each agent | 4 | Configuration (`LOOPS_KEY_FILE`) | No: it changes who signs, not how |
| `--add-about` and `--drop-about` | 4 | The CLI | No |
| Reference fields added to `about` | 5 | The CLI, through the description | No |
| A computed `when` | 6 | What reads show | Only if a fold can use it |
| `--fill` that keeps state through a batch | 7 | `rust/place` | No: the bytes stay the same |
| Named authors, lineages and grants | 4 | Step 3 | Yes, with standing |
| Sharing, and a backup of the live stores | 8 | Step 5, and the workbench | The backup is his to choose now |

None of the first seven changes a record's bytes, an id, a law or a vector. Only named authors (step 3) and sharing (step 5) reach later steps.

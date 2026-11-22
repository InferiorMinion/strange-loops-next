# The first product slice: a place and a CLI

Step 2 of the build sequence: emit, read and fold for one person, signed.
`loops-place` (`rust/place`) is the first code around the pure core that
touches a disk, a clock and a key. `loops` (`rust/cli`) is the CLI over it.
`loops_place::fill` holds what a single emit or a filled changeset writes into its envelopes.
The core gained nothing; `loops-eval` gained one export, `slots_json`.

None of this is protocol yet. The stored line is (`protocol/record.md`);
the directory layout, the CLI and the codes below are a working proposal,
written down here so they can be changed on purpose.

## What is implemented

```
loops init                          make .loops/ here
loops emit <kind> [field=text | field:=json]... ["message"]
      [--of <thing>] [--about <thing>]... [--no-about] [--prev <id>]...
      [--erases <id>]... [--occurred <when> | --no-occurred] [--under <id>]
      [--sets <key>]... [--sets-nothing] [--whole]
      [--base <tick-id>] [--base-id <id>]...
loops emit - [--fill [--base <tick-id>] [--base-id <id>]...]
                                    read one JSONL changeset from stdin
                                    --fill applies the single-emit filler to each line
loops --version                     print the strange-loops-next version
loops ls                            list kinds and their record counts
loops ls --things | loops ls <prefix>
                                    list things, optionally by literal name prefix
loops get <id>                      one record
loops of <thing>                    a thing's values, its about, its heads, how many records are of it
loops read <vertex> [--values]      fold .loops/vertices/<vertex>.json
loops fold <spec.json | -> [--values]
                                    fold a spec given as a file
loops tick <vertex | spec-file | -> [emit envelope flags]
                                    write a tick over everything held, with an optional base for its envelope
loops tick --check <id>             check a tick against this store and print the verdict and reports
loops erase <id>... ["why"]         emit an erasure, and drop the openings held here
loops export [--kind <k>] [--of <thing>] [--about <thing>]
loops import [<file> | -]
loops check                         verify every segment and line
loops whoami
```

* The verbs, payload syntax, labels, discovery and exit codes follow the
  design's CLI: `field=text` is text, `field:=json` a JSON value, a bare last
  argument is the payload's `text`; `.loops/` is found the way git finds
  `.git`; exit 0 ok, 1 refused with its code, 2 usage, and 3 when the file
  system or the system's randomness fails.
* Every write goes through `batch::resolve_batch`, so one record and a
  labelled changeset meet the same checks, `prev-unknown`, `under-unknown`,
  `of-not-root` and `prev-unrelated` included. The batch check reads a held
  record's `of` and `prev` from its stored envelope (`Held::writer_view`), so
  a record whose payload is erased or withheld is held as any other. A
  changeset writes nothing unless every draft is taken.
* An id can be given by any prefix that names one held record, in `--prev`,
  `--erases`, `--under`, `--of` and the `of` verb. A thing with a colon that is not
  `r:` is a natural key, taken as typed.
* A record is of one thing (D8): `--of` names it, and a record without it is
  a root, its own thing. `--about` only mentions: it never makes a head, and
  `loops of`, `latest-by` and the heads read the thing, not the mention.
  `export --of` takes the records of a thing and `export --about` those that
  mention it, as a spec's `from.of` and `from.about` do.
* A revision written without `--of` is of what it replaces; see choice 9.
* `--sets` names a key the emit sets; see choice 10. An emit of a thing the
  store holds a record of sets the fields it names, unless `--sets`,
  `--sets-nothing` or `--whole` states otherwise; see choice 15.
* Without a base, the writer claims to have read everything held. With a base,
  the filler starts from the tick cover, the literal ids and any stated
  `--prev`. It follows held predecessors within the thing and adds that
  view's live records to the stated `prev` (choices 11 and 23).
* An emit of a thing carries the thing's whole `about`, copied from its heads
  unless `--about` or `--no-about` states it; see choices 12 and 13.
* An emit copies `occurred` when every head states the same text, unless
  `--occurred` or `--no-occurred` states it. See choice 19.
* A plain `emit -` changeset states its own `of`, `sets`, `prev`, `about`
  and `occurred`. `emit - --fill` fills each line against the store and the
  records prepared before it. The changeset is written in one segment.
  See choice 20.
* `loops of` shows a thing's values, each key's from its live setters, beside
  its `about` and its heads; `read` and `fold` write the values beside the
  ids with `--values`. The values view is the CLI's, not the protocol's: a
  fold answers ids, and without `--values` `read` and `fold` answer exactly
  that. See choices 16 and 17.
* Every answer is JSON or an id. Lenses are a later step, so `--json` is accepted and changes nothing.
* `read`, `fold`, `of`, `get`, `export` and `tick --check` answer from the lines that check,
  and are then refused `store-not-clean` when a line was refused or a
  segment is altered, as `check` is; `emit`, writing a tick, and `erase` warn and go on. See
  choice 18.

## On disk

```
.loops/segments/<key>/<hash>.jsonl    closed, immutable, named by the SHA-256 of its bytes
.loops/vertices/<name>.json           a spec, as protocol/spec.md reads it
~/.config/loops/keys/device           the signing key's seed, 0600 (LOOPS_KEY_FILE, or LOOPS_KEY as text)
```

A segment holds the lines one key signed in one changeset. Writing it is a
rename into place, so a reader sees a whole changeset or none
(`protocol/laws.md`, torn reads). Nothing appends to a segment. Two copies of
a store merge by adding files, in git or by `export | import`, and what they
hold is the union.

Reading checks every line: the envelope and the writer's checks, the id
against the envelope and commitment, the signature against the id, and the
canonical spelling. A line that fails is listed by `loops check`, never
held. A segment whose name is not the hash of its bytes is listed as altered.
Every verb that reads the store says when either happened (choice 18).

## Choices made here, each cheap to change until stores exist

1. **One closed segment per changeset; no `open.jsonl`.** The design has an
   open segment that a git hook closes at commit. Closing per changeset needs
   no hook and makes a changeset atomic, at the cost of one small file per
   emit. The review finding "closing segments per changeset" stays open:
   compaction, or the open segment, can come when the file count hurts.
2. **`segments/<key>/`, not `segments/<observer>/<device>/`.** Lineages are
   step 3. Until then the signing key is the only writer identity a place can check.
3. **The observer is the signing key, `k:…`, and nothing else.** A line
   proves which key signed it. Nothing yet says which keys speak for an
   observer, so the only observer a key can honestly write as is itself, and
   `emit` refuses any other (`observer-unbound`). `--observer` and a draft's
   `observer` are accepted so the refusal can be said. `loops check` counts
   held lines whose observer is not their signing key as `unbound`: imported
   lines, which a reader holds but cannot attribute. Lineages, anchors and key
   bindings (step 3) replace this rule.
4. **No anchor file and no store identity.** A store is a lineage with
   declarations; that is step 3. `from.store` in a spec is carried and not
   read: a fold reads the one store it is run in.
5. **Admission permits every replacement** of a held, related record
   (`may = true`), as `Spec.run` does. Who may replace whom is standing's rule.
6. **How a payload becomes what a fold reads.** `Line::record` calls
   `loops::record::payload_fields`, the port of Lean's `payloadFields`.
   Each member of a held object becomes a scalar, a list of nameable members,
   or a written object/null. A held non-object supplies no fields. The stored
   opening retains the original JSON for `loops of` and `--values`.
   The direct `payload/` vectors also run through stored lines.
7. **New refusal codes**, outside the protocol's lists: for a line,
   `line-not-json`, `line-shape`, `commit-invalid`, `id-mismatch`,
   `key-invalid`, `signature-invalid`, `line-not-canonical`; for the CLI,
   `observer-unbound`, `id-unknown`, `id-ambiguous`, `vertex-unknown`, `draft-shape`,
   `draft-field-unknown`, `spec-not-json`, `store-not-clean`.
   In an `emit -` changeset these cover only a line's own shape: `draft-shape`
   for a line that is not a JSON object or a label that is not text,
   `draft-field-unknown` for an unknown member, and `member-duplicate` for a
   repeated one. A line's envelope fields go through the core's reader, so a
   field left out or of the wrong JSON type gets the field's own code in the
   writer's order across the changeset, after label refusals; a salt that is
   not base32hex text is `salt-not-16-bytes`. The CLI fills in `observer` and
   `emitted` only when a line leaves them out. `observer-unbound` judges only
   an observer that is a typed reference, so a mistyped or untyped one, on a
   line or in `--observer`, is the writer's `observer-not-typed`.
   Kyle chose this on 2026-11-11.
   The flat line has no envelope member, so it has no envelope that is not an
   object.
8. **Erasure removes bytes only here.** `loops erase` emits a record with
   `erases`, which every reader applies, and rewrites the local segments that
   held the opening. Other copies, and git history, keep the opening until
   they purge too.
9. **`emit` fills in `of` for a revision; the protocol never infers it.**
   Every record with `prev` states `of` (`of-missing`). A revision written
   with `--prev` and no `--of` is given, at write time, the thing of what it
   replaces: the target's `of`, or its id when it is a root. The value is
   written into the envelope, and nothing reads it in later. When the CLI
   can't fill it in, it refuses with the batch's codes: a target this store
   doesn't hold is `prev-unknown`, since there is nothing to fill `of` in
   from, and targets of different things are `prev-unrelated`. On this path
   only, these precede the writer's other checks. This replaces the earlier
   "a revision without `--about` is about what it replaces". Plain `emit -` fills
   nothing in: a changeset states `of` itself, and may use a label there,
   as in `about`, `prev` and `under`.
   Kyle confirmed on 2026-11-10 that only the single-record path fills `of`
   in, that this path refuses `prev-unknown` itself, that `export --about`
   is a mention filter and `export --of` a thing filter, and that the verb
   that shows a thing is `loops of`, which replaced `loops about`.
10. **`--sets <key>` once per key, and `--sets-nothing` for an empty set.**
    `sets` holds key names, so `--sets` takes a key as typed, as `--about`
    takes a mention, and repeats. `--sets-nothing` states `"sets": []`, an
    emit that sets no key. The two together are a usage error. `--sets ""`
    is the writer's `sets-not-keys`. Either flag states `sets` as given, and
    nothing is filled in over it; an emit with neither is choice 15's.
    `emit -` takes `sets` as a line's envelope field, as written; a key that
    starts with `@` is a key, not a label.

11. **`emit` fills in the anchor.** The anchor rule of D8c
    (`notes/2026-11-11-declared-keys.md`, section 5): every emit sets the envelope, so an emit of
    a thing names in `prev` every record of that thing the store holds that
    is live for some key, the envelope among them, and so every head. The
    thing is `--of`, or what `--prev` replaces (choice 9); an emit with
    neither is a root and names nothing, and the first record of a thing the
    store holds nothing of names nothing. Liveness is the core's
    (`loops::live::live`), read over the store as `loops fold` reads it:
    settled, then admitted. An erased or withheld record keeps its envelope
    and is named like any other; a record admission refuses is not named. A
    `--prev` is kept beside what is filled in, whether or not it is live, and
    `prev` is written as the canonical set it is. A whole-read emit names the
    same records and is then the only live record of its thing. Nothing
    refuses an unanchored record, on either path, and nothing reads the
    names in later. `loops erase` writes a root, with `erases` and no `of`,
    so it names nothing; `--erases` and `--under` on an emit change nothing
    here, and neither does a thing named by a root's id rather than a natural
    key: the root is that thing's first record, and is named while it is live.
    What was filled in is not printed: `emit` answers the id, and `loops get`
    shows the `prev` and `about` written.
12. **`emit` copies `about` from the heads.** The head's mentions are the
    thing's, so with no `--about` an emit of a thing is given the union of
    the mentions of the thing's heads, every one of which it now names, in
    canonical order. A root and a first record copy nothing.
13. **`--about` states the whole `about`, and `--no-about` an empty one.**
    `--about`, once or more, replaces the copy rather than adding to it, so
    a task moves from one project to another by naming the new one alone.
    `--no-about` states that the emit mentions nothing, and the thing then
    mentions nothing until an emit names something again. The two together
    are a usage error, as `--sets` with `--sets-nothing` is.
14. **The filler is in `loops-place`.** `loops_place::fill` fills in `of`
    (choice 9), then `sets` (choice 15), then the anchor and `about`
    (choices 11–13), over what a store holds, and `loops emit` calls it
    before the batch check. It sits beside the CLI's other uses of a store so
    a test can drive the writer's rule over a few hundred emits without
    starting the binary for each.
15. **An emit of a held thing sets the fields it names; `--whole` states a
    whole-read emit.** Kyle ruled on 2026-11-15 that re-emitting
    `status=done` updates `status` and keeps the other fields. So on the
    single-record path an emit of a thing the store holds a record of, which
    states neither `--sets` nor `--sets-nothing`, is given the top-level
    payload keys it names as its `sets`: each `field=text`, each
    `field:=json`, and `text` for the bare message, in canonical order. The
    thing is `--of`, or what `--prev` replaces (choice 9); held means a
    record of it is in the store as `loops fold` reads it, settled and
    admitted, an erased or withheld one included. `--whole` states no `sets`:
    the emit is read whole and replaces everything, as choice 11 says. It is
    a usage error beside `--sets` or `--sets-nothing`.
    A root, and the first record of a thing the store holds nothing of, are
    not emits of a held thing: they keep no `sets` and are read whole, so they
    set every key they name. `--whole` changes nothing there.
    An emit of a held thing that names no field is given `"sets": []`, a pure
    join, as `--sets-nothing` gives: it moves the thing with `--about` or
    `--no-about`, or joins its heads, and sets no key. A whole-read emit of
    nothing, which unsets every key, takes `--whole`. `--erases` and
    `--under` change nothing here. The payload on this path is always an
    object, so the keys it names are always its members; a repeated field is
    the writer's `member-duplicate`. Plain `emit -` fills nothing in.
16. **`loops of` shows the thing's values.** It answers `{"thing", "about",
    "heads", "records", "values", "setters"?, "unheld"?}`. `about` is the union
    of the heads' mentions, as an emit copies it (choice 12). `heads` are the
    thing's heads as shown lines, newest first, so the first is the winner a
    latest-by that follows `prev` names and the rest are what `others` was;
    `records` counts the records of the thing. The values are read from the
    live setters, as `loops::live` gives them over the store settled and
    admitted:
    - the keys are those some live record declares and the top-level keys of
      each held live record read whole; a record with `sets` gives only the
      keys it declares, so an undeclared payload key is not shown;
    - `values` maps a key to its value when its one live setter is held and
      holds the key; a key whose one held setter lacks it is unset, and not
      written;
    - `setters` maps every other key to its live setters, newest first, each
      `{"id", "shown", "value"?}`: two or more setters, each with its value
      and none picked or merged, or a setter whose payload is erased or
      withheld, which shows its `shown` and no value. Nothing compares values;
    - `unheld` lists the live records read whole that are erased or withheld,
      `{"id", "shown"}`: what keys they set beside those written is unknown.
    `setters` and `unheld` are written only when they hold something.
17. **`--values` writes the values beside the ids.** `loops read <vertex>
    --values` and `loops fold <spec> --values` answer the protocol's result
    with members added where the protocol has none: each thing of a
    latest-by that follows `keys` gains `about`, `values`, `setters` and `unheld`
    as `loops of` writes them. Its `about` is empty when no head mentions
    anything. Each entry in its `heads` gains `env`, the stored envelope
    that `loops get` writes. Entries in `keys` and `whole` gain nothing. Each
    `{"id", "shown"}` entry that a latest-by following `prev` or a collect
    lists gains `env` whatever its `shown`, and `payload` when it is `here`.
    Inside a slot with `by` each group's list gains the same.
    Counts, sums, minima and maxima list no ids and gain nothing. Taking
    those members out gives the protocol's answer byte for byte, and without
    `--values` that answer is all `read` and `fold` write. The members sit
    beside what they read so one read at the start of a session shows each
    item's fields, and none of their names is one the protocol writes at that
    place, so the answer is not mistaken for a fold's.
18. **A verb that answers from a store that does not check answers, then is
    refused.** Kyle ruled on 2026-11-20 that a verb that answers from the
    store must warn and exit non-zero when the store is not clean; before,
    only `loops check` said so. `read`, `fold`, `of`, `get` and `export`
    write their answer as before, from the lines that check, so the answer
    is what a store holding only those lines gives, byte for byte. Then,
    when a line was refused or a segment is altered, they are refused with
    `check`'s code, exit 1, and stderr says, with the two counts:

    ```
    refused: store-not-clean: 1 line refused, 1 segment altered; `loops check` lists them
    ```

    `check` writes its report and ends with the same refusal, from the same
    function. A clean store adds nothing. A refusal of the verb's own, such
    as `id-unknown` for the id of a refused line, is said instead, and a
    reader that closes stdout does not hide an unclean store. `emit` and
    `erase` read the store to fill in and check what they write, so a
    refused line can leave a record unnamed in `prev` or make a target
    `prev-unknown`. They say the same words on stderr as
    `warning: store-not-clean: …` before they write, and their answer and
    exit code are unchanged. `import` does not read the store and says
    nothing.

19. **`emit` copies `occurred` when the heads agree.** An emit of a held
    thing carries the written `occurred` text when every head states it.
    If any head lacks it or the texts differ, the emit states no `occurred`.
    An erased or withheld head supplies its envelope like any other. A root
    and a first record copy nothing. `--occurred <when>` states a new
    occurrence and `--no-occurred` clears it. The two together are a usage
    error. This builds the ruling of 2026-11-20 in
    `notes/2026-11-15-the-cut.md`, section 5.

20. **A changeset can ask for the single-emit filler.** The owner ruled on
    2026-11-20 that plain `emit -` continues to fill nothing in and a
    collector can request the filler for a whole changeset with one option.
    [The cut, section 7](../notes/2026-11-15-the-cut.md#7-unanchored-records)

    Implementer's choices

    - The invocation is `loops emit - --fill`.
    - Omitting `about`, `occurred` or `sets` requests its default. A stated
      value supplies the entire field. `"about":[]` clears the mentions,
      `"occurred":null` clears the occurrence, `"sets":[]` sets no key,
      and `"sets":null` writes a whole-read record. Only filled input gives
      the two null forms these meanings.
    - A stated `prev` gains every live record of the thing, including live
      non-heads. An empty array and omission both request the full anchor.
      Use plain `emit -` to write exact predecessors and make a fork.
    - Each prepared record has its final id and is available to later lines.
      Infer `of` against all held envelopes, including those just prepared.
      Read liveness and head metadata after settling and admitting that
      combined holding. This is the current-holding case of the filler.
    - Visit lines in input order, preparing a line's labelled prerequisites
      first, in input order among siblings. Substitute their final ids in
      `of`, `prev`, `about` and `under` before filling the line.
    - Only labels schedule prerequisites. Inferring `of` from a literal
      predecessor requires its envelope at preparation time. A stated `of`
      leaves target checks to the final batch, which can see later literal ids.
    - Parse the changeset and bind its observers, then check duplicate,
      unknown and cyclic labels, in that order. In preparation order, each
      line infers `of` (possibly `prev-unknown` or `prev-unrelated`),
      fills the remaining fields and meets the writer's field checks.
      The first such failure ends preparation.
    - Both paths share the single-record filler and the conversion of a
      checked envelope into a reader's record. The usual batch writer checks
      all prepared drafts together before any signature and segment append.
    - Keep the segment and the `{"records":[...]}` answer in input order.
      Each answer entry gives the id and its original label when supplied.
    - A non-object payload supplies no inferred keys and is read whole
      unless it states `sets`. A changeset keeps its one default clock
      reading and the salt chosen for each line.

    For these examples `H` is the only record of `task:1`, a whole-read
    head mentioning `project:p` with occurrence `2009`. Letters in an
    envelope stand for full record ids. The examples show the varying
    envelope fields. Each record also has its kind, observer and emitted time.

    Take each of these three inputs independently against `H`.

    ```jsonl
    {"kind":"task","of":"task:1","payload":{"x":1}}
    {"kind":"task","of":"task:1","about":[],"occurred":null,"sets":[],"prev":[],"payload":{"x":1}}
    {"kind":"task","of":"task:1","sets":null,"payload":{"x":1}}
    ```

    Their respective envelope fields are

    ```jsonl
    {"of":"task:1","prev":["H"],"about":["project:p"],"occurred":"2009","sets":["x"]}
    {"of":"task:1","prev":["H"],"sets":[]}
    {"of":"task:1","prev":["H"],"about":["project:p"],"occurred":"2009"}
    ```

    Now take these two inputs together against the same `H`.

    ```jsonl
    {"kind":"task","of":"task:1","about":["project:q"],"occurred":"2010","payload":{"x":1}}
    {"kind":"task","of":"task:1","payload":{"y":2}}
    ```

    The first record, `A`, changes only `x`. The second copies its metadata
    from `A` and names both `A` and the still-live whole-read `H` in
    canonical id order.

    ```jsonl
    {"of":"task:1","prev":["H"],"about":["project:q"],"occurred":"2010","sets":["x"]}
    {"of":"task:1","prev":["H","A"],"about":["project:q"],"occurred":"2010","sets":["y"]}
    ```

21. **`loops tick` covers the whole holding.** The owner ruled on 2026-11-21
    (TK1–TK5) that a tick is an ordinary record whose payload is the whole
    tick, covering every record folded. [The tick record](../protocol/tick.md)

    Implementer's choices

    - Give the spec first. `-` reads stdin. A file in the working directory
      (the directory given by `-C`, when supplied) takes precedence over a
      vertex of the same name. A bare vertex name reads
      `.loops/vertices/<name>.json`. An explicit path reads the file there.
    - Use the checked lines from one store reading for the tick and its
      envelope. An unclean store gives the same warning as on `emit`.
      The core determines which openings are withheld. The new record is
      appended after the fold.
    - Take the envelope flags of single-record `emit`, including its
      `--observer`, `--emitted` and `--salt` options. The observer must
      bind to the signing key. The spec and the holding supply the whole
      payload. Print the signed record's id. With no flags the tick is a root.

    For example, with just six backups held, run
    `loops tick backups --of tick:backups` twice. The first tick covers
    the six backups. The second covers those six and the first tick, and
    names the first tick in `prev`. A slot counting backups still gives six.

22. **Check a tick on request.** The owner ruled on 2026-11-21 (TK7) that
    a verb checks a tick and a store accepts it under ordinary record
    verification regardless of the tick's verdict. [The check](../protocol/tick.md#the-check)

    Implementer's choices

    - `loops tick --check <id>` takes one full id or unique prefix, as
      `get` does. Use one reading of the store for the subject's opening
      and the whole holding. The check needs no signing key.
    - Print one JSON line using the vector formatter. On a clean store
      `same` exits 0. Every other verdict exits 1 with the explanation on
      stdout. An unclean store prints the verdict from the checked lines,
      then reports `store-not-clean` on stderr and exits 1.

    For example, import a tick and the six backups it covers. Checking its
    id gives `{"answer":"same","closure":"yes","taken-not-covered":0}`.
    Emit a seventh backup and check that id again. The verdict is still
    `same` and `taken-not-covered` is now 1.

23. **List the holding and identify the binary.** The owner ruled on
    2026-11-21 (W1) that adoption's CLI pieces are built beside the tick work,
    with `loops` first on PATH for daily work as the exit.

    Implementer's choices

    - `ls` returns `kinds` rows, one per observed envelope kind. Count
      settled, admitted IDs, including superseded records and records
      without an opening. Use the same unclean-store diagnostic as `of`.
    - `ls --things` returns `things` rows. A positional prefix selects rows
      by a literal, case-sensitive match on the whole thing name (written
      `of`, or the root's full ID). Order rows and distinct head kinds by
      UTF-16 code units. Each row counts all its records and its heads.
    - Resolve the headline from the live setters of `name`. One open text
      value gives `{"state":"text","value":"…"}`. A missing, unset or
      non-text headline is `absent`. An unavailable opening that could
      supply it is `unknown`. Multiple live setters for a declared or
      observed name give `conflict`, even if they wrote the same text.
      In each of those three cases the object has only its `state`.
    - `--version` prints `strange-loops-next` and the Cargo package version.
      It needs neither a store nor a key.

    For example, emit `task --of task:kiln name=Kiln`, then
    `task --of task:kiln status=open` with `loops emit`. `loops ls`
    reports two task records. `loops ls task:` returns
    `{"things":[{"headline":{"state":"text","value":"Kiln"},"heads":1,"kinds":["task"],"records":2,"thing":"task:kiln"}]}`.

24. **A writer can state what it read.** `--base T` selects the ids in
    tick T's `covers`. Each `--base-id R` selects one record, including a
    tick as an ordinary envelope. Both take a full id or unique held prefix.
    The base is the union of one optional tick cover and all literal ids.
    This implements option C of [TK11](../DECISIONS.md#tk11-a-write-against-a-base-that-does-not-hold-what-is-under-it),
    pending the owner's ruling.

    The marker must have kind `tick` and a locally accessible opening.
    Read its common members under the checker’s current rules. A marker
    from another version can supply its cover. Each listed envelope must be
    held, even for records of another thing. A missing envelope gives
    `id-unknown` with its id. An unavailable marker opening gives exit 1
    with `the tick's payload is not held`. Covered openings can be absent.

    For the thing being written, add any stated `prev` targets to the
    selection, then follow `prev` through held envelopes of that thing.
    A missing ancestor ends that path. A stated target stays in the
    envelope even when another record in the effective base is above it.
    Settle and admit inside the resulting set. The filler names its live
    records, derives `sets` there, and copies mentions and agreed occurrence
    from its heads. Explicit links and field overrides work as before. A
    stated predecessor can supply `of` from the writer's full lookup. In
    a filled changeset the same starting selection applies throughout. Add
    earlier prepared forms before taking the closure for each line.
    Tick writes use the base for their envelope and fold the whole holding.

    Suppose a marker covers the original `status` setter of `task:42`. A
    peer then changes that status. Writing `loops emit task --of task:42
    status=done --base T` names the original setter and leaves the peer and
    the new emit as heads. `loops of task:42` shows both status setters.

    If X sets port and owner and Y changes port over X, `--base-id Y`
    includes X under Y. An emit changing either key names both. After the
    append stderr says `the base did not show 1 record that this emit names`.
    If the tick covered neither X nor Y and the writer states `--prev Y`,
    the emit names both and the count is 2. The count includes each distinct
    id in the written `prev` outside the tick cover and literal ids, including
    stated targets and earlier changeset lines. Dead ancestors that the emit
    leaves unnamed cost no notice. When the effective base has no record of
    the thing, stderr says `the base covers no record of <thing>`. If all its
    records of the thing fail admission, stderr says `no record of <thing>
    in the base is admitted`. Covered ticks contribute only their own
    envelopes. These notices follow a successful write; conflict policy
    remains open for step 3.

## Checked by

`rust/place/tests/lines.rs` signs every accepted record vector and requires
the model's `line` and `erased_line` byte for byte, then requires each changed
line to be refused by the check that covers the change. `rust/place/tests/dir.rs`
covers changesets, union in either order, purge with and without an eraser,
a torn last line, an altered segment, and the batch check's view of held
records (`of-not-root`, and `about` making no head). `rust/cli/tests/walk.rs`
drives the binary: emit, revise, fork, fold, erase; a labelled changeset and its
refusals; `of` filled in for a revision, `--of`, and `of-not-root` and
`prev-unrelated` against the store; `under-unknown`, and an erased and a
withheld `under` target held; `sets` on either path, empty included, and
kept on an erased and a withheld record; a latest-by that follows keys over a
keyed chain, a fork, an erased setter and a pure join; count, sum and collect
that follow keys over a head that sets only `title`, a fork and two setters of
one field; a mention making no head; two stores
converging; exit codes; an emit naming every live record of its thing held,
through the note's history, a whole-read emit above keyed ones, a fork merged
by export and import, an erased and a withheld live record, a root and a first
record; `about` copied, replaced by `--about` and emptied by `--no-about`, and
a moved task changing group; and a changeset that fills nothing in.
The filler test `occurred_requires_the_same_text_on_every_head` checks agreement.
The walks `occurred_is_copied_replaced_and_cleared`,
`disagreeing_heads_supply_no_occurred` and `an_erased_or_withheld_head_supplies_occurred`
check the stored envelopes and flag conflict.

The walk also replays a session: tasks and a decision with names, statuses and
messages, an update by `status=done` alone that keeps the name and message,
`sets` inferred and an emit naming no field written as a pure join, and one
`read --values` that shows each open item's name and status, equal without
the added members to `read` without `--values`. It shows a key with two live
setters from a fork merged by export and import, each with its value and
neither picked; an erased and a withheld live setter as unknown, and an
erased whole-read record as `unheld`; an undeclared payload key not shown, a
declared one missing as unset, and `--whole` replacing everything; usage
errors for `--whole` beside `--sets` or `--sets-nothing`; and grouped slots
with payloads inside each group. Over a store with one line changed in
place, and over one with a line cut from a segment, `read`, `fold`, `of`,
`get` and `export` write what a store holding only the remaining lines
answers, then exit 1 with `store-not-clean`; that store exits 0 with nothing
on stderr, `check` lists the line or the segment, and `emit` warns and writes.
`rust/place/tests/anchor.rs` writes 300 random emits as `loops emit` writes
them, one writer and then two that merge now and then, and requires each
history to be anchored (every record above a live record names it) and each
thing's live records, with their erasers, to answer every slot that follows
`keys` as the whole holding does. Expected answers come from the
vectors or from the walk's own statements, not from the code.

Not checked: nothing in the model says what a place is, so there is no law
behind the layout, and no second implementation reads it yet.

## First data

```sh
loops init
node design/rethink/ledger-drafts.mjs | loops emit -
```

writes the decision ledger as one signed changeset of 53 records, each a
root and so its own thing. It is not committed: the first store in this repo
should be signed by its owner's key, not a session's.

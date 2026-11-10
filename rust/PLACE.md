# The first product slice: a place and a CLI

Step 2 of the build sequence: emit, read and fold for one person, signed.
`loops-place` (`rust/place`) is the first code around the pure core that
touches a disk, a clock and a key. `loops` (`rust/cli`) is the CLI over it.
The core gained nothing; `loops-eval` gained one export, `slots_json`.

None of this is protocol yet. The stored line is (`protocol/record.md`);
the directory layout, the CLI and the codes below are a working proposal,
written down here so they can be changed on purpose.

## What is implemented

```
loops init                          make .loops/ here
loops emit <kind> [field=text | field:=json]... ["message"]
      [--about <thing>]... [--prev <id>]... [--erases <id>]...
      [--occurred <when>] [--under <id>]
loops emit -                        drafts as JSONL on stdin: one changeset
loops get <id>                      one record
loops about <thing>                 a thing's winner, its other heads, how many records
loops read <vertex>                 fold .loops/vertices/<vertex>.json
loops fold <spec.json | ->          fold a spec given as a file
loops erase <id>... ["why"]         emit an erasure, and drop the openings held here
loops export [--kind <k>] [--about <thing>]
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
  labelled changeset meet the same checks, `prev-unknown` and
  `prev-unrelated` included. A changeset writes nothing unless every draft is taken.
* An id can be given by any prefix that names one held record.
* A revision written without `--about` is about what it replaces, when its `prev` targets all hold the same things.
* Every answer is JSON or an id. Lenses are a later step, so `--json` is accepted and changes nothing.

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
6. **How a payload becomes what a fold reads.** The model's reader record
   has a text `value`, a numeric `n` and named primitive `fields`
   (`protocol/laws.md`, simplifications). `Line::record` reads a payload
   object's top-level members: text `value` and numeric `n` fill the
   built-ins, other text, numbers and booleans become fields. Nested values,
   nulls and non-object payloads are held and unread. Two consequences to
   settle with the model, not here: `value` and `n` always exist to a `where`
   test (as `""` and `0`), and a nested payload can't be tested at all.
7. **New refusal codes**, outside the protocol's lists: for a line,
   `line-not-json`, `line-shape`, `commit-invalid`, `id-mismatch`,
   `key-invalid`, `signature-invalid`, `line-not-canonical`; for the CLI,
   `observer-unbound`, `id-unknown`, `id-ambiguous`, `vertex-unknown`, `draft-shape`,
   `draft-field-unknown`, `spec-not-json`, `store-not-clean`.
8. **Erasure removes bytes only here.** `loops erase` emits a record with
   `erases`, which every reader applies, and rewrites the local segments that
   held the opening. Other copies, and git history, keep the opening until
   they purge too.

## Checked by

`rust/place/tests/lines.rs` signs every accepted record vector and requires
the model's `line` and `erased_line` byte for byte, then requires each changed
line to be refused by the check that covers the change. `rust/place/tests/dir.rs`
covers changesets, union in either order, purge with and without an eraser,
a torn last line and an altered segment. `rust/cli/tests/walk.rs` drives the
binary: emit, revise, fork, fold, erase; a labelled changeset and its
refusals; two stores converging; exit codes. Expected answers come from the
vectors or from the walk's own statements, not from the code.

Not checked: nothing in the model says what a place is, so there is no law
behind the layout, and no second implementation reads it yet.

## First data

```sh
loops init
node design/rethink/ledger-drafts.mjs | loops emit -
```

writes the decision ledger as one signed changeset of 53 records, each
anchoring its own thing. It is not committed: the first store in this repo
should be signed by its owner's key, not a session's.

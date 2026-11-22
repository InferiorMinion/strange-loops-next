# Roadmap

What is next, in order. This file holds no history and no open questions.

- What was built and ruled, by date: [CHANGELOG.md](CHANGELOG.md).
- What waits on Kyle, each with an example: [DECISIONS.md](DECISIONS.md).
- The long design notes: [notes/](notes/).

## Where we are

- **Steps 0, 1 and 2 are done.** Kyle closed step 2 on 2026-11-20: `loops` emits, reads and folds for one person, signed, over `.loops/`.
- **Step 4 (ticks) has started.** The cut laws are in the model and the result codec is built. What a fold reads from a payload is settled and stated in the model (2026-11-21), so a tick can pin a result hash. The tick record is ruled (2026-11-21, [the note](notes/2026-11-21-tick-record.md)) and is being built.
- **Three implementations agree on every vector:** Lean (the model), JavaScript and Rust. From ticks on, a piece is built in Lean and Rust; the JavaScript is frozen at what it has, as a trial for step 4 (ruled 2026-11-21).
- **One outside user:** rex-tools, a chat room over the `loops` CLI. It found a slow read path (fixed 2026-11-21) and a gap in `--values` (fixed the same day).
- **The old Python `loops` is still first on `PATH`** on Kyle's machine, and his day-to-day stores are still the old ones.

The evidence and its counts are kept where they are made: [the law index](design/laws/laws.json), [vectors](vectors/README.md), [the fuzz report](interop/fuzz/REPORT.md), [the mutation report](model/mutate/REPORT.md) and [Rust boundaries](rust/GAPS.md). The changelog entry of 2026-11-21 has them as they stood on that day.

## Now

The payload values and the result hash's prefix, ruled by Kyle on 2026-11-21, are built. The tick record, ruled the same day, is being built.

| Order | Work | State | Waits on |
| --- | --- | --- | --- |
| 1 | **The tick record:** what a tick holds, how `covers` is stated and checked, and the input key. | Ruled 2026-11-21 ([the note](notes/2026-11-21-tick-record.md)). Being built in seven pieces (changelog, 2026-11-21). Pieces 1 to 3 are in the model (the spec's hash and `env.when`; the result reader and the payload; the claim and the check). Rust has piece 1, so `loops fold` takes a window on `env.when`. The model's piece 3, the claim and the check, is in: the model holds the whole tick record. Rust has the payload and the result reader. The protocol page is in work; then Rust for the check, and the CLI verbs. | [TK10](DECISIONS.md#tk10-seven-details-of-the-check) for the check's details |
| 2 | **Step 4's ruled pieces** (list below). | Ruled 2026-11-20. Not built. | Some wait on the tick record |

### Step 4: ruled and not built

Each is ruled in [the cut note](notes/2026-11-15-the-cut.md); the section is in brackets.

- The filler takes a base, a tick or ids, and names that base's live records (8). margins-claude holds its base lemma until this piece is briefed.
- A CLI verb that shows a result hash.

**Step 4 is done when** a content tick recomputes to the same hash on two machines.
Not in step 4 yet: boundaries and markers, the runner, and what verifying a deep tick tree costs.
Not in step 4 any more: the caching reader's law (issue #8). It is stated when a caching reader is built (ruled 2026-11-21, amending 2026-11-11).

## Next

The order Kyle set on 2026-11-10 is: step 4 (ticks), then step 3 (standing) together with step 5 (sharing).
Kyle ruled on 2026-11-21 for one step between ticks and standing, adoption, with its CLI-only pieces built beside the tick work ([W1](notes/2026-11-21-order-of-work.md#w1-an-adoption-step-before-standing-and-sharing)).

| Order | Step | Builds | Done when |
| --- | --- | --- | --- |
| after ticks; its CLI pieces beside them | Adoption | The old stores imported as a shadow; `ls`, a scan by name prefix, relative times and a plain-text view; reads across several stores; a cache of checked segments | `loops` is first on `PATH` for daily work |
| then | 3, standing | Declarations, admission, stances, grants, presence key, retire through; then name tables and `resolve-by` | A strict store rejects visibly; a steward's changeset takes effect |
| with it | 5, sharing | The git layout, then segment-list sync | Two clones converge |
| later | 6 | Index and search, SQLite segments, collectors, lenses, `serve` | Something is slow, or a surface needs it |

Standing matters once records cross between people: sharing through git repositories across teams, and over HTTP.
Everything wanted before that runs on one machine under one trust root.
Retiring still follows ticks, as the Trust walk found.
Each step extends the model's laws, computes protocol vectors, then implements Rust.

## Small pieces, any time

None needs a ruling. Each is built when it is the cheapest useful thing to do.

- `loops emit` takes `--` to end its options, and a verb makes a signing key without emitting (issue #11).
- The JavaScript sum is exact only to a signed 128-bit limit, and the law index calls it unbounded: make one of them true.
- The fuzz generator puts lists, objects and `null` in the `spec/run` and `result` families only: give them to the `fold`, `realm` and `scenario` families too.

## All steps

| Step | Builds | Done when |
| --- | --- | --- |
| 0, done | The Lean spike: Count and LatestBy, one lens combinator, 23 vectors | The first laws are proved |
| 1, done | Protocol and vectors for layer 0: canonical bytes, salt and commitment, id, signature | The model and the JS writer agree on every vector; Rust checks all fixture families and external standards |
| 2, done | Emit, read, fold for one person: JSONL segment, JSON spec, the two fold families, `"follow": "prev"`, `by`, a signing key, signing by default. `of` names a record's one thing, an id or a natural key; `about` only mentions. D8a, D8b and D8c are built: `sets` in the envelope, every slot that follows `keys`, the slice laws for one keyed thing and for a spec with a keyed slot, and the writer's anchor rule. | It does the old CLI's workable job, signed |
| 3 | Standing: declarations, admission, stances, grants, presence key, retire through; then name tables and `resolve-by` | A strict store rejects visibly; a steward's changeset takes effect |
| 4, started | Ticks: content and witness, boundaries, markers | A content tick recomputes to the same hash on two machines |
| 5 | Sharing: the git layout, then segment-list sync | Two clones converge |
| 6 | Index and search, SQLite segments, collectors, lenses, `serve` | Something is slow, or a surface needs it |

The step numbers are names, not the order. Steps 3 to 6 are proposals.

## Later work, by value

Costs are S, M, or L for relative effort, not schedules. None of these is scheduled.

| Work | Value | Cost | Dependencies and blocking choices |
| --- | --- | --- | --- |
| Integrate a workbench page | Readers build scenarios and try vector presets | M: UI, validation, browser tests | Use the Rust core compiled for the browser (ruled 2026-11-21) and scenario-vector presets; version localStorage drafts; never treat them as canonical storage |
| Turn horizons into Lean scenario vectors | Tests concrete compositions and exposes missing premises | L: model and scenario work | Start with minimal compositions of existing core; mark new boundary, tick, and service premises unimplemented before proof claims |
| Prove binary64 roundtrip, then shortest | Strengthens assurance beyond sampled standard cases | L: formal number reasoning | Existing canonical formatter; neither proof blocks the settled standard choice |
| Bridge source and fold records to hashes | Connects text, record bytes, IDs, and fold outputs | L: byte-level model and evidence | Defined source and writer boundaries; do not assume SHA correctness from bounded IDs |
| Model the place | Gives the product slice laws: segments as a set | M: model and vectors | The choices in `rust/PLACE.md`; a second reader of the layout. The payload half of this row is built (2026-11-21) |
| Build standing, routing, key management, and sync | Supports more than one writer | L: architecture and security work | Step 3; explicit service contracts and key decisions; safe-wrapper versus raw access choice |
| Expand malformed-input fuzz and standards coverage | Finds more boundary and adapter differences | M: generators, triage, and regression pins | The settled D1–D5 boundaries; explicit contract before broadening raw extrema |
| Complete browser, accessibility, and link validation | Checks the reader experience and published references | M: browser and accessibility checks | Workbench and generated pages; validate anchors and external links separately |

"Ask without a hand-written spec" (a scan by name prefix, `ls`, relative times) left this table: it is part of the proposed adoption step.

## Reference

**Why Rust.** Small writers can use JSON in any language.
Phones and browser extensions need embedded readers, and Rust runs there without a server.
Exhaustive enums expose missing cases, and Lean's closed types map directly to enums.
This is design rationale; no bindings exist.

A reader reaches the core in three proposed ways. All three speak the same JSON shapes.

- **Local server:** a browser, LSP or MCP client speaks JSON-RPC to `loops serve`, which reads `.loops/` on disk.
  This suits a laptop and keeps keys out of the browser.
- **WebAssembly:** `loops-wasm` runs inside the page or extension, over browser storage or files the reader picks.
  This suits an annotation extension, static hosting and an offline web app.
- **Native bindings:** `loops-ffi`, through UniFFI, runs inside a Swift or Kotlin app, over the app's own storage.
  This suits a phone.

### Standards to stand on

The record rows are settled; see R1–R6 in [the changelog](CHANGELOG.md#step-1-and-before) and [record rules](protocol/record.md).
Index, keys and provenance export are proposals.

| Concern | Choice |
| --- | --- |
| `emitted` | Canonical UTC instant, with at most nine fractional digits |
| `occurred` | A span: dates, instants and intervals from ISO 8601 and EDTF Level 1; the grammar is closed in `protocol/record.md` |
| Canonical bytes | RFC 8785 (JCS): ECMAScript-exact numbers, no NaN |
| Hash | SHA-256 |
| Id text | Typed prefix plus lowercase unpadded base32hex (`r:`, `st:`, `sl:`, …); algorithm changes go through the version label |
| Signatures | RFC 8032 (Ed25519) over `loops/record/v1:` + id |
| Specs | JSON (RFC 8259), defined by [`protocol/spec.md`](protocol/spec.md); declaration and config shapes are proposals |
| Index, search | SQLite with FTS5 through `rusqlite`, bundled; the CLI only, never the core |
| Keys | The OS keychain behind a `Signer` trait, with a file fallback |
| Provenance export | W3C PROV or PREMIS mapping, undecided |

The premises and domains that the laws keep are in [retained boundaries](notes/retained-boundaries.md).

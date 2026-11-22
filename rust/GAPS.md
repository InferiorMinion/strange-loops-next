# Boundaries and remaining work

The Rust crates implement executable behavior and the accepted rulings, not
runtime enforcement of every Lean theorem premise. This document records
those boundaries, not new protocol decisions. Standards, bounded digest ids,
binary64 input, per-thing head composition, and edited-id rejection are no
longer deferred design questions; their implementations have explicit tests.

## External services and source routing

The evaluator is an input-only fixture/JSONL process, not a storage service.
Source path routing, filesystem/network record retrieval, observer key
management, and production persistence remain outside it. A spec source's
store string is preserved but does not cause I/O during pure execution.

## Raw domains and theorem premises

- Accumulator commutativity/purity and envelope-only predicates are caller
  obligations. Rust closures do not encode Lean proofs such as `Acc.comm`
  or `EnvOnly`; arbitrary closures gain no unconditional order/view law.
- Latest permutation/repeat/convergence laws require addressed records.
  Raw Count/Sum keep repeats; low-level `LatestBy::fold` keeps unchecked
  global state behavior. Whole-view heads/latest compose per-thing heads
  instead. Raw insertion uses full record equality after ordering tests,
  so unchecked equal-order/different-payload records are not collapsed.
- Realm form-independence requires compatible forms: equal envelopes for
  equal ids and equal records for two visible forms. Raw settlement does not
  refuse incompatible forms; its first visible form can depend on input
  order. It checks erasures against original forms, including discarded and
  subsequently refused ones, then admits once against original held records.
- Spec execution does not silently settle or globally deduplicate arbitrary
  input. A record is of one thing, so an id reaches two things only through
  raw forms that disagree on `of`; latest-by's unread count still
  deduplicates ids across things. Raw following collect counts unknown forms after newest-first processing.
  Count, sum, min and max that follow prev fold `whole_heads`, which keeps the
  last raw form of each id before ordering, as the model's `dedup` does.
  Latest-by that follows keys works out each thing's live setters over the raw
  records of that thing, as `keyed_in` reads them; with addressed input each
  id is one setter. Count, sum, min, max and collect that follow keys count
  each thing once, and read a field's live setters through `setters`, which
  keeps one form of each id, as the model's `orderHeads` does.
  A slot with `by` counts its top-level unread and ungrouped records over raw
  input, once per form, as count does; with addressed input that is once per id.
  The distinct-id theorem interpretation requires its stated addressed-input
  premise. Limits restrict known matches, not unread totals.
- Spec's outer recursive duplicate check includes ignored metadata before
  shape/grammar checks. Lower raw helpers retain their wider domains.
  Accepted unused metadata is discarded by the executable renderer.
- Batch meaning uniqueness requires acyclicity and meaning proofs. Resolution
  retains previous-round mappings, first label/held lookup, raw labels,
  mapping/draft order, and the model's global refusal rank. The adapter does
  not claim unique meaning for arbitrary raw drafts.
- `of` is read as written (D8). A reader never resolves it through another
  record and never checks that it names a root; a record whose `of` names a
  revision is of that text. Only a writer refuses `of-not-root`, and only
  against what its `held` lookup gives: the fixture's `held` entries in the
  evaluator, stored envelopes in the place. A record id the writer doesn't
  hold, like a natural key, passes.
- Lens laws retain their validity and success premises.
  No-op and put-get require distinct base ids.
  Put-get also requires the same ordered ids.
  Edited duplicate and empty ids are refused.
  `lens::read` is the public boundary: it refuses a base that repeats an id
  (`base-id-duplicate`) before it reads the text (D1). The raw `lookup`, `diff`
  and `edit` keep first-match behavior and leave the premise to the caller.
  Absent and unknown outcomes leave the base unchanged.

## Numeric and canonical boundaries

`Number` admits only finite binary64 bits. All RFC 8259 **input** lexemes round
once through binary64, then safe-whole values normalize to `Int`. Non-whole
and unsafe-whole numeric payloads are retained but unread by integer folds.
The core and the place use the same projection for all written payload fields, including lists, objects and null. The payload vectors exercise both paths. Raw sum and extrema accept a
projection to optional arbitrary-precision integers. Record-facing adapters
supply `Record::num`, including for the field named `n`.
Raw core `Int` and `Nat` still allow exact unbounded construction. Writers
require magnitude at most 2^53 − 1 for every `Number`, whole or not.
Raw `Json::Int` obeys the same bound; refusal priority is unchanged.
Canonical formatting covers all finite binary64 values, independently of writing. Exact accumulated integer outputs are not
rounded through the input decoder.
The canonical result (`loops::result`, D7) writes every integer as a decimal string from the unbounded value, so
binary64 never touches it; its reader `int_of` takes exactly those strings. The laws that say its bytes lose `shown`
and nothing else are Lean's (`model/Laws/Result.lean`); Rust checks them on the `result` vectors and its own tests,
not for every input.

Rust converts native shortest formatting to ECMAScript notation.
It applies exact midpoint tie-to-even correction.
The external 4,000-case corpus checks compatibility.
Lean roundtrip and shortest proofs remain nonblocking roadmap work.
Canonicalization handles repeated object names with stable UTF-16 sorting.
The first equal-key value is rendered, but validators inspect all raw members.
NaN and infinities are refused; negative zero canonicalizes to zero.
`Number` stays finite under D3. A lexeme past binary64's range is malformed to
the public `parse_json`. The evaluator's input decoder carries it, as the
largest finite magnitude of its sign, only to positions whose readers refuse
any magnitude above 2^53 − 1 alike: a record's or batch's payload, where the
writer answers `number-out-of-range` in its usual place, and a spec, where the
spec parser refuses it. A refused draft or spec is never canonicalized, so the
stand-in never reaches bytes. A reader's records (`fold`, `realm`, `scenario`,
and `spec/run`'s `orders`) keep the strict decoder, as Lean refuses a
nonfinite number there as malformed input.

For tick fixtures, the evaluator uses Lean's JCS spelling `null` for overflow
in a top-level input `spec` or in `samples`. It requires finite binary64 numbers
in a supplied `payload`, in the JSON inside `payload-text`, and in `ticks`
and `records`. The same finite-number boundary applies to the subject payload
and openings in the writer and checker forms. Lean can still give an answer
for some such overflowing inputs.

The test suite consumes every merged JSON fixture, all six JCS byte pairs,
4,000 IEEE-hex number rows and 128 SUPERCOP signing rows. There is no
`standard` allowlist. This is bounded fixture/standard evidence, not a proof of
all RFC inputs or a claim that raw core constructors satisfy writer premises.

## Digests, time, characters, and diagnostics

Reader ids and links use exact bounded 32-byte digests, bytewise ordering and
canonical base32hex roundtrips. No pending unrestricted-Nat-id interpretation
remains. Writer links intentionally remain text until writer validation.

Parsed moments have private bounded seconds and at most nine fraction digits.
The port does not expose arbitrary raw Lean moment construction.
Time parsing accepts only UTC `Z`, seconds 00–59, and fractions ending in a nonzero digit. Typed references and character checks retain
Lean's ASCII rules. Explicit lens lines preserve carriage returns and empty
strings; text-to-line LF splitting must not discard a trailing empty line.

Fixture diagnostic wording is Rust-specific; it does not claim byte-identical
Lean diagnostics. Protocol refusals retain their wire codes and spec paths.
JSON decoder refusals become per-file error rows, while process I/O failures
are fatal. Expected fixture answers never participate in evaluator decoding
or execution. Rust validates ignored metadata for Unicode, with regression tests.

A `String` holds only Unicode scalar values, so no core value holds a lone
surrogate (D2). `loops::json::of_utf16` is where code units enter, and refuses
one. A vector whose text holds a lone surrogate anywhere is malformed. A
record's `payload_text` that is JSON by its syntax but holds one is the
protocol refusal `string-not-unicode`, after raw shape and before every value
check; the evaluator tells the two apart by a syntax-only pass.

## Adapter boundaries settled by D3, D4 and D5

- Overflow: see the numeric boundary above. The mapping is the adapter's; the
  refusal, its stage and its precedence are the writer's own.
- Signing fixtures require a seed of exactly 32 bytes, read only once the
  record is written. Lean and JS now require the same. This is a fixture
  adapter boundary, not a record rule.
- A record input's first envelope and first payload decide its answer, as in
  Lean. Once they are written, a variant that names another record, or a
  payload variant that is refused, is an error and never an answer. `answers`
  gives one answer for a record input.

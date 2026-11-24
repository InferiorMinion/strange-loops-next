# Shadow export v1

Run with Python 3.10 or later on a frozen, consistent copy of one SQLite
store and its matching vertex.

```sh
python3 tools/shadow/export.py workshop.vertex --output-dir ./private-cut
```

The store path is relative to the vertex. The exporter reads the schema,
tick count and facts in one read-only transaction with `query_only` enabled.
It expects the supplied snapshot to be authoritative. For a derived index,
establish agreement with the old log when making the snapshot. The exporter
does not open the old engine or rebuild its index.

If a `-wal` file exists, supply `--quiescent-copy` only when the main file
and WAL were captured consistently and both will remain fixed. The exporter
copies the pair into private scratch under the output directory. SQLite
opens that copy with `mode=ro` and can reconstruct shared memory there.
Without a WAL, the exporter opens the snapshot with `mode=ro&immutable=1`
to prevent sidecar creation even for checkpointed WAL-mode databases.
A snapshot with a rollback journal is also read from a private copy. SQLite
refuses a read-only copy if it requires recovery.

Outputs are private files (0600 on POSIX) in a separate directory. Choose a
fresh destination for a new cut. Publication uses same-filesystem hard links,
map first and facts last, and refuses to overwrite either filename. If a
process dies between links, a lone map can remain. A usable cut requires both
files and a verified trailer. The facts file contains private legacy payloads
and provenance.

## Reviewed map

`<name>.map.json` is a UTF-8 JSON object. The importer reads this map. It
never reads KDL. For example, a workshop map could contain

```json
{
  "format": "shadow-map-v1",
  "source": "workshop",
  "store": "data/workshop.sqlite",
  "vertex_sha256": "0aed6432deddba7c63c0222700bccb677bb2422589e65260b1aa86b919dd3e59",
  "kinds": {"thread": {"by": "name"}, "change": {"collect": "20"}},
  "ref_fields": ["links", "members"]
}
```

The digest is lowercase SHA-256 of the exact vertex file bytes. `source`
is a nonempty single filename component. `store` retains the declared
path. Each nonempty kind occurs once and has exactly one entry, either a
nonempty key field or a canonical unsigned 64-bit collect limit. The limit
describes an old read; the importer considers every fact. Missing kinds are
unnamed. Review this list before import. The vertex digest binds the source
cut to its own vertex revision. The optional `ref_fields` lists distinct,
nonempty top-level payload field names. The names `ref` and anything ending
in `_ref` are reserved. An invalid declaration gives `shadow-map-invalid`.
A reviewer adds this member by hand. The exporter writes no `ref_fields`.
Carry the reviewed list forward by hand on re-export or its pin changes.

Journal v2 pins base32hex SHA-256 of the canonical UTF-8 JSON object
`{"kinds": K, "ref_fields": F}`. K is the entire validated kinds object,
including collect limits. F is the list in canonical UTF-16 order, defaulting
to `[]`. Reordering either declaration or editing a vertex comment leaves
this pin equal. A changed kind, fold or configured field membership gives
`shadow-state-mismatch`. Each new map and export header must still agree.

The exporter requires one each of `name`, `store` and `loops`. Top-level
`observers` and `sources` blocks may repeat. Each takes no arguments, and
the reader checks its syntax and skips its contents. A kind has
one `fold` with one `items "by" "<field>"` or `items "collect" N`. Under
a kind, `edge`, `lifecycle`, `search`, `preview` and `boundary` are skipped,
including their child blocks. A `boundary` beside the kinds is skipped too.
The reader supports newline and semicolon terminators, quoted strings,
ordinary properties on skipped nodes, `//` and nested `/* */` comments, and
`/-` before a node, argument, property or child block. Unsupported statements and malformed
declarations refuse with the vertex line. A `combine` or `discover` vertex
cannot supply a single-store map.

## Fact stream

`<name>.facts.jsonl` uses UTF-8 with one object per LF-terminated line,
including the last. Each object has exactly the members specified below.
Unknown, missing or duplicate members, blank lines, CRLF, a missing trailer
or bytes following the trailer invalidate the cut. JSON member order is free.
Digest verification uses the original bytes including whitespace.

| Line | Members |
|---|---|
| Header | `format` = `"shadow-facts-v1"`, `source`, `vertex_sha256`, `order` = `"arrival"` or `"rowid"`, `ticks` = number of rows omitted from the separate ticks table |
| Fact | `id`, `kind`, `ts_bits`, `observer`, `origin`, `payload`, `signature`, `coordinate` |
| Trailer | `count` = number of fact lines, `digest` = lowercase hex of the final 32-byte digest |

`source` and `vertex_sha256` must agree with the map. Counts are unsigned
64-bit decimal **strings**. Each coordinate component is a signed 64-bit
decimal string. Use ASCII digits, no plus sign, no leading zeros and no
negative zero. Arrival coordinates are `[ordinal, seq]`. When both columns
are absent, rowid coordinates are `[rowid]`. Compare these numerically and
lexicographically. Gaps are valid. Partial schemas, nulls and repeated
coordinates are errors.

The old ID is unique per stream and contains 1 to 128 ASCII characters
in the inclusive range U+0021 to U+007E. The other scalar
members are text except that `signature` may also be JSON null. Preserve
the entire payload JSON text as a string, including its original spaces,
escapes, duplicate names and number spellings. For instance, the outer
member `"payload":"{ \"name\" : \"kiln\" }"` decodes to the old text
`{ "name" : "kiln" }`. Payload validity is decided in row preflight so
an invalid omitted fact can be reported. `ts_bits` holds the original
binary64 epoch seconds as 16 lowercase hex digits in big-endian order.
The mapper rounds once to the nearest microsecond, ties to even, and
uses canonical UTC with any trailing fractional zeros removed.

The ordered digest is defined over the header and every fact line. Initialize
`d = SHA256(header_line)`. For each fact in order, set
`d = SHA256(d || fact_line)`, using the 32 binary bytes of the preceding
digest. The trailer contains `hex(d)`. An empty cut uses the digest of the
header. This permits both readers to stream and uses the existing Rust hash
function. The digest detects an incomplete or altered cut. A person who can
replace the cut can also recompute it.

The Python suite builds SQLite snapshots in test scratch and checks the
export against the invented workshop files in `rust/cli/tests/fixtures/shadow/`.
The Rust suite reads the same bytes and checks signed lines against the
existing writer.

## Imported references and old IDs

The importer indexes the complete group, including omitted facts. In `ref`,
a group ID takes precedence over an address. Otherwise the first colon or
slash separates a lowercase prefix matching `[a-z][a-z0-9-]*` from its
nonempty value. The prefix `r` is reserved. For example, `decision/design/fold`
becomes `decision:design/fold`, even if the group has no such thing.

A field with a nonempty name before `_ref` and each configured `ref_fields`
field can refer to exact group IDs. Split text at commas, trim each part and
ignore empty parts. A configured field also accepts an array of text, with
the same splitting for each element. An unsupported value, including a mixed
array, stays in the payload and contributes no mentions. Configured fields
always stay. For an old ID, prefer its included local copy. Otherwise use
the single distinct effective target among included copies. A keyed target
gives its thing name and a root gives its full `r:` record ID.

On each keyed revision, `about` carries the accumulated resolved `ref`
mentions and the current mentions of each suffix or configured field.
Supplying a field replaces its previous mentions, including with an empty
set. A root starts from empty state. Add the fact's own nonempty observer as
`observer:<old observer>` and suppress self mentions. A self part counts as
resolved for cleaning. Remove a text `ref` when all its parts are accounted
for. Remove a suffix field only when its base member is on this original
fact and all its nonempty text parts resolve, or its value is exactly `""`.
Delimiter-only suffix text stays. For example, `"owner_ref":"potter-id,?"`
stays intact when only the first part resolves. A later paired
`"owner_ref":""` clears the mention and unsets the previously visible field.
Unknown, omitted, ambiguous and cyclic references leave the original field
intact. Independently resolved parts still contribute mentions.

After a complete writable import, `group.json` has a sibling `group.ids.json`.
Its top-level keys are old IDs. Each value lists included occurrences in
source-name order, with exactly these string members:

| Member | Meaning |
|---|---|
| `source` | Reviewed source name |
| `destination` | Store directory relative to the group file |
| `record` | Full new record ID, including `r:` |
| `thing` | Keyed thing name, or the same record ID for a root |

An old ID seen only on omitted rows maps to `[]`. Identical copies in two
destinations have two entries. This 0600 lookup file contains no salts.
The importer atomically publishes it under the group lock, skips an identical
file, and regenerates a missing one on a writable rerun. Check-only never
creates or repairs it. A v1 journal requires fresh shadows. The comparison command below still uses
the earlier payload projection and map schema. See
[choice 31](../../rust/PLACE.md) for recovery, cycle handling and counts.

## Compare a shadow with its source

Start with an initialized shadow store at `workshop/` and a `group.json`
beside it. For this example the group contains

```json
{"state":"private","sources":[{"name":"workshop","facts":"cut/workshop.facts.jsonl","map":"cut/workshop.map.json","destination":"workshop"}]}
```

Review the exported map before the import. Run all three commands from the
directory containing the group, with the Rust `loops` on PATH.

```sh
python3 tools/shadow/export.py workshop.vertex --output-dir cut
loops shadow-import group.json
python3 tools/shadow/compare.py cut/workshop.facts.jsonl cut/workshop.map.json workshop
```

Use `--loops PATH` to select a binary, and `--unnamed` when the import used
`--include-unnamed`. Compare a stationary shadow. The tool checks each shadow
first, then reads record counts and keyed field projections with `ls`. A
values fold supplies root payloads, shadow-only field names and fields that
the comma-separated option cannot express or pass within the OS argument limit.

The JSON report lists sources in argument order and kinds in sorted name
order, with counts only. Keyed things match by kind and key. Roots match by
kind and full payload, including multiplicity. Unmatched roots count on their
respective sides because the two sets of root IDs have no common identity.
Field differences count matched things. Conflict and multiple-head counts
cover the new reading. Exit 0 means the compared readings and record counts
agree, 1 means a difference or a failed comparison, and 2 means bad usage.
A failed read prints a diagnostic on stderr. `--show N` adds at most N
examples across the report, with kind, key and field names.

Supply `--old-json FILE` for an independent comparison with the first
source's `sl read <vertex> --json --all` rows. Its generated metadata is
excluded and rows of unnamed kinds are counted separately. Repeat
`--also FACTS MAP SHADOW` to compare more sources and their union. For each
shared keyed thing, the old-side merge orders all copies by original binary64
time, numeric arrival coordinate, then argument order for exact ties. The
union reports shared things, differing readings and multiple heads. Multiple
heads alone do not change the field-equality verdict.

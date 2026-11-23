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
  "kinds": {"thread": {"by": "name"}, "change": {"collect": "20"}}
}
```

The digest is lowercase SHA-256 of the exact vertex file bytes. `source`
is a nonempty single filename component. `store` retains the declared
path. Each nonempty kind occurs once and has exactly one entry, either a
nonempty key field or a canonical unsigned 64-bit collect limit. The limit
describes an old read; the importer considers every fact. Missing kinds are
unnamed. Review this list before import. The vertex digest binds the source
cut to a vertex revision. Pinning the reviewed **map itself** on subsequent
imports belongs to the journal.

The exporter requires one each of `name`, `store` and `loops`. A kind has
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

The old ID is unique per stream and has 26 characters. The other scalar
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

import Loops
import Gen.Json
import Gen.Encode
import Gen.Records

/-!
# Spec vectors

`spec/parse`: a spec as JSON, and either its parsed form or the refusal.
`spec/run`: a spec and the same records in several arrival orders, and what
each slot holds.
-/

namespace Gen.SpecVectors

open Loops

def o (kvs : List (String × Json)) : Json := .obj kvs
def s (x : String) : Json := .str x
def a (xs : List Json) : Json := .arr xs

/-! ## Parsing -/

/-- A spec counting what one `where` test chooses. -/
def whereOn (field : String) (test : String × Json) : Json :=
  o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("c", o [("count", .bool true), ("where", o [(field, o [test])])])])]

/-- A spec with one slot, as written. -/
def slotOf (slot : Json) : Json :=
  o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o [("c", slot)])]

/-- A spec counting from one source, as written. -/
def fromOf (source : Json) : Json :=
  o [("vertex", s "x"), ("from", a [source]), ("fold", o [("c", o [("count", .bool true)])])]

def parseCases : List (String × Json) := [
  ("phase2-null-fold", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", .null)]),
  ("phase2-unsafe-limit", slotOf (o [("collect", .bool true),
    ("limit", .number (Number.ofScientific false 9007199254740992 0))])),
  ("01-latest-by", o [("vertex", s "decisions"),
    ("note", s "Decisions from notes; the latest per topic."),
    ("from", a [o [("store", s "notes"), ("kind", a [s "decision"])],
      o [("store", s "rfc@home"), ("kind", a [s "decision"])]]),
    ("fold", o [("by-topic", o [("latest-by", s "of"), ("follow", s "prev")])]),
    ("boundary", a [o [("when", s "release")]])]),
  ("02-where-and-limit", o [("vertex", s "open-tasks"),
    ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
    ("fold", o [("recent", o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 5),
      ("where", o [("status", o [("is", s "open")]), ("env.kind", o [("not", s "draft")])])])])]),
  ("03-numeric", o [("vertex", s "estimates"),
    ("from", a [o [("store", s "."), ("kind", a [s "task"]), ("about", s "proj:loops")]]),
    ("fold", o [("tasks", o [("count", .bool true)]),
      ("total", o [("sum", s "est")]),
      ("largest", o [("max", s "est"), ("where", o [("est", o [("at-least", .int 1), ("below", .int 100)])])]),
      ("flagged", o [("count", .bool true), ("where", o [("blocked", o [("present", .bool true)])])])])]),
  ("04-envelope-fields", o [("vertex", s "events"),
    ("from", a [o [("store", s ".")]]),
    ("fold", o [("these", o [("count", .bool true), ("where", o [
      ("env.kind", o [("is", s "event")]),
      ("env.observer", o [("not", s "sl:bot")]),
      ("env.about", o [("is", s "proj:loops")]),
      ("env.prev", o [("is", s (idText (fixtureId 0x41)))]),
      ("env.under", o [("absent", .bool true)]),
      ("env.erases", o [("absent", .bool true)]),
      ("env.emitted", o [("at-least", s "2026-10-01T00:00:00Z")]),
      ("env.occurred", o [("at-least", s "2026-10"), ("below", s "2026-11-01T00:00:00Z"),
        ("not", s "2026-10-01/2026-10-31")])])])])]),
  ("05-no-fold", o [("vertex", s "x"), ("from", a [o [("store", s ".")]])]),
  ("06-from-about-anchor", fromOf (o [("about", s (idText (fixtureId 0x71))), ("store", s "."), ("kind", a [s "note"])])),
  ("10-not-object", a [s "vertex"]),
  ("11-unknown-key", o [("vertex", s "x"), ("query", s "select *")]),
  ("12-vertex-missing", o [("fold", o [])]),
  ("13-two-fold-types", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("both", o [("count", .bool true), ("sum", s "n")])])]),
  ("14-no-fold-type", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("none", o [("where", o [])])])]),
  ("15-regex-refused", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("m", o [("count", .bool true), ("where", o [("title", o [("matches", s "^auth")])])])])]),
  ("16-limit-on-count", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("c", o [("count", .bool true), ("limit", .int 3)])])]),
  ("17-follow-on-sum", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("t", o [("sum", s "est"), ("follow", s "prev")])])]),
  ("18-from-not-array", o [("vertex", s "x"), ("from", o [("store", s ".")])]),
  ("19-sum-of-nothing", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("t", o [("sum", .bool true)])])]),
  ("20-from-missing", o [("vertex", s "x"), ("fold", o [("c", o [("count", .bool true)])])]),
  ("21-from-empty", o [("vertex", s "x"), ("from", a []), ("fold", o [("c", o [("count", .bool true)])])]),
  ("22-env-unknown-field", whereOn "env.id" ("present", .bool true)),
  ("23-env-text-ordered", whereOn "env.kind" ("at-least", s "a")),
  ("24-env-time-not-span", whereOn "env.occurred" ("at-least", s "2026-13")),
  ("25-env-time-as-number", whereOn "env.occurred" ("at-least", .int 2026)),
  ("26-env-id-not-record-id", whereOn "env.prev" ("is", s "r:x")),
  ("27-env-text-as-number", whereOn "env.kind" ("is", .int 5)),
  ("28-payload-bound-not-number", whereOn "est" ("at-least", s "5")),
  ("29-env-time-is-not-span", whereOn "env.occurred" ("is", s "2026-13")),
  ("30-env-id-ordered", whereOn "env.under" ("below", s (idText (fixtureId 0x41)))),
  ("31-present-false", whereOn "blocked" ("present", .bool false)),
  ("32-env-text-as-bool", whereOn "env.kind" ("not", .bool true)),
  ("33-env-id-as-number", whereOn "env.prev" ("is", .int 5)),
  ("34-env-observer-not-typed", whereOn "env.observer" ("is", s "alice")),
  ("35-env-kind-empty", whereOn "env.kind" ("is", s "")),
  ("36-env-about-not-typed", whereOn "env.about" ("not", s "loops")),
  ("37-payload-is-null", whereOn "status" ("is", .null)),
  ("38-where-empty", slotOf (o [("count", .bool true), ("where", o [])])),
  ("39-latest-by-without-follow", slotOf (o [("latest-by", s "of")])),
  ("40-repeated-operator", slotOf (o [("count", .bool true),
    ("where", o [("env.kind", o [("is", s "task"), ("is", s "note")])])])),
  ("41-repeated-key", o [("vertex", s "x"), ("vertex", s "y"), ("from", a [o [("store", s ".")]])]),
  -- a slot is checked in order: keys, one fold type, follow, limit, the
  -- fold's argument, then where, its fields in the order written
  ("42-slot-errors-in-order", slotOf (o [("count", .bool false), ("limit", .int 0), ("follow", s "next"),
    ("where", o [("x", o [("matches", s ".")])])])),
  -- written order, which isn't sorted order
  ("43-where-errors-in-order", slotOf (o [("count", .bool true),
    ("where", o [("x", o [("matches", s ".")]), ("env.id", o [("present", .bool true)])])])),
  ("44-from-kind-empty", fromOf (o [("store", s "."), ("kind", a [s ""])])),
  ("45-from-about-not-typed", fromOf (o [("store", s "."), ("about", s "loops")])),
  ("46-from-kind-none", fromOf (o [("store", s "."), ("kind", a [])])),
  ("47-vertex-empty", o [("vertex", s ""), ("from", a [o [("store", s ".")]])]),
  ("48-vertex-not-text", o [("vertex", .int 5), ("from", a [o [("store", s ".")]])]),
  ("49-store-empty", fromOf (o [("store", s "")])),
  ("50-slot-name-empty", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("", o [("count", .bool true)])])]),
  ("51-sum-field-empty", slotOf (o [("sum", s "")])),
  ("61-min-field-empty", slotOf (o [("min", s "")])),
  ("62-max-field-empty", slotOf (o [("max", s "")])),
  ("52-where-field-empty", slotOf (o [("count", .bool true), ("where", o [("", o [("present", .bool true)])])])),
  -- the repeat found first, depth first in member order: the slot's, though
  -- `vertex` was written first and repeats after it
  ("53-repeat-depth-first", o [("vertex", s "x"),
    ("fold", o [("c", o [("count", .bool true), ("count", .bool true)])]),
    ("vertex", s "y"), ("from", a [o [("store", s ".")]])]),
  -- inside an array element, and before every other check
  ("54-repeat-in-from", o [("vertex", s "x"), ("from", a [o [("store", s "."), ("store", s ".")]]),
    ("query", s "select *")]),
  ("55-slot-limit-before-argument", slotOf (o [("count", .bool false), ("limit", .int 0)])),
  -- `from` checks store, kind, then about, however they are written
  ("56-from-errors-in-order", fromOf (o [("about", s "loops"), ("kind", a [s ""]), ("store", s ".")])),
  ("57-from-store-first", fromOf (o [("kind", a []), ("store", s "")])),
  ("58-slot-not-object", slotOf (.int 5)),
  ("59-slot-unknown-key", slotOf (o [("count", .bool true), ("order", s "newest")])),
  ("60-from-unknown-key", fromOf (o [("store", s "."), ("as", s "x")])),
  -- a repeat under an empty key: its path starts with the empty key
  ("63-repeat-under-empty-key", o [("", o [("x", .int 1), ("x", .int 2)]), ("vertex", s "x"),
    ("from", a [o [("store", s ".")]])]),
  ("64-slot-argument-before-where", slotOf (o [("count", .bool false), ("where", o [])])),
  -- latest-by runs over `of`; the old spelling is an unknown argument
  ("65-latest-by-about", slotOf (o [("latest-by", s "about"), ("follow", s "prev")])),
  ("66-from-of", fromOf (o [("of", s (idText (fixtureId 0x71))), ("store", s "."), ("kind", a [s "note"])])),
  ("67-from-of-not-typed", fromOf (o [("store", s "."), ("of", s "loops")])),
  -- `from` checks about before of, however they are written
  ("68-from-about-before-of", fromOf (o [("of", s "loops"), ("about", s "loops"), ("store", s ".")])),
  ("69-env-of", whereOn "env.of" ("is", s "task:1")),
  ("70-env-of-not-typed", whereOn "env.of" ("is", s "loops")),
  ("71-env-of-ordered", whereOn "env.of" ("at-least", s "task:1")),
  -- count, sum, min and max may follow `prev`; the parsed form keeps it as written
  ("72-follow-on-count", slotOf (o [("count", .bool true), ("follow", s "prev"),
    ("where", o [("status", o [("is", s "open")])])])),
  ("73-follow-on-extrema", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [("lo", o [("min", s "est"), ("follow", s "prev")]),
      ("hi", o [("follow", s "prev"), ("max", s "est")])])]),
  -- any other value is refused as on collect, and before the fold's argument
  ("74-follow-not-prev-on-count", slotOf (o [("count", .bool true), ("follow", s "next")])),
  ("75-follow-not-text-on-max", slotOf (o [("max", s "est"), ("follow", .bool true)])),
  ("76-follow-before-argument", slotOf (o [("sum", s ""), ("follow", s "head")])),
  ("77-argument-after-follow", slotOf (o [("min", s ""), ("follow", s "prev")])),
  -- limit stays collect's, following or not
  ("78-limit-on-followed-count", slotOf (o [("count", .bool true), ("follow", s "prev"), ("limit", .int 2)])),
  -- `by`: the envelope's kind and observer, the members of `about` under a
  -- typed-reference prefix, or a payload field; the parsed form keeps it as written
  ("79-by-forms", o [("vertex", s "x"), ("from", a [o [("store", s ".")]]),
    ("fold", o [
      ("k", o [("by", s "env.kind"), ("count", .bool true)]),
      ("o", o [("sum", s "est"), ("by", s "env.observer")]),
      ("p", o [("min", s "est"), ("follow", s "prev"), ("by", s "env.about:project")]),
      ("q", o [("max", s "est"), ("by", s "env.about:x-2")]),
      ("s", o [("count", .bool true), ("follow", s "prev"), ("by", s "status"),
        ("where", o [("status", o [("present", .bool true)])])]),
      ("c", o [("collect", .bool true), ("limit", .int 3), ("by", s "model")])])]),
  -- a bare name is a payload field, even one that looks like `about:`
  ("80-by-payload-named-about", slotOf (o [("count", .bool true), ("by", s "about:project")])),
  ("81-by-not-text", slotOf (o [("count", .bool true), ("by", .bool true)])),
  ("82-by-empty", slotOf (o [("count", .bool true), ("by", s "")])),
  ("83-by-env-other", slotOf (o [("count", .bool true), ("by", s "env.of")])),
  ("84-by-env-about-alone", slotOf (o [("count", .bool true), ("by", s "env.about")])),
  ("85-by-env-alone", slotOf (o [("count", .bool true), ("by", s "env.")])),
  ("86-by-empty-prefix", slotOf (o [("count", .bool true), ("by", s "env.about:")])),
  ("87-by-prefix-upper", slotOf (o [("count", .bool true), ("by", s "env.about:Project")])),
  ("88-by-prefix-digit-first", slotOf (o [("count", .bool true), ("by", s "env.about:2x")])),
  ("89-by-prefix-colon", slotOf (o [("count", .bool true), ("by", s "env.about:project:loops")])),
  ("90-by-on-latest-by", slotOf (o [("latest-by", s "of"), ("follow", s "prev"), ("by", s "env.kind")])),
  -- collect that follows `prev` groups too; only latest-by refuses `by`
  ("91-by-on-followed-collect", slotOf (o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 2), ("by", s "status")])),
  -- `by` is checked after `limit` and before the fold's argument and `where`
  ("92-limit-before-by", slotOf (o [("count", .bool true), ("by", s ""), ("limit", .int 2)])),
  ("93-by-before-argument", slotOf (o [("sum", s ""), ("by", s "env.of")])),
  ("94-argument-after-by", slotOf (o [("sum", s ""), ("by", s "env.kind")])),
  ("95-by-before-where", slotOf (o [("count", .bool true), ("by", .int 1),
    ("where", o [("env.nope", o [("is", s "x")])])])),
  ("96-follow-before-by", slotOf (o [("count", .bool true), ("by", s "env.about:"), ("follow", s "next")])),
  -- latest-by may follow `keys`; the parsed form writes `follow` as given
  ("97-follow-keys", o [("vertex", s "x"), ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
    ("fold", o [("state", o [("follow", s "keys"), ("latest-by", s "of")]),
      ("open", o [("latest-by", s "of"), ("follow", s "keys"),
        ("where", o [("status", o [("is", s "open")]), ("env.observer", o [("is", s "sl:kyle")])])]),
      ("heads", o [("latest-by", s "of"), ("follow", s "prev")])])]),
  -- every other fold type may follow `keys` too; the parsed form writes it
  ("98-follow-keys-on-count", slotOf (o [("count", .bool true), ("follow", s "keys")])),
  ("99-follow-keys-on-sum", slotOf (o [("sum", s "est"), ("follow", s "keys")])),
  ("100-follow-keys-on-min", slotOf (o [("min", s "est"), ("follow", s "keys")])),
  ("101-follow-keys-on-max", slotOf (o [("max", s "est"), ("follow", s "keys")])),
  ("102-follow-keys-on-collect", slotOf (o [("collect", .bool true), ("follow", s "keys")])),
  -- any other value on latest-by is refused too, its spelling exactly
  ("103-follow-key-on-latest-by", slotOf (o [("latest-by", s "of"), ("follow", s "key")])),
  ("104-follow-keys-upper-on-latest-by", slotOf (o [("latest-by", s "of"), ("follow", s "Keys")])),
  ("105-follow-keys-array-on-latest-by", slotOf (o [("latest-by", s "of"), ("follow", a [s "keys"])])),
  -- following `keys`, the argument and `limit` are still checked
  ("106-argument-after-follow-keys-on-sum", slotOf (o [("sum", s ""), ("follow", s "keys")])),
  ("107-limit-after-follow-keys-on-count", slotOf (o [("count", .bool true), ("limit", .int 0), ("follow", s "keys")])),
  ("108-limit-on-keyed-latest-by", slotOf (o [("latest-by", s "of"), ("follow", s "keys"), ("limit", .int 2)])),
  ("109-by-on-keyed-latest-by", slotOf (o [("latest-by", s "of"), ("follow", s "keys"), ("by", s "env.kind")])),
  ("110-argument-after-follow-keys", slotOf (o [("latest-by", s "about"), ("follow", s "keys")])),
  -- `keys` on count, sum, min, max and collect, with `by`, `where` and `limit`
  ("111-follow-keys-every-fold", o [("vertex", s "x"), ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
    ("fold", o [
      ("open", o [("count", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
      ("est", o [("by", s "env.about:project"), ("sum", s "est"), ("follow", s "keys")]),
      ("lo", o [("min", s "est"), ("follow", s "keys"), ("by", s "status")]),
      ("hi", o [("max", s "est"), ("follow", s "keys"), ("by", s "env.observer"),
        ("where", o [("env.kind", o [("is", s "task")])])]),
      ("recent", o [("limit", .int 3), ("collect", .bool true), ("follow", s "keys"), ("by", s "env.kind")])])]),
  ("112-limit-on-keyed-sum", slotOf (o [("sum", s "est"), ("follow", s "keys"), ("limit", .int 2)])),
  ("113-limit-zero-on-keyed-collect", slotOf (o [("collect", .bool true), ("follow", s "keys"), ("limit", .int 0)])),
  ("114-by-invalid-on-keyed-count", slotOf (o [("count", .bool true), ("follow", s "keys"), ("by", s "env.of")])),
  ("115-follow-keys-upper-on-count", slotOf (o [("count", .bool true), ("follow", s "Keys")])),
  -- a field's tests are an object of at least one test: an empty one, text and an array are refused at the field
  ("116-where-tests-empty", slotOf (o [("count", .bool true), ("where", o [("status", o [])])])),
  ("117-where-tests-not-object", slotOf (o [("count", .bool true), ("where", o [("status", s "open")])])),
  ("118-where-tests-array", slotOf (o [("count", .bool true), ("where", o [("status", a [])])])),
  -- two elements of one array each repeat a key: the refusal is at the first element's repeat, whichever key that is
  -- (a path has no index, so the two keys differ)
  ("119-repeat-first-element", o [("vertex", s "x"), ("from", a [
    o [("store", s "."), ("kind", a [s "note"]), ("kind", a [s "task"])],
    o [("store", s "."), ("of", s "task:1"), ("of", s "task:2")]])]),
  ("120-repeat-first-element-swapped", o [("vertex", s "x"), ("from", a [
    o [("store", s "."), ("of", s "task:1"), ("of", s "task:2")],
    o [("store", s "."), ("kind", a [s "note"]), ("kind", a [s "task"])]])]),
  ("gap03-repeat-key-before-value", o [("vertex", s "x"), ("vertex", o [("x", s "a"), ("x", s "b")]),
    ("from", a [o [("store", s ".")]])]),
  ("gap05-limit-zero", slotOf (o [("collect", .bool true), ("limit", .int 0)])),
  ("gap09-absent", o [("vertex", s "x")]),
  ("gap09-empty-array", o [("vertex", s "x"), ("from", a [])]),
  ("gap09-empty-object", o [("vertex", s "x"), ("from", o [])]),
  ("gap09-empty-text", o [("vertex", s "x"), ("from", s "")]),
  ("gap09-null", o [("vertex", s "x"), ("from", .null)]),
  ("gap08-fractional-bound", slotOf (o [("count", .bool true),
    ("where", o [("est", o [("at-least", .number (Number.ofScientific false 15 (-1)))])])])),
  ("gap08-whole-bound", slotOf (o [("count", .bool true),
    ("where", o [("est", o [("below", .int 2)])])])),
  -- D3: a number past binary64's range is refused where any number out of range is
  ("d3-overflow-bound", slotOf (o [("count", .bool true),
    ("where", o [("est", o [("at-least", .number (Number.ofScientific false 1 999))])])])),
  ("d3-overflow-argument", slotOf (o [("count", .bool true),
    ("where", o [("est", o [("is", .number (Number.ofScientific true 1 999))])])])),
  ("d3-overflow-limit", slotOf (o [("collect", .bool true),
    ("limit", .number (Number.ofScientific false 1 999))]))
] ++ [(-9007199254740992 : Int), -9007199254740991, 9007199254740991, 9007199254740992].flatMap fun i =>
  ["is", "not", "at-least", "below"].map fun op =>
    (s!"gap08-{op}-{i}", slotOf (o [("count", .bool true), ("where", o [("est", o [(op, .int i)])])]))

def jRefusal (r : Refusal) : J := .obj [("refused", .str r.code), ("path", .str r.path)]

def parseVector (name : String) (j : Json) : Except String J := do
  let expect ← match Spec.ofJson j with
    | .ok sp =>
      -- the parsed form parses back to itself
      match Spec.ofJson sp.toJson with
      | .ok sp' => if sp' != sp then throw s!"spec/parse/{name}: no round trip"
      | .error _ => throw s!"spec/parse/{name}: its parsed form is refused"
      pure (.obj [("parsed", Records.ofJson sp.toJson)])
    | .error r => pure (jRefusal r)
  -- a spec that repeats a key also comes as written, since a JSON parser
  -- may keep only one of the repeats
  let text := if j.keysDistinct then [] else [("spec_text", J.str (Records.rawText j))]
  return vector name "spec/parse" ["spec.parse"] (.obj ([("spec", Records.ofJson j)] ++ text)) expect

/-! ## Running -/

def task (id : RecordId) (thing : String) (occurred : Nat) (fields : List (String × Val))
    (prev : List RecordId := []) (ok : Reads occurred := by decide) : Record :=
  { id, kind := "task", observer := "sl:kyle", of := some thing, value := "", prev, occurred := some (claimAt occurred ok),
    emitted := stampAt occurred ok, fields }

-- task:1 opens, then closes; task:2 is open; task:3 is open but its payload is withheld
def t1 := task (fixtureId 0x41) "task:1" 100 [("status", .str "open"), ("est", .int 3)]
def t2 := task (fixtureId 0x42) "task:1" 200 [("status", .str "closed"), ("est", .int 3)] (prev := [fixtureId 0x41])
def t3 := task (fixtureId 0x43) "task:2" 150 [("status", .str "open"), ("est", .int 5), ("blocked", .bool true)]
def t4 := (task (fixtureId 0x44) "task:3" 160 [("status", .str "open"), ("est", .int 8)]).withhold
def n1 : Record := { task (fixtureId 0x45) "task:2" 170 [] with kind := "note" }

-- events with every kind of `occurred`: a month and the same span spelled
-- as days, a day that starts with them, an instant, an open span, one in
-- September, one without, and one withheld, whose envelope tests still answer
def ev (id : RecordId) (occurred : Option Claim) (emitted : Nat) (ok : Reads emitted := by decide) : Record :=
  { id, kind := "event", observer := "sl:kyle", about := [s!"event:{id}"], value := "", prev := [], occurred,
    emitted := stampAt emitted ok }

def e1 := ev (fixtureId 0x51) (some (claim "2026-10")) 100
def e2 := ev (fixtureId 0x52) (some (claimAt 300)) 300
def e3 := ev (fixtureId 0x53) (some (claim "2026-10-01/2026-10-31")) 110
def e4 := ev (fixtureId 0x54) none 200
def e5 := ev (fixtureId 0x55) (some (claim "2026-09")) 120
def e6 := (ev (fixtureId 0x56) (some (claimAt 600)) 600).withhold
def e7 := ev (fixtureId 0x57) (some (claim "2026-10-01T00:02:00Z/..")) 130
def e8 := ev (fixtureId 0x58) (some (claim "2026-10-01")) 140

def timeSpec : Json := o [("vertex", s "events"),
  ("from", a [o [("store", s "."), ("kind", a [s "event"])]]),
  ("fold", o [
    ("october", o [("count", .bool true),
      ("where", o [("env.occurred", o [("at-least", s "2026-10"), ("below", s "2026-11")])])]),
    ("the-month", o [("collect", .bool true), ("where", o [("env.occurred", o [("is", s "2026-10")])])]),
    ("not-the-month", o [("count", .bool true), ("where", o [("env.occurred", o [("not", s "2026-10")])])]),
    ("before-five", o [("count", .bool true),
      ("where", o [("env.occurred", o [("below", s "2026-10-01T00:05:00Z")])])]),
    ("undated", o [("collect", .bool true), ("where", o [("env.occurred", o [("absent", .bool true)])])]),
    ("emitted-late", o [("collect", .bool true),
      ("where", o [("env.emitted", o [("at-least", s "2026-10-01T00:03:00Z")])])])])]

-- numeric bounds and complements on a payload field
def boundSpec : Json := o [("vertex", s "estimates"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("mid", o [("count", .bool true), ("where", o [("est", o [("at-least", .int 4), ("below", .int 8)])])]),
    ("small", o [("count", .bool true), ("where", o [("est", o [("below", .int 4)])])]),
    ("not-three", o [("count", .bool true), ("where", o [("est", o [("not", .int 3)])])]),
    ("total", o [("sum", s "est")]),
    ("unblocked", o [("count", .bool true), ("where", o [("blocked", o [("absent", .bool true)])])])])]

-- every envelope field, tested on records by several observers: a revision
-- by someone else, a record that mentions two things (erased by another), one
-- acting under a grant, and a root that mentions nothing, dated a month before
-- it was stamped and withheld; payload `kind` differs from `env.kind`. Each
-- record is of the first thing it mentions; the root, of its own id.
def said (id : RecordId) (observer : String) (about : List String) (t : Nat)
    (ok : Reads t := by decide) : Record :=
  { id, kind := "task", observer, about, of := about.head?, value := "", prev := [], emitted := stampAt t ok }

def a1 : Record := { said (fixtureId 0x61) "sl:alice" ["task:1"] 100 with fields := [("kind", .str "chore")] }
def a2 : Record := { said (fixtureId 0x62) "sl:bob" ["task:1"] 200 with prev := [fixtureId 0x61], fields := [("kind", .str "bug")] }
def a3 : Record := (said (fixtureId 0x63) "sl:alice" ["task:2", "proj:loops"] 300).erase
def a4 : Record := { said (fixtureId 0x64) "sl:carol" ["task:3"] 400 with under := some (fixtureId 0x99), erases := [fixtureId 0x63] }
def a5 : Record := { said (fixtureId 0x65) "sl:alice" [] 500 with occurred := some (claim "2026-09") }.withhold
def a6 : Record := { said (fixtureId 0x66) "sl:bob" ["task:4"] 150 with kind := "chore" }

def envSpec : Json := o [("vertex", s "envelopes"),
  ("from", a [o [("store", s ".")]]),
  ("fold", o [
    ("by-alice", o [("collect", .bool true), ("where", o [("env.observer", o [("is", s "sl:alice")])])]),
    ("not-alice", o [("count", .bool true), ("where", o [("env.observer", o [("not", s "sl:alice")])])]),
    ("loops", o [("collect", .bool true), ("where", o [("env.about", o [("is", s "proj:loops")])])]),
    ("not-task-2", o [("collect", .bool true), ("where", o [("env.about", o [("not", s "task:2")])])]),
    ("about-nothing", o [("collect", .bool true), ("where", o [("env.about", o [("absent", .bool true)])])]),
    ("of-task-1", o [("collect", .bool true), ("where", o [("env.of", o [("is", s "task:1")])])]),
    ("of-loops", o [("count", .bool true), ("where", o [("env.of", o [("is", s "proj:loops")])])]),
    ("without-of", o [("collect", .bool true), ("where", o [("env.of", o [("absent", .bool true)])])]),
    ("revisions", o [("collect", .bool true), ("where", o [("env.prev", o [("present", .bool true)])])]),
    ("revises-first", o [("count", .bool true), ("where", o [("env.prev", o [("is", s (idText (fixtureId 0x61)))])])]),
    ("granted", o [("collect", .bool true), ("where", o [("env.under", o [("is", s (idText (fixtureId 0x99)))])])]),
    ("ungranted", o [("count", .bool true), ("where", o [("env.under", o [("absent", .bool true)])])]),
    ("erasing", o [("collect", .bool true), ("where", o [("env.erases", o [("is", s (idText (fixtureId 0x63)))])])]),
    ("chores", o [("collect", .bool true), ("where", o [("kind", o [("is", s "chore")])])]),
    ("chore-kind", o [("collect", .bool true), ("where", o [("env.kind", o [("is", s "chore")])])]),
    ("emitted-since", o [("collect", .bool true),
      ("where", o [("env.emitted", o [("at-least", s "2026-10-01T00:04:00Z")])])]),
    ("emitted-is", o [("count", .bool true),
      ("where", o [("env.emitted", o [("is", s "2026-10-01T00:01:40Z")])])]),
    ("alice-current", o [("latest-by", s "of"), ("follow", s "prev"),
      ("where", o [("env.observer", o [("is", s "sl:alice")])])]),
    ("alice-heads", o [("collect", .bool true), ("follow", s "prev"),
      ("where", o [("env.observer", o [("is", s "sl:alice")])])])])]

def taskSpec : Json := o [("vertex", s "tasks"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("open", o [("latest-by", s "of"), ("follow", s "prev"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("current", o [("latest-by", s "of"), ("follow", s "prev")]),
    ("newest", o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 1)]),
    ("every", o [("collect", .bool true)]),
    ("observations", o [("count", .bool true)]),
    ("open-observations", o [("count", .bool true), ("where", o [("status", o [("is", s "open")])])]),
    ("est", o [("sum", s "est")]),
    ("blocked", o [("count", .bool true), ("where", o [("blocked", o [("present", .bool true)])])]),
    ("open-now", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("est-now", o [("sum", s "est"), ("follow", s "prev")])])]

-- Hidden payloads must not supply numbers, even though their placeholder is zero.
-- The erased task's eraser is no task, so `from` does not take it.
def hiddenNumbersSpec : Json := o [("vertex", s "hidden-numbers"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [("count", o [("count", .bool true)]),
    ("sum", o [("sum", s "n")]), ("min", o [("min", s "n")]),
    ("max", o [("max", s "n")])])]

-- a root names the thing its id names: `from.of` takes it and its revision,
-- while `env.of` and `env.about` read only what each envelope says. `x4`
-- mentions the root without being of it: `from.about` takes it and the
-- revision, which mentions the root too, but not the root, which mentions
-- nothing.
def x1 : Record := { said (fixtureId 0x71) "sl:alice" [] 100 with kind := "note" }
def x2 : Record := { said (fixtureId 0x72) "sl:bob" [idText (fixtureId 0x71)] 200 with kind := "note", prev := [fixtureId 0x71] }
def x3 : Record := { said (fixtureId 0x73) "sl:bob" ["task:1"] 300 with kind := "note" }
def x4 : Record := { said (fixtureId 0x74) "sl:carol" ["task:1", idText (fixtureId 0x71)] 400 with kind := "note" }

def rootSpec (source : String × Json) : Json := o [("vertex", s "rooted"),
  ("from", a [o [("store", s "."), source]]),
  ("fold", o [
    ("all", o [("collect", .bool true)]),
    ("current", o [("latest-by", s "of"), ("follow", s "prev")]),
    ("of-root", o [("collect", .bool true), ("where", o [("env.of", o [("is", s (idText (fixtureId 0x71)))])])]),
    ("says-about", o [("collect", .bool true), ("where", o [("env.about", o [("is", s (idText (fixtureId 0x71)))])])])])]

def readableExtremaSpec : Json := o [("vertex", s "extrema"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [("lo", o [("min", s "est")]), ("hi", o [("max", s "est")])])]

def sourceUnionSpec : Json := o [("vertex", s "union"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])],
    o [("store", s "."), ("kind", a [s "note"])]]),
  ("fold", o [("c", o [("count", .bool true)])])]

-- Count, sum, min and max following `prev` fold the whole-reader heads.
-- task:1 opens and closes; task:2's estimate is revised down; task:3's head is
-- withheld; a note of task:2 is a second head there, which `from` does not
-- take; task:4 mentions task:1 and is no head of it.
def c1 := task (fixtureId 0x81) "task:1" 100 [("status", .str "open"), ("est", .int 3)]
def c2 := task (fixtureId 0x82) "task:1" 200 [("status", .str "closed"), ("est", .int 3)] (prev := [fixtureId 0x81])
def c3 := task (fixtureId 0x83) "task:2" 150 [("status", .str "open"), ("est", .int 9)]
def c4 := task (fixtureId 0x84) "task:2" 250 [("status", .str "open"), ("est", .int 8)] (prev := [fixtureId 0x83])
def c5 := (task (fixtureId 0x85) "task:3" 160 [("status", .str "open"), ("est", .int 7)]).withhold
def c6 : Record := { task (fixtureId 0x86) "task:2" 170 [("est", .int 50)] with kind := "note" }
def c7 : Record := { task (fixtureId 0x87) "task:4" 180 [("status", .str "open"), ("est", .int 2)] with about := ["task:1"] }

def followSpec : Json := o [("vertex", s "current"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("current", o [("count", .bool true), ("follow", s "prev")]),
    ("every", o [("count", .bool true)]),
    ("open", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("open-every", o [("count", .bool true), ("where", o [("status", o [("is", s "open")])])]),
    ("by-kyle", o [("count", .bool true), ("follow", s "prev"),
      ("where", o [("env.observer", o [("is", s "sl:kyle")])])]),
    ("est", o [("sum", s "est"), ("follow", s "prev")]),
    ("est-every", o [("sum", s "est")]),
    ("lo", o [("min", s "est"), ("follow", s "prev")]),
    ("hi", o [("max", s "est"), ("follow", s "prev")]),
    ("hi-every", o [("max", s "est")]),
    ("current-heads", o [("collect", .bool true), ("follow", s "prev")])])]

-- a fork: two revisions of task:5's root, by two observers; both are current
def k0 := task (fixtureId 0x88) "task:5" 100 [("status", .str "open"), ("est", .int 4)]
def k1 : Record := { task (fixtureId 0x89) "task:5" 200 [("status", .str "open"), ("est", .int 6)] (prev := [fixtureId 0x88]) with observer := "sl:alice" }
def k2 : Record := { task (fixtureId 0x8a) "task:5" 210 [("status", .str "closed"), ("est", .int 1)] (prev := [fixtureId 0x88]) with observer := "sl:bob" }

-- an erased head: task:6's revision is erased by a record of its own; the
-- revision still supersedes the root, and only its envelope is held
def g0 := task (fixtureId 0x8b) "task:6" 100 [("status", .str "open"), ("est", .int 4)]
def g1 := (task (fixtureId 0x8c) "task:6" 200 [("status", .str "open"), ("est", .int 10)] (prev := [fixtureId 0x8b])).erase
def g2 : Record := { said (fixtureId 0x8d) "sl:kyle" [] 300 with kind := "erasure", erases := [fixtureId 0x8c] }

def followHeadsSpec : Json := o [("vertex", s "current"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("current", o [("count", .bool true), ("follow", s "prev")]),
    ("open", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("est", o [("sum", s "est"), ("follow", s "prev")]),
    ("lo", o [("min", s "est"), ("follow", s "prev")]),
    ("hi", o [("max", s "est"), ("follow", s "prev")]),
    ("est-every", o [("sum", s "est")]),
    ("heads", o [("latest-by", s "of"), ("follow", s "prev")])])]

-- two independent current heads: one held and closed, one withheld. A
-- followed numeric slot with a payload test filters out the first and counts
-- the second unread once, not once for its test and again for its field.
def p1 := task (fixtureId 0x8e) "task:7" 100 [("status", .str "closed"), ("est", .int 99)]
def p2 := (task (fixtureId 0x8f) "task:8" 110 [("status", .str "open"), ("est", .int 5)]).withhold

def followTestSpec : Json := o [("vertex", s "open-estimates"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("est", o [("sum", s "est"), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("lo", o [("min", s "est"), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("hi", o [("max", s "est"), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])])])]

-- Alice's open task is replaced by Bob's withheld revision. Supersession comes
-- first, so the current record is Bob's: its envelope test fails, and the
-- unknown payload test never counts. Filtering by envelope first would bring
-- Alice's back; counting the payload before the envelope would read one unread.
def v1 : Record := { task (fixtureId 0x90) "task:1" 100 [("status", .str "open")] with observer := "sl:alice" }
def v2 : Record := { (task (fixtureId 0x91) "task:1" 200 [("status", .str "open")] (prev := [fixtureId 0x90])).withhold with observer := "sl:bob" }

def followEnvSpec : Json := o [("vertex", s "alice-open"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("alice-open", o [("count", .bool true), ("follow", s "prev"),
      ("where", o [("status", o [("is", s "open")]), ("env.observer", o [("is", s "sl:alice")])])])])]

/-! ### `by` -/

/-- A task with what it mentions and who said it. -/
def tb (id : RecordId) (thing : String) (t : Nat) (observer : String) (about : List String)
    (fields : List (String × Val)) (prev : List RecordId := []) (ok : Reads t := by decide) : Record :=
  { task id thing t fields prev ok with observer, about }

-- Six records over three observers. Alice's second task is withheld; Bob's
-- note has no status and no project; Carol's estimate is text. `prio` holds a
-- whole number, the text "1", a boolean, a fraction and a negative number.
def b1 := tb (fixtureId 0x21) "task:1" 100 "sl:alice" ["project:loops"]
  [("status", .str "open"), ("est", .int 3), ("prio", .int 1)]
def b2 := tb (fixtureId 0x22) "task:2" 110 "sl:bob" ["project:loops", "project:siftd"]
  [("status", .str "closed"), ("est", .int 5), ("prio", .bool true)]
def b3 := (tb (fixtureId 0x23) "task:3" 120 "sl:alice" ["project:siftd"]
  [("status", .str "open"), ("est", .int 7), ("prio", .int 1)]).withhold
def b4 : Record := { tb (fixtureId 0x24) "task:4" 130 "sl:bob" []
  [("est", .int 2), ("prio", .number (Number.ofScientific false 15 (-1)))] with kind := "note" }
def b5 := tb (fixtureId 0x25) "task:5" 140 "sl:carol" ["project:loops", "host:a"]
  [("status", .str "open"), ("est", .str "8"), ("prio", .str "1")]
def b6 := tb (fixtureId 0x26) "task:6" 150 "sl:alice" ["host:b", "project:x-2"]
  [("status", .str "open"), ("est", .int 4), ("prio", .int (-2))]

def byKeysSpec : Json := o [("vertex", s "grouped"),
  ("from", a [o [("store", s ".")]]),
  ("fold", o [
    ("per-kind", o [("count", .bool true), ("by", s "env.kind")]),
    ("per-observer", o [("count", .bool true), ("by", s "env.observer")]),
    ("est-per-observer", o [("sum", s "est"), ("by", s "env.observer")]),
    ("per-status", o [("count", .bool true), ("by", s "status")]),
    ("per-prio", o [("count", .bool true), ("by", s "prio")]),
    ("open-per-observer", o [("count", .bool true), ("by", s "env.observer"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("open-per-status", o [("count", .bool true), ("by", s "status"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("open", o [("count", .bool true), ("where", o [("status", o [("is", s "open")])])])])]

def byAboutSpec : Json := o [("vertex", s "per-project"),
  ("from", a [o [("store", s ".")]]),
  ("fold", o [
    ("per-project", o [("count", .bool true), ("by", s "env.about:project")]),
    ("est-per-project", o [("sum", s "est"), ("by", s "env.about:project")]),
    ("lo-per-project", o [("min", s "est"), ("by", s "env.about:project")]),
    ("hi-per-project", o [("max", s "est"), ("by", s "env.about:project")]),
    ("open-per-project", o [("count", .bool true), ("by", s "env.about:project"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("per-host", o [("count", .bool true), ("by", s "env.about:host")]),
    ("every", o [("count", .bool true)])])]

-- A revision whose `prev` names a record of another thing is refused, by
-- admission over everything held. Grouped by project, the refused record would
-- be alone in its group, where nothing refuses it.
def y1 : Record := { tb (fixtureId 0x31) "task:1" 100 "sl:kyle" ["project:x"] [] with n := 5 }
def y2 : Record := { tb (fixtureId 0x32) "task:2" 200 "sl:kyle" ["project:y"] [] [fixtureId 0x31] with n := 1 }

def byAdmitSpec : Json := o [("vertex", s "admitted"),
  ("from", a [o [("store", s ".")]]),
  ("fold", o [
    ("count", o [("count", .bool true), ("by", s "env.about:project")]),
    ("sum", o [("sum", s "n"), ("by", s "env.about:project")]),
    ("plain", o [("count", .bool true)])])]

-- A task revised as it moves from project x to project y, a task forked in
-- project x, an erased head and a withheld head. The whole view's heads come
-- first: grouping before them would keep the moved task's old version current
-- in x, where its successor is not. The erased head's eraser is no task, so
-- `from` does not take it.
def m0 := tb (fixtureId 0x33) "task:1" 100 "sl:kyle" ["project:x"] [("status", .str "open"), ("est", .int 3)]
def m1 := tb (fixtureId 0x34) "task:1" 200 "sl:kyle" ["project:y"] [("status", .str "open"), ("est", .int 4)] [fixtureId 0x33]
def m2 := tb (fixtureId 0x35) "task:2" 110 "sl:kyle" ["project:x"] [("status", .str "open"), ("est", .int 6)]
def m3 := tb (fixtureId 0x36) "task:2" 210 "sl:alice" ["project:x"] [("status", .str "closed"), ("est", .int 1)] [fixtureId 0x35]
def m4 := tb (fixtureId 0x37) "task:2" 220 "sl:bob" ["project:x", "project:y"] [("status", .str "open"), ("est", .int 2)] [fixtureId 0x35]
def m5 := (tb (fixtureId 0x38) "task:3" 120 "sl:kyle" ["project:y"] [("status", .str "open"), ("est", .int 9)]).erase
def m6 := (tb (fixtureId 0x39) "task:4" 130 "sl:kyle" ["project:z"] [("status", .str "open"), ("est", .int 5)]).withhold
def m7 : Record := { said (fixtureId 0x3d) "sl:kyle" [] 230 with kind := "erasure", erases := [fixtureId 0x38] }

def byFollowSpec : Json := o [("vertex", s "current-per-project"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("current", o [("count", .bool true), ("follow", s "prev"), ("by", s "env.about:project")]),
    ("open", o [("count", .bool true), ("follow", s "prev"), ("by", s "env.about:project"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("open-by-observer", o [("count", .bool true), ("follow", s "prev"), ("by", s "env.observer"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("est", o [("sum", s "est"), ("follow", s "prev"), ("by", s "env.about:project")]),
    ("lo", o [("min", s "est"), ("follow", s "prev"), ("by", s "status")]),
    ("hi", o [("max", s "est"), ("follow", s "prev"), ("by", s "status")]),
    ("every", o [("count", .bool true), ("by", s "env.about:project")]),
    ("current-plain", o [("count", .bool true), ("follow", s "prev")]),
    ("open-plain", o [("count", .bool true), ("follow", s "prev"),
      ("where", o [("status", o [("is", s "open")])])])])]

-- Collect by a payload key: each group is collect's own result, newest
-- first, and `limit` applies within each group.
def byCollectSpec : Json := o [("vertex", s "recent-per-status"),
  ("from", a [o [("store", s ".")]]),
  ("fold", o [
    ("recent", o [("collect", .bool true), ("limit", .int 1), ("by", s "status")]),
    ("all", o [("collect", .bool true), ("by", s "status")]),
    ("open-per-project", o [("collect", .bool true), ("by", s "env.about:project"),
      ("where", o [("status", o [("is", s "open")])])])])]

-- Keys in UTF-16 code-unit order: the empty text, "10" before "2", and
-- U+10000 (a surrogate pair, D800 DC00) before U+E000.
def kk (id : RecordId) (thing : String) (t : Nat) (k : Val) (ok : Reads t := by decide) : Record :=
  tb id thing t "sl:kyle" [] [("k", k)] (ok := ok)
def ko0 := kk (fixtureId 0x40) "task:k0" 100 (.str "2")
def ko1 := kk (fixtureId 0x41) "task:k1" 101 (.str "")
def ko2 := kk (fixtureId 0x42) "task:k2" 102 (.str "")
def ko3 := kk (fixtureId 0x43) "task:k3" 103 (.str "10")
def ko4 := kk (fixtureId 0x44) "task:k4" 104 (.str "𐀀")

def byOrderSpec : Json := o [("vertex", s "key-order"), ("from", a [o [("store", s ".")]]),
  ("fold", o [("per-k", o [("count", .bool true), ("by", s "k")])])]

-- Numbers as a reader reads their spelling, and booleans with their text.
def nk0 := kk (fixtureId 0xc0) "task:n0" 100 (.int 0)
def nk1 := kk (fixtureId 0xc1) "task:n1" 101 (.int 1)
def nk2 := kk (fixtureId 0xc2) "task:n2" 102 (.int 1000)
def nk3 := kk (fixtureId 0xc3) "task:n3" 103 (.int 1000000000000000)
def nk4 := kk (fixtureId 0xc4) "task:n4" 104 (.int 9007199254740991)
def nk5 := kk (fixtureId 0xc5) "task:n5" 105 (.int (-9007199254740991))
def nk6 := kk (fixtureId 0xc6) "task:n6" 106 (.int 9007199254740992)
def nk7 := kk (fixtureId 0xc7) "task:n7" 107 (.int 9007199254740993)
def nk8 := kk (fixtureId 0xc8) "task:n8" 108 (.bool false)
def nk9 := kk (fixtureId 0xc9) "task:n9" 109 (.str "false")
def nk10 := kk (fixtureId 0xca) "task:n10" 110 (.bool true)
def nk11 := kk (fixtureId 0xcb) "task:n11" 111 (.str "true")
def nk12 := kk (fixtureId 0xcc) "task:n12" 112 (.int 1)
def numberRaw : List (RecordId × String × String) := [
  (fixtureId 0xc0, "k", "-0"), (fixtureId 0xc1, "k", "1.0"), (fixtureId 0xc2, "k", "1e3"),
  (fixtureId 0xc3, "k", "1e+15"), (fixtureId 0xc4, "k", "9007199254740991"),
  (fixtureId 0xc5, "k", "-9007199254740991"), (fixtureId 0xc6, "k", "9007199254740992"),
  (fixtureId 0xc7, "k", "9007199254740993")]

def byNumbersSpec : Json := o [("vertex", s "number-keys"), ("from", a [o [("store", s ".")]]),
  ("fold", o [("per-k", o [("count", .bool true), ("by", s "k")]),
    ("plain", o [("count", .bool true)])])]

-- A prefix is matched with its colon: `proj` takes `proj:x` once, however
-- often it is mentioned, and `proj:z`, never `project:y`. A payload field
-- named `about:x` is read from the payload, not from `about`.
def pm := tb (fixtureId 0xd0) "task:p" 100 "sl:kyle" ["proj:x", "proj:x", "project:y", "proj:z"]
  [("about:x", .str "payload")]
def pn := tb (fixtureId 0xd1) "task:q" 110 "sl:kyle" ["x:mention"] [("about:x", .str "payload")]

def byMentionsSpec : Json := o [("vertex", s "mentions"), ("from", a [o [("store", s ".")]]),
  ("fold", o [("per-proj", o [("count", .bool true), ("by", s "env.about:proj")]),
    ("per-project", o [("count", .bool true), ("by", s "env.about:project")]),
    ("per-x", o [("count", .bool true), ("by", s "env.about:x")]),
    ("per-about-x", o [("count", .bool true), ("by", s "about:x")])])]

-- Collect with a limit of 1 per kind: three open tasks, two open notes, a
-- closed task and a withheld task. The limit keeps the newest open record of
-- each kind; the withheld one is unread at the top, whatever the limit.
def lc (id : RecordId) (thing : String) (t : Nat) (kind status : String) (ok : Reads t := by decide) : Record :=
  { tb id thing t "sl:kyle" [] [("status", .str status)] (ok := ok) with kind }
def limitRecords : List Record :=
  [lc (fixtureId 0xe0) "task:l0" 100 "task" "open", lc (fixtureId 0xe1) "task:l1" 110 "task" "open",
    lc (fixtureId 0xe2) "task:l2" 120 "note" "open", lc (fixtureId 0xe3) "task:l3" 130 "task" "closed",
    lc (fixtureId 0xe4) "task:l4" 140 "note" "open", lc (fixtureId 0xe5) "task:l5" 150 "task" "open",
    (lc (fixtureId 0xe6) "task:l6" 160 "task" "open").withhold]

def byLimitSpec : Json := o [("vertex", s "newest-open-per-kind"), ("from", a [o [("store", s ".")]]),
  ("fold", o [("newest", o [("collect", .bool true), ("limit", .int 1), ("by", s "env.kind"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("all", o [("collect", .bool true), ("by", s "env.kind"),
      ("where", o [("status", o [("is", s "open")])])])])]

-- Collect following `prev`, grouped: the moved task is listed only under its
-- current mention, while collect over every record lists both its versions;
-- both heads of the fork; the erased and withheld heads grouped by envelope
-- keys and unread under a payload key; `limit` per group beside an unknown
-- head; the fork's head that mentions two projects listed under each. That
-- head is the newest open one in both of its groups, so a limit taken once
-- over all the heads would list the same; `newest-by-observer` has a group
-- per observer, each with its own newest head, where that limit keeps one.
def byFollowCollectSpec : Json := o [("vertex", s "current-per-project"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("current", o [("collect", .bool true), ("follow", s "prev"), ("by", s "env.about:project")]),
    ("every", o [("collect", .bool true), ("by", s "env.about:project")]),
    ("by-observer", o [("collect", .bool true), ("follow", s "prev"), ("by", s "env.observer")]),
    ("by-status", o [("collect", .bool true), ("follow", s "prev"), ("by", s "status")]),
    ("newest-open", o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 1),
      ("by", s "env.about:project"), ("where", o [("status", o [("is", s "open")])])]),
    ("newest-by-observer", o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 1),
      ("by", s "env.observer")]),
    ("current-plain", o [("collect", .bool true), ("follow", s "prev")])])]

/-! ### Latest-by following `keys` -/

/-- A task emit of `thing` that sets `sets`, or without `sets` is read whole. -/
def ke (id : RecordId) (thing : String) (t : Nat) (sets : Option (List String))
    (fields : List (String × Val)) (prev : List RecordId := []) (observer : String := "sl:kyle")
    (ok : Reads t := by decide) : Record :=
  { task id thing t fields prev ok with sets, observer }

/-- A thing read by its live setters, beside the same thing read by its heads. -/
def keyedSpec : Json := o [("vertex", s "tasks"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("state", o [("latest-by", s "of"), ("follow", s "keys")]),
    ("heads", o [("latest-by", s "of"), ("follow", s "prev")])])]

-- The design note's example: W and W2 are read whole and of task:1, with no
-- `prev`; B sets `status` from W and is withheld. W2 is live for status, W is
-- not, and W is live for every other key; the heads are W2 and B.
def kw := ke (fixtureId 0x101) "task:1" 100 none [("status", .str "open"), ("title", .str "a")]
def kb := (ke (fixtureId 0x102) "task:1" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x101]).withhold
def kw2 := ke (fixtureId 0x103) "task:1" 300 none [("status", .str "open"), ("title", .str "b")]

-- Only whole-read emits: a chain on task:a and a fork on task:b. Live is a
-- head, so `whole` and `heads` are the heads `prev` gives, and there is no key.
def wa0 := ke (fixtureId 0x111) "task:a" 100 none [("status", .str "open")]
def wa1 := ke (fixtureId 0x112) "task:a" 200 none [("status", .str "closed")] [fixtureId 0x111]
def wb0 := ke (fixtureId 0x113) "task:b" 110 none [("status", .str "open")]
def wb1 := ke (fixtureId 0x114) "task:b" 210 none [("status", .str "open")] [fixtureId 0x113] "sl:alice"
def wb2 := ke (fixtureId 0x115) "task:b" 220 none [("status", .str "closed")] [fixtureId 0x113] "sl:bob"

-- A keyed chain: the root is read whole, then `status` is set, then `title`.
-- The head sets only `title`; the record that set `status` is still live for
-- it, and the root for every other key, `est` among them.
def kc0 := ke (fixtureId 0x121) "task:2" 100 none [("title", .str "a"), ("status", .str "open"), ("est", .int 3)]
def kc1 := ke (fixtureId 0x122) "task:2" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x121]
def kc2 := ke (fixtureId 0x123) "task:2" 300 (some ["title"]) [("title", .str "b")] [fixtureId 0x122]

-- Two concurrent setters of `status` from one root: both are live for it, and
-- both are heads. Then a pure join names both and sets nothing: it is the one
-- head, listed nowhere else, and both setters stay live for `status`.
def kf0 := ke (fixtureId 0x131) "task:3" 100 none [("status", .str "open"), ("est", .int 2)]
def kf1 := ke (fixtureId 0x132) "task:3" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x131] "sl:alice"
def kf2 := ke (fixtureId 0x133) "task:3" 210 (some ["status"]) [("status", .str "done")] [fixtureId 0x131] "sl:bob"
def kf3 := ke (fixtureId 0x134) "task:3" 300 (some []) [] [fixtureId 0x132, fixtureId 0x133]

-- A whole-read emit above keyed emits replaces everything under it.
def kr0 := ke (fixtureId 0x141) "task:4" 100 none [("status", .str "open"), ("title", .str "a")]
def kr1 := ke (fixtureId 0x142) "task:4" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x141]
def kr2 := ke (fixtureId 0x143) "task:4" 300 (some ["title"]) [("title", .str "b")] [fixtureId 0x142]
def kr3 := ke (fixtureId 0x144) "task:4" 400 none [("status", .str "open"), ("title", .str "c")] [fixtureId 0x143]

-- An erased and a withheld live setter keep their envelopes, `sets` among
-- them: each is still live for its key, which is unknown, and `shown` says so.
-- The eraser is of no task, so `from` does not take it.
def ku0 := ke (fixtureId 0x151) "task:5" 100 none [("status", .str "open"), ("title", .str "a")]
def ku1 := (ke (fixtureId 0x152) "task:5" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x151]).erase
def ku2 := (ke (fixtureId 0x153) "task:5" 300 (some ["title"]) [("title", .str "b")] [fixtureId 0x152]).withhold
def ku3 : Record := { said (fixtureId 0x154) "sl:kyle" [] 400 with kind := "erasure", erases := [fixtureId 0x152] }

def keyedUnheldSpec : Json := o [("vertex", s "tasks"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("state", o [("latest-by", s "of"), ("follow", s "keys")]),
    ("open", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("est-absent", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("est", o [("absent", .bool true)])])])])]

-- A payload key the record does not declare is not read: the head sets
-- `status` and also writes `est`, which the root, read whole, still sets.
def kn0 := ke (fixtureId 0x161) "task:6" 100 none [("status", .str "open"), ("est", .int 3)]
def kn1 := ke (fixtureId 0x162) "task:6" 200 (some ["status"]) [("status", .str "closed"), ("est", .int 9)] [fixtureId 0x161]

def keyedUndeclaredSpec : Json := o [("vertex", s "tasks"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("state", o [("latest-by", s "of"), ("follow", s "keys")]),
    ("est-3", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("est", o [("is", .int 3)])])]),
    ("est-9", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("est", o [("is", .int 9)])])]),
    ("est-9-heads", o [("latest-by", s "of"), ("follow", s "prev"), ("where", o [("est", o [("is", .int 9)])])])])]

-- `from` and the envelope tests choose heads, each alone. task:7 forks into
-- Alice's task and Bob's note: Alice's head is chosen, so the thing is
-- selected with both heads; `prev` shows Alice's alone. task:8's only head is
-- Bob's. task:9's root is Alice's and superseded by Bob's: its envelope is not
-- read, so nothing of task:9 is chosen.
def ks0 := ke (fixtureId 0x171) "task:7" 100 none [("status", .str "open")]
def ks1 := ke (fixtureId 0x172) "task:7" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x171] "sl:alice"
def ks2 : Record := { ke (fixtureId 0x173) "task:7" 210 (some ["title"]) [("title", .str "b")] [fixtureId 0x171] "sl:bob" with kind := "note" }
def ks3 := ke (fixtureId 0x174) "task:8" 110 none [("status", .str "open")] [] "sl:bob"
def ks4 := ke (fixtureId 0x175) "task:9" 120 none [("status", .str "open")] [] "sl:alice"
def ks5 := ke (fixtureId 0x176) "task:9" 220 (some ["status"]) [("status", .str "open")] [fixtureId 0x175] "sl:bob"

def keyedFromSpec : Json := o [("vertex", s "alice"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("alice", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("env.observer", o [("is", s "sl:alice")])])]),
    ("alice-heads", o [("latest-by", s "of"), ("follow", s "prev"), ("where", o [("env.observer", o [("is", s "sl:alice")])])]),
    ("bob", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("env.observer", o [("is", s "sl:bob")])])]),
    ("tasks", o [("latest-by", s "of"), ("follow", s "keys")])])]

-- Payload tests asked of the thing, over each field's live setters, whether or
-- not `from` takes them. task:10's two live setters of `status` agree;
-- task:11's differ; task:12's live setter of `status` is withheld; task:13
-- declares every key it sets, and no record sets `est`, so `est` is absent.
-- task:14's head is a note, which `from` does not take, beside a task head:
-- the note still answers for `status`.
def kp0 := ke (fixtureId 0x181) "task:10" 100 none [("status", .str "open"), ("est", .int 1)]
def kp1 := ke (fixtureId 0x182) "task:10" 200 (some ["status"]) [("status", .str "open")] [fixtureId 0x181] "sl:alice"
def kp2 := ke (fixtureId 0x183) "task:10" 210 (some ["status"]) [("status", .str "open")] [fixtureId 0x181] "sl:bob"
def kp3 := ke (fixtureId 0x184) "task:11" 110 none [("status", .str "open"), ("est", .int 1)]
def kp4 := ke (fixtureId 0x185) "task:11" 220 (some ["status"]) [("status", .str "open")] [fixtureId 0x184] "sl:alice"
def kp5 := ke (fixtureId 0x186) "task:11" 230 (some ["status"]) [("status", .str "closed")] [fixtureId 0x184] "sl:bob"
def kp6 := ke (fixtureId 0x187) "task:12" 120 none [("status", .str "open"), ("est", .int 1)]
def kp7 := (ke (fixtureId 0x188) "task:12" 240 (some ["status"]) [("status", .str "open")] [fixtureId 0x187]).withhold
def kp8 := ke (fixtureId 0x189) "task:13" 130 (some ["status", "title"]) [("status", .str "open"), ("title", .str "a")]
def kp9 := ke (fixtureId 0x18a) "task:13" 250 (some ["title"]) [("title", .str "b")] [fixtureId 0x189]
def kp10 := ke (fixtureId 0x18b) "task:14" 140 none [("status", .str "open"), ("est", .int 1)]
def kp11 := ke (fixtureId 0x18c) "task:14" 260 (some ["title"]) [("title", .str "b")] [fixtureId 0x18b]
def kp12 : Record := { ke (fixtureId 0x18d) "task:14" 270 (some ["status"]) [("status", .str "closed")] [fixtureId 0x18b] with kind := "note" }

def keyedTestSpec : Json := o [("vertex", s "open"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("open", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("not-open", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("status", o [("not", s "open")])])]),
    ("est-absent", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("est", o [("absent", .bool true)])])]),
    ("est-1", o [("latest-by", s "of"), ("follow", s "keys"), ("where", o [("est", o [("is", .int 1)])])]),
    ("open-est-2", o [("latest-by", s "of"), ("follow", s "keys"),
      ("where", o [("status", o [("is", s "open")]), ("est", o [("is", .int 2)])])]),
    ("open-heads", o [("latest-by", s "of"), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])])])]

-- Only admitted records the reader holds are above a record. task:15's root
-- is read whole; a record of task:16 that names it in `prev` and sets
-- `status` is refused (`prev-unrelated`), and a record of task:15 that sets
-- `title` names a record the reader does not hold. Neither is above the root,
-- which stays a head and live for every key, `title` among them.
def ka0 := ke (fixtureId 0x191) "task:15" 100 none [("status", .str "open"), ("title", .str "a")]
def ka1 := ke (fixtureId 0x192) "task:16" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x191]
def ka2 := ke (fixtureId 0x193) "task:15" 300 (some ["title"]) [("title", .str "b")] [fixtureId 0x1ff]

/-! ### Count, sum, min, max and collect following `keys` -/

/-- A task emit of `thing` that sets `sets`, with what it mentions and who said it. -/
def kt (id : RecordId) (thing : String) (t : Nat) (sets : Option (List String))
    (fields : List (String × Val)) (prev : List RecordId := []) (about : List String := [])
    (observer : String := "sl:kyle") (ok : Reads t := by decide) : Record :=
  { ke id thing t sets fields prev observer ok with about }

-- Count the open tasks. task:1 is opened whole and then retitled: its head
-- sets only `title`, so the head's payload has no `status`. task:2 is closed by
-- one emit and retitled by the next. task:3's head closes it. Read over heads,
-- no task is open and one is closed; read over live setters, task:1 is open and
-- the other two are closed.
def co0 := kt (fixtureId 0x201) "task:1" 100 none [("status", .str "open"), ("title", .str "a")]
def co1 := kt (fixtureId 0x202) "task:1" 200 (some ["title"]) [("title", .str "b")] [fixtureId 0x201]
def co2 := kt (fixtureId 0x203) "task:2" 110 none [("status", .str "open"), ("title", .str "c")]
def co3 := kt (fixtureId 0x204) "task:2" 210 (some ["status"]) [("status", .str "closed")] [fixtureId 0x203]
def co4 := kt (fixtureId 0x205) "task:2" 310 (some ["title"]) [("title", .str "d")] [fixtureId 0x204]
def co5 := kt (fixtureId 0x206) "task:3" 120 none [("status", .str "open"), ("title", .str "e")]
def co6 := kt (fixtureId 0x207) "task:3" 220 (some ["status"]) [("status", .str "closed")] [fixtureId 0x206]

def countOpenSpec : Json := o [("vertex", s "open-tasks"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("open", o [("count", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("open-heads", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("closed", o [("count", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "closed")])])]),
    ("closed-heads", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "closed")])])]),
    ("tasks", o [("count", .bool true), ("follow", s "keys")])])]

-- A fork across whole-read emits: two heads of task:1, both open. Following
-- `keys` the thing counts once, and its two live setters of `est` leave the
-- sum unread; following `prev` each head counts and adds. task:2 is one head.
def fk0 := kt (fixtureId 0x211) "task:1" 100 none [("status", .str "open"), ("est", .int 1)]
def fk1 := kt (fixtureId 0x212) "task:1" 200 none [("status", .str "open"), ("est", .int 2)] [fixtureId 0x211] [] "sl:alice"
def fk2 := kt (fixtureId 0x213) "task:1" 210 none [("status", .str "open"), ("est", .int 3)] [fixtureId 0x211] [] "sl:bob"
def fk3 := kt (fixtureId 0x214) "task:2" 110 none [("status", .str "open"), ("est", .int 5)]

def forkSpec : Json := o [("vertex", s "forked"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("count", o [("count", .bool true), ("follow", s "keys")]),
    ("count-heads", o [("count", .bool true), ("follow", s "prev")]),
    ("open", o [("count", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("open-heads", o [("count", .bool true), ("follow", s "prev"), ("where", o [("status", o [("is", s "open")])])]),
    ("est", o [("sum", s "est"), ("follow", s "keys")]),
    ("est-heads", o [("sum", s "est"), ("follow", s "prev")]),
    ("records", o [("collect", .bool true), ("follow", s "keys")]),
    ("records-heads", o [("collect", .bool true), ("follow", s "prev")])])]

-- A field read from its live setters. task:1's one live setter of `est` is
-- its root, under a head that sets `title`. task:2's two live setters hold the
-- same number, task:3's two different ones. task:4's live setter is withheld
-- and task:7's erased, by an eraser `from` does not take. task:5 declares only
-- `title`, so nothing sets `est`. task:6's one setter holds text.
def nu0 := kt (fixtureId 0x221) "task:1" 100 none [("est", .int 3)]
def nu1 := kt (fixtureId 0x222) "task:1" 200 (some ["title"]) [("title", .str "a")] [fixtureId 0x221]
def nu2 := kt (fixtureId 0x223) "task:2" 110 none [("est", .int 5)]
def nu3 := kt (fixtureId 0x224) "task:2" 210 (some ["est"]) [("est", .int 2)] [fixtureId 0x223] [] "sl:alice"
def nu4 := kt (fixtureId 0x225) "task:2" 220 (some ["est"]) [("est", .int 2)] [fixtureId 0x223] [] "sl:bob"
def nu5 := kt (fixtureId 0x226) "task:3" 120 none [("est", .int 1)]
def nu6 := kt (fixtureId 0x227) "task:3" 230 (some ["est"]) [("est", .int 4)] [fixtureId 0x226] [] "sl:alice"
def nu7 := kt (fixtureId 0x228) "task:3" 240 (some ["est"]) [("est", .int 6)] [fixtureId 0x226] [] "sl:bob"
def nu8 := kt (fixtureId 0x229) "task:4" 130 none [("est", .int 7)]
def nu9 := (kt (fixtureId 0x22a) "task:4" 250 (some ["est"]) [("est", .int 8)] [fixtureId 0x229]).withhold
def nu10 := kt (fixtureId 0x22b) "task:5" 140 (some ["title"]) [("title", .str "b"), ("est", .int 9)]
def nu11 := kt (fixtureId 0x22c) "task:6" 150 none [("est", .str "8")]
def nu12 := kt (fixtureId 0x22d) "task:7" 160 none [("est", .int 9)]
def nu13 := (kt (fixtureId 0x22e) "task:7" 260 (some ["est"]) [("est", .int 10)] [fixtureId 0x22d]).erase
def nu14 : Record := { said (fixtureId 0x22f) "sl:kyle" [] 270 with kind := "erasure", erases := [fixtureId 0x22e] }

def numbersSpec : Json := o [("vertex", s "estimates"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("sum", o [("sum", s "est"), ("follow", s "keys")]),
    ("min", o [("min", s "est"), ("follow", s "keys")]),
    ("max", o [("max", s "est"), ("follow", s "keys")]),
    ("sum-heads", o [("sum", s "est"), ("follow", s "prev")]),
    ("min-heads", o [("min", s "est"), ("follow", s "prev")]),
    ("max-heads", o [("max", s "est"), ("follow", s "prev")]),
    ("count", o [("count", .bool true), ("follow", s "keys")])])]

-- `by` a payload field, read from its live setters. task:1's one setter of
-- `status` is open; task:2's two are closed and done, so it is in both groups;
-- task:3's two are both closed, so it is in that group once; task:4's is
-- withheld, so it is unread; task:5 declares only `title`, so it has no
-- setter; task:6's setter has no `status`. The last two are ungrouped.
def bp0 := kt (fixtureId 0x231) "task:1" 100 none [("status", .str "open"), ("est", .int 1)]
def bp1 := kt (fixtureId 0x232) "task:1" 200 (some ["title"]) [("title", .str "a")] [fixtureId 0x231]
def bp2 := kt (fixtureId 0x233) "task:2" 110 none [("status", .str "open"), ("est", .int 2)]
def bp3 := kt (fixtureId 0x234) "task:2" 210 (some ["status"]) [("status", .str "closed")] [fixtureId 0x233] [] "sl:alice"
def bp4 := kt (fixtureId 0x235) "task:2" 220 (some ["status"]) [("status", .str "done")] [fixtureId 0x233] [] "sl:bob"
def bp5 := kt (fixtureId 0x236) "task:3" 120 none [("status", .str "open"), ("est", .int 3)]
def bp6 := kt (fixtureId 0x237) "task:3" 230 (some ["status"]) [("status", .str "closed")] [fixtureId 0x236] [] "sl:alice"
def bp7 := kt (fixtureId 0x238) "task:3" 240 (some ["status"]) [("status", .str "closed")] [fixtureId 0x236] [] "sl:bob"
def bp8 := kt (fixtureId 0x239) "task:4" 130 none [("status", .str "open"), ("est", .int 4)]
def bp9 := (kt (fixtureId 0x23a) "task:4" 250 (some ["status"]) [("status", .str "closed")] [fixtureId 0x239]).withhold
def bp10 := kt (fixtureId 0x23b) "task:5" 140 (some ["title"]) [("title", .str "b"), ("status", .str "open")]
def bp11 := kt (fixtureId 0x23c) "task:6" 150 none [("est", .int 6)]

def byPayloadKeysSpec : Json := o [("vertex", s "per-status"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("count", o [("count", .bool true), ("follow", s "keys"), ("by", s "status")]),
    ("count-heads", o [("count", .bool true), ("follow", s "prev"), ("by", s "status")]),
    ("est", o [("sum", s "est"), ("follow", s "keys"), ("by", s "status")]),
    ("records", o [("collect", .bool true), ("follow", s "keys"), ("by", s "status")]),
    ("tasks", o [("count", .bool true), ("follow", s "keys")])])]

-- `by` an envelope key, read from the heads. task:1 moves from project x to
-- project y by a later head that sets only `title`. task:2 forks into a head
-- that mentions x and one that mentions y, so it is in both groups and counts
-- once in each. task:3 forks the same way and a pure join that mentions y
-- settles it: the join is the head. task:4's head mentions no project. Under
-- `prev` the fork counts once per head.
def ba0 := kt (fixtureId 0x241) "task:1" 100 none [("status", .str "open"), ("est", .int 1)] [] ["project:x"]
def ba1 := kt (fixtureId 0x242) "task:1" 200 (some ["title"]) [("title", .str "a")] [fixtureId 0x241] ["project:y"]
def ba2 := kt (fixtureId 0x243) "task:2" 110 none [("status", .str "open"), ("est", .int 2)] [] ["project:x"]
def ba3 := kt (fixtureId 0x244) "task:2" 210 (some ["status"]) [("status", .str "closed")] [fixtureId 0x243] ["project:x"] "sl:alice"
def ba4 := kt (fixtureId 0x245) "task:2" 220 (some ["title"]) [("title", .str "b")] [fixtureId 0x243] ["project:y"] "sl:bob"
def ba5 := kt (fixtureId 0x246) "task:3" 120 none [("status", .str "open"), ("est", .int 3)] [] ["project:x"]
def ba6 := kt (fixtureId 0x247) "task:3" 230 (some ["status"]) [("status", .str "closed")] [fixtureId 0x246] ["project:x"] "sl:alice"
def ba7 := kt (fixtureId 0x248) "task:3" 240 (some ["title"]) [("title", .str "c")] [fixtureId 0x246] ["project:y"] "sl:bob"
def ba8 := kt (fixtureId 0x249) "task:3" 300 (some []) [] [fixtureId 0x247, fixtureId 0x248] ["project:y"]
def ba9 := kt (fixtureId 0x24a) "task:4" 130 none [("status", .str "open"), ("est", .int 4)] [] ["project:z"]
def ba10 := kt (fixtureId 0x24b) "task:4" 310 (some ["title"]) [("title", .str "d")] [fixtureId 0x24a]

def byAboutKeysSpec : Json := o [("vertex", s "per-project"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("count", o [("count", .bool true), ("follow", s "keys"), ("by", s "env.about:project")]),
    ("count-heads", o [("count", .bool true), ("follow", s "prev"), ("by", s "env.about:project")]),
    ("open", o [("count", .bool true), ("follow", s "keys"), ("by", s "env.about:project"),
      ("where", o [("status", o [("is", s "open")])])]),
    ("est", o [("sum", s "est"), ("follow", s "keys"), ("by", s "env.about:project")]),
    ("bob", o [("count", .bool true), ("follow", s "keys"), ("by", s "env.about:project"),
      ("where", o [("env.observer", o [("is", s "sl:bob")])])]),
    ("per-observer", o [("count", .bool true), ("follow", s "keys"), ("by", s "env.observer")]),
    ("records", o [("collect", .bool true), ("follow", s "keys"), ("by", s "env.about:project")]),
    ("tasks", o [("count", .bool true), ("follow", s "keys")])])]

-- Collect lists each selected thing's live records, newest first. task:1's
-- root is still live for every key but `status` and `title`; task:2 is
-- settled by a pure join, its two setters of `status` still live under it;
-- task:3's live setter of `status` is erased, and listed as erased.
def cl0 := kt (fixtureId 0x251) "task:1" 100 none [("status", .str "open"), ("title", .str "a"), ("est", .int 1)]
def cl1 := kt (fixtureId 0x252) "task:1" 200 (some ["status"]) [("status", .str "closed")] [fixtureId 0x251]
def cl2 := kt (fixtureId 0x253) "task:1" 300 (some ["title"]) [("title", .str "b")] [fixtureId 0x252]
def cl3 := kt (fixtureId 0x254) "task:2" 110 none [("status", .str "open"), ("title", .str "c")]
def cl4 := kt (fixtureId 0x255) "task:2" 210 (some ["status"]) [("status", .str "closed")] [fixtureId 0x254] [] "sl:alice"
def cl5 := kt (fixtureId 0x256) "task:2" 220 (some ["status"]) [("status", .str "done")] [fixtureId 0x254] [] "sl:bob"
def cl6 := kt (fixtureId 0x257) "task:2" 320 (some []) [] [fixtureId 0x255, fixtureId 0x256]
def cl7 := kt (fixtureId 0x258) "task:3" 120 none [("status", .str "open"), ("title", .str "e")]
def cl8 := (kt (fixtureId 0x259) "task:3" 230 (some ["status"]) [("status", .str "closed")] [fixtureId 0x258]).erase
def cl9 := kt (fixtureId 0x25a) "task:3" 330 (some ["title"]) [("title", .str "f")] [fixtureId 0x259]
def cl10 : Record := { said (fixtureId 0x25b) "sl:kyle" [] 400 with kind := "erasure", erases := [fixtureId 0x259] }

def collectKeysSpec : Json := o [("vertex", s "live"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("live", o [("collect", .bool true), ("follow", s "keys")]),
    ("newest", o [("collect", .bool true), ("follow", s "keys"), ("limit", .int 2)]),
    ("heads", o [("collect", .bool true), ("follow", s "prev")]),
    ("per-observer", o [("collect", .bool true), ("follow", s "keys"), ("by", s "env.observer")]),
    ("newest-per-observer", o [("collect", .bool true), ("follow", s "keys"), ("limit", .int 1),
      ("by", s "env.observer")]),
    ("closed", o [("collect", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "closed")])])]),
    ("open", o [("collect", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])])])]

-- A limit over things, each listed whole. Ordered by each thing's newest live
-- record: task:e (560), task:c (540), task:d (520), task:a (500), task:b (400).
-- task:a and task:b interleave in time (500, 400, 300, 200, 100), so a limit
-- over records falls inside task:a's three live records. task:c is settled by
-- a pure join above two setters of `status`; task:d's live setter of `status`
-- is erased. task:e is forked, its heads by alice and bob, so under `by`
-- `env.observer` it is in two groups, and the newest thing overall is in no
-- group of sl:kyle. By oldest live record the order would be a, d, c, b, e.
def lt0 := kt (fixtureId 0x291) "task:a" 100 none [("status", .str "open"), ("title", .str "a"), ("est", .int 1)]
def lt1 := kt (fixtureId 0x292) "task:a" 300 (some ["status"]) [("status", .str "closed")] [fixtureId 0x291]
def lt2 := kt (fixtureId 0x293) "task:a" 500 (some ["title"]) [("title", .str "b")] [fixtureId 0x292]
def lt3 := kt (fixtureId 0x294) "task:b" 200 none [("status", .str "open"), ("title", .str "c")]
def lt4 := kt (fixtureId 0x295) "task:b" 400 (some ["status"]) [("status", .str "closed")] [fixtureId 0x294]
def lt5 := kt (fixtureId 0x296) "task:c" 150 none [("status", .str "open"), ("title", .str "d")]
def lt6 := kt (fixtureId 0x297) "task:c" 250 (some ["status"]) [("status", .str "closed")] [fixtureId 0x296] [] "sl:alice"
def lt7 := kt (fixtureId 0x298) "task:c" 260 (some ["status"]) [("status", .str "done")] [fixtureId 0x296] [] "sl:bob"
def lt8 := kt (fixtureId 0x299) "task:c" 540 (some []) [] [fixtureId 0x297, fixtureId 0x298]
def lt9 := kt (fixtureId 0x29a) "task:d" 120 none [("status", .str "open"), ("title", .str "e")]
def lt10 := (kt (fixtureId 0x29b) "task:d" 520 (some ["status"]) [("status", .str "closed")] [fixtureId 0x29a]).erase
def lt11 : Record := { said (fixtureId 0x29c) "sl:kyle" [] 600 with kind := "erasure", erases := [fixtureId 0x29b] }
def lt12 := kt (fixtureId 0x29d) "task:e" 130 none [("status", .str "open"), ("title", .str "f")]
def lt13 := kt (fixtureId 0x29e) "task:e" 550 (some ["status"]) [("status", .str "closed")] [fixtureId 0x29d] [] "sl:alice"
def lt14 := kt (fixtureId 0x29f) "task:e" 560 (some ["title"]) [("title", .str "g")] [fixtureId 0x29d] [] "sl:bob"

def collectLimitSpec : Json :=
  let lim (k : Int) (more : List (String × Json) := []) : Json :=
    o ([("collect", .bool true), ("follow", s "keys"), ("limit", .int k)] ++ more)
  o [("vertex", s "newest-things"),
    ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
    ("fold", o [
      ("all", o [("collect", .bool true), ("follow", s "keys")]),
      ("one", lim 1), ("two", lim 2), ("three", lim 3), ("four", lim 4), ("many", lim 9),
      ("closed-one", lim 1 [("where", o [("status", o [("is", s "closed")])])]),
      ("per-observer-one", lim 1 [("by", s "env.observer")]),
      ("per-observer-two", lim 2 [("by", s "env.observer")]),
      ("per-status-one", lim 1 [("by", s "status")])])]

-- Payload tests on every fold type, over the records of `spec/run/40`: tests
-- asked of the thing, over each field's live setters.
def keyedWhereSpec : Json := o [("vertex", s "open"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("count", o [("count", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("sum", o [("sum", s "est"), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("min", o [("min", s "est"), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("max", o [("max", s "est"), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("collect", o [("collect", .bool true), ("follow", s "keys"), ("where", o [("status", o [("is", s "open")])])]),
    ("est-absent", o [("count", .bool true), ("follow", s "keys"), ("where", o [("est", o [("absent", .bool true)])])]),
    ("not-open-per-status", o [("count", .bool true), ("follow", s "keys"), ("by", s "status"),
      ("where", o [("status", o [("not", s "open")])])]),
    ("open-est-1", o [("sum", s "est"), ("follow", s "keys"),
      ("where", o [("status", o [("is", s "open")]), ("est", o [("is", .int 1)])])])])]

-- Two live setters of a field, exactly one of them not held. task:1's second
-- setter of `est` is withheld, task:2's erased by an eraser `from` does not
-- take: each thing is unread to sum, min and max, though one setter is held.
-- task:3 and task:4 do the same for `status`, which `by` reads: each is unread
-- at the top, in no group. task:5's two setters of `est` are held, a control.
def uh0 := kt (fixtureId 0x271) "task:1" 100 none [("est", .int 1), ("status", .str "open")]
def uh1 := kt (fixtureId 0x272) "task:1" 200 (some ["est"]) [("est", .int 4)] [fixtureId 0x271] [] "sl:alice"
def uh2 := (kt (fixtureId 0x273) "task:1" 210 (some ["est"]) [("est", .int 6)] [fixtureId 0x271] [] "sl:bob").withhold
def uh3 := kt (fixtureId 0x274) "task:2" 110 none [("est", .int 2), ("status", .str "open")]
def uh4 := kt (fixtureId 0x275) "task:2" 220 (some ["est"]) [("est", .int 5)] [fixtureId 0x274] [] "sl:alice"
def uh5 := (kt (fixtureId 0x276) "task:2" 230 (some ["est"]) [("est", .int 7)] [fixtureId 0x274] [] "sl:bob").erase
def uh6 : Record := { said (fixtureId 0x277) "sl:kyle" [] 300 with kind := "erasure", erases := [fixtureId 0x276] }
def uh7 := kt (fixtureId 0x278) "task:3" 120 none [("est", .int 3), ("status", .str "open")]
def uh8 := kt (fixtureId 0x279) "task:3" 240 (some ["status"]) [("status", .str "closed")] [fixtureId 0x278] [] "sl:alice"
def uh9 := (kt (fixtureId 0x27a) "task:3" 250 (some ["status"]) [("status", .str "done")] [fixtureId 0x278] [] "sl:bob").withhold
def uh10 := kt (fixtureId 0x27b) "task:4" 130 none [("est", .int 8), ("status", .str "open")]
def uh11 := kt (fixtureId 0x27c) "task:4" 260 (some ["status"]) [("status", .str "closed")] [fixtureId 0x27b] [] "sl:alice"
def uh12 := (kt (fixtureId 0x27d) "task:4" 270 (some ["status"]) [("status", .str "done")] [fixtureId 0x27b] [] "sl:bob").erase
def uh13 : Record := { said (fixtureId 0x27e) "sl:kyle" [] 310 with kind := "erasure", erases := [fixtureId 0x27d] }
def uh14 := kt (fixtureId 0x27f) "task:5" 140 none [("est", .int 9), ("status", .str "open")]
def uh15 := kt (fixtureId 0x280) "task:5" 280 (some ["est"]) [("est", .int 10)] [fixtureId 0x27f] [] "sl:alice"
def uh16 := kt (fixtureId 0x281) "task:5" 290 (some ["est"]) [("est", .int 11)] [fixtureId 0x27f] [] "sl:bob"

def unheldSetterSpec : Json := o [("vertex", s "one-unheld"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("sum", o [("sum", s "est"), ("follow", s "keys")]),
    ("min", o [("min", s "est"), ("follow", s "keys")]),
    ("max", o [("max", s "est"), ("follow", s "keys")]),
    ("sum-heads", o [("sum", s "est"), ("follow", s "prev")]),
    ("per-status", o [("count", .bool true), ("follow", s "keys"), ("by", s "status")]),
    ("est-per-status", o [("max", s "est"), ("follow", s "keys"), ("by", s "status")]),
    ("per-status-heads", o [("count", .bool true), ("follow", s "prev"), ("by", s "status")])])]

-- Every record read whole and no thing forked: each slot that follows `keys`
-- gives what the same slot following `prev` gives, grouped or not.
def wr0 := kt (fixtureId 0x261) "task:1" 100 none [("status", .str "open"), ("est", .int 3)] [] ["project:x"]
def wr1 := kt (fixtureId 0x262) "task:1" 200 none [("status", .str "closed"), ("est", .int 4)] [fixtureId 0x261] ["project:y"]
def wr2 := kt (fixtureId 0x263) "task:2" 110 none [("status", .str "open"), ("est", .int 5)] [] ["project:x"]
def wr3 := (kt (fixtureId 0x264) "task:3" 120 none [("status", .str "open"), ("est", .int 6)] [] ["project:y"]).withhold
def wr4 := kt (fixtureId 0x265) "task:4" 130 none [("status", .str "open")] []

def wholeReadSpec : Json :=
  let pair (n : String) (slot : List (String × Json)) : List (String × Json) :=
    [(n, o (slot ++ [("follow", s "keys")])), (n ++ "-heads", o (slot ++ [("follow", s "prev")]))]
  o [("vertex", s "whole-read"),
    ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
    ("fold", o (pair "count" [("count", .bool true)] ++
      pair "open" [("count", .bool true), ("where", o [("status", o [("is", s "open")])])] ++
      pair "sum" [("sum", s "est")] ++ pair "min" [("min", s "est")] ++ pair "max" [("max", s "est")] ++
      pair "collect" [("collect", .bool true), ("limit", .int 3)] ++
      pair "per-project" [("count", .bool true), ("by", s "env.about:project")] ++
      pair "est-per-status" [("sum", s "est"), ("by", s "status")] ++
      pair "collect-per-project" [("collect", .bool true), ("by", s "env.about:project")]))]

structure RunCase where
  name : String
  laws : List String
  spec : Json
  records : List Record

def runCases : List RunCase := [
  ⟨"phase2-exact-output", ["spec.set", "sum.perm", "numeric.safe-whole"],
    o [("vertex", s "exact-output"), ("from", a [o [("store", s ".")]]),
      ("fold", o [("n", o [("sum", s "n")]), ("field", o [("sum", s "est")])])],
    [{ task (fixtureId 0xf1) "task:1" 100 [("est", .int 9007199254740991)] with n := 9007199254740991 },
      { task (fixtureId 0xf2) "task:2" 110 [("est", .int 2)] with n := 2 }]⟩,
  ⟨"01-tasks", ["spec.set", "spec.select"], taskSpec, [t1, t2, t3, n1]⟩,
  ⟨"02-withheld-payload", ["spec.set", "view.refines", "result.unread"], taskSpec, [t1, t2, t3, t4, n1]⟩,
  ⟨"03-withheld-successor", ["spec.set", "view.no-resurrection", "result.unread"], taskSpec,
    [t1, t2.withhold, t3, n1]⟩,
  ⟨"04-kind-change", ["spec.set", "spec.after-supersession"], taskSpec,
    [t1, { task (fixtureId 0x46) "task:1" 300 [] (prev := [fixtureId 0x41]) with kind := "task-done" }, t3]⟩,
  ⟨"05-about-makes-no-head", ["spec.set", "thing.local"], taskSpec,
    [{ t3 with about := ["task:2", "task:9"] }, task (fixtureId 0x47) "task:2" 400 [("status", .str "closed")] (prev := [fixtureId 0x43])]⟩,
  ⟨"06-times", ["spec.set", "where.time", "where.complements", "where.total"], timeSpec,
    [e1, e2, e3, e4, e5, e6, e7, e8]⟩,
  ⟨"07-numeric-bounds", ["spec.set", "where.complements", "where.typed", "result.unread"], boundSpec,
    [t1, t2, t3, t4, task (fixtureId 0x48) "task:4" 180 [("est", .int 4)], task (fixtureId 0x49) "task:5" 190 [("est", .int 8)],
      task (fixtureId 0x4a) "task:6" 195 [("est", .str "5")]]⟩,
  ⟨"08-envelope-fields", ["spec.set", "where.fields", "where.time", "where.total",
      "spec.after-supersession"], envSpec, [a1, a2, a3, a4, a5, a6]⟩,
  ⟨"09-from-of-root", ["spec.set", "spec.from-thing", "where.fields"],
    rootSpec ("of", s (idText (fixtureId 0x71))), [x1, x2, x3, x4]⟩,
  ⟨"14-from-about-mentions", ["spec.set", "spec.from-thing", "where.fields"],
    rootSpec ("about", s (idText (fixtureId 0x71))), [x1, x2, x3, x4]⟩,
  -- a record about a thing, newer than the thing's head, is not a head of it
  ⟨"15-about-makes-no-head", ["spec.set", "thing.local"], taskSpec,
    [t1, { task (fixtureId 0x4b) "task:1" 500 [("status", .str "open")] with of := none, about := ["task:1"] }]⟩,
  ⟨"16-follow-current", ["spec.set", "spec.follow", "result.unread"], followSpec,
    [c1, c2, c3, c4, c5, c6, c7]⟩,
  -- both heads of a forked thing count
  ⟨"17-follow-fork", ["spec.set", "spec.follow"], followHeadsSpec, [k0, k1, k2]⟩,
  -- count counts an erased head's held envelope; sum, min and max count it unread
  ⟨"18-follow-erased-head", ["spec.set", "spec.follow", "result.unread"], followHeadsSpec, [g0, g1, g2]⟩,
  ⟨"19-follow-payload-test", ["spec.set", "spec.follow", "result.unread"], followTestSpec, [p1, p2]⟩,
  ⟨"20-follow-envelope-after-supersession", ["spec.set", "spec.follow", "spec.after-supersession"],
    followEnvSpec, [v1, v2]⟩,
  -- `by` on the envelope and on payload fields, with a payload `where`; a
  -- withheld record under each key form; a group only where something was
  -- selected; text "1" and the number 1 share a group
  ⟨"21-by-keys", ["spec.set", "by.set", "by.accounting", "by.partition", "result.unread"], byKeysSpec,
    [b1, b2, b3, b4, b5, b6]⟩,
  -- a record mentioning two projects counts in each, once; the withheld one
  -- still has its envelope's key; one with no project is ungrouped
  ⟨"22-by-about", ["spec.set", "by.set", "by.accounting", "result.unread"], byAboutSpec,
    [b1, b2, b3, b4, b5, b6]⟩,
  -- admission refuses a `prev` across things before any group is formed
  ⟨"23-by-after-admission", ["spec.set", "by.after-admission", "by.partition"], byAdmitSpec, [y1, y2]⟩,
  -- the whole view's heads come before any group: a move, a fork, an erased
  -- head and a withheld one
  ⟨"24-by-follow", ["spec.set", "spec.follow", "by.set", "by.after-admission", "by.accounting",
      "by.partition", "result.unread"], byFollowSpec, [m0, m1, m2, m3, m4, m5, m6, m7]⟩,
  -- limit is per group
  ⟨"25-by-collect", ["spec.set", "by.set", "by.accounting", "result.unread"], byCollectSpec,
    [b1, b2, b3, b4, b5, b6]⟩,
  -- group keys in UTF-16 order, as every implementation must write them
  ⟨"26-by-key-order", ["spec.set", "by.set"], byOrderSpec, [ko0, ko1, ko2, ko3, ko4]⟩,
  -- raw number spellings become decimal keys or none; booleans share their text's group
  ⟨"27-by-number-keys", ["spec.set", "by.accounting", "by.partition"], byNumbersSpec,
    [nk0, nk1, nk2, nk3, nk4, nk5, nk6, nk7, nk8, nk9, nk10, nk11, nk12]⟩,
  -- prefixes match with their colon, a repeated mention counts once, and a
  -- bare `about:x` is a payload field
  ⟨"28-by-mentions", ["spec.set", "by.accounting"], byMentionsSpec, [pm, pn]⟩,
  -- collect's limit is per group, and the top-level unread does not depend on it
  ⟨"29-by-collect-limit", ["spec.set", "by.accounting", "result.unread"], byLimitSpec, limitRecords⟩,
  -- collect following `prev`, grouped over the whole view's heads
  ⟨"30-by-follow-collect", ["spec.set", "spec.follow", "by.set", "by.after-admission", "by.accounting",
      "result.unread"], byFollowCollectSpec, [m0, m1, m2, m3, m4, m5, m6, m7]⟩,
  -- latest-by following `keys`: the note's example
  ⟨"31-keys-example", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [kw, kb, kw2]⟩,
  -- only whole-read emits, chained and forked: the keyed state is the heads twice
  ⟨"32-keys-whole-read", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [wa0, wa1, wb0, wb1, wb2]⟩,
  -- the head sets one key, and an older record is still live for another
  ⟨"33-keys-chain", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [kc0, kc1, kc2]⟩,
  -- two live setters of one key
  ⟨"34-keys-concurrent", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [kf0, kf1, kf2]⟩,
  -- a pure join settles two heads and sets nothing
  ⟨"35-keys-pure-join", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [kf0, kf1, kf2, kf3]⟩,
  -- a whole-read emit above keyed emits replaces everything under it
  ⟨"36-keys-whole-replaces", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [kr0, kr1, kr2, kr3]⟩,
  -- an erased and a withheld live setter: the key is unknown, and `shown` says so
  ⟨"37-keys-unheld-setters", ["spec.set", "keyed.live", "keyed.fold", "keyed.select", "result.unread"],
    keyedUnheldSpec, [ku0, ku1, ku2, ku3]⟩,
  -- a payload key the record does not declare is not read
  ⟨"38-keys-undeclared", ["spec.set", "keyed.fold", "keyed.select"], keyedUndeclaredSpec, [kn0, kn1]⟩,
  -- `from` and the envelope tests choose heads, each alone
  ⟨"39-keys-select-heads", ["spec.set", "keyed.select", "spec.after-supersession"], keyedFromSpec,
    [ks0, ks1, ks2, ks3, ks4, ks5]⟩,
  -- payload tests over live setters: agreeing, differing, unheld, and no setter
  ⟨"40-keys-payload-tests", ["spec.set", "keyed.select", "result.unread"], keyedTestSpec,
    [kp0, kp1, kp2, kp3, kp4, kp5, kp6, kp7, kp8, kp9, kp10, kp11, kp12]⟩,
  -- a refused record, and one whose `prev` target is not held, are above nothing
  ⟨"41-keys-admitted-only", ["spec.set", "keyed.live", "keyed.fold"], keyedSpec, [ka0, ka1, ka2]⟩,
  -- count the open tasks: the head only retitled one, and over heads it is not open
  ⟨"42-keys-count-open", ["spec.set", "keyed.slots", "keyed.select", "spec.follow"], countOpenSpec,
    [co0, co1, co2, co3, co4, co5, co6]⟩,
  -- a fork across whole-read emits counts once following `keys`, twice following `prev`
  ⟨"43-keys-fork-once", ["spec.set", "keyed.slots", "spec.follow"], forkSpec, [fk0, fk1, fk2, fk3]⟩,
  -- sum, min and max read a field from its one live setter
  ⟨"44-keys-numbers", ["spec.set", "keyed.slots", "result.unread"], numbersSpec,
    [nu0, nu1, nu2, nu3, nu4, nu5, nu6, nu7, nu8, nu9, nu10, nu11, nu12, nu13, nu14]⟩,
  -- `by` a payload field: a thing joins the group of each live setter's key
  ⟨"45-keys-by-payload", ["spec.set", "keyed.by", "result.unread"], byPayloadKeysSpec,
    [bp0, bp1, bp2, bp3, bp4, bp5, bp6, bp7, bp8, bp9, bp10, bp11]⟩,
  -- `by` an envelope key: a thing joins the group of each key of each chosen head
  ⟨"46-keys-by-about", ["spec.set", "keyed.by", "keyed.select"], byAboutKeysSpec,
    [ba0, ba1, ba2, ba3, ba4, ba5, ba6, ba7, ba8, ba9, ba10]⟩,
  -- collect lists the selected things' live records, a pure join and an erased setter among them
  ⟨"47-keys-collect", ["spec.set", "keyed.slots", "keyed.by", "result.unread"], collectKeysSpec,
    [cl0, cl1, cl2, cl3, cl4, cl5, cl6, cl7, cl8, cl9, cl10]⟩,
  -- payload tests asked of the thing, on every fold type
  ⟨"48-keys-where", ["spec.set", "keyed.slots", "keyed.select", "keyed.by", "result.unread"], keyedWhereSpec,
    [kp0, kp1, kp2, kp3, kp4, kp5, kp6, kp7, kp8, kp9, kp10, kp11, kp12]⟩,
  -- every record read whole, no fork: `keys` gives what `prev` gives
  ⟨"49-keys-whole-read", ["spec.set", "keyed.whole-read", "spec.follow"], wholeReadSpec,
    [wr0, wr1, wr2, wr3, wr4]⟩,
  -- two live setters, one of them withheld or erased: unread, not read from the held one
  ⟨"50-keys-one-setter-unheld", ["spec.set", "keyed.slots", "keyed.by", "result.unread"], unheldSetterSpec,
    [uh0, uh1, uh2, uh3, uh4, uh5, uh6, uh7, uh8, uh9, uh10, uh11, uh12, uh13, uh14, uh15, uh16]⟩,
  -- collect's limit counts things, each listed whole, the newest first
  ⟨"51-keys-collect-limit", ["spec.set", "keyed.slots", "keyed.by", "result.unread"], collectLimitSpec,
    [lt0, lt1, lt2, lt3, lt4, lt5, lt6, lt7, lt8, lt9, lt10, lt11, lt12, lt13, lt14]⟩,
  ⟨"gap04-unread-multi-thing", ["result.unread"],
    o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o [
      ("by", o [("latest-by", s "of"), ("follow", s "prev"),
        ("where", o [("status", o [("is", s "open")])])]),
      ("collect", o [("collect", .bool true), ("follow", s "prev"),
        ("where", o [("status", o [("is", s "open")])])])])],
    [{ (task (fixtureId 0xa1) "task:a" 100 []).withhold with about := ["task:a", "task:b"] }]⟩,
  ⟨"gap05-unread-before-limit", ["result.unread"],
    o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o [
      ("plain", o [("collect", .bool true), ("limit", .int 1), ("where", o [("status", o [("is", s "open")])])]),
      ("follow", o [("collect", .bool true), ("follow", s "prev"), ("limit", .int 1),
        ("where", o [("status", o [("is", s "open")])])]),
      ("all", o [("collect", .bool true), ("where", o [("status", o [("is", s "open")])])])])],
    [task (fixtureId 0xb1) "task:a" 300 [("status", .str "open")],
      (task (fixtureId 0xb2) "task:b" 200 []).withhold, (task (fixtureId 0xb3) "task:c" 100 []).withhold]⟩,
  ⟨"gap06-direct-admission", ["spec.set"], fromOf (o [("store", s ".")]),
    [task (fixtureId 0xc1) "task:a" 100 [], task (fixtureId 0xc2) "task:b" 200 [] [fixtureId 0xc1]]⟩,
  ⟨"gap07-store-team", ["spec.set"], fromOf (o [("store", s "team")]), [t1]⟩,
  ⟨"gap07-store-other", ["spec.set"], fromOf (o [("store", s "other")]), [t1]⟩,
  ⟨"gap12-no-unknown", ["where.set", "result.unread"],
    o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o [
      ("payload-first", o [("count", .bool true), ("where", o [("status", o [("is", s "open")]),
        ("env.kind", o [("is", s "note")])])]),
      ("envelope-first", o [("count", .bool true), ("where", o [("env.kind", o [("is", s "note")]),
        ("status", o [("is", s "open")])])]),
      ("unknown", o [("count", .bool true), ("where", o [("status", o [("is", s "open")]),
        ("env.kind", o [("is", s "task")])])])])], [t4]⟩,
  ⟨"gap13-missing-and-withheld", ["where.total", "result.unread"],
    o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o (
      [("is", s "open"), ("not", s "open"), ("present", .bool true), ("absent", .bool true),
       ("at-least", .int 1), ("below", .int 1)].map fun test =>
        (test.1, o [("count", .bool true), ("where", o [("missing", o [test])])])))],
    [task (fixtureId 0xd1) "task:a" 100 [], (task (fixtureId 0xd2) "task:b" 110 []).withhold]⟩,
  ⟨"gap14-field-types", ["result.unread", "numeric.safe-whole"],
    o [("vertex", s "x"), ("from", a [o [("store", s ".")]]), ("fold", o [
      ("sum", o [("sum", s "est")]), ("min", o [("min", s "est")]),
      ("max", o [("max", s "est")])])],
    [task (fixtureId 0xe1) "task:1" 100 [("est", .int 2)],
      task (fixtureId 0xe2) "task:2" 110 [("est", .int (-3))],
      task (fixtureId 0xe3) "task:3" 120 [("est", .number (Number.ofScientific false 15 (-1)))],
      task (fixtureId 0xe4) "task:4" 130 [("est", .int 2)],
      task (fixtureId 0xe5) "task:5" 140 [("est", .str "1.5")],
      task (fixtureId 0xe6) "task:6" 150 [("est", .bool true)], task (fixtureId 0xe7) "task:7" 160 [],
      (task (fixtureId 0xe8) "task:8" 170 [("est", .int 7)]).withhold,
      task (fixtureId 0xe9) "task:9" 180 [("est", .number (Number.ofScientific false 9007199254740992 0))]]⟩,
  ⟨"11-hidden-numbers", ["spec.set", "result.unread"], hiddenNumbersSpec,
    [(task (fixtureId 0x76) "task:6" 100 []).withhold, (task (fixtureId 0x77) "task:7" 200 []).erase,
      { said (fixtureId 0x7a) "sl:kyle" [] 300 with kind := "erasure", erases := [fixtureId 0x77] }]⟩,
  ⟨"12-readable-extrema", ["spec.set", "result.unread"], readableExtremaSpec,
    [task (fixtureId 0x78) "task:8" 100 [("est", .int 3)], task (fixtureId 0x79) "task:9" 200 [("est", .int 5)]]⟩,
  ⟨"13-source-union", ["spec.set", "spec.select"], sourceUnionSpec,
    [task (fixtureId 0x78) "task:8" 100 []]⟩
]

def jShownIds (hs : List (RecordId × Shown)) : J :=
  .arr (hs.map fun (i, sh) => .obj [("id", .str (idText i)), ("shown", jShown sh)])

/-- What one group holds: what the slot holds without `by`. -/
def jGroup : GroupValue → J
  | .nat r => .obj [("value", .num r.value), ("unread", .num r.unread)]
  | .int r => .obj [("value", .int r.value), ("unread", .num r.unread)]
  | .opt r => .obj [("value", match r.value with
      | some v => .int v
      | none => .null), ("unread", .num r.unread)]
  | .heads hs u => .obj [("heads", jShownIds hs), ("unread", .num u)]

/-- A thing's keyed state: each declared key's live setters, the live
whole-read emits and the heads. -/
def jKeyed (k : KeyedView) : J :=
  .obj [("keys", .obj (k.keys.map fun (n, hs) => (n, jShownIds hs))), ("whole", jShownIds k.whole),
    ("heads", jShownIds k.heads)]

def jSlot : SlotValue → J
  | .nat r => jGroup (.nat r)
  | .int r => jGroup (.int r)
  | .opt r => jGroup (.opt r)
  | .heads hs u => jGroup (.heads hs u)
  | .things ts u => .obj [("things", .obj (ts.map fun (t, hs) => (t, jShownIds hs))),
      ("unread", .num u)]
  | .keyed ts u => .obj [("things", .obj (ts.map fun (t, k) => (t, jKeyed k))), ("unread", .num u)]
  | .groups gs u m => .obj [("groups", .obj (gs.map fun (g, v) => (g, jGroup v))),
      ("unread", .num u), ("ungrouped", .num m)]

/-- Payload fields a run vector writes as raw number text: record, field,
spelling. The record holds what a reader reads the spelling as. -/
def rawSpellings : String → List (RecordId × String × String)
  | "27-by-number-keys" => numberRaw
  | _ => []

/-- A record as the input writes it, with any raw spellings in its fields. -/
def jRecordRaw (raw : List (RecordId × String × String)) (r : Record) : J :=
  match jRecord r with
  | .obj kvs => .obj (kvs.map fun (k, v) =>
      if k != "fields" then (k, v) else match v with
        | .obj fs => (k, .obj (fs.map fun (f, x) =>
            match raw.find? fun (i, g, _) => i == r.id && g == f with
            | some (_, _, t) => (f, .raw t)
            | none => (f, x)))
        | x => (k, x))
  | j => j

def rotations (xs : List Record) : List (List Record) :=
  (List.range xs.length).map fun i => xs.drop i ++ xs.take i

def runVector (c : RunCase) : Except String J := do
  let sp ← match Spec.ofJson c.spec with
    | .ok sp => pure sp
    | .error r => throw s!"spec/run/{c.name}: refused {r.code} at {r.path}"
  -- Settling keeps an erased form only where a held record erases it, so a
  -- reader that settles first reads these records as they are written.
  if c.records.any fun r => r.shown == .erased && !c.records.any (·.erases.contains r.id) then
    throw s!"spec/run/{c.name}: an erased record has no eraser"
  let orders := rotations c.records
  let results := orders.map fun xs => J.obj ((sp.run xs).map fun (n, v) => (n, jSlot v))
  let first := results.headD .null
  if !(results.all (·.render 0 == first.render 0)) then
    throw s!"spec/run/{c.name}: arrival orders disagree"
  return vector c.name "spec/run" c.laws
    (.obj [("spec", Records.ofJson c.spec), ("orders", .arr (orders.map fun o => .arr (o.map (jRecordRaw (rawSpellings c.name)))))])
    first

def vectors : Except String (List (String × String × J)) := do
  let parsed ← parseCases.mapM fun (n, j) => do pure ("spec/parse", n, ← parseVector n j)
  let runs ← runCases.mapM fun c => do pure ("spec/run", c.name, ← runVector c)
  return parsed ++ runs

end Gen.SpecVectors

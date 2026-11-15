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

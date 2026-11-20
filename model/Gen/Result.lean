import Loops
import Gen.Json
import Gen.Encode
import Gen.Sha2
import Gen.Records
import Gen.Spec

/-!
# Result vectors: the canonical result (D7)

`vectors/result/` pins the result codec. Each vector's input is a spec and
the records a reader holds, in every arrival order, as `spec/run` writes them;
its `expect` is the vertex's whole result as canonical text (`resultText`) and
that text's hash with SHA-256 (`resultHash`).

Some inputs are `spec/run`'s own, chosen so that every shape a result holds is
here: fixed-member objects, maps by thing, group key and payload key, id lists,
integers and `null`, keyed state, groups with `ungrouped` and `unread`. The
others are new: integers past 2^53 either way, zero, `null`, empty lists and
maps, and holdings that differ only in what is shown.

Gen checks, before it writes a vector, that every arrival order gives the same
text; that past its first 16 bytes the preimage reads as no JSON, so it is no
record's commitment preimage (`result.no-commit`); and that the cases of one
`same` group give the same text while their
readings differ: a reader that holds a payload, one that has it withheld and
one that had it erased hold the same result.
-/

namespace Gen.ResultVectors

open Loops Gen Gen.SpecVectors

/-- What a result vector expects: the canonical text and its hash. -/
def resultExpect (r : List (String × SlotValue)) : J :=
  .obj [("canonical", .str (resultText r)), ("hash", .str (resultHash sha256 r))]

/-- The reading a reader prints, `shown` and all: what `spec/run` expects. -/
def reading (r : List (String × SlotValue)) : String := (J.obj (r.map fun (n, v) => (n, jSlot v))).render 0

structure ResultCase where
  name : String
  laws : List String
  spec : Json
  records : List Record
  /-- Cases of one group hold one result with `shown` set aside. -/
  same : Option String := none

/-! ## Inputs of `spec/run` -/

/-- The `spec/run` inputs taken whole: between them, every shape. -/
def fromRun : List (String × List String) := [
  ("phase2-exact-output", ["result.canonical", "result.exact"]),
  ("01-tasks", ["result.canonical"]),
  ("02-withheld-payload", ["result.canonical", "result.shown"]),
  ("11-hidden-numbers", ["result.canonical"]),
  ("12-readable-extrema", ["result.canonical"]),
  ("gap14-field-types", ["result.canonical", "result.exact"]),
  ("17-follow-fork", ["result.canonical"]),
  ("21-by-keys", ["result.canonical"]),
  ("25-by-collect", ["result.canonical"]),
  ("26-by-key-order", ["result.canonical"]),
  ("27-by-number-keys", ["result.canonical", "result.exact"]),
  ("31-keys-example", ["result.canonical", "result.shown"]),
  ("37-keys-unheld-setters", ["result.canonical", "result.shown"]),
  ("45-keys-by-payload", ["result.canonical"]),
  ("47-keys-collect", ["result.canonical"]),
  ("51-keys-collect-limit", ["result.canonical"])
]

def runInputs : Except String (List ResultCase) :=
  fromRun.mapM fun (n, laws) => match runCases.find? (·.name == n) with
    | some c => pure { name := n, laws, spec := c.spec, records := c.records }
    | none => throw s!"result/{n}: no spec/run case of that name"

/-! ## New inputs -/

def numbersSpec : Json := o [("vertex", s "numbers"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [("count", o [("count", .bool true)]), ("sum", o [("sum", s "n")]),
    ("min", o [("min", s "n")]), ("max", o [("max", s "n")])])]

-- Two of the least values the writer takes, and one more: the sum is past
-- -2^53, and every integer written is negative.
def neg1 : Record := { task (fixtureId 0x1c1) "task:n1" 100 [] with n := -9007199254740991 }
def neg2 : Record := { task (fixtureId 0x1c2) "task:n2" 110 [] with n := -9007199254740991 }
def neg3 : Record := { task (fixtureId 0x1c3) "task:n3" 120 [] with n := -2 }

-- A sum that comes back to zero: `0`, never `-0`.
def zero1 : Record := { task (fixtureId 0x1c4) "task:z1" 100 [] with n := 5 }
def zero2 : Record := { task (fixtureId 0x1c5) "task:z2" 110 [] with n := -5 }

-- Nothing is selected: zero, `null` for min and max, an empty list and an
-- empty map. The one record is a note, which `from` does not take.
def nothingSpec : Json := o [("vertex", s "nothing"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [("count", o [("count", .bool true)]), ("sum", o [("sum", s "n")]),
    ("min", o [("min", s "n")]), ("max", o [("max", s "n")]),
    ("every", o [("collect", .bool true)]), ("current", o [("latest-by", s "of"), ("follow", s "prev")]),
    ("state", o [("latest-by", s "of"), ("follow", s "keys")]),
    ("per-status", o [("count", .bool true), ("by", s "status")])])]

def nothing1 : Record := { task (fixtureId 0x1c6) "task:x" 100 [("status", .str "open")] with kind := "note", n := 4 }

-- A grouped slot with something in each place: two groups, two records with
-- no key (ungrouped), and one whose key is on a payload the reader does not
-- hold, so whose `where` is unknown too (unread). `ungrouped` and `unread`
-- differ, so a codec that swaps them is seen.
def groupedSpec : Json := o [("vertex", s "grouped"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("per-status", o [("count", .bool true), ("by", s "status")]),
    ("est-per-status", o [("sum", s "est"), ("by", s "status")]),
    ("small-per-status", o [("collect", .bool true), ("by", s "status"),
      ("where", o [("est", o [("below", .int 5)])])])])]

def gr1 := task (fixtureId 0x1c7) "task:g1" 100 [("status", .str "open"), ("est", .int 3)]
def gr2 := task (fixtureId 0x1c8) "task:g2" 110 [("status", .str "closed"), ("est", .int 9)]
def gr3 := task (fixtureId 0x1c9) "task:g3" 120 [("status", .str "open"), ("est", .int 2)]
def gr4 := task (fixtureId 0x1ca) "task:g4" 130 [("est", .int 1)]
def gr5 := (task (fixtureId 0x1cb) "task:g5" 140 [("status", .str "open"), ("est", .int 4)]).withhold
def gr6 := task (fixtureId 0x1cc) "task:g6" 150 [("est", .int 7)]

-- One holding three ways. Every slot lists ids and reads no payload, so a
-- reader that holds `sh2`'s payload, one that has it withheld and one that
-- had it erased hold the same result; the readings differ in `shown`.
def shownSpec : Json := o [("vertex", s "listing"),
  ("from", a [o [("store", s "."), ("kind", a [s "task"])]]),
  ("fold", o [
    ("current", o [("latest-by", s "of"), ("follow", s "prev")]),
    ("every", o [("collect", .bool true)]),
    ("state", o [("latest-by", s "of"), ("follow", s "keys")]),
    ("per-kind", o [("collect", .bool true), ("by", s "env.kind")]),
    ("observations", o [("count", .bool true)])])]

def sh1 := task (fixtureId 0x1d1) "task:1" 100 [("status", .str "open")]
def sh2 := task (fixtureId 0x1d2) "task:1" 200 [("status", .str "closed")] (prev := [fixtureId 0x1d1])
def sh3 := task (fixtureId 0x1d3) "task:2" 150 [("status", .str "open")]
-- The eraser is no task, so `from` takes it in no slot.
def shEraser : Record := { said (fixtureId 0x1d4) "sl:kyle" [] 300 with kind := "erasure", erases := [fixtureId 0x1d2] }

def newCases : List ResultCase := [
  ⟨"r01-negative-past-2-53", ["result.canonical", "result.exact"], numbersSpec, [neg1, neg2, neg3], none⟩,
  ⟨"r02-zero-sum", ["result.canonical", "result.exact"], numbersSpec, [zero1, zero2], none⟩,
  ⟨"r03-nothing-read", ["result.canonical", "result.exact"], nothingSpec, [nothing1], none⟩,
  ⟨"r04-grouped-ungrouped-unread", ["result.canonical"], groupedSpec, [gr1, gr2, gr3, gr4, gr5, gr6], none⟩,
  ⟨"r05-shown-held", ["result.canonical", "result.shown"], shownSpec, [sh1, sh2, sh3], some "shown"⟩,
  ⟨"r06-shown-withheld", ["result.canonical", "result.shown"], shownSpec, [sh1, sh2.withhold, sh3], some "shown"⟩,
  ⟨"r07-shown-erased", ["result.canonical", "result.shown"], shownSpec, [sh1, sh2.erase, sh3, shEraser],
    some "shown"⟩
]

/-! ## Writing -/

/-- A case's result, the same in every arrival order. -/
def resultOf (c : ResultCase) : Except String (List (String × SlotValue)) := do
  let sp ← match Spec.ofJson c.spec with
    | .ok sp => pure sp
    | .error r => throw s!"result/{c.name}: refused {r.code} at {r.path}"
  if c.records.any fun r => r.shown == .erased && !c.records.any (·.erases.contains r.id) then
    throw s!"result/{c.name}: an erased record has no eraser"
  let results := (rotations c.records).map sp.run
  let first := results.headD []
  if !(results.all fun r => resultText r == resultText first) then
    throw s!"result/{c.name}: arrival orders disagree"
  -- A commitment hashes 16 salt bytes and a payload's JCS: past 16 bytes, the
  -- preimage must not read as JSON (`resultPreimage_ne_commit` says why).
  let past16 := String.ofList ((resultLabel ++ resultText first).toList.drop 16)
  if (resultLabel ++ resultText first).utf8ByteSize - past16.utf8ByteSize != 16 then
    throw s!"result/{c.name}: the label's first 16 characters are not 16 bytes"
  if (Read.parseJson past16).isOk then
    throw s!"result/{c.name}: the preimage is a salt and a JSON value"
  return first

def resultVector (c : ResultCase) (r : List (String × SlotValue)) : J :=
  vector c.name "result" (c.laws ++ ["result.no-commit"])
    (.obj [("spec", Records.ofJson c.spec),
      ("orders", .arr ((rotations c.records).map fun o => .arr (o.map (jRecordRaw (rawSpellings c.name)))))])
    (resultExpect r)

/-- The cases of a `same` group give one text, and not one reading. -/
def checkSame (cs : List (ResultCase × List (String × SlotValue))) : Except String Unit := do
  for g in (cs.filterMap (·.1.same)).eraseDups do
    let rs := (cs.filter (·.1.same == some g)).map (·.2)
    let first := rs.headD []
    if !(rs.all fun r => resultText r == resultText first) then
      throw s!"result: the cases of {g} give different texts"
    if rs.all fun r => reading r == reading first then
      throw s!"result: the cases of {g} read alike, so they show nothing about `shown`"

def vectors : Except String (List (String × J)) := do
  let cs := (← runInputs) ++ newCases
  let rs ← cs.mapM fun c => do pure (c, ← resultOf c)
  checkSame rs
  return rs.map fun (c, r) => (c.name, resultVector c r)

end Gen.ResultVectors

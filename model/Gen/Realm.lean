import Loops
import Gen.Json
import Gen.Encode

/-!
# Realm vectors

What a reader makes of every form its places hold, before any vertex asks:
one record per id (`settled`, with what the reader holds of each), the
records refused for their `prev` (`refused`), each thing's state over the
rest (`things`), and Sum over them with what it couldn't read (`sum`).
`input.forms` lists the forms in one list; every rotation must agree.
-/

namespace Gen.RealmVectors

open Loops

def r (id : RecordId) (kind : String) (of : Option String) (t : Nat) (prev : List RecordId := [])
    (about : List String := []) (ok : Reads t := by decide) : Record :=
  { id, kind, observer := "sl:kyle", about, of, value := "", prev, occurred := some (claimAt t ok),
    emitted := stampAt t ok }

-- one record, held in full in one place and erased in another
def full : Record := { r (fixtureId 0x51) "load" (some "host:a") 100 with n := 40 }
def eraseRec : Record := { r (fixtureId 0x52) "erase" none 110 with erases := [fixtureId 0x51] }

-- a record of thing B naming one of thing A in `prev`
def a1 := r (fixtureId 0x61) "facet" (some "thing:A") 1
def b1 := r (fixtureId 0x62) "facet" (some "thing:B") 2 [fixtureId 0x61]

-- a record of one thing that mentions two, then a revision of it: `about`
-- makes no head, so neither is a head of the second thing
def d1 := r (fixtureId 0x71) "status" (some "svc:auth") 1 (about := ["svc:auth", "svc:billing"])
def d2 := r (fixtureId 0x72) "status" (some "svc:auth") 2 [fixtureId 0x71]

-- a root starts a thing named by its id; its revision states that id in `of`,
-- and another root is no fork of either
def m1 := r (fixtureId 0x81) "note" none 1
def m2 := r (fixtureId 0x82) "note" (some (idText (fixtureId 0x81))) 2 [fixtureId 0x81]
def m3 := r (fixtureId 0x83) "config" none 3
-- a revision without `of` is of its own id, so its `prev` crosses things
def m4 := r (fixtureId 0x84) "note" none 4 [fixtureId 0x81]

-- a revision that erases what it replaces: any kind may erase
def x1 := { r (fixtureId 0xa1) "note" (some "person:alice") 1 with n := 7 }
def x2 := { r (fixtureId 0xa2) "note" (some "person:alice") 2 [fixtureId 0xa1] with erases := [fixtureId 0xa1] }

-- a task finished by a record of another kind
def t1 := r (fixtureId 0x91) "task" (some "task:1") 1
def t2 := r (fixtureId 0x92) "task-done" (some "task:1") 2 [fixtureId 0x91]

-- `of` is read as written: a record of `t1`'s id is of that id, not of
-- `t1`'s thing, so it may not replace `t1`
def y1 := r (fixtureId 0x93) "task" (some (idText (fixtureId 0x91))) 3 [fixtureId 0x91]

structure Case where
  name : String
  laws : List String
  forms : List Record

def cases : List Case := [
  ⟨"phase2-exact-output", ["settle.set", "sum.perm", "numeric.safe-whole"],
    [{ r (fixtureId 0xf1) "load" (some "host:a") 100 with n := 9007199254740991 },
      { r (fixtureId 0xf2) "load" (some "host:b") 110 with n := 2 }]⟩,
  ⟨"01-erased-without-record-is-withheld", ["settle.set", "settle.keeps"], [full, full.erase]⟩,
  ⟨"02-here-beats-withheld", ["settle.set", "settle.keeps"], [full.withhold, full, full.withhold]⟩,
  ⟨"03-erase-record-sticks", ["settle.set", "settle.erased"], [full, full.erase, eraseRec, full]⟩,
  ⟨"04-cross-thing-prev-refused", ["admit.no-cross", "thing.local"], [a1, b1]⟩,
  ⟨"05-about-makes-no-head", ["thing.local"], [d1, d2]⟩,
  ⟨"06-root-names-its-thing", ["thing.local"], [m1, m2, m3]⟩,
  ⟨"07-kind-change-supersedes", ["thing.local"], [t1, t2]⟩,
  ⟨"08-prev-without-of-unrelated", ["admit.no-cross", "thing.local"], [m1, m4]⟩,
  ⟨"09-revision-erases", ["settle.set", "settle.erased"], [x1, x2]⟩,
  ⟨"gap01-id-order", ["settle.set", "admit.no-cross"],
    [r (fixtureId 52) "note" (some "thing:A") 1, r (fixtureId 53) "note" (some "thing:B") 2 [fixtureId 52],
      r (fixtureId 0) "note" (some "thing:C") 3, r (fixtureId 1) "note" (some "thing:D") 4 [fixtureId 0]]⟩,
  ⟨"gap02-lone-erased", ["settle.keeps"], [full.erase]⟩,
  ⟨"gap10-refused-eraser", ["settle.erased", "admit.no-cross"],
    [full, { r (fixtureId 0x52) "erase" (some "host:b") 110 [fixtureId 0x51] with erases := [fixtureId 0x51] }]⟩,
  ⟨"gap10-withheld-eraser", ["settle.erased"], [full, eraseRec.withhold]⟩,
  ⟨"10-successor-mentions-another-thing", ["admit.no-cross", "thing.local"],
    [r (fixtureId 0xb1) "note" (some "thing:A") 1,
      r (fixtureId 0xb2) "note" (some "thing:A") 2 [fixtureId 0xb1] (about := ["thing:A", "thing:B"])]⟩,
  ⟨"11-of-read-as-written", ["admit.no-cross", "thing.local"], [t1, y1]⟩
]

def allow : Record → Record → Bool := fun _ _ => true

def realmJ (xs : List Record) : J :=
  let settled := settle xs
  let refused := settled.filterMap fun r => (refusalOf allow settled r).map (r.id, ·)
  let kept := admit allow settled
  let sum := Sum.acc.select (fun _ => .yes) kept
  .obj [
    ("settled", .arr (settled.map fun r => .obj [("id", .str (idText r.id)), ("shown", jShown r.shown)])),
    ("refused", .arr (refused.map fun (i, code) => .obj [("id", .str (idText i)), ("code", .str code)])),
    ("things", jPerThing (perThing kept)),
    ("sum", .obj [("value", .int sum.value), ("unread", .num sum.unread)])]

def rotations (xs : List Record) : List (List Record) :=
  (List.range xs.length).map fun i => xs.drop i ++ xs.take i

def realmVector (c : Case) : Except String J := do
  let orders := rotations c.forms
  let results := orders.map realmJ
  let first := results.headD .null
  if !(results.all (·.render 0 == first.render 0)) then
    throw s!"realm/{c.name}: arrival orders disagree"
  return vector c.name "realm" c.laws
    (.obj [("forms", .arr (c.forms.map jRecord))]) first

def vectors : Except String (List (String × J)) :=
  cases.mapM fun c => do pure (c.name, ← realmVector c)

end Gen.RealmVectors

import Loops.Batch

/-!
# Raw envelope and draft decoding

The library checks raw envelope shape before it decodes values: an unknown
key in any envelope (`envelope-field-unknown`), then any envelope that is not
an object (`envelope-not-object`). A required field left out, or a field of
the wrong JSON type, is then refused with that field's own code, in its place.
The caller supplies JSON with member order and repeated names intact.
-/

namespace Loops.Read

abbrev Dec := Except String

def bad {α : Type} (path what : String) : Dec α := throw s!"{path}: {what}"

def idx (path : String) (i : Nat) : String := s!"{path}[{i}]"
def dot (path k : String) : String := s!"{path}.{k}"

/-- An object's members, each name once. -/
def object (path : String) : Json → Dec (List (String × Json))
  | .obj kvs => do
    let names := kvs.map (·.1)
    for (k, i) in names.zip (List.range names.length) do
      if (names.take i).contains k then bad path s!"repeats the member \"{k}\""
    pure kvs
  | _ => bad path "expected an object"

/-- An object's members, each name once and each one `allowed`. -/
def members? (path : String) (allowed : List String) (j : Json) : Dec (List (String × Json)) := do
  let kvs ← object path j
  for (k, _) in kvs do
    if !allowed.contains k then bad path s!"has a member \"{k}\" this input doesn't take"
  pure kvs

def req (path : String) (kvs : List (String × Json)) (k : String) : Dec Json :=
  match kvs.lookup k with
  | some v => pure v
  | none => bad path s!"is missing \"{k}\""

def text (path : String) : Json → Dec String
  | .str s => pure s
  | _ => bad path "expected text"

def int (path : String) : Json → Dec Int
  | .int i => pure i
  | _ => bad path "expected an integer"

def list (path : String) : Json → Dec (List Json)
  | .arr xs => pure xs
  | _ => bad path "expected an array"

/-- Each element of an array, decoded with its own path. -/
def each {α : Type} (path : String) (f : String → Json → Dec α) (j : Json) : Dec (List α) := do
  let xs ← list path j
  (xs.zip (List.range xs.length)).mapM fun (x, i) => f (idx path i) x

def texts (path : String) : Json → Dec (List String) := each path text

def opt {α : Type} (kvs : List (String × Json)) (k : String) (f : Json → Dec α) : Dec (Option α) :=
  match kvs.lookup k with
  | some v => some <$> f v
  | none => pure none

/-- Bytes from lowercase, unpadded base32hex. Only the spelling `base32` writes
is taken, so trailing bits must be zero and one text names one byte string. -/
def unbase32 (s : String) : Except String (List UInt8) := do
  match Loops.unbase32 s with
  | some bs => pure bs
  | none => throw "is not canonical lowercase base32hex (0-9, a-v): bad alphabet, length or unused bits"

/-- Bytes written in base32hex: a salt, a seed. -/
def bytes (path : String) (j : Json) : Dec (List UInt8) := do
  let s ← text path j
  match unbase32 s with
  | .ok bs => pure bs
  | .error why => bad path s!"\"{s}\" {why}"

/-- A field's text, or `unreadable` when it holds another JSON type. -/
def textOr : Json → String
  | .str s => s
  | _ => unreadable

/-- A set field's members. A set that is not an array reads as one unreadable
member; a member that is not text reads as `unreadable` in its place, so a
label beside it is still a label. -/
def textsOr : Json → List String
  | .arr xs => xs.map textOr
  | _ => [unreadable]

/-- An envelope's values. A required field left out, or any field of the wrong
JSON type, reads as `unreadable`, which that field's own check refuses: the
refusal is the field's code, in the field's place (`Env.problem`). Raw shape is
checked before this, so the envelope is an object here; a repeated member name
is still malformed input. -/
private def decodeValues (path : String) (j : Json) : Dec Env := do
  let kvs ← match j with
    | .obj _ => object path j
    | _ => throw "envelope-not-object"
  let field (k : String) : Option Json := kvs.lookup k
  pure
    { kind := ((field "kind").map textOr).getD unreadable,
      observer := ((field "observer").map textOr).getD unreadable,
      emitted := ((field "emitted").map textOr).getD unreadable,
      occurred := (field "occurred").map textOr,
      about := ((field "about").map textsOr).getD [],
      of := (field "of").map textOr,
      prev := ((field "prev").map textsOr).getD [],
      under := (field "under").map textOr,
      erases := ((field "erases").map textsOr).getD [] }

/-- Read one envelope. Raw shape precedes duplicate and value decoding. -/
def readOne (path : String) (j : Json) : Dec Env := do
  if let some why := envelopesShapeProblem [j] then throw why
  decodeValues path j

/-- A singleton uses the per-envelope boundary; multiple variants share one shape stage. -/
def readMany (path : String) (j : Json) : Dec (List Env) := do
  let raw ← list path j
  match raw with
  | [one] => return [← readOne (idx path 0) one]
  | _ =>
    if let some why := envelopesShapeProblem raw then throw why
    each path decodeValues j

/-- A salt, written in base32hex. One left out, of another JSON type, or not
canonical base32hex reads as no bytes, which `write` refuses as
`salt-not-16-bytes`, in that check's place. -/
def salt : Option Json → List UInt8
  | some (.str s) => (Loops.unbase32 s).getD []
  | _ => []

private def draftValues (path : String) (j : Json) : Dec Draft := do
  let kvs ← members? path ["label", "env", "salt", "payload"] j
  let label ← opt kvs "label" (text (dot path "label"))
  let e ← decodeValues (dot path "env") ((kvs.lookup "env").getD .null)
  pure { label := label, env := e, salt := salt (kvs.lookup "salt"), payload := ← req path kvs "payload" }

/-- Check every occurrence of a draft's env member before labels and values. -/
def draft (path : String) (j : Json) : Dec Draft := do
  if let some why := draftsShapeProblem [j] then throw why
  draftValues path j

/-- Check all draft envelopes before any label, envelope or salt decoding. -/
def drafts (path : String) (j : Json) : Dec (List Draft) := do
  let raw ← list path j
  if let some why := draftsShapeProblem raw then throw why
  each path draftValues j

end Loops.Read

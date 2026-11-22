import Loops.SpecHash

/-! The wire payload of an ordinary tick record. Reading it requires no store. -/
namespace Loops

structure TickPayload where
  rules : Rules
  spec : Spec
  covers : List RecordId
  withheld : List RecordId
  result : List (String × SlotValue)
  deriving Repr, DecidableEq

def tickIds (xs : List RecordId) := Json.arr (xs.map (.str ∘ idText))

/-- Raw members after the applicable hash checks. Their spec vocabulary and result shape are opaque here. -/
structure TickMembers where
  rules : Rules
  spec : Json
  covers : List RecordId
  withheld : List RecordId
  result : Json
  deriving Repr

/-- Write the spec hash under the stated rules and the result hash under this
codec's label. A newer reader's result hash need not be reconstructible here. -/
def TickMembers.toJson (m : TickMembers) (H : Hash) : Json :=
  .obj ([("rules", .str m.rules.text), ("spec", m.spec), ("spec-hash", .str (specHashOf H m.rules m.spec)),
    ("covers", tickIds m.covers), ("result", m.result), ("result-hash", .str (resultHashOf H m.result))] ++
    if m.withheld.isEmpty then [] else [("withheld", tickIds m.withheld)])

def TickPayload.toJson (p : TickPayload) (H : Hash) : Json :=
  TickMembers.toJson ⟨p.rules, p.spec.toJson, p.covers, p.withheld, resultJson p.result⟩ H

def TickPayload.bytes (p : TickPayload) (H : Hash) := utf8 (p.toJson H).canon

inductive TickReadError where
  | invalid (fault : Refusal)
  | otherRules (rules : Rules)
  | unreadableSpec
  deriving Repr, DecidableEq

namespace TickReader

def requiredNames := ["rules", "spec", "spec-hash", "covers", "result", "result-hash"]

mutual
/-- The duplicate-name path for a tick includes array positions. Names are
visited in input order, searching a member's value before the next member. -/
def repeatAt (path : Option String) : Json → Option String
  | .arr xs => repeatIn (path.getD "") 0 xs
  | .obj xs => repeatMember path [] xs
  | _ => none

def repeatIn (path : String) (i : Nat) : List Json → Option String
  | [] => none
  | x :: xs => (repeatAt (some s!"{path}[{i}]") x).orElse fun _ => repeatIn path (i+1) xs

def repeatMember (path : Option String) (seen : List String) : List (String × Json) → Option String
  | [] => none
  | (k,v) :: xs =>
    let child := memberPath path k
    if seen.contains k then some child
    else (repeatAt (some child) v).orElse fun _ => repeatMember path (k :: seen) xs
end

def field (kvs : List (String × Json)) (k : String) : Parse Json :=
  match kvs.lookup k with
  | some j => .ok j
  | none => refuse "tick-member-missing" k

def idAt (k : String) (x : Json × Nat) : Parse RecordId :=
  match recordIdOf? x.1 with
  | some n => pure n
  | none => refuse "tick-id-invalid" s!"{k}[{x.2}]"

def ids (k : String) (j : Json) : Parse (List RecordId) := do
  let .arr xs := j | refuse "tick-ids-not-array" k
  let ns ← xs.zipIdx.mapM (idAt k)
  if !ns.Nodup then refuse "tick-id-duplicate" k
  if !ns.Pairwise (· < ·) then refuse "tick-ids-unsorted" k
  return ns

def hashText (k pre : String) (j : Json) : Parse String := do
  let .str s := j | refuse "tick-hash-invalid" k
  if !pre.toList.isPrefixOf s.toList || !byteText 32 (String.ofList (s.toList.drop pre.length)) then refuse "tick-hash-invalid" k
  return s

def withheldIds (covers : List RecordId) : Option Json → Parse (List RecordId)
  | none => pure []
  | some j => do
    let xs ← TickReader.ids "withheld" j
    if xs.isEmpty then refuse "tick-withheld-empty" "withheld"
    if !xs.all covers.contains then refuse "tick-withheld-outside" "withheld"
    pure xs

def values (H : Hash) (current : Rules) (rawRules spec sh rawCovers result rh : Json) (rawWithheld : Option Json) : Parse TickMembers := do
  let some rules := Rules.ofJson? rawRules | refuse "tick-rules-invalid" "rules"
  unless spec matches .obj _ do refuse "tick-spec-not-object" "spec"
  let covers ← TickReader.ids "covers" rawCovers
  let withheld ← withheldIds covers rawWithheld
  let specHash ← TickReader.hashText "spec-hash" "spec:" sh
  let resHash ← TickReader.hashText "result-hash" "res:" rh
  if specHash != specHashOf H rules spec then refuse "tick-spec-hash-mismatch" "spec-hash"
  if rules.number ≤ current.number && resHash != resultHashOf H result then refuse "tick-result-hash-mismatch" "result-hash"
  return ⟨rules, spec, covers, withheld, result⟩

end TickReader

/-- Check the parts shared by all versions, including the result-hash fit for supported versions. -/
def readTickMembers (H : Hash) (current : Rules) (payload : Json) : Parse TickMembers := do
  let .obj kvs := payload | refuse "tick-not-object" ""
  if !(Json.obj kvs).keysDistinct then
    refuse "tick-member-duplicate" ((TickReader.repeatAt none (.obj kvs)).getD "")
  if let some (name, _) := kvs.find? (fun kv => !(TickReader.requiredNames ++ ["withheld"]).contains kv.1) then
    refuse "tick-member-unknown" name
  let rawRules ← TickReader.field kvs "rules"
  let spec ← TickReader.field kvs "spec"
  let sh ← TickReader.field kvs "spec-hash"
  let rawCovers ← TickReader.field kvs "covers"
  let result ← TickReader.field kvs "result"
  let rh ← TickReader.field kvs "result-hash"
  TickReader.values H current rawRules spec sh rawCovers result rh (kvs.lookup "withheld")

/-- Today's interpretation of the raw spec, also used for an older tick's diagnostic. -/
def TickMembers.readSpec (m : TickMembers) : Except TickReadError Spec := do
  let sp ← (Spec.ofJson m.spec).mapError fun _ => .unreadableSpec
  if m.spec.canon != sp.toJson.canon then throw (.invalid ⟨"tick-spec-noncanonical", "spec"⟩)
  return sp

def TickMembers.readSlots (m : TickMembers) (sp : Spec) : Parse TickPayload := do
  let r ← (readResult sp m.result).mapError fun path => ⟨"tick-result-invalid", path⟩
  return ⟨m.rules, sp, m.covers, m.withheld, r⟩

/-- Interpret the hash-checked members only under the reader's own rules. -/
def TickMembers.readPayload (m : TickMembers) (current : Rules) : Except TickReadError TickPayload := do
  if m.rules != current then throw (.otherRules m.rules)
  let sp ← m.readSpec
  (m.readSlots sp).mapError .invalid

def readTickPayload (H : Hash) (current : Rules) (j : Json) : Except TickReadError TickPayload := do
  let m ← (readTickMembers H current j).mapError TickReadError.invalid
  m.readPayload current

end Loops

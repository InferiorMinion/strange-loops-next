import Loops.Spec
import Loops.Stored

/-!
# The canonical result (D7)

A vertex's whole result, every slot, as bytes and a hash: the result codec,
version 1. It is separate from the reading a reader prints, and it is versioned.

* **Integers are decimal text.** Every integer the result holds is a JSON
  string of decimal digits: no leading zero, `-` before a negative and only
  there, `0` for zero. No integer is a JSON number, so none is ever rounded or
  capped: a sum past 2^53 keeps every digit.
* **Position tells an integer from text.** An integer sits only at `value`,
  `unread` and `ungrouped`, and no position holds both. `value` is `null` for
  a min or max that read nothing. No tag is written.
* **Ids are listed without `shown`.** What a reader holds of a listed record
  is the reading's, not the result's, so two holdings that differ only in what
  is shown give the same bytes. A list keeps its order.
* **The bytes are JCS** (RFC 8785) of that JSON, as `Json.canon` writes it, in
  UTF-8. A map's members are by thing, group key or payload key, which every
  slot already writes in UTF-16 order, so JCS keeps that order; the slots and
  each fixed-member object are sorted by JCS.
* **The hash** is `H("loops/result-hash/v1:" ‖ bytes)`, written `res:` and base32hex,
  as the signature frames an id with `loops/record/v1:`. The label names the
  codec's version. It is 21 bytes, not 16: a commitment hashes a 16-byte salt
  and then JCS, and past 16 bytes this preimage is no JSON
  (`resultPreimage_ne_commit`), so no record commits to it.

`protocol/result.md` is the codec this implements.
-/

namespace Loops

/-! ## Integers as text -/

/-- A decimal digit's character: `0` to `9` for 0 to 9. -/
def digitChar (d : Nat) : Char := Char.ofNat (48 + d)

/-- A whole number's decimal digits, least significant first: `[0]` for zero,
and otherwise no zero last. -/
def revDigits (n : Nat) : List Char :=
  if n < 10 then [digitChar n] else digitChar (n % 10) :: revDigits (n / 10)
termination_by n
decreasing_by omega

/-- A whole number in decimal, most significant digit first, with no leading
zero: `0`, `7`, `9007199254740993`. -/
def natDigits (n : Nat) : List Char := (revDigits n).reverse

/-- An integer as the codec writes it: its decimal digits, and `-` before a
negative one. Zero is `0`, never `-0`. -/
def intText : Int → String
  | .ofNat n => String.ofList (natDigits n)
  | .negSucc n => String.ofList ('-' :: natDigits (n + 1))

/-- A decimal digit's value, for `0` to `9`. -/
def digitVal? (c : Char) : Option Nat :=
  if 48 ≤ c.toNat ∧ c.toNat ≤ 57 then some (c.toNat - 48) else none

/-- Digits least significant first, as a whole number; `none` if one is not a digit. -/
def revValue : List Char → Option Nat
  | [] => some 0
  | c :: cs => do
    let d ← digitVal? c
    let n ← revValue cs
    pure (10 * n + d)

/-- A whole number as the codec reads it: `0`, or digits with no leading zero. -/
def natOf? : List Char → Option Nat
  | [] => none
  | ['0'] => some 0
  | c :: cs => if c = '0' then none else revValue (c :: cs).reverse

/-- An integer as the codec reads it: a whole number, or `-` and one that is
not zero. Nothing else: no `+`, no leading zero, no `-0`, no space. -/
def intOf? (s : String) : Option Int :=
  match s.toList with
  | '-' :: cs => match natOf? cs with
    | some (n + 1) => some (Int.negSucc n)
    | _ => none
  | cs => (natOf? cs).map Int.ofNat

/-! ## The JSON the bytes are made of -/

/-- An integer at `value`, `unread` or `ungrouped`. -/
def intJson (i : Int) : Json := .str (intText i)

/-- Listed records by id alone, in the order listed; what is shown of each is left out. -/
def idsJson (hs : List (RecordId × Shown)) : Json := .arr (hs.map fun h => .str (idText h.1))

/-- One group's value, or a slot's without `by`, its members in JCS order. -/
def GroupValue.result : GroupValue → Json
  | .nat r => .obj [("unread", intJson r.unread), ("value", intJson r.value)]
  | .int r => .obj [("unread", intJson r.unread), ("value", intJson r.value)]
  | .opt r => .obj [("unread", intJson r.unread), ("value", match r.value with
      | some v => intJson v
      | none => .null)]
  | .heads hs u => .obj [("heads", idsJson hs), ("unread", intJson u)]

/-- A thing's keyed state: its heads, each declared key's live setters, the
live whole-read emits. -/
def KeyedView.result (k : KeyedView) : Json :=
  .obj [("heads", idsJson k.heads), ("keys", .obj (k.keys.map fun p => (p.1, idsJson p.2))),
    ("whole", idsJson k.whole)]

/-- A slot's value in the canonical result. -/
def SlotValue.result : SlotValue → Json
  | .nat r => (GroupValue.nat r).result
  | .int r => (GroupValue.int r).result
  | .opt r => (GroupValue.opt r).result
  | .heads hs u => (GroupValue.heads hs u).result
  | .things ts u => .obj [("things", .obj (ts.map fun p => (p.1, idsJson p.2))), ("unread", intJson u)]
  | .keyed ts u => .obj [("things", .obj (ts.map fun p => (p.1, p.2.result))), ("unread", intJson u)]
  | .groups gs u m => .obj [("groups", .obj (gs.map fun p => (p.1, p.2.result))),
      ("ungrouped", intJson m), ("unread", intJson u)]

/-- A vertex's whole result: every slot, by name. -/
def resultJson (r : List (String × SlotValue)) : Json := .obj (r.map fun p => (p.1, p.2.result))

/-! ## Bytes and hash -/

/-- The canonical result's text: JCS of `resultJson`. -/
def resultText (r : List (String × SlotValue)) : String := (resultJson r).canon

/-- The canonical result's bytes: its text in UTF-8. -/
def resultBytes (r : List (String × SlotValue)) : List UInt8 := utf8 (resultText r)

/-- The codec's version, which frames what is hashed. -/
def resultLabel : String := "loops/result-hash/v1:"

/-- What the result hash covers: the label, then the canonical bytes. -/
def resultPreimage (r : List (String × SlotValue)) : List UInt8 := utf8 resultLabel ++ resultBytes r

/-- Hash a supplied result member before interpreting its slot vocabulary. -/
def resultHashOf (H : Hash) (j : Json) : String :=
  "res:" ++ base32 (H (utf8 resultLabel ++ utf8 j.canon))

def resultHash (H : Hash) (r : List (String × SlotValue)) : String := resultHashOf H (resultJson r)

/-! ## What `shown` is -/

/-- Listed records with what is shown of each set aside: every one read as held. -/
def unshownIds (hs : List (RecordId × Shown)) : List (RecordId × Shown) := hs.map fun h => (h.1, .here)

def GroupValue.unshown : GroupValue → GroupValue
  | .heads hs u => .heads (unshownIds hs) u
  | v => v

def KeyedView.unshown (k : KeyedView) : KeyedView :=
  ⟨k.keys.map fun p => (p.1, unshownIds p.2), unshownIds k.whole, unshownIds k.heads⟩

/-- A slot's value with `shown` set aside: what the canonical result keeps of it. -/
def SlotValue.unshown : SlotValue → SlotValue
  | .heads hs u => .heads (unshownIds hs) u
  | .things ts u => .things (ts.map fun p => (p.1, unshownIds p.2)) u
  | .keyed ts u => .keyed (ts.map fun p => (p.1, p.2.unshown)) u
  | .groups gs u m => .groups (gs.map fun p => (p.1, p.2.unshown)) u m
  | v => v

/-- A whole result with `shown` set aside in every slot. -/
def unshownResult (r : List (String × SlotValue)) : List (String × SlotValue) :=
  r.map fun p => (p.1, p.2.unshown)

/-! ## Reading the result for a spec -/

/-- Decode exactly the canonical text of a 32-byte record id. -/
def recordIdOf? (j : Json) : Option RecordId := do
  let .str s := j | none
  let 'r' :: ':' :: chars := s.toList | none
  let bs ← unbase32 (String.ofList chars)
  if h : bs.length = 32 then
    some ⟨bs.foldl (fun n b => 256 * n + b.toNat) 0, bytes_bounded bs h⟩
  else none

namespace ResultReader

/-- A failure identifies the position in the supplied result. -/
abbrev Read := Except String

def member (p k : String) := memberPath (some p) k

def object (p : String) : Json → Read (List (String × Json))
  | .obj kvs => if (kvs.map Prod.fst).Nodup then pure kvs else throw p
  | _ => throw p

def fixed (p : String) (names : List String) (j : Json) : Read (List (String × Json)) := do
  let kvs ← object p j
  if let some (k, _) := kvs.find? (fun kv => !names.contains kv.1) then throw (member p k)
  if let some k := names.find? (fun k => (kvs.lookup k).isNone) then throw (member p k)
  return kvs

def get (p : String) (kvs : List (String × Json)) (k : String) (f : String → Json → Read α) : Read α := do
  let path := member p k
  let some j := kvs.lookup k | throw path
  f path j

def required (p : String) : Option α → Read α
  | some x => .ok x
  | none => .error p

def nat (p : String) : Json → Read Nat
  | .str s => required p (natOf? s.toList)
  | _ => throw p

def integer (p : String) : Json → Read Int
  | .str s => required p (intOf? s)
  | _ => throw p

def optInt (p : String) : Json → Read (Option Int)
  | .null => pure none
  | j => some <$> integer p j

def ids (p : String) : Json → Read (List (RecordId × Shown))
  | .arr xs => xs.zipIdx.mapM fun (j, i) => do
      let id ← required s!"{p}[{i}]" (recordIdOf? j)
      return (id, .here)
  | _ => throw p

def named (f : String → Json → Read α) (p : String) (j : Json) : Read (List (String × α)) := do
  let kvs ← object p j
  kvs.mapM fun (k, v) => do return (k, ← f (member p k) v)

def group (ty : FoldType) (p : String) (j : Json) : Read GroupValue := do
  let name := match ty with | .collect .. => "heads" | _ => "value"
  let kvs ← fixed p ["unread", name] j
  let u ← get p kvs "unread" nat
  match ty with
  | .count _ => return .nat ⟨← get p kvs "value" nat, u⟩
  | .sum .. => return .int ⟨← get p kvs "value" integer, u⟩
  | .min .. | .max .. => return .opt ⟨← get p kvs "value" optInt, u⟩
  | .collect .. => return .heads (← get p kvs "heads" ids) u
  | .latestBy _ => throw p

def keyed (p : String) (j : Json) : Read KeyedView := do
  let kvs ← fixed p ["heads", "keys", "whole"] j
  let heads ← get p kvs "heads" ids
  let keys ← get p kvs "keys" (named ids)
  let whole ← get p kvs "whole" ids
  return ⟨keys, whole, heads⟩

def slot (s : Slot) (p : String) (j : Json) : Read SlotValue := do
  if s.group.isSome then
    let kvs ← fixed p ["groups", "ungrouped", "unread"] j
    let gs ← get p kvs "groups" (named (group s.type))
    let m ← get p kvs "ungrouped" nat
    return .groups gs (← get p kvs "unread" nat) m
  if let .latestBy follow := s.type then
    let kvs ← fixed p ["things", "unread"] j
    let u ← get p kvs "unread" nat
    match follow with
    | .prev => return .things (← get p kvs "things" (named ids)) u
    | .keys => return .keyed (← get p kvs "things" (named keyed)) u
  return match ← group s.type p j with
    | .nat r => .nat r | .int r => .int r | .opt r => .opt r | .heads hs u => .heads hs u

end ResultReader

/-- Reconstruct the result in spec slot order. Listed records have no wire
visibility and are represented with `shown = here`. Array order is retained. -/
def readResult (s : Spec) (j : Json) : Except String (List (String × SlotValue)) := do
  let kvs ← ResultReader.fixed "result" (s.slots.map Slot.name) j
  s.slots.mapM fun slot => do
    return (slot.name, ← ResultReader.get "result" kvs slot.name (ResultReader.slot slot))

end Loops

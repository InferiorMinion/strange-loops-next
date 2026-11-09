import Loops.Record

/-!
# Which keys speak for an observer

A stored line proves which key signed a record. It does not say whose key
that is. This file is the smallest rule that does:

* **An anchor starts a lineage.** It is a record of kind `lineage-anchor`
  whose observer is the key that signed it. The lineage is named by the
  anchor's id, and that key is its root.
* **A binding adds a key.** It is a record of kind `key-binding` whose
  observer is the lineage and whose `about` names the key. It counts only when
  a key that already speaks for the lineage signed it.

Both read only the envelope and the signing key, so a withheld or erased
payload never changes who speaks. Nothing here removes a key: retiring one is
standing's next step, and the first rule that makes this depend on more than
the set.
-/

namespace Loops

/-- A record, and the key whose signature over its id checked. -/
structure Signed where
  record : Record
  key : String
  deriving Repr, DecidableEq, Inhabited

def anchorKind : String := "lineage-anchor"
def bindingKind : String := "key-binding"

/-- The lineage an anchor starts: `sl:` and the anchor's digest, as its id writes it. -/
def lineageOf (i : RecordId) : String := "sl:" ++ (idText i).drop 2

/-- A key, as a stored line and a typed reference write it. -/
def keyRef (s : String) : Bool := s.startsWith "k:"

/-- `a` starts lineage `L`: it is an anchor, it speaks as the key that signed
it, and its id names `L`. -/
def Signed.anchors (a : Signed) (L : String) : Bool :=
  a.record.kind == anchorKind && a.record.observer == a.key && lineageOf a.record.id == L

/-- `b` says key `k` speaks for `L`. Whether that counts depends on who signed `b`. -/
def Signed.binds (b : Signed) (L k : String) : Bool :=
  b.record.kind == bindingKind && b.record.observer == L && keyRef k && b.record.about.contains k

/-- Key `k` speaks for lineage `L`, by what `xs` holds: it is the root an
anchor pins, or a key that speaks for `L` signed a binding naming it. -/
inductive Speaks (xs : List Signed) (L : String) : String → Prop where
  | root {a : Signed} : a ∈ xs → a.anchors L = true → Speaks xs L a.key
  | bound {b : Signed} {k : String} :
      b ∈ xs → b.binds L k = true → Speaks xs L b.key → Speaks xs L k

/-- A record is its observer's word: the observer is the signing key itself,
or a lineage that key speaks for. -/
def Attributed (xs : List Signed) (s : Signed) : Prop :=
  s.record.observer = s.key ∨ Speaks xs s.record.observer s.key

/-! ## Computing it -/

def roots (xs : List Signed) (L : String) : List String :=
  (xs.filter (·.anchors L)).map (·.key)

/-- One round: the keys named by bindings a key in `ks` signed. -/
def boundBy (xs : List Signed) (L : String) (ks : List String) : List String :=
  (xs.filter fun b => b.record.kind == bindingKind && b.record.observer == L && ks.contains b.key).flatMap
    fun b => b.record.about.filter keyRef

def speakersIn (xs : List Signed) (L : String) : Nat → List String
  | 0 => roots xs L
  | n + 1 => speakersIn xs L n ++ boundBy xs L (speakersIn xs L n)

/-- The keys found in as many rounds as there are records. -/
def speakers (xs : List Signed) (L : String) : List String := speakersIn xs L xs.length

end Loops

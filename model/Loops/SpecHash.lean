import Loops.Result

/-! The version of the evaluation rules frames the parsed spec's JCS bytes. -/
namespace Loops

/-- Rules versions are strictly positive, with arbitrary precision. -/
structure Rules where
  number : Nat
  positive : 0 < number
  deriving DecidableEq, Repr

def Rules.current : Rules := ⟨1, by decide⟩

def Rules.text (v : Rules) : String := String.ofList (natDigits v.number)

instance : Ord Rules := ⟨fun a b => compare a.number b.number⟩

/-- Only the canonical decimal string identifies a rules version. -/
def Rules.ofJson? : Json → Option Rules
  | .str s => do
    let n ← natOf? s.toList
    if h : 0 < n then
      some ⟨n, h⟩
    else none
  | _ => none

def specLabel (v : Rules) : String := "loops/spec-hash/v" ++ v.text ++ ":"

/-- The raw-member entry point also works before the spec language is known. -/
def specPreimageOf (v : Rules) (j : Json) : List UInt8 := utf8 (specLabel v) ++ utf8 j.canon

def specHashOf (H : Hash) (v : Rules) (j : Json) : String :=
  "spec:" ++ base32 (H (specPreimageOf v j))

def Spec.hash (H : Hash) (v : Rules) (sp : Spec) : String := specHashOf H v sp.toJson

end Loops

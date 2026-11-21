import Laws.By
import Loops.Stored

namespace Loops

theorem Stored.read_opening (env : Env) (salt : List UInt8) (p : Json) (r : Record) :
    (make env salt p).readInto r = { r with fields := payloadFields p, shown := .here } := rfl

theorem Stored.fields_payload_only (e e' : Env) (s s' : List UInt8) (p : Json) (r r' : Record) :
    ((make e s p).readInto r).fields = ((make e' s' p).readInto r').fields := rfl

theorem Stored.read_erased (e : Env) (commit : String) (r : Record) :
    (Stored.mk e (.erased commit)).readInto r = r.erase := rfl

theorem payloadFields_nonobject (p : Json) (h : ∀ kvs, p ≠ .obj kvs) : payloadFields p = [] := by
  cases p <;> first | rfl | exact False.elim (h _ rfl)

theorem payloadFields_lookup (kvs : List (String × Json)) (f : String) :
    (payloadFields (.obj kvs)).lookup f = (kvs.lookup f).map readHeld := by
  induction kvs with
  | nil => rfl
  | cons kv rest ih =>
    obtain ⟨k, v⟩ := kv
    simp only [payloadFields, List.map_cons, List.lookup] at ih ⊢
    cases (f == k) <;> simp [ih]

theorem readHeld_members_nameable (xs : List Json) (vs : List Val)
    (h : readHeld (.arr xs) = .many vs) : ∀ v ∈ vs, v.nameable := by
  obtain rfl := HeldField.many.inj h
  intro v hv
  obtain ⟨_, _, hm⟩ := List.mem_filterMap.mp hv
  exact (Option.filter_eq_some_iff.mp hm).2

theorem Cond.payload_present (r : Record) (f : String) (h : r.shown = .here) :
    Cond.eval ⟨.payload f, .present⟩ r = .ofBool (r.get f).isSome := by
  cases hv : r.get f <;> simp [Cond.eval, Field.read, Op.test, h, hv]

theorem HeldField.list_is (vs : List Val) (a : Val) :
    (Op.is (.val a)).test [(HeldField.many vs).item] = vs.contains a := by
  cases vs <;> simp [HeldField.item, Arg.matches, Op.test]

theorem HeldField.empty_item : (HeldField.many []).item = .other := rfl

theorem HeldField.list_bounds (vs : List Val) (a : Arg) :
    (Op.atLeast a).test [(HeldField.many vs).item] = false ∧
    (Op.below a).test [(HeldField.many vs).item] = false := by
  cases vs <;> cases a <;> simp [HeldField.item, Op.test, Item.atLeast, Item.below]

theorem Record.num_many (r : Record) (f : String) (vs : List Val) (h : r.get f = some (.many vs)) :
    r.num f = none := by simp [Record.num, h]

theorem HeldField.list_key (vs : List Val) (k : String) :
    k ∈ (HeldField.many vs).keys ↔ ∃ v ∈ vs, v.key = some k := by
  simp [HeldField.keys, mem_canonSet]

end Loops

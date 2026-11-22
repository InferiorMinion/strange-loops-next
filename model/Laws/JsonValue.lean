import Laws.SpecText
import Laws.Write

/-! Recover the JSON value of duplicate-free, finite wire data from JCS.
Numbers are compared by their JCS numeral; strings remain strings. -/
namespace Loops
open List SpecText

inductive Json.WireValue where
  | literal (token : String)
  | str (s : String)
  | arr (xs : List Json.WireValue)
  | obj (xs : List (String × Json.WireValue))

private theorem sorted_child {ms : List (String × Json)} {x} (h : x ∈ sortedMembers ms) :
    x ∈ ms := by
  simpa using (sortBy_go (fun kv : String × Json => utf16 kv.1) ms [] (by simp [Sorted])).2.1 x h

def Json.wireValue (j : Json) : Json.WireValue :=
  match j with
  | .null | .bool _ | .int _ | .number _ => .literal j.canon
  | .str s => .str s
  | .arr xs => .arr (xs.map Json.wireValue)
  | .obj ms => .obj ((sortedMembers ms).attach.map fun ⟨m,_⟩ => (m.1, m.2.wireValue))
termination_by sizeOf j
decreasing_by
  · decreasing_tactic
  · rename_i h
    have h := sorted_child h
    have := List.sizeOf_lt_of_mem h
    cases m; simp_all +arith; omega

private def numeric (c : Char) := c.isDigit || ['-','+','.','e'].contains c

set_option maxHeartbeats 4000000 in
private theorem numeral_chars (n : Number) (finite : n.finite = true) :
    ∀ c ∈ n.canon.toList, numeric c := by
  have natChars (v : Nat) : ∀ c ∈ (toString v).toList, numeric c := by
    rw [Nat.toString_eq_ofList_toDigits, String.toList_ofList]
    intro c hc
    simp [numeric, Nat.isDigit_of_mem_toDigits (by decide : 0 < 10) (by decide) hc]
  have takeChars (v i : Nat) : ∀ c ∈ (toString v).toList.take i, numeric c :=
    fun c h => natChars v c (mem_of_mem_take h)
  have dropChars (v i : Nat) : ∀ c ∈ (toString v).toList.drop i, numeric c :=
    fun c h => natChars v c (mem_of_mem_drop h)
  have zeros (v : Nat) : ∀ c ∈ (String.ofList (List.replicate v '0')).toList, numeric c := by
    intro c hc
    simp at hc
    rw [hc.2]; decide
  let safe (s : String) := s.toList.all numeric = true
  have app (a b : String) : safe (a ++ b) ↔ safe a ∧ safe b := by
    simp only [safe, String.toList_append, all_append, Bool.and_eq_true]
  have ds (v : Nat) : safe (toString v) := all_eq_true.2 (natChars v)
  have take (v i : Nat) : safe (String.ofList ((toString v).toList.take i)) := by
    simpa only [safe, String.toList_ofList, all_eq_true] using takeChars v i
  have drop (v i : Nat) : safe (String.ofList ((toString v).toList.drop i)) := by
    simpa only [safe, String.toList_ofList, all_eq_true] using dropChars v i
  have literalZeros (v : Nat) : safe (Number.zeros v) := all_eq_true.2 (zeros v)
  suffices safe n.canon from all_eq_true.1 this
  have fmt (neg m e) : safe (Number.decimalText neg m e) := by
    unfold Number.decimalText
    simp only [Id.run, pure]
    repeat' split
    all_goals simp only [app, ds, take, drop, literalZeros, true_and, and_true] <;> decide
  unfold Number.canon
  simp only [finite, Bool.not_true, Bool.false_eq_true, ↓reduceIte, Id.run, pure]
  split
  · decide
  · exact fmt _ _ _

/-- Finiteness is the numerical premise of JCS. It applies recursively to raw
members whose future shape the tick reader need not understand. -/
inductive Json.Finite : Json → Prop
  | null : Json.Finite .null
  | bool (b) : Json.Finite (.bool b)
  | int (i) : Json.Finite (.int i)
  | number {n} : n.finite = true → Json.Finite (.number n)
  | str (s) : Json.Finite (.str s)
  | arr {xs} : (∀ x ∈ xs, Json.Finite x) → Json.Finite (.arr xs)
  | obj {ms} : (∀ m ∈ ms, Json.Finite m.2) → Json.Finite (.obj ms)

mutual
theorem Json.finite_of_inRange : (j : Json) → j.inRange = true → j.Finite
  | .null, _ => .null
  | .bool b, _ => .bool b
  | .int i, _ => .int i
  | .number n, h => .number (Number.finite_of_inRange (n := n) h)
  | .str s, _ => .str s
  | .arr xs, h => .arr (finiteArray xs h)
  | .obj ms, h => .obj (finiteMembers ms h)

private theorem finiteArray : (xs : List Json) → Json.allInRange xs = true → ∀ x ∈ xs, Json.Finite x
  | [], _, _, h => False.elim (not_mem_nil h)
  | a :: xs, h, x, hx => by
    obtain ⟨ha, hxs⟩ := Bool.and_eq_true_iff.1 h
    rcases mem_cons.1 hx with eq | hm
    · exact eq.symm ▸ a.finite_of_inRange ha
    · exact finiteArray xs hxs x hm

private theorem finiteMembers : (ms : List (String × Json)) → Json.membersInRange ms = true →
    ∀ m ∈ ms, Json.Finite m.2
  | [], _, _, h => False.elim (not_mem_nil h)
  | (k,v) :: ms, h, m, hm => by
    obtain ⟨hv, hms⟩ := Bool.and_eq_true_iff.1 h
    rcases mem_cons.1 hm with eq | hm
    · exact eq.symm ▸ v.finite_of_inRange hv
    · exact finiteMembers ms hms m hm
end

private def bareChar (c : Char) := !([',',']','}','"','[','{'].contains c)

private theorem bare_of_numeric {c : Char} (h : numeric c) : bareChar c := by
  simp [bareChar]
  exact ⟨fun e => by subst c; contradiction, fun e => by subst c; contradiction,
    fun e => by subst c; contradiction, fun e => by subst c; contradiction,
    fun e => by subst c; contradiction, fun e => by subst c; contradiction⟩

/-- The hypotheses needed to split a JCS text back into its constituent values. -/
private inductive Shape : Json → Prop
  | bare {j} : j.wireValue = .literal j.canon → (∀ c ∈ j.canon.toList, bareChar c) → Shape j
  | str (s) : Shape (.str s)
  | arr {xs} : (∀ x ∈ xs, Shape x) → Shape (.arr xs)
  | obj {ms} : (ms.map Prod.fst).Nodup → (∀ x ∈ ms, Shape x.2) → Shape (.obj ms)

private theorem shape_of_finite {j} (hf : Json.Finite j) (hd : j.keysDistinct = true) : Shape j := by
  induction hf with
  | null => exact .bare (by rw [Json.wireValue]) (all_eq_true.1 (by decide))
  | bool b => cases b <;> exact .bare (by rw [Json.wireValue]) (all_eq_true.1 (by decide))
  | int i =>
    apply Shape.bare (by rw [Json.wireValue])
    have digits (n : Nat) (c : Char) (h : c ∈ (toString n).toList) : bareChar c := by
      rw [Nat.toString_eq_ofList_toDigits, String.toList_ofList] at h
      apply bare_of_numeric
      simp [numeric, Nat.isDigit_of_mem_toDigits (by decide : 0 < 10) (by decide) h]
    cases i with
    | ofNat n => exact digits n
    | negSucc n =>
      change ∀ c ∈ ("-" ++ toString (n+1)).toList, bareChar c
      simp only [String.toList_append, minus_toList, singleton_append, mem_cons, forall_eq_or_imp]
      exact ⟨by decide, digits _⟩
  | number h => exact .bare (by rw [Json.wireValue]) (fun c hc => bare_of_numeric (numeral_chars _ h c hc))
  | str s => exact .str s
  | arr _ ih => exact .arr (fun x h => ih x h (Json.keysDistinct_arr.1 hd x h))
  | obj _ ih => exact .obj (Json.keysDistinct_obj.1 hd).1 (fun x h => ih x h ((Json.keysDistinct_obj.1 hd).2 x h))

private def leading : List Char → Nat
  | '"' :: _ => 1 | '[' :: _ => 2 | '{' :: _ => 3 | _ => 0

private theorem bare_leading {j : Json} (chars : ∀ c ∈ j.canon.toList, bareChar c) (xs : List Char) :
    leading (j.canon.toList ++ xs) = 0 := by
  obtain ⟨c, cs, eq, _⟩ := Json.canon_head j
  have hc := chars c (eq ▸ mem_cons_self)
  rw [eq, cons_append]
  unfold leading
  split <;> first | rfl | (rename_i he; obtain rfl := (cons.inj he).1; contradiction)

private def treeKind : Json.WireValue → Nat
  | .literal _ => 0 | .str _ => 1 | .arr _ => 2 | .obj _ => 3

private theorem shape_leading {j} (s : Shape j) (xs) : leading (j.canon.toList ++ xs) = treeKind j.wireValue := by
  cases s with
  | bare repr chars => rw [repr]; exact bare_leading chars xs
  | str s => rw [Json.wireValue, canon_str]; rfl
  | arr _ => rw [Json.wireValue, canon_arr]; rfl
  | obj _ _ => simp only [Json.wireValue, Json.canon, String.toList_append]; rfl

private theorem object_spelling {ms : List (String × Json)} (hd : (ms.map Prod.fst).Nodup) (X : Chars) :
    (Json.obj ms).canon.toList ++ X = '{' :: separated memberText '}' (sortedMembers ms) X := by
  have hp := sortBy_perm (key := fun kv : String × Json => utf16 kv.1)
    (fun a ha b hb eq => eq_of_mem_of_nodup_map hd a ha b hb (utf16_inj eq)) (Pairwise.of_map Prod.fst (fun _ _ ne e => ne (e ▸ rfl)) hd)
  rw [canon_member_order hp.symm hd, canon_obj (sortBy_sorted _ _)]
  simp [separated, sortedMembers]

private theorem obj_view (ms : List (String × Json)) :
    (Json.obj ms).wireValue = .obj ((sortedMembers ms).map fun p => (p.1, p.2.wireValue)) := by
  rw [Json.wireValue]
  exact congrArg Json.WireValue.obj (attach_map_val (f := fun p : String × Json => (p.1, p.2.wireValue)))

private theorem wire_prefix {a b : Json} (sa : Shape a) (sb : Shape b) {X Y : Chars}
    (hx : boundary X) (hy : boundary Y) (h : a.canon.toList ++ X = b.canon.toList ++ Y) :
    a.wireValue = b.wireValue ∧ X = Y := by
  have k := (shape_leading sa X).symm.trans ((congrArg leading h).trans (shape_leading sb Y))
  induction sa generalizing b X Y with
  | bare repr chars =>
    cases sb <;> try (simp only [repr, Json.wireValue, treeKind] at k; contradiction)
    rename_i repB charsB
    have safe {j : Json} (hs : ∀ c ∈ j.canon.toList, bareChar c) : ∀ c ∈ j.canon.toList, !delimiter c := by
      intro c hc
      have := hs c hc
      simp [bareChar, delimiter] at this ⊢
      grind
    obtain ⟨eq, rfl⟩ := primitive_boundary (safe chars) (safe charsB) hx hy h
    exact ⟨by rw [repr, repB, String.toList_inj.1 eq], rfl⟩
  | str s =>
    cases sb <;> try (simp only [*, Json.wireValue, treeKind] at k; contradiction)
    obtain ⟨rfl, rfl⟩ := quoted_prefix h
    exact ⟨rfl, rfl⟩
  | arr fs ih =>
    cases sb <;> try (simp only [*, Json.wireValue, treeKind] at k; contradiction)
    rename_i bs gs
    simp only [canon_arr, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨eq, rest⟩ := splitElementsMap Json.wireValue (close := ']') (by decide) (by decide)
      (fun j => value_start j _ (by decide)) (fun a ha b hb U V hu hv e =>
        ih a ha (gs b hb) hu hv e ((shape_leading (fs a ha) U).symm.trans ((congrArg leading e).trans (shape_leading (gs b hb) V)))) h
    exact ⟨by simpa only [Json.wireValue, Json.WireValue.arr.injEq] using eq, rest⟩
  | obj hd fs ih =>
    cases sb <;> try (simp only [*, Json.wireValue, treeKind] at k; contradiction)
    rename_i ns hn gs
    rw [object_spelling hd, object_spelling hn] at h
    obtain ⟨eq, rest⟩ := splitElementsMap (fun kv : String × Json => (kv.1,kv.2.wireValue))
      (f := memberText) (close := '}') (by decide) (by decide)
      (fun kv => ⟨_, _, (memberText_head kv).choose_spec, by decide⟩) (by
        intro a ha b hb U V hu hv e
        simp only [memberText, cons_append, append_assoc, cons.injEq, true_and] at e
        obtain ⟨keys, tails⟩ := escapes_prefix _ _ e
        have ev := (cons.inj tails).2
        have aa := sorted_child ha
        have bb := sorted_child hb
        obtain ⟨vals, rest⟩ := ih a aa (gs b bb) hu hv ev
          ((shape_leading (fs a aa) U).symm.trans ((congrArg leading ev).trans (shape_leading (gs b bb) V)))
        exact ⟨Prod.ext (String.toList_inj.1 keys) vals, rest⟩) (cons.inj h).2
    exact ⟨by rw [obj_view, obj_view, eq], rest⟩

theorem Json.wireValue_of_canon {a b : Json} (fa : a.Finite) (fb : b.Finite)
    (da : a.keysDistinct = true) (db : b.keysDistinct = true) (same : a.canon = b.canon) :
    a.wireValue = b.wireValue :=
  (wire_prefix (shape_of_finite fa da) (shape_of_finite fb db) (X := []) (Y := []) rfl rfl (by rw [same])).1

/-- The checked record writer already supplies both domain premises. -/
theorem Written.payload_wireValue (a b : Written) (same : a.payload.canon = b.payload.canon) :
    a.payload.wireValue = b.payload.wireValue :=
  Json.wireValue_of_canon (a.payload.finite_of_inRange a.ok.range) (b.payload.finite_of_inRange b.ok.range)
    a.ok.members b.ok.members same

end Loops

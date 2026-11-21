import Loops.Fold
import Laws.Fold

/-!
# Laws of Sum, Min and Max

Latest laws live in `Laws.Things`.
* **Sum** is order-free and counts repeats, so the store hands each id over
  once. It is exact: over floating point, addition isn't associative and
  the order of arrival would change the total.
* **Min and Max**: the set decides. Order and repeats change nothing.
-/

namespace Loops

open List

/-- A readable numeric value lies inside the safe-whole range. -/
theorem Record.num_safe {r : Record} {field : String} {i : Int}
    (h : r.num field = some i) : -(2^53 - 1) ≤ i ∧ i ≤ 2^53 - 1 := by
  unfold Record.num at h
  split at h
  · cases hg : r.get field <;> simp only [hg] at h
    · cases h
    · rename_i v
      cases v with
      | many | other => contradiction
      | one v =>
        cases v <;> simp only at h
        all_goals try { cases h }
        split at h
        · rename_i hs
          cases h
          exact hs
        · cases h
  · cases h

/-- A nonwhole numeric payload is unread by a numeric slot. -/
theorem Record.num_number {r : Record} {field : String} {n : Number}
    (h : r.get field = some (.one (.number n))) : r.num field = none := by
  simp [Record.num, h]

/-! ## Sum -/

theorem Sum.batch (p : Record → Option Int) (s : Int) (xs ys : List Record) :
    Sum.fold p (Sum.fold p s xs) ys = Sum.fold p s (xs ++ ys) := by
  simp [Sum.fold, List.foldl_append]

theorem Sum.perm (p : Record → Option Int) {xs ys : List Record} (h : xs ~ ys) (s : Int) : Sum.fold p s xs = Sum.fold p s ys :=
  h.foldl_eq' (fun x _ y _ z => by simp only [Sum.step]; omega) s

theorem Sum.fold_eq (p : Record → Option Int) (s : Int) (xs : List Record) : Sum.fold p s xs = s + Sum.fold p 0 xs := by
  induction xs generalizing s with
  | nil => simp [Sum.fold]
  | cons x xs ih =>
    simp only [Sum.fold, foldl_cons, Sum.step] at *
    rw [ih, ih (0 + (p x).getD 0)]; omega

/-- A repeated delivery is added again: the store must deduplicate by id. -/
theorem Sum.repeats (p : Record → Option Int) (xs : List Record) : Sum.fold p 0 (xs ++ xs) = 2 * Sum.fold p 0 xs := by
  rw [← Sum.batch, Sum.fold_eq]; omega

/-! ## Max -/

theorem Max.batch (p : Record → Option Int) (s : Option Int) (xs ys : List Record) :
    Max.fold p (Max.fold p s xs) ys = Max.fold p s (xs ++ ys) := by
  simp [Max.fold, List.foldl_append]

theorem Max.fold_some (p : Record → Option Int) (a : Int) (xs : List Record) :
    Max.fold p (some a) xs = some ((xs.filterMap p).foldl max a) := by
  induction xs generalizing a with
  | nil => rfl
  | cons x xs ih =>
    cases hx : p x <;> simp [Max.fold, Max.step, hx] at * <;> apply ih

/-- Max is the largest value seen, or nothing until the first readable record. -/
theorem Max.fold_eq (p : Record → Option Int) (xs : List Record) : Max.fold p none xs = (xs.filterMap p).max? := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    change Max.fold p (Max.step p none x) xs = _
    cases hx : p x with
    | none => simpa [Max.step, hx] using ih
    | some v => simpa only [Max.step, hx, List.filterMap_cons_some (f := p) hx, Option.elim, List.max?_cons'] using Max.fold_some p v xs

/-- The set decides: the same records, in any order, with any repeats, give the same Max. -/
theorem Max.set_decides (p : Record → Option Int) {xs ys : List Record} (h : ∀ r, r ∈ xs ↔ r ∈ ys) :
    Max.fold p none xs = Max.fold p none ys := by
  have hv : ∀ v, v ∈ xs.filterMap p ↔ v ∈ ys.filterMap p := by
    intro v; simp only [mem_filterMap]
    exact ⟨fun ⟨r, hr, e⟩ => ⟨r, (h r).1 hr, e⟩, fun ⟨r, hr, e⟩ => ⟨r, (h r).2 hr, e⟩⟩
  rw [Max.fold_eq, Max.fold_eq]
  cases hx : (xs.filterMap p).max? with
  | none =>
    rw [max?_eq_none_iff] at hx
    have : ys.filterMap p = [] := eq_nil_iff_forall_not_mem.2 fun v hv' => by
      have := (hv v).2 hv'; rw [hx] at this; simp at this
    rw [this]; rfl
  | some a =>
    rw [max?_eq_some_iff] at hx
    symm; rw [max?_eq_some_iff]
    exact ⟨(hv a).1 hx.1, fun b hb => hx.2 b ((hv b).2 hb)⟩

/-! ## Min, by symmetry -/

theorem Min.batch (p : Record → Option Int) (s : Option Int) (xs ys : List Record) :
    Min.fold p (Min.fold p s xs) ys = Min.fold p s (xs ++ ys) := by
  simp [Min.fold, List.foldl_append]

/-- Min is Max on negated values. -/
theorem Min.neg_max (p : Record → Option Int) (s : Option Int) (xs : List Record) :
    Min.fold p s xs = (Max.fold (fun r => (p r).map (- ·)) (s.map (- ·)) xs).map (- ·) := by
  induction xs generalizing s with
  | nil => cases s <;> simp [Min.fold, Max.fold]
  | cons x xs ih =>
    let q := fun r => (p r).map (- ·)
    change Min.fold p (Min.step p s x) xs = (Max.fold q (Max.step q (s.map (- ·)) x) xs).map (- ·)
    rw [ih]
    have h : (Min.step p s x).map (- ·) = Max.step q (s.map (- ·)) x := by
      cases hx : p x <;> cases s <;> simp [Min.step, Max.step, q, hx]
    rw [h]

theorem Min.set_decides (p : Record → Option Int) {xs ys : List Record} (h : ∀ r, r ∈ xs ↔ r ∈ ys) :
    Min.fold p none xs = Min.fold p none ys := by
  rw [Min.neg_max, Min.neg_max]
  exact congrArg (Option.map (- ·)) (Max.set_decides _ h)

end Loops

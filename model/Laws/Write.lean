import Loops.Batch
import Laws.Record

/-!
# Laws of writing

* **Checked.** `write` succeeds exactly when every check passes
  (`writeProblem_none`, in `Loops.Stored`), and what it returns carries the
  proof (`Written`). A batch writes through `write`, so every record it
  returns carries the same proof.
* **Shape.** Raw shape across envelopes, an unknown key and then an envelope
  that is not an object, depends only on which envelopes there are
  (`envelopesShapeProblem_perm`, `draftsShapeProblem_perm`). The empty text
  the reader puts in place of a field it can't read fails every field's check
  (`unreadable_refused`); `Laws.Read` carries it to the refusal.
* **Only a root leaves `of` out.** An envelope `write` accepts with `prev`
  states `of` (`Env.problem_of`).
* **`sets` holds key names.** Every key an accepted envelope declares is
  non-empty text (`Env.problem_sets`). `sets-not-keys` follows
  `erases-not-record-id` and precedes `of-missing`
  (`Env.problem_sets_not_keys`, `Env.problem_of_missing_sets`).
* **Closed codes.** Every refusal `writeProblem` gives is in `writeCodes`, so
  choosing among a batch's refusals by that list never loses one.
* **The set decides the refusal.** Which refusal a batch reports depends only
  on which refusals there are, not on the order of its drafts.
* **Member order.** A written payload's commitment doesn't depend on the
  order its members are listed in: `write` established the distinct names
  that `canon_member_order` needs.
-/

namespace Loops

open List

/-- Raw shape depends only on the set of envelope keys, not values or member order. -/
theorem envelopeKeysProblem_set {xs ys : List (String × Json)}
    (h : ∀ k, k ∈ xs.map Prod.fst ↔ k ∈ ys.map Prod.fst) :
    envelopeKeysProblem xs = envelopeKeysProblem ys := by
  have hall : xs.all (fun (k, _) => envelopeFields.contains k) =
      ys.all (fun (k, _) => envelopeFields.contains k) := by
    apply Bool.eq_iff_iff.mpr
    simp only [List.all_eq_true]
    constructor
    · intro hx (k, v) hm
      obtain ⟨⟨k', v'⟩, hm', he⟩ := List.mem_map.1 ((h k).2 (List.mem_map_of_mem hm))
      simp only at he
      subst k'
      exact hx (k, v') hm'
    · intro hy (k, v) hm
      obtain ⟨⟨k', v'⟩, hm', he⟩ := List.mem_map.1 ((h k).1 (List.mem_map_of_mem hm))
      simp only at he
      subst k'
      exact hy (k, v') hm'
  simp only [envelopeKeysProblem, hall]

/-- Raw shape across envelopes (`envelope-field-unknown`, then
`envelope-not-object`) depends only on which envelopes there are, not on their order. -/
theorem envelopesShapeProblem_perm {xs ys : List Json} (h : xs.Perm ys) :
    envelopesShapeProblem xs = envelopesShapeProblem ys := by
  have hk : envelopesKeysProblem xs = envelopesKeysProblem ys := by
    unfold envelopesKeysProblem
    apply envelopeKeysProblem_set
    intro k
    exact ((h.flatMap_right _).map Prod.fst).mem_iff
  have ho : xs.all Json.isObj = ys.all Json.isObj := by
    apply Bool.eq_iff_iff.mpr
    simp only [List.all_eq_true]
    exact ⟨fun hx j hj => hx j (h.mem_iff.2 hj), fun hy j hj => hy j (h.mem_iff.1 hj)⟩
  simp only [envelopesShapeProblem, hk, ho]

/-- A batch's raw shape does not depend on the order of its drafts. -/
theorem draftsShapeProblem_perm {xs ys : List Json} (h : xs.Perm ys) :
    draftsShapeProblem xs = draftsShapeProblem ys :=
  envelopesShapeProblem_perm (h.flatMap_right _)

/-- The empty text the reader puts in place of a field it can't read fails
every field's own check. That the reader puts it there, and that an envelope
holding it is refused, are `Laws.Read`'s; which code is reported, and the
order, are pinned by the vectors. -/
theorem unreadable_refused :
    unreadable.isEmpty = true ∧ typedRef unreadable = false ∧ validEmitted unreadable = false ∧
      occurredSpan? unreadable = none ∧ recordId unreadable = false ∧ keyName unreadable = false := by
  refine ⟨rfl, ?_, ?_, ?_, ?_, rfl⟩ <;> decide

/-- Only a root leaves `of` out: an envelope `write` accepts with `prev` states `of`. -/
theorem Env.problem_of {e : Env} (h : e.problem = none) (hp : e.prev ≠ []) : e.of.isSome = true := by
  cases ho : e.of with
  | some _ => rfl
  | none =>
    cases hpv : e.prev with
    | nil => exact absurd hpv hp
    | cons p ps =>
      unfold Env.problem at h
      simp only [ho, hpv] at h
      repeat (split at h <;> try simp_all)

/-- Every key an accepted envelope declares is a key name: non-empty text. -/
theorem Env.problem_sets {e : Env} (h : e.problem = none) : e.keysNamed = true := by
  cases hk : e.keysNamed
  · unfold Env.problem at h
    repeat (split at h <;> try simp_all)
  · rfl

/-- `sets-not-keys` follows `erases-not-record-id`: it speaks only when every
member of `erases` is a record id, and some declared key is not a key name. -/
theorem Env.problem_sets_not_keys {e : Env} (h : e.problem = some "sets-not-keys") :
    e.erases.all recordId = true ∧ e.keysNamed = false := by
  unfold Env.problem at h
  repeat' split at h
  all_goals simp only [reduceCtorEq, Option.some.injEq] at h
  all_goals (try simp at h)
  rename_i _ _ _ _ _ _ _ _ hx hk
  exact ⟨by simpa using hx, by simpa using hk⟩

/-- `of-missing` follows `sets-not-keys`: it speaks only when every declared
key is a key name. -/
theorem Env.problem_of_missing_sets {e : Env} (h : e.problem = some "of-missing") :
    e.keysNamed = true := by
  unfold Env.problem at h
  repeat' split at h
  all_goals simp only [reduceCtorEq, Option.some.injEq] at h
  all_goals (try simp at h)
  rename_i _ _ _ _ _ _ _ _ _ hk _
  simpa using hk

theorem Env.problem_codes {e : Env} {c : String} (h : e.problem = some c) : c ∈ writeCodes := by
  unfold Env.problem at h
  repeat' split at h
  all_goals cases h
  all_goals simp [writeCodes]

theorem writeProblem_codes {env : Env} {salt : List UInt8} {payload : Json} {c : String}
    (h : writeProblem env salt payload = some c) : c ∈ writeCodes := by
  unfold writeProblem at h
  split at h
  · rename_i he; cases h; exact Env.problem_codes he
  · repeat' split at h
    all_goals cases h
    all_goals simp [writeCodes]

/-! ## Overflow (D3)

A number spelling past binary64's range rounds to a nonfinite value. The
writer's number check refuses it by the same name as any other number out of
range, and in the same place: last, after the envelope, the salt and repeated
member names. -/

/-- A number in the writer's range is finite. -/
theorem Number.finite_of_inRange {n : Number} (h : n.inRange = true) : n.finite = true := by
  simp only [Number.inRange, Bool.and_eq_true] at h
  exact h.1

/-- A nonfinite number is refused by the number check, under a good envelope and salt. -/
theorem writeProblem_nonfinite {env : Env} {salt : List UInt8} {n : Number}
    (he : env.problem = none) (hs : salt.length = 16) (hn : n.finite = false) :
    writeProblem env salt (.number n) = some "number-out-of-range" := by
  have hr : (Json.number n).inRange = false := by
    cases h : n.inRange
    · simp [Json.inRange, h]
    · rw [Number.finite_of_inRange h] at hn; cases hn
  simp [writeProblem, he, hs, Json.keysDistinct, hr]

theorem Env.problem_ne_range {e : Env} : e.problem ≠ some "number-out-of-range" := by
  intro h
  unfold Env.problem at h
  repeat' split at h
  all_goals simp at h

/-- The number check is last: it speaks only when the envelope, the salt and the member names pass. -/
theorem writeProblem_range_last {env : Env} {salt : List UInt8} {payload : Json}
    (h : writeProblem env salt payload = some "number-out-of-range") :
    env.problem = none ∧ salt.length = 16 ∧ payload.keysDistinct = true ∧ payload.inRange = false := by
  unfold writeProblem at h
  split at h
  · rename_i why he
    cases h
    exact absurd he Env.problem_ne_range
  · rename_i he
    by_cases hs : salt.length = 16
    · by_cases hk : payload.keysDistinct = true
      · by_cases hr : payload.inRange = true
        · simp [hs, hk, hr] at h
        · exact ⟨he, hs, hk, by simpa using hr⟩
      · simp [hs, hk] at h
    · simp [hs] at h

theorem firstRefusal_set {cs ds : List String} (h : ∀ c, c ∈ cs ↔ c ∈ ds)
    (hw : ∀ c ∈ cs, c ∈ writeCodes) : firstRefusal cs = firstRefusal ds := by
  unfold firstRefusal
  have hp : cs.contains = ds.contains := by
    funext c
    cases hc : cs.contains c <;> cases hd : ds.contains c <;> simp_all
  rw [hp]
  cases hf : writeCodes.find? ds.contains with
  | some c => rfl
  | none =>
    have hcs : cs = [] := by
      cases cs with
      | nil => rfl
      | cons c cs' =>
        have hm := hw c mem_cons_self
        have := List.find?_eq_none.1 hf c hm
        simp at this
        exact absurd ((h c).1 mem_cons_self) this
    have hds : ds = [] := by
      cases ds with
      | nil => rfl
      | cons d ds' => exact absurd ((h d).2 mem_cons_self) (by simp [hcs])
    simp [hcs, hds]

theorem keysDistinct_obj {kvs : List (String × Json)} (h : (Json.obj kvs).keysDistinct = true) :
    (kvs.map (·.1)).Nodup := by
  simp only [Json.keysDistinct, Bool.and_eq_true, decide_eq_true_eq] at h
  exact h.1

/-- A written payload's commitment doesn't depend on its members' order. -/
theorem Written.commit_member_order (H : Hash) (w : Written) {kvs kvs' : List (String × Json)}
    (hw : w.payload = .obj kvs) (hp : kvs ~ kvs') :
    commitOf H w.salt (.obj kvs') = w.stored.commit H := by
  have hd := keysDistinct_obj (hw ▸ w.ok.members)
  simp only [Written.stored, make, Stored.commit, commitOf, hw, canon_member_order hp hd]

end Loops

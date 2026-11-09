import Laws.Things

/-!
# What a slice must hold

A reader holding everything settles, admits, then reads a thing. A reader
holding part of the set gets the same state for that thing when its part
holds three kinds of record (`Slice`):

* every form of the records about the thing;
* every record whose `erases` names one of them;
* every record one of them names in `prev`.

Settling a record reads its own forms and the `erases` of others. Admitting it
reads the things of its held `prev` targets. A thing's state reads only its
admitted records. Each step looks one hop out, so one hop is enough
(`stateIn_slice`). The second and third kinds matter only through their
envelopes: the proof never reads their payloads.

Neither addition can be dropped (`slice_needs_erasers`, `slice_needs_targets`).
-/

namespace Loops

open List

/-- What a part `S` of the held records `W` must hold to answer for thing `t`. -/
structure Slice (t : String) (S W : List Record) : Prop where
  sub : ∀ r ∈ S, r ∈ W
  members : ∀ r ∈ W, r.isAbout t = true → r ∈ S
  erasers : ∀ r ∈ W, r.isAbout t = true → ∀ e ∈ W, e.erases.contains r.id = true → e ∈ S
  targets : ∀ r ∈ W, r.isAbout t = true → ∀ q ∈ W, q.id ∈ r.prev → q ∈ S

theorem things_env {a b : Record} (h : SameEnvelope a b) : a.things = b.things := by
  simp [Record.things, h.about, h.id]

theorem isAbout_env {a b : Record} (h : SameEnvelope a b) (t : String) :
    a.isAbout t = b.isAbout t := by
  simp [Record.isAbout, things_env h]

theorem shares_env {a a' b b' : Record} (ha : SameEnvelope a a') (hb : SameEnvelope b b') :
    shares a b = shares a' b' := by
  simp [shares, things_env ha, things_env hb]

/-- The record kept for an id has the envelope of one of its forms. -/
theorem pick_form {xs : List Record} {i : RecordId} {r : Record} (h : pick xs i = some r) :
    ∃ a ∈ xs, a.id = i ∧ SameEnvelope r a := by
  unfold pick at h
  cases hb : baseForm (xs.filter (·.id == i)) with
  | none => simp [hb] at h
  | some b =>
    simp only [hb, Option.map_some, Option.some.injEq] at h
    obtain ⟨a, ha, he⟩ := baseForm_mem hb
    have hm := mem_filter.1 ha
    refine ⟨a, hm.1, by simpa using hm.2, ?_⟩
    subst h
    split
    · exact (erase_envelope b).trans he
    · exact he

theorem baseForm_some {forms : List Record} {a : Record} (ha : a ∈ forms) :
    ∃ b, baseForm forms = some b := by
  unfold baseForm
  split
  · exact ⟨_, rfl⟩
  · cases forms with
    | nil => simp at ha
    | cons c s => exact ⟨_, rfl⟩

/-- A held form's id keeps a record. -/
theorem pick_some {xs : List Record} {a : Record} (ha : a ∈ xs) : ∃ r, pick xs a.id = some r := by
  obtain ⟨b, hb⟩ := baseForm_some (mem_filter.2 ⟨ha, by simp⟩ : a ∈ xs.filter (·.id == a.id))
  exact ⟨if erasedIn xs a.id then b.erase else b, by simp only [pick, hb, Option.map_some]⟩

/-- A slice keeps, for each record about the thing, what the whole keeps. -/
theorem pick_slice {t : String} {S W : List Record} (hF : Forms W) (h : Slice t S W)
    {a : Record} (ha : a ∈ W) (hat : a.isAbout t = true) : pick S a.id = pick W a.id := by
  unfold pick
  have hforms : ∀ r, r ∈ S.filter (·.id == a.id) ↔ r ∈ W.filter (·.id == a.id) := fun r => by
    simp only [mem_filter]
    constructor
    · exact fun ⟨hr, e⟩ => ⟨h.sub r hr, e⟩
    · rintro ⟨hr, e⟩
      have e' : r.id = a.id := by simpa using e
      exact ⟨h.members r hr (by rw [isAbout_env (hF r hr a ha e').1]; exact hat), e⟩
  have hFS : Forms (S.filter (·.id == a.id)) := fun x hx y hy e =>
    hF x (h.sub x (mem_filter.1 hx).1) y (h.sub y (mem_filter.1 hy).1) e
  have he : erasedIn S a.id = erasedIn W a.id := by
    unfold erasedIn
    apply Bool.eq_iff_iff.2
    simp only [any_eq_true]
    exact ⟨fun ⟨e, he, p⟩ => ⟨e, h.sub e he, p⟩, fun ⟨e, he, p⟩ => ⟨e, h.erasers a ha hat e he p, p⟩⟩
  rw [baseForm_eq hFS (fun x hx => by simpa using (mem_filter.1 hx).2) hforms, he]

/-- The records about the thing settle the same in the slice and in the whole. -/
theorem settle_slice {t : String} {S W : List Record} (hF : Forms W) (h : Slice t S W)
    {r : Record} (ht : r.isAbout t = true) : r ∈ settle S ↔ r ∈ settle W := by
  constructor
  · intro hr
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    obtain ⟨a, ha, hai, hra⟩ := pick_form hp
    have hat : a.isAbout t = true := by rw [← isAbout_env hra]; exact ht
    have haW := h.sub a ha
    subst hai
    exact mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, haW, rfl⟩, by rw [← pick_slice hF h haW hat]; exact hp⟩
  · intro hr
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    obtain ⟨a, haW, hai, hra⟩ := pick_form hp
    have hat : a.isAbout t = true := by rw [← isAbout_env hra]; exact ht
    subst hai
    exact mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, h.members a haW hat, rfl⟩,
      by rw [pick_slice hF h haW hat]; exact hp⟩

/-- Two records kept for one id, in the slice and in the whole, share an envelope. -/
theorem settle_slice_env {t : String} {S W : List Record} (hF : Forms W) (h : Slice t S W)
    {q q' : Record} (hq : q ∈ settle W) (hq' : q' ∈ settle S) (e : q.id = q'.id) :
    SameEnvelope q q' := by
  obtain ⟨j, -, hj⟩ := mem_filterMap.1 hq
  obtain ⟨b, hb, -, hqb⟩ := pick_form hj
  obtain ⟨j', -, hj'⟩ := mem_filterMap.1 hq'
  obtain ⟨b', hb', -, hqb'⟩ := pick_form hj'
  have hbb : b.id = b'.id := by rw [← hqb.id, ← hqb'.id]; exact e
  exact hqb.trans ((hF b hb b' (h.sub b' hb') hbb).1.trans hqb'.symm)

/-- A record about the thing is entitled in the slice exactly when it is in the whole. -/
theorem entitled_slice {may : Record → Record → Bool} (hm : EnvOnly may) {t : String}
    {S W : List Record} (hF : Forms W) (h : Slice t S W) {r : Record} (hr : r ∈ settle W)
    (ht : r.isAbout t = true) : entitled may (settle S) r = entitled may (settle W) r := by
  obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
  obtain ⟨a, haW, -, hra⟩ := pick_form hp
  have hat : a.isAbout t = true := by rw [← isAbout_env hra]; exact ht
  unfold entitled
  apply Bool.eq_iff_iff.2
  simp only [all_eq_true]
  constructor
  · intro H p hp q hq
    by_cases e : q.id = p
    · obtain ⟨j, -, hj⟩ := mem_filterMap.1 hq
      obtain ⟨b, hb, -, hqb⟩ := pick_form hj
      have hbS : b ∈ S := h.targets a haW hat b hb (by rw [← hqb.id, e, ← hra.prev]; exact hp)
      obtain ⟨q', hq'⟩ := pick_some hbS
      have hq'S : q' ∈ settle S := mem_filterMap.2 ⟨b.id, mem_idsOf.2 ⟨b, hbS, rfl⟩, hq'⟩
      have hid : q.id = q'.id := by rw [pick_id hq', hqb.id]
      have se := settle_slice_env hF h hq hq'S hid
      have := H p hp q' hq'S
      rw [hid, shares_env (SameEnvelope.refl r) se, hm r r q q' (SameEnvelope.refl r) se]
      exact this
    · simp [e]
  · intro H p hp q' hq'
    by_cases e : q'.id = p
    · obtain ⟨j, -, hj⟩ := mem_filterMap.1 hq'
      obtain ⟨b, hb, -, hqb⟩ := pick_form hj
      have hbW := h.sub b hb
      obtain ⟨q, hq⟩ := pick_some hbW
      have hqW : q ∈ settle W := mem_filterMap.2 ⟨b.id, mem_idsOf.2 ⟨b, hbW, rfl⟩, hq⟩
      have hid : q.id = q'.id := by rw [pick_id hq, hqb.id]
      have se := settle_slice_env hF h hqW hq' hid
      have := H p hp q hqW
      rw [← hid, ← shares_env (SameEnvelope.refl r) se, ← hm r r q q' (SameEnvelope.refl r) se]
      exact this
    · simp [e]

/-- **A slice answers for its thing.** The state of `t` over a slice is its
state over everything held. -/
theorem stateIn_slice {may : Record → Record → Bool} (hm : EnvOnly may) {t : String}
    {S W : List Record} (hF : Forms W) (h : Slice t S W) :
    stateIn t (realm may S) = stateIn t (realm may W) := by
  unfold realm
  apply stateIn_local (admit_distinct may (settle_distinct S)).addressed
    (admit_distinct may (settle_distinct W)).addressed
  intro r
  constructor
  · rintro ⟨hr, ht⟩
    obtain ⟨hs, he⟩ := mem_admit.1 hr
    have hw := (settle_slice hF h ht).1 hs
    exact ⟨mem_admit.2 ⟨hw, by rw [← entitled_slice hm hF h hw ht]; exact he⟩, ht⟩
  · rintro ⟨hr, ht⟩
    obtain ⟨hw, he⟩ := mem_admit.1 hr
    exact ⟨mem_admit.2 ⟨(settle_slice hF h ht).2 hw, by rw [entitled_slice hm hF h hw ht]; exact he⟩, ht⟩

/-! ## Neither addition can be dropped

`sl.x` is about `k1`. `sl.e`, about `k2`, erases it. `sl.s`, about `k1`, names
`sl.p` in `prev`, and `sl.p` is about `k2`. A part holding only the records
about `k1` reads `x` as here and admits `s`. A reader holding everything reads
`x` as erased and refuses `s`. -/

def sl.x : Record :=
  { id := 1, kind := "task", observer := "sl:x", about := ["k1"], value := "v", prev := [], emitted := default }
def sl.e : Record :=
  { id := 2, kind := "note", observer := "sl:x", about := ["k2"], value := "", prev := [], emitted := default,
    erases := [1] }
def sl.s : Record :=
  { id := 4, kind := "task", observer := "sl:x", about := ["k1"], value := "", prev := [3], emitted := default }
def sl.p : Record :=
  { id := 3, kind := "task", observer := "sl:x", about := ["k2"], value := "", prev := [], emitted := default }

/-- Without the eraser, the part reads a payload the whole has erased. -/
theorem slice_needs_erasers :
    stateIn "k1" (realm (fun _ _ => true) [sl.x]) ≠ stateIn "k1" (realm (fun _ _ => true) [sl.x, sl.e]) := by
  decide

/-- Without the `prev` target, the part admits a record the whole refuses. -/
theorem slice_needs_targets :
    stateIn "k1" (realm (fun _ _ => true) [sl.s]) ≠ stateIn "k1" (realm (fun _ _ => true) [sl.s, sl.p]) := by
  decide

end Loops

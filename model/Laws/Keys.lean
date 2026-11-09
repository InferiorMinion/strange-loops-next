import Loops.Keys

/-!
# Laws for who speaks

* **`speaks.grows`**: holding more never silences a key (`speaks_grows`,
  `attributed_grows`). Nothing yet retires one.
* **`speaks.set`**: the set decides (`speaks_set`).
* **`speaks.view`**: a view changes no answer (`speaks_view`): the rule reads
  envelopes and signing keys only.
* **`speaks.anchored`**: no anchor, no speaker (`speaks_anchored`), and a key
  speaks only as a root or because a speaking key bound it (`speaks_cases`,
  `speaks_needs_binding`). A stranger's binding binds nothing.
* **`speaks.computed`**: every key the rounds find speaks (`speakers_sound`).
  That the rounds find every key is not proved here.
-/

namespace Loops

open List

theorem speaks_grows {xs ys : List Signed} {L k : String} (h : ∀ s ∈ xs, s ∈ ys)
    (hk : Speaks xs L k) : Speaks ys L k := by
  induction hk with
  | root ha hr => exact .root (h _ ha) hr
  | bound hb hbind _ ih => exact .bound (h _ hb) hbind ih

theorem speaks_set {xs ys : List Signed} (h : ∀ s, s ∈ xs ↔ s ∈ ys) (L k : String) :
    Speaks xs L k ↔ Speaks ys L k :=
  ⟨speaks_grows fun s hs => (h s).1 hs, speaks_grows fun s hs => (h s).2 hs⟩

theorem attributed_grows {xs ys : List Signed} {s : Signed} (h : ∀ s ∈ xs, s ∈ ys)
    (hs : Attributed xs s) : Attributed ys s :=
  hs.imp id (speaks_grows h)

/-- A reader's view of a signed record: the same key, the record through the view. -/
def Signed.through (f : Record → Record) (s : Signed) : Signed := { s with record := f s.record }

theorem anchors_view {f : Record → Record} (hf : View f) (s : Signed) (L : String) :
    (s.through f).anchors L = s.anchors L := by
  have e := hf.envelope s.record
  simp [Signed.anchors, Signed.through, e.kind, e.observer, e.id]

theorem binds_view {f : Record → Record} (hf : View f) (s : Signed) (L k : String) :
    (s.through f).binds L k = s.binds L k := by
  have e := hf.envelope s.record
  simp [Signed.binds, Signed.through, e.kind, e.observer, e.about]

theorem speaks_view {f : Record → Record} (hf : View f) (xs : List Signed) (L k : String) :
    Speaks (xs.map (Signed.through f)) L k ↔ Speaks xs L k := by
  constructor
  · intro h
    induction h with
    | root ha hr =>
      obtain ⟨s, hs, rfl⟩ := mem_map.1 ha
      rw [anchors_view hf] at hr
      exact Speaks.root (a := s) hs hr
    | bound hb hbind _ ih =>
      obtain ⟨s, hs, rfl⟩ := mem_map.1 hb
      rw [binds_view hf] at hbind
      exact Speaks.bound (b := s) hs hbind ih
  · intro h
    induction h with
    | @root a ha hr =>
      exact .root (a := a.through f) (mem_map.2 ⟨a, ha, rfl⟩) (by rw [anchors_view hf]; exact hr)
    | @bound b k hb hbind _ ih =>
      exact .bound (b := b.through f) (mem_map.2 ⟨b, hb, rfl⟩) (by rw [binds_view hf]; exact hbind) ih

/-- No anchor, no speaker. -/
theorem speaks_anchored {xs : List Signed} {L k : String} (h : Speaks xs L k) :
    ∃ a ∈ xs, a.anchors L = true := by
  induction h with
  | root ha hr => exact ⟨_, ha, hr⟩
  | bound _ _ _ ih => exact ih

/-- A key speaks only as a root, or because a key that speaks bound it. -/
theorem speaks_cases {xs : List Signed} {L k : String} (h : Speaks xs L k) :
    (∃ a ∈ xs, a.anchors L = true ∧ a.key = k) ∨
      (∃ b ∈ xs, b.binds L k = true ∧ Speaks xs L b.key) := by
  cases h with
  | root ha hr => exact .inl ⟨_, ha, hr, rfl⟩
  | bound hb hbind hs => exact .inr ⟨_, hb, hbind, hs⟩

/-- A binding counts only if a speaking key signed it: a key that is no root,
and that no speaking key's binding names, does not speak. -/
theorem speaks_needs_binding {xs : List Signed} {L k : String}
    (hroot : ∀ a ∈ xs, a.anchors L = true → a.key ≠ k)
    (hbind : ∀ b ∈ xs, b.binds L k = true → ¬ Speaks xs L b.key) : ¬ Speaks xs L k := by
  intro h
  rcases speaks_cases h with ⟨a, ha, hr, e⟩ | ⟨b, hb, hbd, hs⟩
  · exact hroot a ha hr e
  · exact hbind b hb hbd hs

theorem speakersIn_sound {xs : List Signed} {L : String} (n : Nat) {k : String}
    (h : k ∈ speakersIn xs L n) : Speaks xs L k := by
  induction n generalizing k with
  | zero =>
    obtain ⟨a, ha, rfl⟩ := mem_map.1 h
    exact .root (mem_filter.1 ha).1 (mem_filter.1 ha).2
  | succ n ih =>
    rcases mem_append.1 h with h | h
    · exact ih h
    · obtain ⟨b, hb, hk⟩ := mem_flatMap.1 h
      obtain ⟨hbx, hp⟩ := mem_filter.1 hb
      obtain ⟨hka, hkr⟩ := mem_filter.1 hk
      simp only [Bool.and_eq_true, beq_iff_eq] at hp
      refine .bound hbx ?_ (ih (by simpa using hp.2))
      simp [Signed.binds, hp.1.1, hp.1.2, hkr, hka]

theorem speakers_sound {xs : List Signed} {L k : String} (h : k ∈ speakers xs L) : Speaks xs L k :=
  speakersIn_sound _ h

end Loops

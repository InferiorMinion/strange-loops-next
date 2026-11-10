import Loops.Keys

/-!
# Laws for who speaks

* **`speaks.grows`**: holding more never silences a key (`speaks_grows`,
  `attributed_grows`). Nothing yet retires one.
* **`speaks.set`**: the set decides (`speaks_set`).
* **`speaks.view`**: a view changes no answer (`speaks_view`): the rule reads
  envelopes and signing keys only.
* **`speaks.anchored`**: no anchor, no speaker (`speaks_anchored`).
* **`speaks.two-tier`**: only a root's binding counts (`speaks_needs_binding`),
  so a record signed by a key that is no root changes no answer
  (`bound_binds_nothing`).
* **`speaks.computed`**: the computed list is exactly the keys that speak
  (`roots_iff`, `speakers_iff`).
-/

namespace Loops

open List

theorem root_grows {xs ys : List Signed} {L k : String} (h : ∀ s ∈ xs, s ∈ ys) :
    Root xs L k → Root ys L k
  | ⟨a, ha, hr⟩ => ⟨a, h a ha, hr⟩

theorem speaks_grows {xs ys : List Signed} {L k : String} (h : ∀ s ∈ xs, s ∈ ys) :
    Speaks xs L k → Speaks ys L k
  | .inl hr => .inl (root_grows h hr)
  | .inr ⟨b, hb, hbind, hr⟩ => .inr ⟨b, h b hb, hbind, root_grows h hr⟩

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

theorem root_view {f : Record → Record} (hf : View f) (xs : List Signed) (L k : String) :
    Root (xs.map (Signed.through f)) L k ↔ Root xs L k := by
  constructor
  · rintro ⟨a, ha, hr, hk⟩
    obtain ⟨s, hs, rfl⟩ := mem_map.1 ha
    rw [anchors_view hf] at hr
    exact ⟨s, hs, hr, hk⟩
  · rintro ⟨a, ha, hr, hk⟩
    exact ⟨a.through f, mem_map.2 ⟨a, ha, rfl⟩, by rw [anchors_view hf]; exact hr, hk⟩

theorem speaks_view {f : Record → Record} (hf : View f) (xs : List Signed) (L k : String) :
    Speaks (xs.map (Signed.through f)) L k ↔ Speaks xs L k := by
  constructor
  · rintro (hr | ⟨b, hb, hbind, hr⟩)
    · exact .inl ((root_view hf xs L k).1 hr)
    · obtain ⟨s, hs, rfl⟩ := mem_map.1 hb
      rw [binds_view hf] at hbind
      exact .inr ⟨s, hs, hbind, (root_view hf xs L _).1 hr⟩
  · rintro (hr | ⟨b, hb, hbind, hr⟩)
    · exact .inl ((root_view hf xs L k).2 hr)
    · exact .inr ⟨b.through f, mem_map.2 ⟨b, hb, rfl⟩, by rw [binds_view hf]; exact hbind,
        (root_view hf xs L _).2 hr⟩

/-- No anchor, no speaker. -/
theorem speaks_anchored {xs : List Signed} {L k : String} (h : Speaks xs L k) :
    ∃ a ∈ xs, a.anchors L = true := by
  rcases h with ⟨a, ha, hr, _⟩ | ⟨_, _, _, a, ha, hr, _⟩ <;> exact ⟨a, ha, hr⟩

/-- Only a root's binding counts: a key that is no root, and that no root's
binding names, does not speak. -/
theorem speaks_needs_binding {xs : List Signed} {L k : String} (hroot : ¬ Root xs L k)
    (hbind : ∀ b ∈ xs, b.binds L k = true → ¬ Root xs L b.key) : ¬ Speaks xs L k := by
  rintro (hr | ⟨b, hb, hbd, hr⟩)
  · exact hroot hr
  · exact hbind b hb hbd hr

/-- A bound key binds nothing: a record signed by a key that is no root
changes no answer, whatever it says. -/
theorem bound_binds_nothing {xs : List Signed} {L : String} {s : Signed}
    (hs : ¬ Root (s :: xs) L s.key) (k : String) : Speaks (s :: xs) L k ↔ Speaks xs L k := by
  have root {k'} (h : Root (s :: xs) L k') : Root xs L k' := by
    obtain ⟨a, ha, hr, hk⟩ := h
    rcases mem_cons.1 ha with rfl | ha
    · exact absurd ⟨a, mem_cons_self, hr, rfl⟩ hs
    · exact ⟨a, ha, hr, hk⟩
  refine ⟨?_, speaks_grows fun _ h => mem_cons_of_mem _ h⟩
  rintro (hr | ⟨b, hb, hbind, hr⟩)
  · exact .inl (root hr)
  · rcases mem_cons.1 hb with rfl | hb
    · exact absurd hr hs
    · exact .inr ⟨b, hb, hbind, root hr⟩

theorem roots_iff {xs : List Signed} {L k : String} : k ∈ roots xs L ↔ Root xs L k := by
  constructor
  · intro h
    obtain ⟨a, ha, rfl⟩ := mem_map.1 h
    exact ⟨a, (mem_filter.1 ha).1, (mem_filter.1 ha).2, rfl⟩
  · rintro ⟨a, ha, hr, rfl⟩
    exact mem_map.2 ⟨a, mem_filter.2 ⟨ha, hr⟩, rfl⟩

/-- The computed list is exactly the keys that speak. -/
theorem speakers_iff {xs : List Signed} {L k : String} : k ∈ speakers xs L ↔ Speaks xs L k := by
  constructor
  · intro h
    rcases mem_append.1 h with h | h
    · exact .inl (roots_iff.1 h)
    · obtain ⟨b, hb, hk⟩ := mem_flatMap.1 h
      obtain ⟨hbx, hp⟩ := mem_filter.1 hb
      obtain ⟨hka, hkr⟩ := mem_filter.1 hk
      simp only [Bool.and_eq_true, beq_iff_eq] at hp
      refine .inr ⟨b, hbx, ?_, roots_iff.1 (by simpa using hp.2)⟩
      simp [Signed.binds, hp.1.1, hp.1.2, hkr, hka]
  · rintro (hr | ⟨b, hb, hbind, hr⟩)
    · exact mem_append.2 (.inl (roots_iff.2 hr))
    · simp only [Signed.binds, Bool.and_eq_true, beq_iff_eq] at hbind
      refine mem_append.2 (.inr (mem_flatMap.2 ⟨b, mem_filter.2 ⟨hb, ?_⟩, mem_filter.2 ⟨?_, hbind.1.2⟩⟩))
      · simpa [hbind.1.1.1, hbind.1.1.2] using roots_iff.2 hr
      · simpa using hbind.2

end Loops

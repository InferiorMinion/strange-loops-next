import Laws.Slice
import Laws.Spec

/-!
# What a spec's slice must hold

`stateIn_slice` says what a part must hold to answer for one thing. A spec
reads across things: `from` takes records by kind, by mention and by thing,
and its slots count them or show the current ones. A reader holding part of
the set gets the spec's whole answer when its part holds five kinds of record
(`SpecSlice`):

* every form of the records the spec takes;
* every record whose `erases` names one of them;
* every record one of them names in `prev`;
* every record that names one of them in `prev` (a successor);
* every record a successor names in `prev`.

The first three are `Slice`'s, with "taken by the spec" in place of "of the
thing". The last two are new. A taken record is current only when no admitted
record of its thing replaces it, and the replacer may be of a kind, or carry a
mention, that the spec does not take. A successor replaces only when it is
admitted, and admission reads the thing of each of its held `prev` targets.
Admission reads what is held and not what is admitted, so nothing further
out matters (`Spec.run_slice`).

Only the taken records are read whole. The proof reads the envelopes of the
other four kinds and never their payloads.

Neither new kind can be dropped (`spec_slice_needs_successors`,
`spec_slice_needs_their_targets`). The first three cannot either, as
`slice_needs_erasers` and `slice_needs_targets` show for a spec that takes
one thing.
-/

namespace Loops

open List

/-- What a part `S` of the held records `W` must hold to answer for spec `sp`. -/
structure SpecSlice (sp : Spec) (S W : List Record) : Prop where
  sub : ∀ r ∈ S, r ∈ W
  taken : ∀ r ∈ W, sp.takes r = true → r ∈ S
  erasers : ∀ r ∈ W, sp.takes r = true → ∀ e ∈ W, e.erases.contains r.id = true → e ∈ S
  targets : ∀ r ∈ W, sp.takes r = true → ∀ q ∈ W, q.id ∈ r.prev → q ∈ S
  successors : ∀ r ∈ W, sp.takes r = true → ∀ s ∈ W, r.id ∈ s.prev → s ∈ S
  theirs : ∀ r ∈ W, sp.takes r = true → ∀ s ∈ W, r.id ∈ s.prev → ∀ q ∈ W, q.id ∈ s.prev → q ∈ S

/-! ## What settling and admission read of a part -/

/-- Whether a spec takes a record is a fact of its envelope. -/
theorem Spec.takes_env (sp : Spec) {a b : Record} (h : SameEnvelope a b) :
    sp.takes a = sp.takes b := by
  have e : a.isOf = b.isOf := funext (isOf_env h)
  simp only [Spec.takes, Source.takes, h.kind, h.about, e]

/-- A settled record has the envelope of a held form. -/
theorem settle_form {xs : List Record} {r : Record} (hr : r ∈ settle xs) :
    ∃ a ∈ xs, SameEnvelope r a := by
  obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
  obtain ⟨a, ha, -, hra⟩ := pick_form hp
  exact ⟨a, ha, hra⟩

/-- A held form settles to a record with its envelope. -/
theorem settle_has {xs : List Record} (hF : Forms xs) {a : Record} (ha : a ∈ xs) :
    ∃ r ∈ settle xs, SameEnvelope r a := by
  obtain ⟨r, hp⟩ := pick_some ha
  obtain ⟨b, hb, hbi, hrb⟩ := pick_form hp
  exact ⟨r, mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, ha, rfl⟩, hp⟩, hrb.trans (hF b hb a ha hbi).1⟩

theorem Forms.sub {S W : List Record} (hF : Forms W) (sub : ∀ r ∈ S, r ∈ W) : Forms S :=
  fun a ha b hb e => hF a (sub a ha) b (sub b hb) e

/-- A record whose held `prev` targets are all in the part is entitled in the
part exactly when it is in the whole. Only envelopes are read. -/
theorem entitled_part {may : Record → Record → Bool} (hm : EnvOnly may) {S W : List Record}
    (hF : Forms W) (sub : ∀ r ∈ S, r ∈ W) {r r' : Record} (he : SameEnvelope r r')
    (ht : ∀ q ∈ W, q.id ∈ r.prev → q ∈ S) :
    entitled may (settle S) r = entitled may (settle W) r' := by
  unfold entitled
  apply Bool.eq_iff_iff.2
  simp only [all_eq_true]
  constructor
  · intro H p hp q hq
    by_cases e : q.id = p
    · obtain ⟨b, hb, hqb⟩ := settle_form hq
      have hbS : b ∈ S := ht b hb (by rw [← hqb.id, e, he.prev]; exact hp)
      obtain ⟨q', hq', hq'b⟩ := settle_has (hF.sub sub) hbS
      have se : SameEnvelope q q' := hqb.trans hq'b.symm
      have := H p (by rw [he.prev]; exact hp) q' hq'
      rw [se.id, shares_env he.symm se, hm r' r q q' he.symm se]
      exact this
    · simp [e]
  · intro H p hp q' hq'
    by_cases e : q'.id = p
    · obtain ⟨b, hb, hq'b⟩ := settle_form hq'
      obtain ⟨q, hq, hqb⟩ := settle_has hF (sub b hb)
      have se : SameEnvelope q' q := hq'b.trans hqb.symm
      have := H p (by rw [← he.prev]; exact hp) q hq
      rw [se.id, shares_env he se, hm r r' q' q he se]
      exact this
    · simp [e]

/-- A form in the part with its held `prev` targets is admitted in the part
exactly when it is in the whole. -/
theorem admit_part {may : Record → Record → Bool} (hm : EnvOnly may) {S W : List Record}
    (hF : Forms W) (sub : ∀ r ∈ S, r ∈ W) {a : Record} (ha : a ∈ S)
    (ht : ∀ q ∈ W, q.id ∈ a.prev → q ∈ S) :
    (∃ r ∈ realm may S, SameEnvelope r a) ↔ (∃ r ∈ realm may W, SameEnvelope r a) := by
  constructor
  · rintro ⟨r, hr, hra⟩
    obtain ⟨hs, he⟩ := mem_admit.1 hr
    obtain ⟨r', hr', hr'a⟩ := settle_has hF (sub a ha)
    refine ⟨r', mem_admit.2 ⟨hr', ?_⟩, hr'a⟩
    rw [← entitled_part hm hF sub (hra.trans hr'a.symm) (by rw [hra.prev]; exact ht)]
    exact he
  · rintro ⟨r', hr', hr'a⟩
    obtain ⟨hs, he⟩ := mem_admit.1 hr'
    obtain ⟨r, hr, hra⟩ := settle_has (hF.sub sub) ha
    refine ⟨r, mem_admit.2 ⟨hr, ?_⟩, hra⟩
    rw [entitled_part hm hF sub (hra.trans hr'a.symm) (by rw [hra.prev]; exact ht)]
    exact he

/-! ## The taken records -/

section Taken

variable {sp : Spec} {S W : List Record}

/-- A spec slice keeps, for each taken record, what the whole keeps. -/
theorem pick_taken (hF : Forms W) (h : SpecSlice sp S W) {a : Record} (ha : a ∈ W)
    (hat : sp.takes a = true) : pick S a.id = pick W a.id := by
  unfold pick
  have hforms : ∀ r, r ∈ S.filter (·.id == a.id) ↔ r ∈ W.filter (·.id == a.id) := fun r => by
    simp only [mem_filter]
    constructor
    · exact fun ⟨hr, e⟩ => ⟨h.sub r hr, e⟩
    · rintro ⟨hr, e⟩
      have e' : r.id = a.id := by simpa using e
      exact ⟨h.taken r hr (by rw [sp.takes_env (hF r hr a ha e').1]; exact hat), e⟩
  have hFS : Forms (S.filter (·.id == a.id)) := fun x hx y hy e =>
    hF x (h.sub x (mem_filter.1 hx).1) y (h.sub y (mem_filter.1 hy).1) e
  have he : erasedIn S a.id = erasedIn W a.id := by
    unfold erasedIn
    apply Bool.eq_iff_iff.2
    simp only [any_eq_true]
    exact ⟨fun ⟨e, he, p⟩ => ⟨e, h.sub e he, p⟩, fun ⟨e, he, p⟩ => ⟨e, h.erasers a ha hat e he p, p⟩⟩
  rw [baseForm_eq hFS (fun x hx => by simpa using (mem_filter.1 hx).2) hforms, he]

/-- The taken records settle the same in the slice and in the whole. -/
theorem settle_taken (hF : Forms W) (h : SpecSlice sp S W) {r : Record}
    (ht : sp.takes r = true) : r ∈ settle S ↔ r ∈ settle W := by
  constructor
  · intro hr
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    obtain ⟨a, ha, hai, hra⟩ := pick_form hp
    have hat : sp.takes a = true := by rw [← sp.takes_env hra]; exact ht
    have haW := h.sub a ha
    subst hai
    exact mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, haW, rfl⟩, by rw [← pick_taken hF h haW hat]; exact hp⟩
  · intro hr
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    obtain ⟨a, haW, hai, hra⟩ := pick_form hp
    have hat : sp.takes a = true := by rw [← sp.takes_env hra]; exact ht
    subst hai
    exact mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, h.taken a haW hat, rfl⟩,
      by rw [pick_taken hF h haW hat]; exact hp⟩

variable {may : Record → Record → Bool}

/-- A taken record is admitted in the slice exactly when it is in the whole. -/
theorem realm_taken (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) {r : Record}
    (ht : sp.takes r = true) : r ∈ realm may S ↔ r ∈ realm may W := by
  have key : ∀ {X : List Record}, r ∈ settle X → (∀ x ∈ X, x ∈ W) →
      entitled may (settle S) r = entitled may (settle W) r := by
    intro X hr hX
    obtain ⟨a, ha, hra⟩ := settle_form hr
    have hat : sp.takes a = true := by rw [← sp.takes_env hra]; exact ht
    exact entitled_part hm hF h.sub (SameEnvelope.refl r)
      (fun q hq hp => h.targets a (hX a ha) hat q hq (by rw [← hra.prev]; exact hp))
  unfold realm
  constructor
  · intro hr
    obtain ⟨hs, he⟩ := mem_admit.1 hr
    exact mem_admit.2 ⟨(settle_taken hF h ht).1 hs, by rw [← key hs h.sub]; exact he⟩
  · intro hr
    obtain ⟨hw, he⟩ := mem_admit.1 hr
    exact mem_admit.2 ⟨(settle_taken hF h ht).2 hw, by rw [key hw fun _ hx => hx]; exact he⟩

/-- A taken record is replaced in the slice exactly when it is in the whole:
its successors and their `prev` targets are in the slice, so each successor
is admitted in both or in neither. -/
theorem replaced_taken (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) {r : Record}
    (hr : r ∈ realm may W) (ht : sp.takes r = true) (t : String) :
    (∃ q ∈ realm may S, q.isOf t = true ∧ r.id ∈ q.prev) ↔
      (∃ q ∈ realm may W, q.isOf t = true ∧ r.id ∈ q.prev) := by
  obtain ⟨a, ha, hra⟩ := settle_form (mem_admit.1 hr).1
  have hat : sp.takes a = true := by rw [← sp.takes_env hra]; exact ht
  constructor
  · rintro ⟨q, hq, hqt, hp⟩
    obtain ⟨b, hb, hqb⟩ := settle_form (mem_admit.1 hq).1
    have hab : a.id ∈ b.prev := by rw [← hra.id, ← hqb.prev]; exact hp
    obtain ⟨q', hq', hq'b⟩ := (admit_part hm hF h.sub hb
      (h.theirs a ha hat b (h.sub b hb) hab)).1 ⟨q, hq, hqb⟩
    exact ⟨q', hq', by rw [isOf_env (hq'b.trans hqb.symm)]; exact hqt,
      by rw [hq'b.prev, ← hqb.prev]; exact hp⟩
  · rintro ⟨q', hq', hqt, hp⟩
    obtain ⟨b, hb, hq'b⟩ := settle_form (mem_admit.1 hq').1
    have hab : a.id ∈ b.prev := by rw [← hra.id, ← hq'b.prev]; exact hp
    have hbS : b ∈ S := h.successors a ha hat b hb hab
    obtain ⟨q, hq, hqb⟩ := (admit_part hm hF h.sub hbS
      (h.theirs a ha hat b hb hab)).2 ⟨q', hq', hq'b⟩
    exact ⟨q, hq, by rw [isOf_env (hqb.trans hq'b.symm)]; exact hqt,
      by rw [hqb.prev, ← hq'b.prev]; exact hp⟩

/-- A head of a thing: admitted, of the thing, and named in no `prev` of an
admitted record of it. -/
theorem mem_stateIn_heads {xs : List Record} (hx : Addressed xs) {t : String} {r : Record} :
    r ∈ (stateIn t xs).heads ↔
      r ∈ xs ∧ r.isOf t = true ∧ ¬ ∃ q ∈ xs, q.isOf t = true ∧ r.id ∈ q.prev := by
  unfold stateIn
  rw [(LatestBy.inv_of (hx.filter (·.isOf t))).heads_mem r]
  simp only [mem_filter]
  constructor
  · rintro ⟨⟨hr, ht⟩, hn⟩
    exact ⟨hr, ht, fun ⟨q, hq, hqt, hp⟩ => hn ⟨q, ⟨hq, hqt⟩, hp⟩⟩
  · rintro ⟨hr, ht, hn⟩
    exact ⟨⟨hr, ht⟩, fun ⟨q, ⟨hq, hqt⟩, hp⟩ => hn ⟨q, hq, hqt, hp⟩⟩

theorem realm_addressed (may : Record → Record → Bool) (xs : List Record) :
    Addressed (realm may xs) :=
  (admit_distinct may (settle_distinct xs)).addressed

/-- **A taken record is current in the slice exactly when it is in the whole.** -/
theorem heads_taken (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) (t : String)
    {r : Record} (ht : sp.takes r = true) :
    r ∈ (stateIn t (realm may S)).heads ↔ r ∈ (stateIn t (realm may W)).heads := by
  rw [mem_stateIn_heads (realm_addressed may S), mem_stateIn_heads (realm_addressed may W)]
  constructor
  · rintro ⟨hr, hrt, hn⟩
    have hw := (realm_taken hm hF h ht).1 hr
    exact ⟨hw, hrt, fun hq => hn ((replaced_taken hm hF h hw ht t).2 hq)⟩
  · rintro ⟨hw, hrt, hn⟩
    exact ⟨(realm_taken hm hF h ht).2 hw, hrt, fun hq => hn ((replaced_taken hm hF h hw ht t).1 hq)⟩

end Taken

/-! ## Every slot -/

/-- Two newest-first lists with the same records are the same list. -/
theorem newest_ext {xs ys : List Record} (hx : xs.Pairwise Newer) (hy : ys.Pairwise Newer)
    (h : ∀ r, r ∈ xs ↔ r ∈ ys) : xs = ys := by
  have ne : ∀ {a b : Record}, Newer a b → a ≠ b := fun h e => by subst e; exact Newer.irrefl _ h
  apply Perm.eq_of_pairwise (le := Newer) (fun a b _ _ h₁ h₂ => absurd h₂ h₁.asymm) hx hy
  exact (perm_ext_iff_of_nodup (hx.imp ne) (hy.imp ne)).2 h

/-- Two parts of canonical sets with the same members are the same list. -/
theorem canon_ext {xs ys l l' : List String} (hl : l.Sublist (canonSet xs))
    (hl' : l'.Sublist (canonSet ys)) (h : ∀ s, s ∈ l ↔ s ∈ l') : l = l' := by
  have sx : l.Pairwise fun a b => lexLt (utf16 a) (utf16 b) = true :=
    (sortBy_sorted utf16 xs).sublist hl
  have sy : l'.Pairwise fun a b => lexLt (utf16 a) (utf16 b) = true :=
    (sortBy_sorted utf16 ys).sublist hl'
  have ne : ∀ {a b : String}, lexLt (utf16 a) (utf16 b) = true → a ≠ b := fun h e => by
    subst e; rw [lexLt_irrefl] at h; exact absurd h (by simp)
  apply Perm.eq_of_pairwise (le := fun a b => lexLt (utf16 a) (utf16 b) = true)
    (fun a b _ _ h₁ h₂ => by rw [lexLt_asymm h₁] at h₂; exact absurd h₂ (by simp)) sx sy
  exact (perm_ext_iff_of_nodup (sx.imp ne) (sy.imp ne)).2 h

theorem filterMap_filter_some {α β : Type} (g : α → Option β) (l : List α) :
    (l.filter fun a => (g a).isSome).filterMap g = l.filterMap g := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    cases e : g a with
    | none => simp [e, ih]
    | some b => simp [e, ih]

/-- An accumulating fold reads only the records its test does not refuse. -/
theorem Acc.select_filter {σ} (f : Acc σ) (ev : Record → Tri) (p : Record → Bool)
    (hp : ∀ r, p r = false → ev r = .no) (xs : List Record) :
    f.select ev (xs.filter p) = f.select ev xs := by
  have e : ∀ v : Tri, v ≠ .no → (xs.filter p).filter (ev · == v) = xs.filter (ev · == v) := by
    intro v hv
    rw [filter_filter]
    apply filter_congr
    intro r _
    cases hpr : p r
    · have := hp r hpr
      simp [this, Ne.symm hv]
    · simp
  simp only [Acc.select, e .yes (by decide), e .unknown (by decide)]

/-- A head the slot chooses is one the spec takes. -/
theorem chosen_takes {sel : Record → Bool} {cs : List Cond} {r : Record}
    (h : chosen sel cs r = true) : sel r = true := by
  unfold chosen at h
  exact (Bool.and_eq_true _ _ ▸ h).1

section Slots

variable {sp : Spec} {S W : List Record} {may : Record → Record → Bool}

/-- The chosen heads of each thing are the same list. -/
theorem chosenHeads_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W)
    (cs : List Cond) (t : String) :
    (stateIn t (realm may S)).heads.filter (chosen sp.takes cs) =
      (stateIn t (realm may W)).heads.filter (chosen sp.takes cs) := by
  apply newest_ext
  · exact ((LatestBy.inv_of ((realm_addressed may S).filter _)).heads_sorted).sublist filter_sublist
  · exact ((LatestBy.inv_of ((realm_addressed may W).filter _)).heads_sorted).sublist filter_sublist
  · intro r
    simp only [mem_filter]
    constructor
    · rintro ⟨hr, hc⟩; exact ⟨(heads_taken hm hF h t (chosen_takes hc)).1 hr, hc⟩
    · rintro ⟨hr, hc⟩; exact ⟨(heads_taken hm hF h t (chosen_takes hc)).2 hr, hc⟩

/-- A thing with a head is one of the things held. -/
theorem head_thing {xs : List Record} (hx : Addressed xs) {t : String} {r : Record}
    (hr : r ∈ (stateIn t xs).heads) : t ∈ thingsOf xs := by
  obtain ⟨hm, ht, -⟩ := (mem_stateIn_heads hx).1 hr
  exact mem_canonSet.2 (mem_map.2 ⟨r, hm, isOf_iff.1 ht⟩)

/-- The chosen current records, over every thing, are the same set. -/
theorem perHeads_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W)
    (cs : List Cond) (r : Record) :
    r ∈ (perHeads (realm may S)).filter (chosen sp.takes cs) ↔
      r ∈ (perHeads (realm may W)).filter (chosen sp.takes cs) := by
  simp only [mem_filter, mem_perHeads]
  constructor
  · rintro ⟨⟨t, -, hr⟩, hc⟩
    have hw := (heads_taken hm hF h t (chosen_takes hc)).1 hr
    exact ⟨⟨t, head_thing (realm_addressed may W) hw, hw⟩, hc⟩
  · rintro ⟨⟨t, -, hw⟩, hc⟩
    have hr := (heads_taken hm hF h t (chosen_takes hc)).2 hw
    exact ⟨⟨t, head_thing (realm_addressed may S) hr, hr⟩, hc⟩

/-- Latest-by, with the things written out. -/
theorem latestOf_eq (sel : Record → Bool) (cs : List Cond) (xs : List Record) :
    latestOf sel cs xs =
      .things
        ((thingsOf xs).filterMap fun t =>
          match ((stateIn t xs).heads.filter (chosen sel cs)).filter (payTest cs · == .yes) with
          | [] => none
          | ks => some (t, shownIds ks))
        (idsOf ((thingsOf xs).flatMap fun t =>
          ((stateIn t xs).heads.filter (chosen sel cs)).filter (payTest cs · == .unknown))).length := by
  simp only [latestOf, perThing, filterMap_map, flatMap_map, Function.comp_def]
  rfl

theorem latestOf_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) (cs : List Cond) :
    latestOf sp.takes cs (realm may S) = latestOf sp.takes cs (realm may W) := by
  have H := chosenHeads_slice hm hF h cs
  have hS := realm_addressed may S
  have hW := realm_addressed may W
  rw [latestOf_eq, latestOf_eq]
  simp only [H]
  generalize hg : (fun t =>
    match ((stateIn t (realm may W)).heads.filter (chosen sp.takes cs)).filter
        (payTest cs · == .yes) with
    | [] => none
    | ks => some (t, shownIds ks)) = g
  have gsome : ∀ t, (g t).isSome = true → ∃ r,
      r ∈ (stateIn t (realm may W)).heads.filter (chosen sp.takes cs) := by
    intro t ht
    subst hg
    dsimp only at ht
    split at ht
    · cases ht
    · rename_i ks hks
      cases hk : ((stateIn t (realm may W)).heads.filter (chosen sp.takes cs)).filter
          (payTest cs · == .yes) with
      | nil => exact absurd hk (hks)
      | cons k _ =>
        exact ⟨k, (mem_filter.1 (hk ▸ mem_cons_self : k ∈ _)).1⟩
  have things : (thingsOf (realm may S)).filter (fun t => (g t).isSome) =
      (thingsOf (realm may W)).filter (fun t => (g t).isSome) := by
    apply canon_ext filter_sublist filter_sublist
    intro t
    simp only [mem_filter]
    constructor
    · rintro ⟨-, ht⟩
      obtain ⟨r, hr⟩ := gsome t ht
      exact ⟨head_thing hW (mem_filter.1 hr).1, ht⟩
    · rintro ⟨-, ht⟩
      obtain ⟨r, hr⟩ := gsome t ht
      rw [← H] at hr
      exact ⟨head_thing hS (mem_filter.1 hr).1, ht⟩
  have unread : idsOf ((thingsOf (realm may S)).flatMap fun t =>
        ((stateIn t (realm may W)).heads.filter (chosen sp.takes cs)).filter
          (payTest cs · == .unknown)) =
      idsOf ((thingsOf (realm may W)).flatMap fun t =>
        ((stateIn t (realm may W)).heads.filter (chosen sp.takes cs)).filter
          (payTest cs · == .unknown)) := by
    apply idsOf_ext
    intro i
    simp only [mem_flatMap]
    constructor
    · rintro ⟨r, ⟨t, -, hr⟩, e⟩
      exact ⟨r, ⟨t, head_thing hW (mem_filter.1 (mem_filter.1 hr).1).1, hr⟩, e⟩
    · rintro ⟨r, ⟨t, -, hr⟩, e⟩
      have hr' := (mem_filter.1 hr).1
      rw [← H] at hr'
      exact ⟨r, ⟨t, head_thing hS (mem_filter.1 hr').1, hr⟩, e⟩
  rw [← filterMap_filter_some g (thingsOf (realm may S)),
    ← filterMap_filter_some g (thingsOf (realm may W)), things, unread]

theorem collectOf_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) (cs : List Cond)
    (follow : Bool) (limit : Option Nat) :
    collectOf sp.takes cs follow limit (realm may S) =
      collectOf sp.takes cs follow limit (realm may W) := by
  have hS := realm_addressed may S
  have hW := realm_addressed may W
  have hp : newestFirst ((if follow then (perThing (realm may S)).flatMap (·.2.heads)
        else realm may S).filter (chosen sp.takes cs)) =
      newestFirst ((if follow then (perThing (realm may W)).flatMap (·.2.heads)
        else realm may W).filter (chosen sp.takes cs)) := by
    cases follow
    · simp only [Bool.false_eq_true, ↓reduceIte]
      apply newestFirst_eq (hS.filter _) (hW.filter _)
      intro r
      simp only [mem_filter]
      constructor
      · rintro ⟨hr, hc⟩; exact ⟨(realm_taken hm hF h (chosen_takes hc)).1 hr, hc⟩
      · rintro ⟨hr, hc⟩; exact ⟨(realm_taken hm hF h (chosen_takes hc)).2 hr, hc⟩
    · simp only [↓reduceIte]
      exact newestFirst_eq ((perHeads_addressed hS).filter _) ((perHeads_addressed hW).filter _)
        (perHeads_slice hm hF h cs)
  unfold collectOf
  simp only [hp]

/-- The whole-reader heads the spec takes are the same set in the slice and
in the whole: a taken record is current in one exactly when in the other. -/
theorem wholeHeads_taken_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W)
    (r : Record) :
    r ∈ (wholeHeads (realm may S)).filter sp.takes ↔
      r ∈ (wholeHeads (realm may W)).filter sp.takes := by
  simp only [mem_filter, mem_wholeHeads (realm_addressed may S),
    mem_wholeHeads (realm_addressed may W)]
  constructor
  · rintro ⟨⟨t, -, hr⟩, ht⟩
    have hw := (heads_taken hm hF h t ht).1 hr
    exact ⟨⟨t, head_thing (realm_addressed may W) hw, hw⟩, ht⟩
  · rintro ⟨⟨t, -, hw⟩, ht⟩
    have hr := (heads_taken hm hF h t ht).2 hw
    exact ⟨⟨t, head_thing (realm_addressed may S) hr, hr⟩, ht⟩

/-- An accumulating slot that follows `prev` gives the same value over the slice. -/
theorem followed_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) {σ} (f : Acc σ)
    (cs : List Cond) :
    f.select (fun r => if sp.takes r then Cond.all cs r else Tri.no) (wholeHeads (realm may S)) =
      f.select (fun r => if sp.takes r then Cond.all cs r else Tri.no) (wholeHeads (realm may W)) := by
  have hno : ∀ r, sp.takes r = false →
      (fun r => if sp.takes r then Cond.all cs r else Tri.no) r = .no := fun r hr => by simp [hr]
  rw [← f.select_filter _ sp.takes hno, ← f.select_filter _ sp.takes hno (wholeHeads (realm may W))]
  exact Acc.select_set_decides _ _ ((wholeHeads_distinct (realm_addressed may S)).filter _)
    ((wholeHeads_distinct (realm_addressed may W)).filter _) (wholeHeads_taken_slice hm hF h)

/-- Every slot gives the same value over the slice. -/
theorem Slot.run_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) (s : Slot) :
    s.run sp.takes (realm may S) = s.run sp.takes (realm may W) := by
  have hS : Distinct (realm may S) := admit_distinct may (settle_distinct S)
  have hW : Distinct (realm may W) := admit_distinct may (settle_distinct W)
  have acc : ∀ {σ} (f : Acc σ) (cs : List Cond),
      f.select (fun r => if sp.takes r then Cond.all cs r else Tri.no) (realm may S) =
        f.select (fun r => if sp.takes r then Cond.all cs r else Tri.no) (realm may W) := by
    intro σ f cs
    have hno : ∀ r, sp.takes r = false →
        (fun r => if sp.takes r then Cond.all cs r else Tri.no) r = .no := fun r hr => by simp [hr]
    rw [← f.select_filter _ sp.takes hno (realm may S), ← f.select_filter _ sp.takes hno (realm may W)]
    apply Acc.select_set_decides _ _ (hS.filter _) (hW.filter _)
    intro r
    simp only [mem_filter]
    constructor
    · rintro ⟨hr, ht⟩; exact ⟨(realm_taken hm hF h ht).1 hr, ht⟩
    · rintro ⟨hr, ht⟩; exact ⟨(realm_taken hm hF h ht).2 hr, ht⟩
  unfold Slot.run
  split
  all_goals dsimp only
  iterate 4
    simp only [accPool]
    split
    · rw [followed_slice hm hF h]
    · rw [acc]
  · exact latestOf_slice hm hF h _
  · exact collectOf_slice hm hF h _ _ _

/-- **A spec slice answers for its spec.** Running the spec over the slice
gives what running it over everything held gives: every slot, the values, the
current records with what is shown of each, and the unread counts. -/
theorem Spec.run_slice (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W) :
    sp.run (settle S) may = sp.run (settle W) may := by
  unfold Spec.run
  exact map_congr_left fun s _ => by
    rw [show admit may (settle S) = realm may S from rfl, show admit may (settle W) = realm may W from rfl,
      Slot.run_slice hm hF h s]

end Slots

/-! ## Neither new kind can be dropped

`ss.a` is a task of `k1`. `ss.c`, a note of `k1`, names it in `prev`, and
also names `ss.b`, a note of `k2`. The spec takes tasks.

Holding everything, `ss.c` is refused (`prev-unrelated`), so `ss.a` is
current. A part without `ss.b` admits `ss.c`, which replaces `ss.a`. A reader
holding `ss.a` and `ss.d`, a note that properly replaces it, reads `ss.a` as
replaced; a part holding only `ss.a`, which is all the spec takes, reads it
as current. -/

def ss.spec : Spec :=
  { vertex := "v", sources := [{ store := ".", kinds := ["task"] }],
    slots := [{ name := "current", type := .latestBy }] }

def ss.a : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default }
def ss.b : Record :=
  { id := 2, kind := "note", observer := "sl:x", of := some "k2", value := "", prev := [], emitted := default }
def ss.c : Record :=
  { id := 3, kind := "note", observer := "sl:x", of := some "k1", value := "", prev := [1, 2],
    emitted := default }
def ss.d : Record :=
  { id := 4, kind := "note", observer := "sl:x", of := some "k1", value := "", prev := [1],
    emitted := default }

/-- Without the successor, the part shows a record the whole has replaced. -/
theorem spec_slice_needs_successors :
    ss.spec.run (settle [ss.a]) ≠ ss.spec.run (settle [ss.a, ss.d]) := by
  decide

/-- Without the successor's other `prev` target, the part admits a successor
the whole refuses, and loses a record the whole shows. -/
theorem spec_slice_needs_their_targets :
    ss.spec.run (settle [ss.a, ss.c]) ≠ ss.spec.run (settle [ss.a, ss.b, ss.c]) := by
  decide

end Loops

import Loops.TickCheck
import Laws.TickRead
import Laws.Slice
import Laws.IdSource

namespace Loops
open List

private theorem mask_env (W : List RecordId) (r : Record) : SameEnvelope (tickMask W r) r := by
  unfold tickMask
  split
  · exact withhold_envelope _
  · exact SameEnvelope.refl _

private theorem mask_id (W : List RecordId) (r : Record) : (tickMask W r).id = r.id := (mask_env W r).id

/-- Restriction reads only the named records, irrespective of their kind. -/
theorem tick_level (sp : Spec) (C W : List RecordId) (xs ys : List Record)
    (h : coveredForms C xs = coveredForms C ys) : tickRun sp C W xs = tickRun sp C W ys := by
  simp only [tickRun, h]

theorem tick_level_append (sp : Spec) (C W : List RecordId) (xs extra : List Record)
    (h : ∀ r ∈ extra, r.id ∉ C) : tickRun sp C W (xs ++ extra) = tickRun sp C W xs := by
  apply tick_level
  simpa [coveredForms, filter_append] using h

/-- Exchanging a named withheld form for any other form of its envelope, including an opening. -/
theorem tick_withheld (sp : Spec) (C W : List RecordId) (before after : List Record) {a b : Record}
    (env : SameEnvelope a b) (w : a.id ∈ W) :
    tickRun sp C W (before ++ a :: after) = tickRun sp C W (before ++ b :: after) := by
  have wb : b.id ∈ W := env.id ▸ w
  by_cases h : a.id ∈ C
  · have hb : b.id ∈ C := env.id ▸ h
    simp [tickRun, coveredForms, filter_append, h, hb, tickMask, w, wb, withhold_congr env]
  · have hb : b.id ∉ C := env.id ▸ h
    simp [tickRun, coveredForms, filter_append, h, hb]

private abbrev masked (C W : List RecordId) (xs : List Record) :=
  (coveredForms C xs).map (tickMask W)

private theorem mask_mem {C W xs r} : r ∈ masked C W xs ↔ ∃ a ∈ xs, a.id ∈ C ∧ tickMask W a = r := by
  simp [masked, coveredForms, and_assoc]

/-- The two disjoint missing counts are zero exactly when both classes of required input are held. -/
theorem tickMissing_zero {C W xs} : tickMissing C W xs = ⟨0,0⟩ ↔
    ∀ i ∈ C, i ∈ idsOf xs ∧ (i ∉ W → tickOpen xs i = true) := by
  simp [tickMissing, filter_eq_nil_iff]
  grind

private theorem open_witness {xs i} (h : tickOpen xs i = true) :
    ∃ a ∈ xs, a.id = i ∧ a.shown = .here := by
  simpa using (Bool.and_eq_true_iff.1 h).1

/-- The access test is exactly an opening in the settled whole holding. -/
theorem tickOpen_settled {xs i} : tickOpen xs i = true ↔
    ∃ r ∈ settle xs, r.id = i ∧ r.shown = .here := by
  constructor
  · intro h
    obtain ⟨a, ha, rfl, hh⟩ := open_witness h
    exact settle_keeps ha hh (by simpa [tickOpen] using (Bool.and_eq_true_iff.1 h).2)
  · rintro ⟨r, hr, rfl, hh⟩
    obtain ⟨j, _, hj⟩ := mem_filterMap.1 hr
    have id := pick_id hj
    have he : erasedIn xs j = false := by
      cases h : erasedIn xs j
      · rfl
      · have := settle_erased h hr id
        rw [hh] at this; contradiction
    unfold pick at hj
    simp [he] at hj
    unfold baseForm at hj
    split at hj
    · rename_i a found
      have mem := mem_filter.1 (mem_of_find?_eq_some found)
      cases hj
      simp [tickOpen, id, he, any_eq_true]
      exact ⟨_, mem.1, by simpa using mem.2, hh⟩
    · simp only [Option.map_eq_some_iff] at hj
      obtain ⟨a, _, e⟩ := hj
      subst r; contradiction

private theorem open_base {xs i a} (ha : a ∈ xs) (hi : a.id = i) (hh : a.shown = .here)
    (consistent : Forms xs) : baseForm (xs.filter (·.id == i)) = some a := by
  obtain ⟨b, hb⟩ := Option.isSome_iff_exists.1 (find?_isSome.2
    ⟨a, mem_filter.2 ⟨ha, by simpa using hi⟩, by simp [hh]⟩ :
    ((xs.filter (·.id == i)).find? (·.shown == .here)).isSome)
  have hm := mem_filter.1 (mem_of_find?_eq_some hb)
  have e := (consistent b hm.1 a ha ((by simpa using hm.2 : b.id = i).trans hi.symm)).2
    (by simpa using find?_some hb) hh
  simp [baseForm, hb, e]

private theorem masked_forms {xs : List Record} (h : Forms xs) (C W) : Forms (masked C W xs) := by
  rintro _ ha _ hb eq
  obtain ⟨a, hax, _, rfl⟩ := mask_mem.1 ha
  obtain ⟨b, hbx, _, rfl⟩ := mask_mem.1 hb
  rw [mask_id, mask_id] at eq
  have ab := h a hax b hbx eq
  refine ⟨((mask_env W a).trans ab.1).trans (mask_env W b).symm, ?_⟩
  by_cases w : b.id ∈ W <;> simp [tickMask, eq, w, Record.withhold]
  exact ab.2

/-- An additional copy of a withheld id contributes no new masked form. -/
theorem tick_withheld_copy (sp : Spec) (C W : List RecordId) (xs : List Record) (a b : Record)
    (consistent : Forms xs) (ha : a ∈ xs) (env : SameEnvelope a b) (w : a.id ∈ W) :
    tickRun sp C W xs = tickRun sp C W (b :: xs) := by
  suffices settle (masked C W xs) = settle (masked C W (b :: xs)) by simp only [tickRun, this]
  apply settle_eq (masked_forms consistent C W)
  intro r
  simp only [mask_mem, mem_cons]
  constructor
  · rintro ⟨f, hf, hc, he⟩
    exact ⟨f, Or.inr hf, hc, he⟩
  · rintro ⟨f, (rfl | hf), hc, he⟩
    · refine ⟨a, ha, env.id ▸ hc, ?_⟩
      rw [← he]
      simp [tickMask, w, show f.id ∈ W from env.id ▸ w, withhold_congr env]
    · exact ⟨_, hf, hc, he⟩

private theorem masked_has {C W xs i} (member : i ∈ C) (held : i ∈ idsOf xs) :
    ∃ r ∈ masked C W xs, r.id = i := by
  obtain ⟨a, ha, rfl⟩ := mem_idsOf.1 held
  exact ⟨_, mask_mem.2 ⟨a, ha, member, rfl⟩, mask_id W a⟩

private theorem masked_eraser {C W xs ys i} (joint : Forms (xs ++ ys))
    (held : ∀ j ∈ C, j ∈ idsOf ys) (h : erasedIn (masked C W xs) i = true) :
    erasedIn (masked C W ys) i = true := by
  obtain ⟨r, hr, he⟩ := any_eq_true.1 h
  obtain ⟨a, ha, hc, rfl⟩ := mask_mem.1 hr
  obtain ⟨b, hb, eq⟩ := mem_idsOf.1 (held _ hc)
  have ab := (joint a (mem_append_left _ ha) b (mem_append_right _ hb) eq.symm).1
  apply any_eq_true.2
  exact ⟨_, mask_mem.2 ⟨b, hb, eq ▸ hc, rfl⟩, by simpa only [(mask_env W a).erases, (mask_env W b).erases, ab.erases] using he⟩

private theorem joint_swap {xs ys : List Record} (h : Forms (xs ++ ys)) : Forms (ys ++ xs) :=
  fun a ha b hb => h a (by simpa [or_comm] using ha) b (by simpa [or_comm] using hb)

private theorem filterMap_on {α β} {f g : α → Option β} (xs : List α) (h : ∀ x ∈ xs, f x = g x) :
    xs.filterMap f = xs.filterMap g := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    simp only [filterMap_cons, h x mem_cons_self, ih (fun a ha => h a (mem_cons_of_mem _ ha))]

theorem tick_inputs (sp : Spec) (C W : List RecordId) (xs ys : List Record)
    (joint : Forms (xs ++ ys)) (hx : tickMissing C W xs = ⟨0,0⟩) (hy : tickMissing C W ys = ⟨0,0⟩) :
    resultBytes (tickRun sp C W xs) = resultBytes (tickRun sp C W ys) := by
  have x := tickMissing_zero.1 hx
  have y := tickMissing_zero.1 hy
  have ids (fs : List Record) (held : ∀ i ∈ C, i ∈ idsOf fs) (i : RecordId) :
      (∃ r ∈ masked C W fs, r.id = i) ↔ i ∈ C := by
    constructor
    · rintro ⟨r, hr, rfl⟩
      obtain ⟨a, _, hc, rfl⟩ := mask_mem.1 hr
      simpa only [mask_id] using hc
    · exact fun hc => masked_has hc (held _ hc)
  suffices settle (masked C W xs) = settle (masked C W ys) by simp only [tickRun, ← this]
  unfold settle
  rw [idsOf_ext fun i => (ids xs (fun i hi => (x i hi).1) i).trans (ids ys (fun i hi => (y i hi).1) i).symm]
  apply filterMap_on
  intro i hi
  have hc := (ids ys (fun i hi => (y i hi).1) i).1 (mem_idsOf.1 hi)
  have he : erasedIn (masked C W xs) i = erasedIn (masked C W ys) i := Bool.eq_iff_iff.2 ⟨masked_eraser joint (fun j hj => (y j hj).1),
    masked_eraser (joint_swap joint) (fun j hj => (x j hj).1)⟩
  unfold pick
  rw [he]
  congr 1
  by_cases hw : i ∈ W
  · apply baseForm_eq (i := i)
    · exact fun a ha b hb => masked_forms (fun r hr t ht => joint r (mem_append_left _ hr) t (mem_append_left _ ht)) C W
        a (mem_filter.1 ha).1 b (mem_filter.1 hb).1
    · intro a ha; simpa using (mem_filter.1 ha).2
    · have transfer {A B : List Record} (h : Forms (A ++ B)) (held : i ∈ idsOf B) (r : Record) :
          r ∈ (masked C W A).filter (·.id == i) → r ∈ (masked C W B).filter (·.id == i) := by
        intro hr
        obtain ⟨a, ha, _, rfl⟩ := mask_mem.1 (mem_filter.1 hr).1
        have ai : a.id = i := by simpa [mask_id] using (mem_filter.1 hr).2
        obtain ⟨b, hb, bi⟩ := mem_idsOf.1 held
        have ab := (h a (mem_append_left _ ha) b (mem_append_right _ hb) (ai.trans bi.symm)).1
        exact mem_filter.2 ⟨mask_mem.2 ⟨b, hb, bi ▸ hc, by simp [tickMask, ai, bi, hw, withhold_congr ab]⟩,
          (mem_filter.1 hr).2⟩
      exact fun r => ⟨transfer joint (y i hc).1 r, transfer (joint_swap joint) (x i hc).1 r⟩
  · obtain ⟨a, ha, ai, ah⟩ := open_witness ((x i hc).2 hw)
    obtain ⟨b, hb, bi, bh⟩ := open_witness ((y i hc).2 hw)
    have eq := (joint a (mem_append_left _ ha) b (mem_append_right _ hb) (ai.trans bi.symm)).2 ah bh
    have openIn {fs : List Record} {r : Record} (hr : r ∈ fs) (ri : r.id = i) : r ∈ masked C W fs :=
      mask_mem.2 ⟨r, hr, ri ▸ hc, by simp [tickMask, ri, hw]⟩
    rw [open_base (openIn ha ai) ai ah (masked_forms (fun r hr t ht => joint r (mem_append_left _ hr) t (mem_append_left _ ht)) C W),
        open_base (openIn hb bi) bi bh (masked_forms (fun r hr t ht => joint r (mem_append_right _ hr) t (mem_append_right _ ht)) C W), eq]

private theorem names_nodup {α} {ms : List (String × α)} (h : KeyOrdered ms) : (ms.map Prod.fst).Nodup := by
  apply pairwise_map.2
  exact h.imp (fun {a b} lt e => by
    change lexLt (utf16 a.1) (utf16 b.1) = true at lt
    change a.1 = b.1 at e
    simp [e, lexLt_irrefl] at lt)

private theorem group_fits {v : GroupValue} {ty : FoldType} (hg : ty.Groups) (h : v.tag = ty.tag) :
    v.FitsFold ty := by
  cases v <;> cases ty <;> simp_all [GroupValue.tag, FoldType.tag, FoldType.Groups, GroupValue.FitsFold]

/-- The evaluator always produces the reader's slot shape, including the maps inside keyed slots. -/
theorem Slot.run_fitsSlot (s : Slot) (hf : s.Fits) (sel : Record → Bool) (xs : List Record) :
    (s.run sel xs).FitsSlot s := by
  cases eg : s.group with
  | none =>
    have ho := s.plain_ordered sel xs
    rw [Slot.run, eg]
    cases ht : s.type <;> simp only [Slot.plain, ht] at ho ⊢
    case latestBy f =>
      cases f <;> simp only [SlotValue.FitsSlot, eg, ht, latestOf, keyedOf, SlotValue.Ordered] at ho ⊢
      · exact names_nodup ho
      · exact ⟨names_nodup ho.1, fun t ht => names_nodup (ho.2 t ht)⟩
    all_goals split <;> simp [keyedValue, collectOf, SlotValue.FitsSlot, eg, ht]
  | some b =>
    have hg := (hf.2.2.2 b (by simp [eg])).2
    have ho := s.run_ordered sel xs
    unfold Slot.run at ho ⊢
    simp only [eg] at ho ⊢
    by_cases mode : s.type.follow = some .keys <;> simp only [mode, ↓reduceIte] at ho ⊢
    · simp only [Slot.keyedGrouped, SlotValue.Ordered, SlotValue.FitsSlot, eg] at ho ⊢
      refine ⟨names_nodup ho, ?_⟩
      intro g hm
      obtain ⟨k, _, rfl⟩ := mem_map.1 hm
      exact group_fits hg (keyedValue_tag ..)
    · simp only [Slot.grouped, SlotValue.Ordered, SlotValue.FitsSlot, eg] at ho ⊢
      refine ⟨names_nodup ho, ?_⟩
      intro g hm
      obtain ⟨k, _, rfl⟩ := mem_map.1 hm
      exact group_fits hg (Slot.plain_tag ..)

private theorem open_append {xs extra : List Record} {i : RecordId}
    (outside : ∀ r ∈ extra, r.id ≠ i) (safe : ∀ r ∈ extra, i ∉ r.erases) :
    tickOpen (xs ++ extra) i = tickOpen xs i := by
  have hn : (extra.any fun r => r.id == i && r.shown == .here) = false := by
    simpa [any_eq_false] using fun a ha => (show a.id = i → a.shown ≠ .here from fun e => absurd e (outside a ha))
  have he : (extra.any fun r => r.erases.contains i) = false := by simpa using safe
  simp only [tickOpen, erasedIn, any_append, hn, he, Bool.or_false]

private theorem made_replay_ready (sp : Spec) (xs extra : List Record)
    (outside : ∀ r ∈ extra, r.id ∉ idsOf xs)
    (safe : ∀ r ∈ extra, ∀ i ∈ idsOf xs, i ∉ r.erases) :
    replayIssue (makeTick sp xs).covers (makeTick sp xs).withheld (xs ++ extra) = none := by
  let C := idsOf xs
  let W := C.filter fun i => !tickOpen xs i
  change replayIssue C W (xs ++ extra) = none
  have restrict : coveredForms C (xs ++ extra) = coveredForms C xs := by
    simpa [coveredForms, filter_append, C] using outside
  have declared : (coveredForms C xs).any (fun r => r.erases.any (fun i => C.contains i && !W.contains i)) = false := by
    apply any_eq_false.2
    intro r hr
    intro bad
    obtain ⟨i, hi, bad⟩ := any_eq_true.1 bad
    have hc : i ∈ C := by simpa using (Bool.and_eq_true_iff.1 bad).1
    have he : erasedIn xs i = true := any_eq_true.2 ⟨r, (mem_filter.1 hr).1, by simpa using hi⟩
    simp [W, hc, tickOpen, he] at bad
  have missing : tickMissing C W (xs ++ extra) = ⟨0,0⟩ := by
    apply tickMissing_zero.2
    intro i hi
    obtain ⟨r, hr, hri⟩ := mem_idsOf.1 hi
    refine ⟨mem_idsOf.2 ⟨r, mem_append_left _ hr, hri⟩, ?_⟩
    intro nw
    rw [open_append (fun a ha e => outside a ha (e ▸ hi)) (fun a ha => safe a ha i hi)]
    simpa [W, hi] using nw
  rw [replayIssue, restrict, declared, missing]
  rfl

/-- The checker's ordinary wire path on the payload of a made tick, even after safe additions. -/
theorem checkTick_made_of_width (H : Hash) (sp : Spec) (hf : sp.Fits) (xs extra : List Record)
    (hspec : (H (specPreimageOf Rules.current sp.toJson)).length = 32)
    (hresult : (H (resultPreimage (makeTick sp xs).result)).length = 32)
    (outside : ∀ r ∈ extra, r.id ∉ idsOf xs)
    (safe : ∀ r ∈ extra, ∀ i ∈ idsOf xs, i ∉ r.erases) :
    (checkTick H Rules.current ⟨"tick", some ((makeTick sp xs).toJson H)⟩ (xs ++ extra)).answer = .same := by
  let p := makeTick sp xs
  have hs := idsOf_sorted xs
  have hw : p.withheld.Pairwise (· < ·) := hs.filter _
  have sub : ∀ i ∈ p.withheld, i ∈ p.covers := fun _ hi => (mem_filter.1 hi).1
  let ys := admit (fun _ _ => true) (settle (masked p.covers p.withheld xs))
  let values := fun s : Slot => s.run sp.takes ys
  have members := TickPayload.read_members p H Rules.current (Spec.toJson_distinct hf)
    (resultJson_distinct_of_slots sp values hf (fun s hm => s.run_fitsSlot (hf.2.2.2.2 _ hm) _ _)) hspec hresult hs hw sub
  have slots : readResult sp (resultJson p.result) = .ok (unshownResult p.result) :=
    readResult_written sp values hf.2.2.2.1 (fun s hm => s.run_fitsSlot (hf.2.2.2.2 _ hm) _ _)
  have replay := made_replay_ready sp xs extra outside safe
  have late := tick_level_append sp p.covers p.withheld xs extra outside
  change (checkTick _ _ ⟨"tick", some (p.toJson H)⟩ _).answer = _
  simp only [checkTick, members]
  simp [show p.spec = sp from rfl, show p.rules = Rules.current from rfl, checkMembers, TickMembers.readSpec, Spec.ofJson_toJson hf, TickMembers.readSlots, slots, replayTick,
    show replayIssue p.covers p.withheld (xs ++ extra) = none from replay, late,
    show tickRun sp p.covers p.withheld xs = p.result from rfl, resultBytes, resultText,
    Except.mapError, bind, Except.bind, pure, Except.pure]
  rfl

theorem checkTick_made_late (sp : Spec) (hf : sp.Fits) (xs extra : List Record)
    (outside : ∀ r ∈ extra, r.id ∉ idsOf xs)
    (safe : ∀ r ∈ extra, ∀ i ∈ idsOf xs, i ∉ r.erases) :
    (checkTick Gen.sha256 Rules.current ⟨"tick", some ((makeTick sp xs).toJson Gen.sha256)⟩ (xs ++ extra)).answer = .same :=
  checkTick_made_of_width Gen.sha256 sp hf xs extra (Gen.sha256_length _) (Gen.sha256_length _) outside safe

theorem checkTick_made (sp : Spec) (hf : sp.Fits) (xs : List Record) :
    (checkTick Gen.sha256 Rules.current ⟨"tick", some ((makeTick sp xs).toJson Gen.sha256)⟩ xs).answer = .same := by
  simpa using checkTick_made_late sp hf xs [] (by simp) (by simp)

end Loops

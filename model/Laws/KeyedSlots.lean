import Laws.By

/-!
# Count, sum, min, max and collect following `keys`

A slot that follows `keys` reads things: `thingEv` selects one as latest-by
following `keys` does, by its heads, each alone, and then by its payload tests
asked of the thing. The set decides every such slot (`keyedValue_set_decides`,
`Slot.keyedGrouped_set_decides`, in `Laws.Spec`). This file says what each one
holds.

* **Count counts things** (`keyedValue_count`, `thingEv_yes`): each thing one of
  whose heads is chosen and whose tests hold of it, once, however many of its
  heads are chosen.
* **Sum, min and max read a field from its one live setter**
  (`soleSetter_eq`, `Acc.overThings_value`): a thing with no live setter of the
  field, or two or more (`soleSetter_two`), or one whose payload is not held or
  does not hold the field as a number, is unread, once (`Acc.overThings_unread`).
  The core compares no values: two setters of the same number are two setters.
* **Collect lists the selected things' live records** (`keyedValue_collect`),
  newest first, a pure-join head among them, and counts each selected thing its
  tests cannot answer for once.
* **`by` over things** (`By.thingKeys_payload`, `By.thingKeys_envelope`,
  `By.thingKeys_unheld`, `By.thingKeys_unset`): a payload key is read from each
  live setter of the field, and a setter whose payload is not held makes it
  unread; an envelope key from each chosen head, and it is never unread
  (`Slot.keyedGrouped_envelope_unread`). Every selected thing, or one whose
  tests are unknown, is exactly one of grouped, ungrouped or unread
  (`By.keyedAccounting`).
* **Read whole and unforked, `keys` is `prev`** (`Slot.keys_as_prev`): when
  every record is read whole and every thing has one head, each of these slots,
  grouped or not, gives what the same slot following `prev` gives.
-/

namespace Loops

open List

/-! ## Selecting a thing -/

/-- A slot that follows `keys` selects a thing exactly when `from` and the
envelope tests choose one of its heads and its payload tests hold of the thing. -/
theorem thingEv_yes {sel : Record → Bool} {cs : List Cond} {xs : List Record} {t : String} :
    thingEv sel cs xs t = .yes ↔
      (∃ h ∈ (stateIn t xs).heads, chosen sel cs h = true) ∧ thingTest cs (recordsOf t xs) = .yes := by
  unfold thingEv
  split
  · rename_i hc; simp only [any_eq_true] at hc; simp [hc]
  · rename_i hc; simp only [any_eq_true] at hc; simp [hc]

/-- A thing none of whose heads is chosen is not selected, whatever its tests. -/
theorem thingEv_unchosen {sel : Record → Bool} {cs : List Cond} {xs : List Record} {t : String}
    (h : ∀ hd ∈ (stateIn t xs).heads, chosen sel cs hd = false) : thingEv sel cs xs t = .no := by
  unfold thingEv
  split
  · rename_i hc
    obtain ⟨x, hx, e⟩ := any_eq_true.1 hc
    rw [h x hx] at e; cases e
  · rfl

/-! ## Count -/

/-- **Count following `keys` counts things**, each selected thing once, and
each thing its tests cannot answer for once. -/
theorem keyedValue_count (f : Option Follow) (xs : List Record) (ev : String → Tri) :
    keyedValue (.count f) xs ev =
      .nat ⟨((thingsOf xs).filter (ev · == .yes)).length, ((thingsOf xs).filter (ev · == .unknown)).length⟩ :=
  rfl

/-- So a slot counting things following `keys` holds the number of things one
of whose heads is chosen and whose tests hold of the thing. A thing is counted
once however many of its heads are chosen: the things are distinct. -/
theorem Slot.keys_count {s : Slot} (hs : s.type = .count (some .keys)) (hg : s.group = none)
    (sel : Record → Bool) (xs : List Record) :
    s.run sel xs = .nat ⟨((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).length,
      ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .unknown)).length⟩ ∧
    (thingsOf xs).Nodup := by
  refine ⟨?_, canonSet_nodup _⟩
  simp only [Slot.run, hg, Slot.plain, hs, ↓reduceIte, keyedValue_count]

/-! ## Sum, min and max -/

theorem soleSetter_eq {xs : List Record} {f t : String} {r : Record} :
    soleSetter xs f t = some r ↔ setters (recordsOf t xs) f = [r] := by
  unfold soleSetter
  split <;> simp_all

/-- **Two live setters of a field leave it unread**, also when they hold the
same number: the core never compares values. -/
theorem soleSetter_two {xs : List Record} {f t : String} (h : 2 ≤ (setters (recordsOf t xs) f).length) :
    soleSetter xs f t = none := by
  unfold soleSetter
  split
  · rename_i r e; rw [e] at h; simp at h
  · rfl

/-- A field no record of the thing sets has no live setter, and is unread. -/
theorem soleSetter_unset {xs : List Record} {f t : String} (h : setters (recordsOf t xs) f = []) :
    soleSetter xs f t = none := by
  simp [soleSetter, h]

/-- What an accumulating fold over things reads: each selected thing's record. -/
theorem Acc.overThings_value {σ} (f : Acc σ) (pick : String → Option Record) (ev : String → Tri)
    (ts : List String) :
    (f.overThings pick ev ts).value = f.fold (((ts.filter (ev · == .yes)).filterMap pick).filter f.readable) :=
  rfl

/-- **What it cannot read, it counts once**: a thing its tests cannot answer
for, and a selected thing with no record to read, or one the fold cannot read. -/
theorem Acc.overThings_unread {σ} (f : Acc σ) (pick : String → Option Record) (ev : String → Tri)
    (ts : List String) :
    (f.overThings pick ev ts).unread =
      (ts.filter fun t => ev t == .unknown || (ev t == .yes && !(pick t).any f.readable)).length := by
  simp only [Acc.overThings, filter_filter]
  induction ts with
  | nil => rfl
  | cons t ts ih =>
    simp only [filter_cons]
    cases ev t <;> cases (pick t).any f.readable <;> simp [ih] <;> omega

/-! ## Collect -/

/-- **Collect following `keys` lists exactly the selected things' live
records**, a pure-join head among them, each with what is shown of it. -/
theorem keyedValue_collect {xs : List Record} (hx : Addressed xs) (f : Option Follow) (ev : String → Tri)
    {hs : List (RecordId × Shown)} {u : Nat} (h : keyedValue (.collect f none) xs ev = .heads hs u)
    {p : RecordId × Shown} :
    p ∈ hs ↔ ∃ t ∈ thingsOf xs, ev t = .yes ∧ ∃ r ∈ liveIn xs t, (r.id, r.shown) = p := by
  simp only [keyedValue, limitTo, SlotValue.heads.injEq] at h
  obtain ⟨rfl, -⟩ := h
  have ha : Addressed (((thingsOf xs).filter (ev · == .yes)).flatMap (liveIn xs)) := fun a ha b hb =>
    hx a (by obtain ⟨_, _, ha⟩ := mem_flatMap.1 ha; exact liveIn_sub ha) b
      (by obtain ⟨_, _, hb⟩ := mem_flatMap.1 hb; exact liveIn_sub hb)
  simp only [shownIds, mem_map, mem_newestFirst_iff ha, mem_flatMap, mem_filter, beq_iff_eq]
  constructor
  · rintro ⟨_, ⟨q, ⟨t, ⟨ht, he⟩, hq⟩, rfl⟩, rfl⟩
    exact ⟨t, ht, he, q, hq, rfl⟩
  · rintro ⟨t, ht, he, q, hq, rfl⟩
    exact ⟨q.unlinked, ⟨q, ⟨t, ⟨ht, he⟩, hq⟩, rfl⟩, rfl⟩

/-- And it counts each selected thing its tests cannot answer for once, before the limit. -/
theorem keyedValue_collect_unread (f : Option Follow) (limit : Option Nat) (xs : List Record)
    (ev : String → Tri) :
    ∃ hs, keyedValue (.collect f limit) xs ev = .heads hs ((thingsOf xs).filter (ev · == .unknown)).length :=
  ⟨_, rfl⟩

/-! ## `by` over things -/

/-- A payload key is read from the field's live setters, each key of each. -/
theorem By.thingKeys_payload {f : String} (sel : Record → Bool) (cs : List Cond) (xs : List Record)
    (t : String) :
    (By.payload f).thingKeys sel cs xs t =
      if (setters (recordsOf t xs) f).all ((By.payload f).keys · |>.isSome) then
        some (canonSet ((setters (recordsOf t xs) f).flatMap fun r => ((By.payload f).keys r).getD []))
      else none := rfl

/-- **An envelope key is never unread**: it is read from the chosen heads,
each of which holds its envelope. -/
theorem By.thingKeys_envelope {b : By} (hb : ∀ f, b ≠ .payload f) (sel : Record → Bool) (cs : List Cond)
    (xs : List Record) (t : String) :
    b.thingKeys sel cs xs t =
      some (canonSet (((stateIn t xs).heads.filter (chosen sel cs)).flatMap fun h => (b.keys h).getD [])) := by
  cases b with
  | payload f => exact absurd rfl (hb f)
  | _ => simp [By.thingKeys, By.keys]

/-- A live setter whose payload is not held makes a payload key unread. -/
theorem By.thingKeys_unheld {f : String} {sel : Record → Bool} {cs : List Cond} {xs : List Record}
    {t : String} {r : Record} (hr : r ∈ setters (recordsOf t xs) f) (hs : r.shown ≠ .here) :
    (By.payload f).thingKeys sel cs xs t = none := by
  rw [By.thingKeys_payload]
  split
  · rename_i hall
    have := (all_eq_true.1 hall) r hr
    simp [By.keys, hs] at this
  · rfl

/-- With no live setter of the field, the thing has no key: it is ungrouped. -/
theorem By.thingKeys_unset {f : String} {sel : Record → Bool} {cs : List Cond} {xs : List Record}
    {t : String} (h : setters (recordsOf t xs) f = []) :
    (By.payload f).thingKeys sel cs xs t = some [] := by
  rw [By.thingKeys_payload, h]
  rfl

/-- A slot with `by` that follows `keys`, written out. -/
theorem Slot.keyedGrouped_eq (s : Slot) (b : By) (sel : Record → Bool) (xs : List Record) :
    s.keyedGrouped b sel xs =
      .groups ((canonSet (((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).flatMap
          fun t => (b.thingKeys sel s.tests.conds xs t).getD [])).map fun g =>
        (g, (keyedValue s.type xs fun t =>
          if thingEv sel s.tests.conds xs t == .yes &&
            ((b.thingKeys sel s.tests.conds xs t).getD []).contains g then .yes else .no).group))
        (((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .unknown)).length +
          ((thingsOf xs).filter fun t => thingEv sel s.tests.conds xs t == .yes &&
            (b.thingKeys sel s.tests.conds xs t).isNone).length)
        ((thingsOf xs).filter fun t => thingEv sel s.tests.conds xs t == .yes &&
          b.thingKeys sel s.tests.conds xs t == some []).length := rfl

/-- **Grouped by an envelope key, only a thing whose tests are unknown is unread.** -/
theorem Slot.keyedGrouped_envelope_unread {b : By} (hb : ∀ f, b ≠ .payload f) (s : Slot)
    (sel : Record → Bool) (xs : List Record) :
    ∃ gs m, s.keyedGrouped b sel xs =
      .groups gs ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .unknown)).length m := by
  have : ((thingsOf xs).filter fun t => thingEv sel s.tests.conds xs t == .yes &&
      (b.thingKeys sel s.tests.conds xs t).isNone) = [] :=
    filter_eq_nil_iff.2 fun t _ => by simp [By.thingKeys_envelope hb]
  rw [Slot.keyedGrouped_eq, this]
  exact ⟨_, _, rfl⟩

/-- **Accounting over things.** Every thing a slot with `by` that follows
`keys` selects, or whose tests are unknown, is exactly one of grouped (it has a
key), ungrouped (its key is readably absent) or unread (its tests are unknown,
or its key cannot be read). -/
theorem By.keyedAccounting (keys : String → Option (List String)) (ev : String → Tri) (ts : List String) :
    (ts.filter (ev · != .no)).length =
      (ts.filter fun t => ev t == .yes && ((keys t).getD []) != []).length +
      (ts.filter fun t => ev t == .yes && keys t == some []).length +
      ((ts.filter (ev · == .unknown)).length +
        (ts.filter fun t => ev t == .yes && (keys t).isNone).length) := by
  induction ts with
  | nil => rfl
  | cons t ts ih =>
    simp only [filter_cons]
    cases he : ev t <;> rcases hk : keys t with _ | (_ | ⟨k, ks⟩) <;> simp_all <;> omega

/-! ## Read whole and unforked, `keys` is `prev` -/

/-- The same fold type, following `f`. -/
def FoldType.following (f : Follow) : FoldType → FoldType
  | .count _ => .count (some f)
  | .sum x _ => .sum x (some f)
  | .min x _ => .min x (some f)
  | .max x _ => .max x (some f)
  | .latestBy _ => .latestBy f
  | .collect _ l => .collect (some f) l

theorem FoldType.following_follow (f : Follow) (ty : FoldType) : (ty.following f).follow = some f := by
  cases ty <;> rfl

section WholeRead

variable {xs : List Record} {hd : String → Record}

theorem whole_hd_mem (hx : Addressed xs) (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t])
    {t : String} (ht : t ∈ thingsOf xs) :
    hd t ∈ recordsOf t xs ∧ liveEnv (recordsOf t xs) (hd t) = true :=
  (mem_stateIn_heads_iff hx).1 (by rw [hh t ht]; exact mem_singleton_self _)

theorem whole_hd_thing (hx : Addressed xs) (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t])
    {t : String} (ht : t ∈ thingsOf xs) : (hd t).thing = t :=
  isOf_iff.1 (mem_filter.1 (whole_hd_mem hx hh ht).1).2

/-- Read whole, a record is live for a key exactly when it is the head. -/
theorem whole_liveFor_iff (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    {r : Record} (hr : r ∈ recordsOf t xs) (k : String) :
    liveFor (recordsOf t xs) k r = true ↔ r = hd t := by
  have henv : liveFor (recordsOf t xs) k r = liveEnv (recordsOf t xs) r := by
    apply Bool.eq_iff_iff.2
    rw [liveFor_iff]
    simp only [liveEnv, isEmpty_iff, eq_nil_iff_forall_not_mem]
    constructor
    · rintro ⟨-, h⟩ q hq
      have := h q hq
      simp [Record.setsKey, hw q (mem_filter.1 (mem_above.1 hq).1).1] at this
    · intro h
      exact ⟨by simp [Record.setsKey, hw r (mem_filter.1 hr).1], fun q hq => absurd hq (h q)⟩
  rw [henv]
  constructor
  · intro hl
    have := (mem_stateIn_heads_iff hx).2 ⟨hr, hl⟩
    rw [hh t ht] at this
    exact mem_singleton.1 this
  · rintro rfl; exact (whole_hd_mem hx hh ht).2

theorem whole_live_iff (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    {r : Record} (hr : r ∈ recordsOf t xs) : live (recordsOf t xs) r = true ↔ r = hd t := by
  rw [live_iff]
  constructor
  · rintro (hl | ⟨k, hk⟩)
    · have := (mem_stateIn_heads_iff hx).2 ⟨hr, hl⟩
      rw [hh t ht] at this
      exact mem_singleton.1 this
    · exact (whole_liveFor_iff hx hw hh ht hr k).1 hk
  · rintro rfl; exact .inl (whole_hd_mem hx hh ht).2

theorem whole_setters (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    (k : String) : setters (recordsOf t xs) k = [hd t] := by
  unfold setters
  apply sorted_ext (orderHeads_sorted (addressed_filter (addressed_recordsOf hx t) _)) (pairwise_singleton _ _)
  intro r
  rw [mem_orderHeads (addressed_filter (addressed_recordsOf hx t) _), mem_filter, mem_singleton]
  constructor
  · rintro ⟨hr, hl⟩; exact (whole_liveFor_iff hx hw hh ht hr k).1 hl
  · rintro rfl
    exact ⟨(whole_hd_mem hx hh ht).1, (whole_liveFor_iff hx hw hh ht (whole_hd_mem hx hh ht).1 k).2 rfl⟩

theorem whole_soleSetter (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    (f : String) : soleSetter xs f t = some (hd t) :=
  soleSetter_eq.2 (whole_setters hx hw hh ht f)

/-- Read whole and unforked, a thing's payload tests are its head's. -/
theorem whole_thingTest (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    (cs : List Cond) : thingTest cs (recordsOf t xs) = payTest cs (hd t) := by
  unfold thingTest payTest Cond.all
  generalize hds : cs.filter (!·.field.onEnvelope) = ds
  have hp : ∀ c ∈ ds, c.onThing (recordsOf t xs) = c.eval (hd t) := by
    intro c hc
    rw [← hds] at hc
    obtain ⟨f, op⟩ := c
    cases f with
    | env e => simp [Field.onEnvelope] at hc
    | payload f =>
      have hm := (whole_hd_mem hx hh ht).1
      exact Cond.onThing_agree rfl hm ((whole_liveFor_iff hx hw hh ht hm f).2 rfl)
        fun q hq hl => by rw [(whole_liveFor_iff hx hw hh ht hq f).1 hl]
  clear hds
  induction ds with
  | nil => rfl
  | cons c ds ih =>
    simp only [foldr_cons]
    rw [hp c mem_cons_self, ih fun d hd => hp d (mem_cons_of_mem _ hd)]

/-- Read whole and unforked, a thing is selected as its head is. -/
theorem whole_thingEv (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    (sel : Record → Bool) (cs : List Cond) :
    thingEv sel cs xs t = if chosen sel cs (hd t) then payTest cs (hd t) else .no := by
  unfold thingEv
  rw [hh t ht, whole_thingTest hx hw hh ht]
  simp

/-- A head's selection, as an accumulating slot over heads makes it. -/
theorem ev_chosen (sel : Record → Bool) (cs : List Cond) (r : Record) :
    (if sel r then Cond.all cs r else Tri.no) = if chosen sel cs r then payTest cs r else .no := by
  have he := Cond.all_envelope_ne cs r
  unfold chosen
  rw [Cond.all_split]
  revert he
  cases sel r <;> cases Cond.all (cs.filter (·.field.onEnvelope)) r <;> cases payTest cs r <;> decide

theorem whole_heads_flat (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) :
    (perThing xs).flatMap (·.2.heads) = (thingsOf xs).map hd := by
  unfold perThing
  rw [flatMap_map]
  have : ∀ L : List String, (∀ t ∈ L, t ∈ thingsOf xs) →
      L.flatMap (fun t => (stateIn t xs).heads) = L.map hd := by
    intro L hL
    induction L with
    | nil => rfl
    | cons t L ih =>
      rw [flatMap_cons, map_cons, hh t (hL t mem_cons_self), ih fun u hu => hL u (mem_cons_of_mem _ hu)]
      rfl
  exact this _ fun _ h => h

theorem whole_distinct (hx : Addressed xs) (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) :
    Distinct ((thingsOf xs).map hd) := by
  unfold Distinct
  rw [map_map, Nodup, pairwise_map]
  apply (canonSet_nodup _).imp_of_mem
  intro t u ht hu hne e
  have ha := hx _ (mem_filter.1 (whole_hd_mem hx hh ht).1).1 _ (mem_filter.1 (whole_hd_mem hx hh hu).1).1 e
  exact hne (by rw [← whole_hd_thing hx hh ht, ← whole_hd_thing hx hh hu, ha])

theorem whole_wholeHeads (hx : Addressed xs) (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t])
    (r : Record) : r ∈ wholeHeads xs ↔ r ∈ (thingsOf xs).map hd := by
  rw [mem_wholeHeads hx]
  simp only [mem_map]
  constructor
  · rintro ⟨t, ht, hr⟩; rw [hh t ht] at hr; exact ⟨t, ht, (mem_singleton.1 hr).symm⟩
  · rintro ⟨t, ht, rfl⟩; exact ⟨t, ht, by rw [hh t ht]; exact mem_singleton_self _⟩

theorem filterMap_some_eq {α β} {L : List α} {f : α → Option β} {g : α → β} (h : ∀ a ∈ L, f a = some (g a)) :
    L.filterMap f = L.map g := by
  induction L with
  | nil => rfl
  | cons a L ih =>
    rw [filterMap_cons, h a mem_cons_self, ih fun b hb => h b (mem_cons_of_mem _ hb)]
    rfl

theorem length_filter_map_on {α β} {L : List α} {g : α → β} {p : β → Bool} {q : α → Bool}
    (h : ∀ a ∈ L, p (g a) = q a) : ((L.map g).filter p).length = (L.filter q).length := by
  rw [filter_map, length_map]
  congr 1
  exact filter_congr fun a ha => h a ha

/-- An accumulating fold over the heads, one per thing, is the fold over the
things those heads are of. -/
theorem Acc.select_as_things {σ} (f : Acc σ) (ev : Record → Tri) (hx : Addressed xs)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) (pick : String → Option Record)
    (e : String → Tri) (hpick : ∀ t ∈ thingsOf xs, pick t = some (hd t))
    (he : ∀ t ∈ thingsOf xs, ev (hd t) = e t) :
    f.select ev (wholeHeads xs) = f.overThings pick e (thingsOf xs) := by
  rw [Acc.select_set_decides f ev (wholeHeads_distinct hx) (whole_distinct hx hh) (whole_wholeHeads hx hh)]
  simp only [Acc.select, Acc.overThings]
  have hy : ((thingsOf xs).map hd).filter (ev · == .yes) =
      ((thingsOf xs).filter (e · == .yes)).map hd := by
    rw [filter_map]
    congr 1
    exact filter_congr fun t ht => by simp [he t ht]
  have hsub : ∀ t ∈ (thingsOf xs).filter (e · == .yes), t ∈ thingsOf xs := fun t ht => (mem_filter.1 ht).1
  rw [hy, filterMap_some_eq fun t ht => hpick t (hsub t ht)]
  congr 1
  rw [length_filter_map_on (q := fun t => e t == .unknown) fun t ht => by simp [he t ht],
    length_filter_map_on (q := fun t => !(pick t).any f.readable) fun t ht => by simp [hpick t (hsub t ht)]]

/-- Collect over the heads, one per thing, lists what collect over those things lists. -/
theorem collectOf_as_things (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) (sel : Record → Bool) (cs : List Cond)
    (limit : Option Nat) (fl : Option Follow) (e : String → Tri)
    (he : ∀ t ∈ thingsOf xs, e t = if chosen sel cs (hd t) then payTest cs (hd t) else .no) :
    collectOf sel cs true limit xs = keyedValue (.collect fl limit) xs e := by
  unfold collectOf keyedValue
  simp only [↓reduceIte, whole_heads_flat hh]
  have hP : Addressed ((thingsOf xs).map hd) := (whole_distinct hx hh).addressed
  have hc : Addressed (((thingsOf xs).map hd).filter (chosen sel cs)) := hP.filter _
  have hL : Addressed (((thingsOf xs).filter (e · == .yes)).flatMap (liveIn xs)) := fun a ha b hb =>
    hx a (by obtain ⟨_, _, ha⟩ := mem_flatMap.1 ha; exact liveIn_sub ha) b
      (by obtain ⟨_, _, hb⟩ := mem_flatMap.1 hb; exact liveIn_sub hb)
  have hliv : ∀ t ∈ thingsOf xs, ∀ q, q ∈ liveIn xs t ↔ q = hd t := fun t ht q => by
    unfold liveIn
    rw [mem_filter]
    constructor
    · rintro ⟨hq, hl⟩; exact (whole_live_iff hx hw hh ht hq).1 hl
    · rintro rfl
      exact ⟨(whole_hd_mem hx hh ht).1, (whole_live_iff hx hw hh ht (whole_hd_mem hx hh ht).1).2 rfl⟩
  have hyes : ∀ t ∈ thingsOf xs, e t = .yes ↔ chosen sel cs (hd t) = true ∧ payTest cs (hd t) = .yes :=
    fun t ht => by rw [he t ht]; split <;> simp_all
  congr 1
  · congr 2
    apply sorted_ext
      (((LatestBy.inv_of hc.unlinked).heads_sorted).filter _)
      ((LatestBy.inv_of hL.unlinked).heads_sorted)
    intro r
    change r ∈ (newestFirst _).filter _ ↔ r ∈ newestFirst _
    rw [mem_filter, mem_newestFirst_iff hc, mem_newestFirst_iff hL]
    simp only [mem_filter, mem_map, mem_flatMap, beq_iff_eq]
    constructor
    · rintro ⟨⟨q, ⟨⟨t, ht, rfl⟩, hch⟩, rfl⟩, hp⟩
      rw [payTest_unlinked] at hp
      exact ⟨hd t, ⟨t, ⟨ht, (hyes t ht).2 ⟨hch, hp⟩⟩, (hliv t ht _).2 rfl⟩, rfl⟩
    · rintro ⟨q, ⟨t, ⟨ht, hy⟩, hq⟩, rfl⟩
      rw [(hliv t ht q).1 hq]
      obtain ⟨hch, hp⟩ := (hyes t ht).1 hy
      exact ⟨⟨hd t, ⟨⟨t, ht, rfl⟩, hch⟩, rfl⟩, by rw [payTest_unlinked]; exact hp⟩
  · rw [newestFirst_unread cs hc, filter_filter]
    have hd' : Distinct ((((thingsOf xs).map hd).filter fun r => payTest cs r == .unknown && chosen sel cs r)) :=
      pairwise_map.2 ((pairwise_map.1 (whole_distinct hx hh)).sublist filter_sublist)
    rw [idsOf_length hd']
    apply length_filter_map_on
    intro t ht
    rw [he t ht]
    cases chosen sel cs (hd t) <;> simp

/-- **The core**: over records all read whole with one head per thing, a slot
without `by` that follows `keys`, selecting things by `e`, gives what the same
slot following `prev` gives, when `e` selects each thing as `from` and the
`where` of the slot select its head. -/
theorem keyedValue_as_prev (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) (s : Slot) (hty : ∀ f, s.type ≠ .latestBy f)
    (sel : Record → Bool) (e : String → Tri)
    (he : ∀ t ∈ thingsOf xs, e t = if chosen sel s.tests.conds (hd t) then payTest s.tests.conds (hd t) else .no) :
    keyedValue (s.type.following .keys) xs e = ({ s with type := s.type.following .prev } : Slot).plain sel xs := by
  have hev : ∀ t ∈ thingsOf xs, (fun r => if sel r then Cond.all s.tests.conds r else Tri.no) (hd t) = e t :=
    fun t ht => by simp only; rw [ev_chosen, he t ht]
  rcases s with ⟨name, type, tests, group⟩
  simp only at hty he hev
  cases type with
  | count f =>
    simp only [FoldType.following, Slot.plain, accPool, Option.some.injEq, reduceCtorEq, ↓reduceIte]
    rw [Acc.select_as_things Count.acc _ hx hh (fun t => some (hd t)) e (fun _ _ => rfl) hev]
    have ht : ∀ L : List Record, L.filter (fun _ => true) = L := fun L => filter_eq_self.2 fun _ _ => rfl
    simp [keyedValue, Acc.overThings, Count.acc, Acc.fold, ← Count.fold.eq_1, Count.fold_eq, ht]
  | sum f _ =>
    simp only [FoldType.following, Slot.plain, accPool, Option.some.injEq, reduceCtorEq, ↓reduceIte, keyedValue]
    rw [Acc.select_as_things _ _ hx hh _ e (fun t ht => whole_soleSetter hx hw hh ht f) hev]
  | min f _ =>
    simp only [FoldType.following, Slot.plain, accPool, Option.some.injEq, reduceCtorEq, ↓reduceIte, keyedValue]
    rw [Acc.select_as_things _ _ hx hh _ e (fun t ht => whole_soleSetter hx hw hh ht f) hev]
  | max f _ =>
    simp only [FoldType.following, Slot.plain, accPool, Option.some.injEq, reduceCtorEq, ↓reduceIte, keyedValue]
    rw [Acc.select_as_things _ _ hx hh _ e (fun t ht => whole_soleSetter hx hw hh ht f) hev]
  | latestBy f => exact absurd rfl (hty f)
  | collect _ limit =>
    simp only [FoldType.following, Slot.plain, Option.some.injEq, reduceCtorEq, ↓reduceIte, decide_true]
    exact (collectOf_as_things hx hw hh sel tests.conds limit _ e he).symm

theorem canonSet_eq_nil {l : List String} : canonSet l = [] ↔ l = [] := by
  simp only [eq_nil_iff_forall_not_mem, mem_canonSet]

/-- Read whole and unforked, the keys of a thing whose head is chosen are its head's. -/
theorem whole_thingKeys (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t]) {t : String} (ht : t ∈ thingsOf xs)
    (b : By) (sel : Record → Bool) (cs : List Cond) (hc : chosen sel cs (hd t) = true) :
    b.thingKeys sel cs xs t = (b.keys (hd t)).map canonSet := by
  have hsrc : ∀ o : Option (List String),
      (if [hd t].all (fun _ => o.isSome) then some (canonSet ([hd t].flatMap fun _ => o.getD [])) else none) =
        o.map canonSet := fun o => by cases o <;> simp
  cases b <;> simp only [By.thingKeys, whole_setters hx hw hh ht, hh t ht, filter_cons, hc, ↓reduceIte,
    filter_nil] <;> exact hsrc _

theorem whole_length_filter (hx : Addressed xs) (hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t])
    {p : Record → Bool} {q : String → Bool} (h : ∀ t ∈ thingsOf xs, p (hd t) = q t) :
    ((wholeHeads xs).filter p).length = ((thingsOf xs).filter q).length := by
  rw [((perm_of_same (wholeHeads_distinct hx) (whole_distinct hx hh) (whole_wholeHeads hx hh)).filter p).length_eq]
  exact length_filter_map_on h

/-- A slot without `by` that follows `keys` is `keyedValue` over the things it selects. -/
theorem Slot.plain_keys (s : Slot) (hty : ∀ f, s.type ≠ .latestBy f) (sel : Record → Bool) (xs : List Record) :
    ({ s with type := s.type.following .keys } : Slot).plain sel xs =
      keyedValue (s.type.following .keys) xs (thingEv sel s.tests.conds xs) := by
  rcases s with ⟨_, ty, _, _⟩
  cases ty with
  | latestBy f => exact absurd rfl (hty f)
  | _ => rfl

/-- **Read whole and unforked, `keys` is `prev`.** When every record is read
whole and every thing has one head, a count, sum, min, max or collect that
follows `keys` gives what the same slot following `prev` gives, grouped or not:
each thing's live records are its one head, each field's live setter is that
head, and the envelope is read from it. Latest-by is outside, since following
`keys` it gives each thing's keyed state. -/
theorem Slot.keys_as_prev (hx : Addressed xs) (hw : ∀ r ∈ xs, r.sets = none)
    (hu : ∀ t ∈ thingsOf xs, ∃ h, (stateIn t xs).heads = [h]) (s : Slot) (hty : ∀ f, s.type ≠ .latestBy f)
    (sel : Record → Bool) :
    ({ s with type := s.type.following .keys } : Slot).run sel xs =
      ({ s with type := s.type.following .prev } : Slot).run sel xs := by
  let hd : String → Record := fun t => ((stateIn t xs).heads.head?).getD default
  have hh : ∀ t ∈ thingsOf xs, (stateIn t xs).heads = [hd t] := fun t ht => by
    obtain ⟨h, e⟩ := hu t ht
    simp only [hd, e, head?_cons, Option.getD_some]
  have hE : ∀ t ∈ thingsOf xs, thingEv sel s.tests.conds xs t =
      if chosen sel s.tests.conds (hd t) then payTest s.tests.conds (hd t) else .no :=
    fun t ht => whole_thingEv hx hw hh ht sel _
  rcases s with ⟨name, ty, tests, group⟩
  simp only at hty hE
  unfold Slot.run
  cases group with
  | none =>
    exact (Slot.plain_keys ⟨name, ty, tests, none⟩ hty sel xs).trans
      (keyedValue_as_prev hx hw hh ⟨name, ty, tests, none⟩ hty sel _ hE)
  | some b =>
    let sp : Slot := ⟨name, ty.following .prev, tests, some b⟩
    simp only [FoldType.following_follow, Option.some.injEq, reduceCtorEq, ↓reduceIte]
    rw [Slot.keyedGrouped_eq, Slot.grouped_eq]
    have hpool : sp.pool xs = wholeHeads xs := by
      cases ty <;> first | rfl | exact absurd rfl (hty _)
    rw [hpool]
    have hEv : ∀ t ∈ thingsOf xs, sp.ev sel (hd t) = thingEv sel tests.conds xs t := fun t ht => by
      simp only [Slot.ev, sp]; rw [ev_chosen, hE t ht]
    have hch : ∀ t ∈ thingsOf xs, thingEv sel tests.conds xs t = .yes → chosen sel tests.conds (hd t) = true :=
      fun t ht e => by rw [hE t ht] at e; split at e <;> simp_all
    have hK : ∀ t ∈ thingsOf xs, thingEv sel tests.conds xs t = .yes →
        b.thingKeys sel tests.conds xs t = (b.keys (hd t)).map canonSet :=
      fun t ht e => whole_thingKeys hx hw hh ht b sel _ (hch t ht e)
    have hmem : ∀ (o : Option (List String)) (g : String), g ∈ (o.map canonSet).getD [] ↔ g ∈ o.getD [] :=
      fun o g => by cases o <;> simp [mem_canonSet]
    have hIn : ∀ g, ∀ t ∈ thingsOf xs, (thingEv sel tests.conds xs t == .yes &&
        ((b.thingKeys sel tests.conds xs t).getD []).contains g) = inGroup b (sp.ev sel) g (hd t) := by
      intro g t ht
      unfold inGroup
      rw [hEv t ht]
      cases e : thingEv sel tests.conds xs t
      · rw [hK t ht e]
        apply Bool.eq_iff_iff.2
        simp only [beq_self_eq_true, Bool.true_and, contains_iff_mem, hmem]
      · rfl
      · rfl
    congr 1
    · have hG : canonSet (((thingsOf xs).filter (thingEv sel tests.conds xs · == .yes)).flatMap
          fun t => (b.thingKeys sel tests.conds xs t).getD []) = groupKeys b (sp.ev sel) (wholeHeads xs) := by
        unfold groupKeys
        apply canonSet_eq
        intro g
        simp only [mem_flatMap, mem_filter, beq_iff_eq]
        constructor
        · rintro ⟨t, ⟨ht, he⟩, hgk⟩
          refine ⟨hd t, ⟨(whole_wholeHeads hx hh _).2 (mem_map_of_mem ht), by rw [hEv t ht, he]⟩, ?_⟩
          rw [hK t ht he, hmem] at hgk; exact hgk
        · rintro ⟨r, ⟨hr, he⟩, hgk⟩
          obtain ⟨t, ht, rfl⟩ := mem_map.1 ((whole_wholeHeads hx hh r).1 hr)
          rw [hEv t ht] at he
          exact ⟨t, ⟨ht, he⟩, by rw [hK t ht he, hmem]; exact hgk⟩
      rw [hG]
      apply map_congr_left
      intro g _
      congr 2
      refine keyedValue_as_prev hx hw hh ⟨name, ty, [], none⟩ hty (inGroup b (sp.ev sel) g) _ ?_
      intro t ht
      have h0 : Tests.conds [] = [] := rfl
      simp only [h0, chosen, payTest, filter_nil, Cond.all, foldr_nil, beq_self_eq_true, Bool.and_true]
      simp only [hIn g t ht]
    · congr 1
      · exact (whole_length_filter hx hh fun t ht => by rw [hEv t ht]).symm
      · refine (whole_length_filter hx hh fun t ht => ?_).symm
        rw [hEv t ht]
        cases e : thingEv sel tests.conds xs t
        · rw [hK t ht e]; cases b.keys (hd t) <;> rfl
        · rfl
        · rfl
    · refine (whole_length_filter hx hh fun t ht => ?_).symm
      rw [hEv t ht]
      cases e : thingEv sel tests.conds xs t
      · rw [hK t ht e]
        cases b.keys (hd t) with
        | none => rfl
        | some l =>
          apply Bool.eq_iff_iff.2
          simp [canonSet_eq_nil]
      · rfl
      · rfl

end WholeRead

end Loops

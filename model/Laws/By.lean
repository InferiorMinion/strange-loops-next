import Laws.Spec

/-!
# Grouping with `by`

A slot with `by` (`Slot.grouped`) splits what its slot without `by` selects
into groups by a key read from each record alone. Settling, admission and the
whole-reader heads come first, over everything held, and each group runs the
slot without `by` over all of it, choosing only its own records
(`Slot.grouped_eq`). The set decides each group's answer (`Slot.grouped_set_decides`,
in `Laws.Spec`), and a spec slice answers for a grouped slot
(`Slot.grouped_slice`, in `Laws.SpecSlice`). This file says where every
selected record goes:

* **Accounting.** A record the slot selects, or whose `where` is unknown, is in
  exactly one of: grouped (it has at least one key), ungrouped (its key is
  readably absent), or unread (its `where` is unknown, or its key can't be
  read) (`By.accounting`).
* **Memberships.** The groups hold each grouped record once for every key it
  has (`By.memberships`): a record mentioning two projects is in both, once in
  each. So the groups add up to memberships, not records; with one key per
  record, as every `by` but `env.about:<prefix>` gives, they add up to records
  (`By.accounting_single`).
* **No empty groups.** A group exists only where something was selected into
  it (`groupKeys_nonempty`).
* **Partition.** A grouped count's groups, ungrouped and unread add up to the
  plain count's value and unread when each record has one key
  (`By.count_partition`), and its groups to its keyed memberships in any case
  (`By.count_memberships`).
-/

namespace Loops

open List

/-! ## Keys -/

theorem canonSet_nodup (xs : List String) : (canonSet xs).Nodup := by
  have s : (canonSet xs).Pairwise fun a b => lexLt (utf16 a) (utf16 b) = true := sortBy_sorted utf16 xs
  exact s.imp fun h e => by subst e; rw [lexLt_irrefl] at h; exact absurd h (by simp)

/-- A record joins each of its groups once. -/
theorem By.keys_nodup (b : By) (r : Record) : ((b.keys r).getD []).Nodup := by
  cases b with
  | kind | observer => simp [By.keys]
  | about p => simpa [By.keys] using canonSet_nodup _
  | payload f =>
    simp only [By.keys]
    split
    · cases (r.get f).bind Val.key <;> simp
    · simp

/-- Every `by` but `env.about:<prefix>` gives a record at most one key. -/
def By.Single : By → Prop
  | .about _ => False
  | _ => True

theorem By.keys_single {b : By} (hb : b.Single) (r : Record) : ((b.keys r).getD []).length ≤ 1 := by
  cases b with
  | kind | observer => simp [By.keys]
  | about p => cases hb
  | payload f =>
    simp only [By.keys]
    split
    · cases (r.get f).bind Val.key <;> simp
    · simp

/-! ## No empty groups -/

/-- **A group exists only where something was selected into it.** -/
theorem groupKeys_nonempty {b : By} {ev : Record → Tri} {pool : List Record} {g : String}
    (h : g ∈ groupKeys b ev pool) : ∃ r ∈ pool, inGroup b ev g r = true := by
  unfold groupKeys at h
  obtain ⟨r, hr, hg⟩ := mem_flatMap.1 (mem_canonSet.1 h)
  obtain ⟨hp, he⟩ := mem_filter.1 hr
  exact ⟨r, hp, by simp only [inGroup, Bool.and_eq_true]; exact ⟨he, contains_iff_mem.2 hg⟩⟩

/-! ## Where every selected record goes -/

/-- **Accounting.** What the slot reads and does not refuse is the grouped,
the ungrouped and the unread, each record in exactly one. -/
theorem By.accounting (b : By) (ev : Record → Tri) (pool : List Record) :
    (pool.filter (ev · != .no)).length =
      (pool.filter fun r => ev r == .yes && ((b.keys r).getD []) != []).length +
      (pool.filter fun r => ev r == .yes && b.keys r == some []).length +
      ((pool.filter (ev · == .unknown)).length +
        (pool.filter fun r => ev r == .yes && (b.keys r).isNone).length) := by
  induction pool with
  | nil => rfl
  | cons r rs ih =>
    simp only [filter_cons]
    cases he : ev r <;> rcases hk : b.keys r with _ | (_ | ⟨k, ks⟩) <;> simp_all <;> omega

/-- Counting a list of keys against the groups: a record whose keys are all
among the groups is counted once per key. -/
theorem count_keys {G ks : List String} (hG : G.Nodup) (hk : ks.Nodup) (hs : ∀ k ∈ ks, k ∈ G) :
    (G.filter (ks.contains ·)).length = ks.length := by
  apply Perm.length_eq
  apply (perm_ext_iff_of_nodup (hG.sublist filter_sublist) hk).2
  intro k
  simp only [mem_filter, contains_iff_mem]
  exact ⟨fun h => h.2, fun h => ⟨hs k h, h⟩⟩

theorem sum_map_add' {α : Type} (l : List α) (f g : α → Nat) :
    (l.map fun a => f a + g a).sum = (l.map f).sum + (l.map g).sum := by
  induction l with
  | nil => rfl
  | cons a l ih => simp only [map_cons, sum_cons, ih]; omega

theorem sum_map_ite_one {α : Type} (l : List α) (p : α → Bool) :
    (l.map fun a => if p a then 1 else 0).sum = (l.filter p).length := by
  induction l with
  | nil => rfl
  | cons a l ih => cases h : p a <;> simp [h, ih] <;> omega

/-- What a test does not refuse is what it selects and what it cannot answer for. -/
theorem length_ne_no (ev : Record → Tri) (xs : List Record) :
    (xs.filter (ev · != .no)).length =
      (xs.filter (ev · == .yes)).length + (xs.filter (ev · == .unknown)).length := by
  induction xs with
  | nil => rfl
  | cons r rs ih => cases h : ev r <;> simp [h, ih] <;> omega

/-- **Memberships.** The groups hold each selected record once for each of its
keys: summed over the groups, the group sizes are the number of keys of every
selected record. -/
theorem By.memberships (b : By) (ev : Record → Tri) (pool : List Record) :
    ((groupKeys b ev pool).map fun g => (pool.filter (inGroup b ev g)).length).sum =
      ((pool.filter (ev · == .yes)).map fun r => ((b.keys r).getD []).length).sum := by
  generalize hG : groupKeys b ev pool = G
  have hnd : G.Nodup := hG ▸ canonSet_nodup _
  have hsub : ∀ r ∈ pool, ev r = .yes → ∀ k ∈ (b.keys r).getD [], k ∈ G := by
    intro r hr he k hk
    rw [← hG]
    unfold groupKeys
    exact mem_canonSet.2 (mem_flatMap.2 ⟨r, mem_filter.2 ⟨hr, by simp [he]⟩, hk⟩)
  clear hG
  induction pool with
  | nil => simp only [filter_nil, length_nil, map_nil, sum_nil]; induction G <;> simp_all
  | cons r rs ih =>
    have ih := ih fun q hq => hsub q (mem_cons_of_mem _ hq)
    have step : ∀ g, ((r :: rs).filter (inGroup b ev g)).length =
        (if inGroup b ev g r then 1 else 0) + (rs.filter (inGroup b ev g)).length := by
      intro g; cases h : inGroup b ev g r <;> simp [h] <;> omega
    simp only [step, sum_map_add', ih, sum_map_ite_one]
    cases he : ev r
    · simp only [filter_cons, he, beq_self_eq_true, ↓reduceIte, map_cons, sum_cons]
      have : G.filter (inGroup b ev · r) = G.filter (((b.keys r).getD []).contains ·) := by
        apply filter_congr; intro g _; simp [inGroup, he]
      rw [this, count_keys hnd (b.keys_nodup r) (hsub r mem_cons_self he)]
    all_goals
      have : G.filter (inGroup b ev · r) = [] := by
        apply filter_eq_nil_iff.2; intro g _; simp [inGroup, he]
      simp [this, he]

/-- With one key per record, a record's keys count it once if it has one. -/
theorem sum_keys_single {b : By} (hb : b.Single) (ev : Record → Tri) (pool : List Record) :
    ((pool.filter (ev · == .yes)).map fun r => ((b.keys r).getD []).length).sum =
      (pool.filter fun r => ev r == .yes && ((b.keys r).getD []) != []).length := by
  induction pool with
  | nil => rfl
  | cons r rs ih =>
    have h1 := b.keys_single hb r
    revert h1
    cases he : ev r <;> rcases hk : (b.keys r).getD [] with _ | ⟨k, _ | ⟨k', ks⟩⟩ <;>
      simp [he, hk, ih] <;> omega

/-- With one key per record, the groups, the ungrouped and the unread add up to
the records the slot reads and does not refuse. -/
theorem By.accounting_single {b : By} (hb : b.Single) (ev : Record → Tri) (pool : List Record) :
    ((groupKeys b ev pool).map fun g => (pool.filter (inGroup b ev g)).length).sum +
      (pool.filter fun r => ev r == .yes && b.keys r == some []).length +
      ((pool.filter (ev · == .unknown)).length +
        (pool.filter fun r => ev r == .yes && (b.keys r).isNone).length) =
      (pool.filter (ev · != .no)).length := by
  rw [By.accounting b ev pool, By.memberships, sum_keys_single hb]

/-! ## The slot's own counts -/

/-- **A grouped slot, written out.** Its groups are its keys, each with the
slot without `by` or `where` run over everything held, choosing only that
group's records; then the unread and the ungrouped. -/
theorem Slot.grouped_eq (s : Slot) (b : By) (sel : Record → Bool) (xs : List Record) :
    s.grouped b sel xs =
      .groups ((groupKeys b (s.ev sel) (s.pool xs)).map fun g =>
          (g, ({ s with tests := [], group := none } : Slot).plain (inGroup b (s.ev sel) g) xs |>.group))
        ((s.pool xs |>.filter (s.ev sel · == .unknown)).length +
          (s.pool xs |>.filter fun r => s.ev sel r == .yes && (b.keys r).isNone).length)
        (s.pool xs |>.filter fun r => s.ev sel r == .yes && b.keys r == some []).length := rfl

/-- A group's count: what its count slot holds. -/
def GroupValue.count : GroupValue → Nat
  | .nat r => r.value
  | _ => 0

theorem Count.select_value (ev : Record → Tri) (xs : List Record) :
    (Count.acc.select ev xs).value = (xs.filter (ev · == .yes)).length := by
  simp [Acc.select, Count.acc, Acc.fold, ← Count.fold.eq_1, Count.fold_eq]

/-- A count group holds the number of records selected into it. -/
theorem count_group (f : Bool) (s : Slot) (b : By) (sel : Record → Bool) (xs : List Record) (g : String)
    (hs : s.type = .count f) :
    (({ s with tests := [], group := none } : Slot).plain (inGroup b (s.ev sel) g) xs).group.count =
      ((s.pool xs).filter (inGroup b (s.ev sel) g)).length := by
  simp only [Slot.plain, hs, Slot.pool, SlotValue.group, GroupValue.count, Count.select_value]
  congr 1
  apply filter_congr
  intro r _
  cases inGroup b (s.ev sel) g r <;> simp [Tests.conds, Cond.all]

/-- **A grouped count's groups hold its memberships.** -/
theorem By.count_memberships {f : Bool} {s : Slot} {b : By} (sel : Record → Bool) (xs : List Record)
    (hs : s.type = .count f) :
    ∃ gs u m, s.grouped b sel xs = .groups gs u m ∧
      (gs.map (·.2.count)).sum =
        (((s.pool xs).filter (s.ev sel · == .yes)).map fun r => ((b.keys r).getD []).length).sum := by
  refine ⟨_, _, _, Slot.grouped_eq s b sel xs, ?_⟩
  rw [← By.memberships, map_map]
  congr 1
  exact map_congr_left fun g _ => count_group f s b sel xs g hs

/-- **Partition.** With one key per record, a grouped count's groups,
ungrouped and unread add up to the value and unread of the same count without
`by`: the groups partition what the plain count selects, and grouping loses and
invents nothing. -/
theorem By.count_partition {f : Bool} {s : Slot} {b : By} (hb : b.Single) (sel : Record → Bool)
    (xs : List Record) (hs : s.type = .count f) :
    ∃ gs u m r, s.grouped b sel xs = .groups gs u m ∧ ({ s with group := none } : Slot).run sel xs = .nat r ∧
      (gs.map (·.2.count)).sum + m + u = r.value + r.unread := by
  refine ⟨_, _, _, _, Slot.grouped_eq s b sel xs, by simp only [Slot.run, Slot.plain, hs]; rfl, ?_⟩
  simp only [map_map]
  rw [show ((groupKeys b (s.ev sel) (s.pool xs)).map ((·.2.count) ∘ fun g =>
      (g, (({ s with tests := [], group := none } : Slot).plain (inGroup b (s.ev sel) g) xs).group))) =
      (groupKeys b (s.ev sel) (s.pool xs)).map fun g => ((s.pool xs).filter (inGroup b (s.ev sel) g)).length
    from map_congr_left fun g _ => count_group f s b sel xs g hs]
  rw [By.accounting_single hb, Count.select_value, Count.select_unread]
  simp only [Slot.pool, hs]
  exact length_ne_no _ _

end Loops

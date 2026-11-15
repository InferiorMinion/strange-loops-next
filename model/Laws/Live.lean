import Loops.Spec
import Laws.Families
import Laws.Things

/-!
# Laws of live setters

* **Above is reachability over `prev`.** A record is above another when it
  names it, or names a record above it, through the records held
  (`mem_upward`, `mem_above`). So above depends only on which records are
  held (`above_congr`).
* **Live for the envelope is a head.** Nothing names it (`liveEnv_iff`), so
  the heads of a thing are exactly its records live for the envelope
  (`mem_stateIn_heads_iff`), and every head is live (`head_live`).
* **Live is live for some key** (`live_iff`): a record is live exactly when
  it is a head or live for some key, a key it does not declare among them
  when it is read whole.
* **The keyed state is exact** (`keyedIn_keys`, `keyedIn_setters`,
  `keyedIn_whole`, `keyedIn_heads`): an entry for every key some live record
  declares and no other, each holding exactly that key's live setters; the
  live whole-read emits; the heads. A pure join is in none of the first two
  lists (`pureJoin_only_heads`). For a thing whose records are all read whole
  the keyed state is the heads twice and no key (`keyedIn_whole_read`).
* **The set decides** a thing's keyed state (`keyedIn_set_decides`) and the
  slot that reads it (`keyedOf_set_decides`).
-/

namespace Loops

open List

/-! ## Above -/

/-- Id `i` is reached from `ids` upward through `rs`: it is in `ids`, or it is
the id of a record of `rs` that names an id reached. -/
inductive Reach (rs : List Record) (ids : List RecordId) : RecordId → Prop
  | base {i : RecordId} : i ∈ ids → Reach rs ids i
  | up {q : Record} {p : RecordId} : q ∈ rs → p ∈ q.prev → Reach rs ids p → Reach rs ids q.id

theorem Reach.mono {rs : List Record} {ids ids' : List RecordId} (h : ∀ i ∈ ids, i ∈ ids')
    {i : RecordId} (hr : Reach rs ids i) : Reach rs ids' i := by
  induction hr with
  | base hi => exact .base (h _ hi)
  | up hq hp _ ih => exact .up hq hp ih

theorem namesIn_iff {ids : List RecordId} {q : Record} :
    namesIn ids q = true ↔ ∃ p ∈ q.prev, p ∈ ids := by
  simp [namesIn]

/-- `upward` gives exactly the ids reached. -/
theorem mem_upward {rs : List Record} {ids : List RecordId} {i : RecordId} :
    i ∈ upward rs ids ↔ Reach rs ids i := by
  fun_induction upward rs ids with
  | case1 ids hf =>
    constructor
    · exact .base
    · intro hr
      induction hr with
      | base hi => exact hi
      | @up q p hq hp _ ih =>
        have hn := find?_eq_none.1 hf q hq
        have hq' : namesIn ids q = true := namesIn_iff.2 ⟨p, hp, ih⟩
        by_cases hc : q.id ∈ ids
        · exact hc
        · exact absurd (by simp [hc, hq']) hn
  | case2 ids q hf ih =>
    rw [ih]
    clear ih
    have hq := mem_of_find?_eq_some hf
    have hp := find?_some hf
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at hp
    constructor
    · intro hr
      induction hr with
      | base hi =>
        rcases mem_cons.1 hi with rfl | hi
        · obtain ⟨p, hp, hpi⟩ := namesIn_iff.1 hp.2
          exact .up hq hp (.base hpi)
        · exact .base hi
      | up hq' hp' _ ih' => exact .up hq' hp' ih'
    · exact Reach.mono fun i hi => mem_cons_of_mem _ hi

theorem mem_above {rs : List Record} {r q : Record} :
    q ∈ above rs r ↔ q ∈ rs ∧ ∃ p ∈ q.prev, Reach rs [r.id] p := by
  simp only [above, mem_filter, namesIn_iff, mem_upward]

theorem Reach.congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') {ids : List RecordId}
    {i : RecordId} (hr : Reach rs ids i) : Reach rs' ids i := by
  induction hr with
  | base hi => exact .base hi
  | up hq hp _ ih => exact .up ((h _).1 hq) hp ih

/-- What is above a record depends only on which records are held. -/
theorem above_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (r q : Record) :
    q ∈ above rs r ↔ q ∈ above rs' r := by
  rw [mem_above, mem_above, h]
  constructor
  · rintro ⟨hq, p, hp, hr⟩; exact ⟨hq, p, hp, hr.congr h⟩
  · rintro ⟨hq, p, hp, hr⟩; exact ⟨hq, p, hp, hr.congr fun q => (h q).symm⟩

/-- A record reached from `r` is `r`, or something names `r`. -/
theorem Reach.namer {rs : List Record} {r : Record} {i : RecordId} (hr : Reach rs [r.id] i) :
    i = r.id ∨ ∃ q ∈ rs, r.id ∈ q.prev := by
  induction hr with
  | base hi => exact .inl (mem_singleton.1 hi)
  | up hq hp _ ih =>
    rcases ih with rfl | h
    · exact .inr ⟨_, hq, hp⟩
    · exact .inr h

/-! ## Liveness -/

/-- A setter of `k` is live for `k` exactly when no record above it sets `k`. -/
theorem liveFor_iff {rs : List Record} {k : String} {r : Record} :
    liveFor rs k r = true ↔ r.setsKey k = true ∧ ∀ q ∈ above rs r, q.setsKey k = false := by
  simp [liveFor]

/-- Live for the envelope exactly when no record held names it: a head. -/
theorem liveEnv_iff {rs : List Record} {r : Record} :
    liveEnv rs r = true ↔ ¬∃ q ∈ rs, r.id ∈ q.prev := by
  simp only [liveEnv, isEmpty_iff, eq_nil_iff_forall_not_mem]
  constructor
  · rintro h ⟨q, hq, hp⟩
    exact h q (mem_above.2 ⟨hq, r.id, hp, .base (mem_singleton_self _)⟩)
  · intro h q hq
    obtain ⟨-, p, hp, hr⟩ := mem_above.1 hq
    rcases hr.namer with rfl | hn
    · exact h ⟨q, (mem_above.1 hq).1, hp⟩
    · exact h hn

theorem liveFor_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (k : String) (r : Record) :
    liveFor rs k r = liveFor rs' k r := by
  apply Bool.eq_iff_iff.2
  simp only [liveFor_iff]
  exact and_congr_right fun _ => forall_congr' fun q => imp_congr_left (above_congr h r q)

theorem liveEnv_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (r : Record) :
    liveEnv rs r = liveEnv rs' r := by
  apply Bool.eq_iff_iff.2
  simp only [liveEnv_iff, h]

theorem live_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (r : Record) :
    live rs r = live rs' r := by
  unfold live
  rw [liveEnv_congr h r]
  congr 1
  split
  · congr 1
    apply Bool.eq_iff_iff.2
    simp only [any_eq_true, above_congr h r]
  · exact congrArg _ (funext fun k => liveFor_congr h k r)

theorem length_le_sum {L : List String} {s : String} (hs : s ∈ L) :
    s.length ≤ (L.map String.length).sum := by
  induction L with
  | nil => cases hs
  | cons a L ih =>
    rcases mem_cons.1 hs with rfl | hs
    · simp
    · have := ih hs; simp only [map_cons, sum_cons]; omega

/-- A string no member of `L` is. -/
theorem exists_fresh (L : List String) : ∃ k, k ∉ L := by
  refine ⟨String.ofList (List.replicate ((L.map String.length).sum + 1) 'a'), fun hm => ?_⟩
  have := length_le_sum hm
  simp [String.length_ofList] at this
  omega

/-- **A record is live exactly when it is live for some key, the envelope among
them.** A whole-read record sets every key, and the records above it declare
finitely many, so it is live for a key they leave out unless one of them is
read whole too. -/
theorem live_iff {rs : List Record} {r : Record} :
    live rs r = true ↔ liveEnv rs r = true ∨ ∃ k, liveFor rs k r = true := by
  unfold live
  simp only [Bool.or_eq_true]
  apply or_congr_right
  split
  · rename_i hs
    simp only [Bool.not_eq_true', any_eq_false, Record.whole, Option.isNone_iff_eq_none]
    constructor
    · intro hw
      obtain ⟨k, hk⟩ := exists_fresh ((above rs r).flatMap Record.declares)
      refine ⟨k, liveFor_iff.2 ⟨by simp [Record.setsKey, hs], fun q hq => ?_⟩⟩
      cases hqs : q.sets with
      | none => exact absurd hqs (hw q hq)
      | some ks =>
        simp only [Record.setsKey, hqs, contains_eq_mem, decide_eq_false_iff_not]
        exact fun hm => hk (mem_flatMap.2 ⟨q, hq, by simp [Record.declares, hqs, hm]⟩)
    · rintro ⟨k, hk⟩ q hq hqs
      have := (liveFor_iff.1 hk).2 q hq
      simp [Record.setsKey, hqs] at this
  · rename_i ks hs
    simp only [any_eq_true]
    constructor
    · rintro ⟨k, -, hk⟩; exact ⟨k, hk⟩
    · rintro ⟨k, hk⟩
      refine ⟨k, ?_, hk⟩
      have := (liveFor_iff.1 hk).1
      simpa [Record.setsKey, hs] using this

/-! ## A thing's keyed state -/

/-- The records of a thing. -/
abbrev recordsOf (t : String) (xs : List Record) : List Record := xs.filter (·.isOf t)

theorem addressed_filter {xs : List Record} (hx : Addressed xs) (p : Record → Bool) :
    Addressed (xs.filter p) := fun a ha b hb => hx a (mem_filter.1 ha).1 b (mem_filter.1 hb).1

theorem addressed_recordsOf {xs : List Record} (hx : Addressed xs) (t : String) :
    Addressed (recordsOf t xs) := addressed_filter hx _

theorem mem_stateIn_heads_iff {xs : List Record} (hx : Addressed xs) {t : String} {r : Record} :
    r ∈ (stateIn t xs).heads ↔ r ∈ recordsOf t xs ∧ liveEnv (recordsOf t xs) r = true := by
  rw [liveEnv_iff]
  exact (LatestBy.inv_of (addressed_recordsOf hx t)).heads_mem r

/-- Every head of a thing is live. -/
theorem head_live {xs : List Record} (hx : Addressed xs) {t : String} {r : Record}
    (h : r ∈ (keyedIn t xs).heads) : live (recordsOf t xs) r = true := by
  have := ((mem_stateIn_heads_iff hx).1 h).2
  simp [live, this]

theorem keyedIn_heads (t : String) (xs : List Record) : (keyedIn t xs).heads = (stateIn t xs).heads := rfl

/-- The keys listed are exactly those some live record of the thing declares. -/
theorem keyedIn_keys {t : String} {xs : List Record} {k : String} :
    k ∈ (keyedIn t xs).keys.map (·.1) ↔
      ∃ r ∈ recordsOf t xs, live (recordsOf t xs) r = true ∧ k ∈ r.declares := by
  simp only [keyedIn, map_map, Function.comp_def, map_id', liveKeys, mem_canonSet, mem_flatMap,
    mem_filter]
  constructor
  · rintro ⟨r, ⟨hr, hl⟩, hk⟩; exact ⟨r, hr, hl, hk⟩
  · rintro ⟨r, hr, hl, hk⟩; exact ⟨r, ⟨hr, hl⟩, hk⟩

/-- Each key's entry holds exactly that key's live setters. -/
theorem keyedIn_setters {t : String} {xs : List Record} (hx : Addressed xs) {k : String}
    {ss : List Record} (h : (k, ss) ∈ (keyedIn t xs).keys) {r : Record} :
    r ∈ ss ↔ r ∈ recordsOf t xs ∧ liveFor (recordsOf t xs) k r = true := by
  simp only [keyedIn, mem_map, Prod.mk.injEq] at h
  obtain ⟨_, -, rfl, rfl⟩ := h
  rw [mem_orderHeads (addressed_filter (addressed_recordsOf hx t) _), mem_filter]

/-- `whole` holds exactly the live whole-read emits. -/
theorem keyedIn_whole {t : String} {xs : List Record} (hx : Addressed xs) {r : Record} :
    r ∈ (keyedIn t xs).whole ↔
      r ∈ recordsOf t xs ∧ r.whole = true ∧ live (recordsOf t xs) r = true := by
  simp only [keyedIn]
  rw [mem_orderHeads (addressed_filter (addressed_recordsOf hx t) _), mem_filter, Bool.and_eq_true]

/-- A pure join, which sets no key, is listed under no key and not as whole:
only among the heads. -/
theorem pureJoin_only_heads {t : String} {xs : List Record} (hx : Addressed xs) {r : Record}
    (hj : r.sets = some []) :
    (∀ e ∈ (keyedIn t xs).keys, r ∉ e.2) ∧ r ∉ (keyedIn t xs).whole := by
  refine ⟨fun ⟨k, ss⟩ he hr => ?_, fun hr => ?_⟩
  · have := (liveFor_iff.1 ((keyedIn_setters hx he).1 hr).2).1
    simp [Record.setsKey, hj] at this
  · have := ((keyedIn_whole hx).1 hr).2.1
    simp [Record.whole, hj] at this

/-- Two lists in content order holding the same records are the same list. -/
theorem sorted_ext {xs ys : List Record} (hx : xs.Pairwise Newer) (hy : ys.Pairwise Newer)
    (h : ∀ r, r ∈ xs ↔ r ∈ ys) : xs = ys := by
  have ne : ∀ {a b : Record}, Newer a b → a ≠ b := fun h e => by subst e; exact Newer.irrefl _ h
  apply Perm.eq_of_pairwise (le := Newer) (fun a b _ _ h₁ h₂ => absurd h₂ h₁.asymm) hx hy
  exact (perm_ext_iff_of_nodup (hx.imp ne) (hy.imp ne)).2 h

/-- **For a thing whose records are all read whole, the keyed state is its
heads twice and no key.** Every record sets every key, so live for a key is
live for the envelope: nothing is above it. -/
theorem keyedIn_whole_read {t : String} {xs : List Record} (hx : Addressed xs)
    (hw : ∀ r ∈ recordsOf t xs, r.sets = none) :
    (keyedIn t xs).keys = [] ∧ (keyedIn t xs).whole = (stateIn t xs).heads ∧
      (keyedIn t xs).heads = (stateIn t xs).heads := by
  refine ⟨?_, ?_, rfl⟩
  · apply eq_nil_iff_forall_not_mem.2
    intro ⟨k, ss⟩ he
    obtain ⟨r, hr, -, hk⟩ := keyedIn_keys.1 (mem_map_of_mem (f := (·.1)) he)
    simp [Record.declares, hw r hr] at hk
  · have hs : (stateIn t xs).heads.Pairwise Newer := (LatestBy.inv_of (addressed_recordsOf hx t)).heads_sorted
    apply sorted_ext (orderHeads_sorted (addressed_filter (addressed_recordsOf hx t) _)) hs
    intro r
    change r ∈ (keyedIn t xs).whole ↔ _
    rw [keyedIn_whole hx, mem_stateIn_heads_iff hx]
    · constructor
      · rintro ⟨hr, -, hl⟩
        refine ⟨hr, ?_⟩
        unfold live at hl
        simp only [hw r hr, Bool.or_eq_true, Bool.not_eq_true', any_eq_false] at hl
        rcases hl with hl | hl
        · exact hl
        · simp only [liveEnv, isEmpty_iff, eq_nil_iff_forall_not_mem]
          intro q hq
          exact hl q hq (by simp [Record.whole, hw q (mem_above.1 hq).1])
      · rintro ⟨hr, hl⟩
        exact ⟨hr, by simp [Record.whole, hw r hr], by simp [live, hl]⟩

/-! ## The set decides -/

theorem orderHeads_set_decides {xs ys : List Record} (hx : Addressed xs) (hy : Addressed ys)
    (h : ∀ r, r ∈ xs ↔ r ∈ ys) : orderHeads xs = orderHeads ys :=
  sorted_ext (orderHeads_sorted hx) (orderHeads_sorted hy) fun r => by
    rw [mem_orderHeads hx, mem_orderHeads hy, h]

theorem recordsOf_same {xs ys : List Record} (h : ∀ r, r ∈ xs ↔ r ∈ ys) (t : String) (r : Record) :
    r ∈ recordsOf t xs ↔ r ∈ recordsOf t ys := by
  simp only [mem_filter, h]

theorem mem_filter_congr {A B : List Record} {p q : Record → Bool} (h : ∀ r, r ∈ A ↔ r ∈ B)
    (hpq : ∀ r, p r = q r) (r : Record) : r ∈ A.filter p ↔ r ∈ B.filter q := by
  simp only [mem_filter, h, hpq]

/-- **A thing's keyed state depends only on the set held.** -/
theorem keyedIn_set_decides {xs ys : List Record} (hx : Addressed xs) (hy : Addressed ys)
    (h : ∀ r, r ∈ xs ↔ r ∈ ys) (t : String) : keyedIn t xs = keyedIn t ys := by
  have hr := recordsOf_same h t
  have ax := addressed_recordsOf hx t
  have ay := addressed_recordsOf hy t
  have hk : liveKeys (recordsOf t xs) = liveKeys (recordsOf t ys) := by
    unfold liveKeys
    apply canonSet_eq
    intro k
    simp only [mem_flatMap, mem_filter, hr, live_congr hr]
  unfold keyedIn
  simp only
  rw [show (xs.filter (·.isOf t)) = recordsOf t xs from rfl, show (ys.filter (·.isOf t)) = recordsOf t ys from rfl, hk,
    stateIn_local hx hy fun r => by rw [h]]
  congr 1
  · exact map_congr_left fun k _ => by
      congr 1
      apply orderHeads_set_decides (addressed_filter ax _) (addressed_filter ay _)
      exact mem_filter_congr hr (liveFor_congr hr k)
  · apply orderHeads_set_decides (addressed_filter ax _) (addressed_filter ay _)
    exact mem_filter_congr hr fun r => by rw [live_congr hr]

theorem Tri.agree_congr {ts ts' : List Tri} (h : ∀ a, a ∈ ts ↔ a ∈ ts') (n : Tri) :
    Tri.agree ts n = Tri.agree ts' n := by
  have he : ts.isEmpty = ts'.isEmpty := by
    apply Bool.eq_iff_iff.2
    simp only [isEmpty_iff, eq_nil_iff_forall_not_mem, h]
  have ha : ∀ b : Tri, (ts.all (· == b)) = (ts'.all (· == b)) := fun b => by
    apply Bool.eq_iff_iff.2
    simp only [all_eq_true, h]
  simp only [Tri.agree, he, ha]

/-- A payload test asked of a thing depends only on which of its records are held. -/
theorem Cond.onThing_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (c : Cond) :
    c.onThing rs = c.onThing rs' := by
  unfold Cond.onThing
  split
  · rfl
  · apply Tri.agree_congr
    intro a
    simp only [mem_map, mem_filter, h, liveFor_congr h]

theorem thingTest_congr {rs rs' : List Record} (h : ∀ q, q ∈ rs ↔ q ∈ rs') (cs : List Cond) :
    thingTest cs rs = thingTest cs rs' := by
  unfold thingTest
  congr 1
  funext c t
  rw [Cond.onThing_congr h]

/-- **Latest-by following `keys` depends only on the set held.** -/
theorem keyedOf_set_decides (sel : Record → Bool) (cs : List Cond) {xs ys : List Record}
    (hx : Addressed xs) (hy : Addressed ys) (h : ∀ r, r ∈ xs ↔ r ∈ ys) :
    keyedOf sel cs xs = keyedOf sel cs ys := by
  unfold keyedOf
  rw [thingsOf_same h]
  have e : ∀ t, thingTest cs (xs.filter (·.isOf t)) = thingTest cs (ys.filter (·.isOf t)) :=
    fun t => thingTest_congr (recordsOf_same h t) cs
  simp only [keyedIn_set_decides hx hy h, e]

/-! ## Selection: heads choose, the thing answers -/

theorem Tri.agree_nil (n : Tri) : Tri.agree [] n = n := rfl

/-- Setters that all give one answer give it. -/
theorem Tri.agree_same {ts : List Tri} {a : Tri} (hne : ts ≠ []) (h : ∀ x ∈ ts, x = a) (n : Tri) :
    Tri.agree ts n = a := by
  unfold Tri.agree
  have he : ts.isEmpty = false := by cases ts <;> simp_all
  have hall : ∀ b, ts.all (· == b) = (a == b) := fun b => by
    obtain ⟨x, hx⟩ := exists_mem_of_ne_nil ts hne
    apply Bool.eq_iff_iff.2
    simp only [all_eq_true, beq_iff_eq]
    constructor
    · intro hb; rw [← h x hx]; exact hb x hx
    · intro e y hy; rw [h y hy]; exact e
  cases a <;> simp [he, hall]

/-- Setters that answer differently answer unknown. -/
theorem Tri.agree_differ {ts : List Tri} {a b : Tri} (ha : a ∈ ts) (hb : b ∈ ts) (hab : a ≠ b)
    (n : Tri) : Tri.agree ts n = .unknown := by
  have he : ts.isEmpty = false := by cases ts <;> simp_all
  unfold Tri.agree
  simp only [he, Bool.false_eq_true, ↓reduceIte]
  have hy : ts.all (· == .yes) = false := by
    simp only [all_eq_false, beq_iff_eq]
    by_cases h : a = .yes
    · exact ⟨b, hb, fun e => hab (h.trans e.symm)⟩
    · exact ⟨a, ha, h⟩
  have hn : ts.all (· == .no) = false := by
    simp only [all_eq_false, beq_iff_eq]
    by_cases h : a = .no
    · exact ⟨b, hb, fun e => hab (h.trans e.symm)⟩
    · exact ⟨a, ha, h⟩
  simp [hy, hn]

/-- A setter that answers unknown makes the answer unknown. -/
theorem Tri.agree_unknown {ts : List Tri} (h : Tri.unknown ∈ ts) (n : Tri) :
    Tri.agree ts n = .unknown := by
  have he : ts.isEmpty = false := by cases ts <;> simp_all
  unfold Tri.agree
  have hy : ts.all (· == .yes) = false := all_eq_false.2 ⟨_, h, by decide⟩
  have hn : ts.all (· == .no) = false := all_eq_false.2 ⟨_, h, by decide⟩
  simp [he, hy, hn]

theorem Cond.onThing_payload {c : Cond} {f : String} (hc : c.field = .payload f) (rs : List Record) :
    c.onThing rs = Tri.agree ((rs.filter (liveFor rs f)).map c.eval) (.ofBool (c.op.test [])) := by
  simp [Cond.onThing, hc]

/-- A record with `sets` that does not declare a field is never asked about it:
it is no setter of it. -/
theorem liveFor_undeclared {rs : List Record} {r : Record} {ks : List String} {f : String}
    (hs : r.sets = some ks) (hf : f ∉ ks) : liveFor rs f r = false := by
  simp [liveFor, Record.setsKey, hs, hf]

/-- **One answer from every live setter is the thing's answer.** -/
theorem Cond.onThing_agree {c : Cond} {f : String} (hc : c.field = .payload f) {rs : List Record}
    {r : Record} (hr : r ∈ rs) (hl : liveFor rs f r = true) {a : Tri}
    (h : ∀ q ∈ rs, liveFor rs f q = true → c.eval q = a) : c.onThing rs = a := by
  rw [Cond.onThing_payload hc]
  apply Tri.agree_same (ne_nil_of_mem (mem_map_of_mem (mem_filter.2 ⟨hr, hl⟩)))
  intro x hx
  obtain ⟨q, hq, rfl⟩ := mem_map.1 hx
  exact h q (mem_filter.1 hq).1 (mem_filter.1 hq).2

/-- **Two live setters that answer differently make the thing's answer unknown.** -/
theorem Cond.onThing_differ {c : Cond} {f : String} (hc : c.field = .payload f) {rs : List Record}
    {q r : Record} (hq : q ∈ rs) (hlq : liveFor rs f q = true) (hr : r ∈ rs)
    (hlr : liveFor rs f r = true) (hd : c.eval q ≠ c.eval r) : c.onThing rs = .unknown := by
  rw [Cond.onThing_payload hc]
  exact Tri.agree_differ (mem_map_of_mem (mem_filter.2 ⟨hq, hlq⟩))
    (mem_map_of_mem (mem_filter.2 ⟨hr, hlr⟩)) hd _

/-- **A live setter whose payload is not held makes the thing's answer unknown**,
whatever the others say. -/
theorem Cond.onThing_unheld {c : Cond} {f : String} (hc : c.field = .payload f) {rs : List Record}
    {r : Record} (hr : r ∈ rs) (hl : liveFor rs f r = true) (hs : r.shown ≠ .here) :
    c.onThing rs = .unknown := by
  rw [Cond.onThing_payload hc]
  apply Tri.agree_unknown
  refine mem_map.2 ⟨r, mem_filter.2 ⟨hr, hl⟩, ?_⟩
  simp [Cond.eval, Field.read, hc, hs]

/-- **A field with no live setter is absent:** the test answers as on a held
payload without the field. -/
theorem Cond.onThing_unset {c : Cond} {f : String} (hc : c.field = .payload f) {rs : List Record}
    (h : ∀ r ∈ rs, liveFor rs f r = false) : c.onThing rs = .ofBool (c.op.test []) := by
  rw [Cond.onThing_payload hc]
  have : rs.filter (liveFor rs f) = [] := filter_eq_nil_iff.2 fun r hr => by simp [h r hr]
  rw [this]; rfl

/-- The things latest-by following `keys` keeps: one of whose heads `from` and
the envelope tests choose, and whose payload tests hold of the thing; each with
its whole keyed state. -/
theorem keyedOf_things {sel : Record → Bool} {cs : List Cond} {xs : List Record}
    {ts : List (String × KeyedView)} {u : Nat} (h : keyedOf sel cs xs = .keyed ts u)
    {t : String} {v : KeyedView} :
    (t, v) ∈ ts ↔ t ∈ thingsOf xs ∧ (∃ hd ∈ (keyedIn t xs).heads, chosen sel cs hd = true) ∧
      thingTest cs (recordsOf t xs) = .yes ∧ v = (keyedIn t xs).view := by
  unfold keyedOf at h
  simp only [SlotValue.keyed.injEq] at h
  obtain ⟨rfl, -⟩ := h
  simp only [mem_map, mem_filter, mem_filterMap, Prod.mk.injEq, beq_iff_eq]
  constructor
  · rintro ⟨⟨t', k, a⟩, ⟨⟨t'', ht'', he⟩, ha⟩, rfl, rfl⟩
    split at he
    · rename_i hc
      simp only [Option.some.injEq, Prod.mk.injEq] at he
      obtain ⟨rfl, rfl, rfl⟩ := he
      exact ⟨ht'', any_eq_true.1 hc, ha, rfl⟩
    · cases he
  · rintro ⟨ht, hc, ha, rfl⟩
    refine ⟨⟨t, keyedIn t xs, thingTest cs (recordsOf t xs)⟩, ⟨⟨t, ht, ?_⟩, ha⟩, rfl, rfl⟩
    rw [ite_eq_left_iff.2 (fun hn => absurd (any_eq_true.2 hc) hn)]

/-- A chosen thing whose payload tests are unknown is unread once. -/
theorem keyedOf_unread {sel : Record → Bool} {cs : List Cond} {xs : List Record}
    {ts : List (String × KeyedView)} {u : Nat} (h : keyedOf sel cs xs = .keyed ts u) :
    u = ((thingsOf xs).filter fun t => (keyedIn t xs).heads.any (chosen sel cs) &&
      thingTest cs (recordsOf t xs) == .unknown).length := by
  unfold keyedOf at h
  simp only [SlotValue.keyed.injEq] at h
  obtain ⟨-, rfl⟩ := h
  induction thingsOf xs with
  | nil => rfl
  | cons t ts ih =>
    simp only [filterMap_cons]
    by_cases hc : (keyedIn t xs).heads.any (chosen sel cs) = true
    · simp only [hc, ↓reduceIte, filter_cons, Bool.true_and]
      have e : recordsOf t xs = xs.filter (·.isOf t) := rfl
      by_cases hu : thingTest cs (xs.filter (·.isOf t)) = .unknown
      · simp only [e, hu, beq_self_eq_true, ↓reduceIte, length_cons, ih]
      · have hu' : (thingTest cs (xs.filter (·.isOf t)) == .unknown) = false := by simpa using hu
        simp only [e, hu', Bool.false_eq_true, ↓reduceIte, ih]
    · simp only [hc, filter_cons, Bool.false_and]
      simpa using ih

end Loops

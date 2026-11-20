import Laws.Live
import Laws.Slice

/-!
# What a part must hold to read a thing by its keys

A reader holding everything of a thing reads each key's live setters. A reader
holding part of it sees only what its own records name, so a record another
replaced can look live. These laws say when a part reads as the whole does.

* **Witnessed is exact** (`witnessed_iff`). A part reads its records as the
  whole does, for a key, exactly when every held setter that is dead for the
  key has a setter of it above, through held records. No premise.
* **Convex is enough** (`witnessed_of_convex`): a part that holds every record
  between two of its records, and the key's live setters, is witnessed. Not
  the other way (`convex_not_needed`).
* **Anchored writers make the live records a slice** (`anchored_slice`): when
  every record names every live record under it, the live records alone read
  every key as the whole does. Every record sets the envelope, so the rule is
  the D8c anchor with the envelope counted as a key.
* **Heads** (`heads_slice`, `live_named_by_head`): a head is live for the
  envelope, so the same law gives the heads, and every live record is a head
  or is named by one. With the anchor over payload keys only it fails
  (`heads_need_env`).
* **The rule gives anchored names only to a holder that is closed**
  (`anchored_of_written`): one that holds what the writers of its records
  held. A holder missing a record reads wrongly from its own live records
  (`closed_needed`).
* **A cut** (`down_reads`, `closedOver_of_down`, `anchored_of_down`): a part
  closed downward, one that holds every held record its records name, reads
  as a past of the holding, and is closed when the holding is. So the rule
  carries to it.

Three of these need `prev` to have no cycle among the records held
(`NoCycle`). An id is a hash of an envelope that holds its `prev`, so real
records have none. The model's records carry ids as numbers and do not, so the
laws state it.
-/

namespace Loops

open List

/-! ## A key: a payload key, or the envelope -/

/-- A record sets `some k` when it sets payload key `k`. Every record sets
`none`, the envelope. -/
def setsK (o : Option String) (r : Record) : Bool :=
  match o with
  | none => true
  | some k => r.setsKey k

/-- Live for a key, the envelope among the keys. -/
def liveK (rs : List Record) (o : Option String) (r : Record) : Bool :=
  setsK o r && !(above rs r).any (setsK o)

theorem liveK_some (rs : List Record) (k : String) (r : Record) :
    liveK rs (some k) r = liveFor rs k r := rfl

theorem liveK_none (rs : List Record) (r : Record) : liveK rs none r = liveEnv rs r := by
  cases h : above rs r <;> simp [liveK, setsK, liveEnv, h]

theorem liveK_iff {rs : List Record} {o : Option String} {r : Record} :
    liveK rs o r = true ↔ setsK o r = true ∧ ∀ q ∈ above rs r, setsK o q = false := by
  simp [liveK]

/-- A setter that is not live has a setter above it. -/
theorem dead_iff {rs : List Record} {o : Option String} {r : Record} (hs : setsK o r = true) :
    liveK rs o r = false ↔ ∃ q ∈ above rs r, setsK o q = true := by
  simp [liveK, hs]

/-- Live is live for some key, the envelope among them. -/
theorem live_iff_liveK {rs : List Record} {r : Record} :
    live rs r = true ↔ ∃ o, liveK rs o r = true := by
  rw [live_iff]
  constructor
  · rintro (h | ⟨k, h⟩)
    · exact ⟨none, by rw [liveK_none]; exact h⟩
    · exact ⟨some k, h⟩
  · rintro ⟨o, h⟩
    cases o with
    | none => exact .inl (by rw [← liveK_none]; exact h)
    | some k => exact .inr ⟨k, h⟩

/-! ## Above, in a part and in the whole -/

theorem Reach.sub {U W : List Record} (h : ∀ q ∈ U, q ∈ W) {ids : List RecordId} {i : RecordId}
    (hr : Reach U ids i) : Reach W ids i := by
  induction hr with
  | base hi => exact .base hi
  | up hq hp _ ih => exact .up (h _ hq) hp ih

/-- What a part sees above a record, the whole sees too. -/
theorem above_sub {U W : List Record} (h : ∀ q ∈ U, q ∈ W) {r q : Record} (hq : q ∈ above U r) :
    q ∈ above W r := by
  obtain ⟨hqU, p, hp, hr⟩ := mem_above.1 hq
  exact mem_above.2 ⟨h q hqU, p, hp, hr.sub h⟩

/-- A held record that names another is above it. -/
theorem above_of_names {W : List Record} {r q : Record} (hq : q ∈ W) (h : r.id ∈ q.prev) :
    q ∈ above W r :=
  mem_above.2 ⟨hq, r.id, h, .base (mem_singleton_self _)⟩

theorem reach_of_above {W : List Record} {a b : Record} (h : b ∈ above W a) : Reach W [a.id] b.id := by
  obtain ⟨hb, p, hp, hr⟩ := mem_above.1 h
  exact .up hb hp hr

theorem Reach.trans {W : List Record} {i j p : RecordId} (h₁ : Reach W [i] j) (h₂ : Reach W [j] p) :
    Reach W [i] p := by
  induction h₂ with
  | base hi => rw [mem_singleton.1 hi]; exact h₁
  | up hq hp _ ih => exact .up hq hp ih

/-- Above is transitive. -/
theorem above_trans {W : List Record} {a b c : Record} (hb : b ∈ above W a) (hc : c ∈ above W b) :
    c ∈ above W a := by
  obtain ⟨hcW, p, hp, hr⟩ := mem_above.1 hc
  exact mem_above.2 ⟨hcW, p, hp, (reach_of_above hb).trans hr⟩

/-- Fewer records held, nothing less live: a part reads a record live for a
key whenever the whole does. -/
theorem liveK_part {U W : List Record} (h : ∀ q ∈ U, q ∈ W) {o : Option String} {r : Record}
    (hl : liveK W o r = true) : liveK U o r = true :=
  liveK_iff.2 ⟨(liveK_iff.1 hl).1, fun q hq => (liveK_iff.1 hl).2 q (above_sub h hq)⟩

theorem live_part {U W : List Record} (h : ∀ q ∈ U, q ∈ W) {r : Record} (hl : live W r = true) :
    live U r = true := by
  obtain ⟨o, ho⟩ := live_iff_liveK.1 hl
  exact live_iff_liveK.2 ⟨o, liveK_part h ho⟩

/-! ## Witnessed: the exact law -/

/-- Every held setter of the key that the whole reads as dead has a setter of
the key above it, through held records. -/
def Witnessed (U W : List Record) (o : Option String) : Prop :=
  ∀ a ∈ U, setsK o a = true → liveK W o a = false → ∃ b ∈ above U a, setsK o b = true

/-- **Exact.** A part reads each of its records as the whole does, for a key,
if and only if it is witnessed for that key. -/
theorem witnessed_iff {U W : List Record} (h : ∀ q ∈ U, q ∈ W) (o : Option String) :
    Witnessed U W o ↔ ∀ a ∈ U, liveK U o a = liveK W o a := by
  constructor
  · intro hw a ha
    apply Bool.eq_iff_iff.2
    constructor
    · intro hl
      cases hd : liveK W o a with
      | true => rfl
      | false =>
        obtain ⟨b, hb, hsb⟩ := hw a ha (liveK_iff.1 hl).1 hd
        rw [(liveK_iff.1 hl).2 b hb] at hsb
        cases hsb
    · exact liveK_part h
  · intro he a ha hs hd
    exact (dead_iff hs).1 (by rw [he a ha]; exact hd)

/-! ## No cycle over `prev` -/

/-- No record held is above itself. -/
def NoCycle (W : List Record) : Prop := ∀ r ∈ W, r ∉ above W r

/-- Going up, there is strictly less above. -/
theorem above_length_lt {W : List Record} (hW : NoCycle W) {a b : Record} (hb : b ∈ above W a) :
    (above W b).length < (above W a).length := by
  have hbW := (mem_above.1 hb).1
  have e : above W b = (above W a).filter (namesIn (upward W [b.id])) := by
    unfold above
    rw [filter_filter]
    apply filter_congr
    intro x hx
    cases hn : namesIn (upward W [b.id]) x with
    | false => rfl
    | true =>
      have hxb : x ∈ above W b := mem_filter.2 ⟨hx, hn⟩
      have := (mem_filter.1 (above_trans hb hxb)).2
      simp [this]
  rw [e]
  apply length_filter_lt_length_iff_exists.2
  refine ⟨b, hb, fun hn => hW b hbW (mem_filter.2 ⟨hbW, hn⟩)⟩

/-- With no cycle, a setter that is not live has a live setter above it. -/
theorem live_above {W : List Record} (hW : NoCycle W) (o : Option String) :
    ∀ a, setsK o a = true → liveK W o a = false → ∃ b ∈ above W a, liveK W o b = true := by
  intro a
  generalize hn : (above W a).length = n
  induction n using Nat.strongRecOn generalizing a with
  | _ n ih =>
    intro hs hd
    obtain ⟨b, hb, hsb⟩ := (dead_iff hs).1 hd
    cases hl : liveK W o b with
    | true => exact ⟨b, hb, hl⟩
    | false =>
      obtain ⟨c, hc, hlc⟩ := ih _ (hn ▸ above_length_lt hW hb) b rfl hsb hl
      exact ⟨c, above_trans hb hc, hlc⟩

/-! ## Convex: one way to be witnessed -/

/-- The part holds every record of the whole that lies between two of its own. -/
def Convex (U W : List Record) : Prop :=
  ∀ a ∈ U, ∀ c ∈ U, ∀ b ∈ W, b ∈ above W a → c ∈ above W b → b ∈ U

/-- In a convex part, what the whole sees above a held record among held
records, the part sees too. -/
theorem above_convex {U W : List Record} (hc : Convex U W) {a : Record} (ha : a ∈ U) :
    ∀ {i : RecordId}, Reach W [a.id] i → ∀ c ∈ U, i ∈ c.prev → c ∈ above W a → c ∈ above U a := by
  intro i hr
  induction hr with
  | base hi =>
    intro c hcU hp _
    rw [mem_singleton.1 hi] at hp
    exact above_of_names hcU hp
  | @up q p hq hp hr ih =>
    intro c hcU hpc hca
    have hqa : q ∈ above W a := mem_above.2 ⟨hq, p, hp, hr⟩
    have hcq : c ∈ above W q := above_of_names (mem_above.1 hca).1 hpc
    have hqU : q ∈ U := hc a ha c hcU q hq hqa hcq
    exact above_trans (ih q hqU hp hqa) (above_of_names hcU hpc)

/-- A convex part that holds the key's live setters is witnessed for the key. -/
theorem witnessed_of_convex {U W : List Record} (hW : NoCycle W)
    (hc : Convex U W) (o : Option String) (hl : ∀ a ∈ W, liveK W o a = true → a ∈ U) :
    Witnessed U W o := by
  intro a ha hs hd
  obtain ⟨b, hb, hlb⟩ := live_above hW o a hs hd
  obtain ⟨hbW, p, hp, hr⟩ := mem_above.1 hb
  exact ⟨b, above_convex hc ha hr b (hl b hbW hlb) hp hb, (liveK_iff.1 hlb).1⟩

/-! ## Anchored: the writer pays once -/

/-- The live records of what is held. -/
abbrev liveOf (W : List Record) : List Record := W.filter (live W)

/-- What the D8c writer's rule leaves behind, with the envelope counted as a
key every record sets: every record names every live record under it. -/
def Anchored (W : List Record) : Prop :=
  ∀ x ∈ W, ∀ y ∈ above W x, live W x = true → x.id ∈ y.prev

/-- Two records set a payload key in common. -/
def sharesKey (x y : Record) : Bool :=
  match x.sets, y.sets with
  | none, none => true
  | none, some ks => !ks.isEmpty
  | some ks, none => !ks.isEmpty
  | some xs, some ys => xs.any ys.contains

theorem sharesKey_of {x y : Record} {k : String} (hx : x.setsKey k = true) (hy : y.setsKey k = true) :
    sharesKey x y = true := by
  cases hxs : x.sets <;> cases hys : y.sets <;> simp_all [sharesKey, Record.setsKey]
  · rintro rfl; simp at hy
  · rintro rfl; simp at hx
  · exact ⟨k, hx, hy⟩

/-- The rule over payload keys only, as first agreed: a record names every
live record under it that sets a payload key it sets. -/
def AnchoredKeys (W : List Record) : Prop :=
  ∀ x ∈ W, ∀ y ∈ above W x, sharesKey x y = true → live W x = true → x.id ∈ y.prev

theorem Anchored.keys {W : List Record} (h : Anchored W) : AnchoredKeys W :=
  fun x hx y hy _ hl => h x hx y hy hl

/-- When the setters of a key name the live setters of it under them, the live
records are witnessed for that key. -/
theorem witnessed_of_named {W : List Record} (hW : NoCycle W) (o : Option String)
    (h : ∀ x ∈ W, ∀ y ∈ above W x, setsK o x = true → setsK o y = true → live W x = true →
      x.id ∈ y.prev) :
    Witnessed (liveOf W) W o := by
  intro a ha hs hd
  obtain ⟨haW, hal⟩ := mem_filter.1 ha
  obtain ⟨b, hb, hlb⟩ := live_above hW o a hs hd
  have hbW := (mem_above.1 hb).1
  have hbs := (liveK_iff.1 hlb).1
  have hbl : b ∈ liveOf W := mem_filter.2 ⟨hbW, live_iff_liveK.2 ⟨o, hlb⟩⟩
  exact ⟨b, above_of_names hbl (h a haW b hb hs hbs hal), hbs⟩

theorem anchored_witnessed {W : List Record} (hW : NoCycle W) (h : Anchored W) (o : Option String) :
    Witnessed (liveOf W) W o :=
  witnessed_of_named hW o fun x hx y hy _ _ hl => h x hx y hy hl

/-- **The slice is the answer.** With anchored names, a reader holding exactly
the live records reads every key, the envelope among them, as a reader of
everything does. -/
theorem anchored_slice {W : List Record} (hW : NoCycle W) (h : Anchored W) (o : Option String)
    {a : Record} (ha : a ∈ W) :
    (live W a = true ∧ liveK (liveOf W) o a = true) ↔ liveK W o a = true := by
  have hsub : ∀ q ∈ liveOf W, q ∈ W := fun q hq => (mem_filter.1 hq).1
  have he := (witnessed_iff hsub o).1 (anchored_witnessed hW h o)
  constructor
  · rintro ⟨hl, hk⟩
    rw [← he a (mem_filter.2 ⟨ha, hl⟩)]; exact hk
  · intro hk
    have hl : live W a = true := live_iff_liveK.2 ⟨o, hk⟩
    exact ⟨hl, by rw [he a (mem_filter.2 ⟨ha, hl⟩)]; exact hk⟩

/-- The rule over payload keys gives the payload keys, as issue #5 first agreed. -/
theorem anchoredKeys_slice {W : List Record} (hW : NoCycle W) (h : AnchoredKeys W) (k : String)
    {a : Record} (ha : a ∈ W) :
    (live W a = true ∧ liveFor (liveOf W) k a = true) ↔ liveFor W k a = true := by
  have hsub : ∀ q ∈ liveOf W, q ∈ W := fun q hq => (mem_filter.1 hq).1
  have he := (witnessed_iff hsub (some k)).1
    (witnessed_of_named hW (some k) fun x hx y hy hsx hsy hl => h x hx y hy (sharesKey_of hsx hsy) hl)
  constructor
  · rintro ⟨hl, hk⟩
    have := he a (mem_filter.2 ⟨ha, hl⟩)
    rw [liveK_some, liveK_some] at this
    rw [← this]; exact hk
  · intro hk
    have hl : live W a = true := live_iff_liveK.2 ⟨some k, hk⟩
    have := he a (mem_filter.2 ⟨ha, hl⟩)
    rw [liveK_some, liveK_some] at this
    exact ⟨hl, by rw [this]; exact hk⟩

/-! ## Heads -/

/-- **Heads from the slice.** A head is live for the envelope, so a reader
holding exactly the live records of anchored writers finds the whole's heads. -/
theorem heads_slice {W : List Record} (hW : NoCycle W) (h : Anchored W) {a : Record} (ha : a ∈ W) :
    (live W a = true ∧ liveEnv (liveOf W) a = true) ↔ liveEnv W a = true := by
  have := anchored_slice hW h none ha
  rw [liveK_none, liveK_none] at this
  exact this

/-- **The heads list the slice.** Every live record is a head or is named by one. -/
theorem live_named_by_head {W : List Record} (hW : NoCycle W) (h : Anchored W) {a : Record}
    (ha : a ∈ W) (hl : live W a = true) :
    liveEnv W a = true ∨ ∃ b ∈ W, liveEnv W b = true ∧ a.id ∈ b.prev := by
  cases he : liveEnv W a with
  | true => exact .inl rfl
  | false =>
    obtain ⟨b, hb, hlb⟩ := live_above hW none a rfl (by rw [liveK_none]; exact he)
    exact .inr ⟨b, (mem_above.1 hb).1, by rw [← liveK_none]; exact hlb, h a ha b hb hl⟩

/-! ## What the rule needs of the holder

`saw y` is what the writer of `y` held when it wrote `y`. A writer names only
what it holds, holds what the writers of its records held, and follows the
rule: it names every record it holds that is live there. A holder is *closed*
when it too holds what the writers of its records held. -/

structure Anchoring (saw : Record → List Record) (W : List Record) : Prop where
  /-- you can only replace what you hold -/
  names : ∀ y ∈ W, ∀ p ∈ y.prev, ∃ x ∈ saw y, x.id = p
  /-- a writer holds what the writers of its records held -/
  trans : ∀ y ∈ W, ∀ x ∈ saw y, ∀ z ∈ saw x, z ∈ saw y
  /-- the rule -/
  rule : ∀ y ∈ W, ∀ x ∈ saw y, live (saw y) x = true → x.id ∈ y.prev

def ClosedOver (saw : Record → List Record) (W : List Record) : Prop := ∀ y ∈ W, ∀ x ∈ saw y, x ∈ W

/-- The three clauses as checks over a given holding. -/
theorem Anchoring.of_checks {saw : Record → List Record} {W : List Record}
    (h₁ : W.all (fun y => y.prev.all fun p => (saw y).any (·.id == p)) = true)
    (h₂ : W.all (fun y => (saw y).all fun x => (saw x).all fun z => (saw y).contains z) = true)
    (h₃ : W.all (fun y => (saw y).all fun x => !live (saw y) x || y.prev.contains x.id) = true) :
    Anchoring saw W where
  names := fun y hy p hp => by
    obtain ⟨x, hx, e⟩ := any_eq_true.1 (all_eq_true.1 (all_eq_true.1 h₁ y hy) p hp)
    exact ⟨x, hx, by simpa using e⟩
  trans := fun y hy x hx z hz => by
    simpa using all_eq_true.1 (all_eq_true.1 (all_eq_true.1 h₂ y hy) x hx) z hz
  rule := fun y hy x hx hl => by
    simpa [hl] using all_eq_true.1 (all_eq_true.1 h₃ y hy) x hx

/-- A record above another was written by a holder of it. -/
theorem saw_of_reach {saw : Record → List Record} {W : List Record} (ha : Addressed W)
    (hw : Anchoring saw W) (hc : ClosedOver saw W) {x : Record} (hx : x ∈ W) :
    ∀ {i : RecordId}, Reach W [x.id] i → ∀ y ∈ W, i ∈ y.prev → x ∈ saw y := by
  intro i hr
  induction hr with
  | base hi =>
    intro y hy hp
    obtain ⟨x', hx', e⟩ := hw.names y hy _ hp
    rw [mem_singleton.1 hi] at e
    rw [← ha x' (hc y hy x' hx') x hx e]; exact hx'
  | @up q p hq hp _ ih =>
    intro y hy hpy
    obtain ⟨q', hq', e⟩ := hw.names y hy _ hpy
    have : q' = q := ha q' (hc y hy q' hq') q hq e
    subst this
    exact hw.trans y hy q' hq' x (ih q' hq hp)

/-- **Writers that follow the rule leave anchored names in a closed holding.** -/
theorem anchored_of_written {saw : Record → List Record} {W : List Record} (ha : Addressed W)
    (hw : Anchoring saw W) (hc : ClosedOver saw W) : Anchored W := by
  intro x hx y hy hl
  obtain ⟨hyW, p, hp, hr⟩ := mem_above.1 hy
  have hxs : x ∈ saw y := saw_of_reach ha hw hc hx hr y hyW hp
  exact hw.rule y hyW x hxs (live_part (hc y hyW) hl)

/-! ## A cut: a part closed downward

A part is *closed downward* when it holds every held record one of its
records names in `prev`. Such a part is a state the holding could have been
in: it sees above a record exactly what the whole sees there among its own
records, and it holds what the writers of its records held. So the writer's
rule, which is over what each writer held and not over what is live now,
carries to it, and its live records read alone read as it does. -/

/-- `S` is a part of `W` that holds every record of `W` its records name. -/
structure Down (S W : List Record) : Prop where
  part : ∀ r ∈ S, r ∈ W
  names : ∀ y ∈ S, ∀ x ∈ W, x.id ∈ y.prev → x ∈ S

/-- Whatever reaches a name of a held record does so through held records. -/
theorem Down.reach {S W : List Record} (h : Down S W) {r : Record} :
    ∀ {i : RecordId}, Reach W [r.id] i → ∀ b ∈ S, i ∈ b.prev → Reach S [r.id] i := by
  intro i hr
  induction hr with
  | base hi => exact fun _ _ _ => .base hi
  | @up q p hq hp _ ih => exact fun b hb hqb => .up (h.names b hb q hq hqb) hp (ih q (h.names b hb q hq hqb) hp)

/-- A part closed downward sees above a record what the whole sees there
among the part's records. -/
theorem Down.above_iff {S W : List Record} (h : Down S W) {r q : Record} :
    q ∈ above S r ↔ q ∈ S ∧ q ∈ above W r := by
  constructor
  · exact fun hq => ⟨(mem_above.1 hq).1, above_sub h.part hq⟩
  · rintro ⟨hqS, hq⟩
    obtain ⟨-, p, hp, hr⟩ := mem_above.1 hq
    exact mem_above.2 ⟨hqS, p, hp, h.reach hr q hqS hp⟩

/-- A part closed downward holds every held record under one of its own. -/
theorem Down.below {S W : List Record} (h : Down S W) {x b : Record} (hx : x ∈ W) (hb : b ∈ S)
    (hxb : b ∈ above W x) : x ∈ S := by
  obtain ⟨-, p, hp, hr⟩ := mem_above.1 hxb
  have key : ∀ {i : RecordId}, Reach W [x.id] i → ∀ b ∈ S, i ∈ b.prev → x ∈ S := by
    intro i hr
    induction hr with
    | base hi => exact fun b hb hib => h.names b hb x hx (mem_singleton.1 hi ▸ hib)
    | @up q p hq hp _ ih => exact fun b hb hqb => ih q (h.names b hb q hq hqb) hp
  exact key hr b hb hp

/-- **A cut reads as a past.** A part closed downward reads a record as live
for a key exactly when no setter of the key in the part is above it in the
whole: what a holder of exactly those records reads, with nothing missing
between them. -/
theorem down_reads {S W : List Record} (h : Down S W) (o : Option String) (r : Record) :
    liveK S o r = true ↔ setsK o r = true ∧ ∀ q ∈ S, q ∈ above W r → setsK o q = false := by
  rw [liveK_iff]
  exact and_congr_right fun _ =>
    ⟨fun hl q hq hqa => hl q (h.above_iff.2 ⟨hq, hqa⟩), fun hl q hq => hl q (h.above_iff.1 hq).1 (h.above_iff.1 hq).2⟩

theorem NoCycle.part {U W : List Record} (hW : NoCycle W) (h : ∀ q ∈ U, q ∈ W) : NoCycle U :=
  fun r hr hra => hW r (h r hr) (above_sub h hra)

theorem Anchoring.part {saw : Record → List Record} {U W : List Record} (hw : Anchoring saw W)
    (h : ∀ q ∈ U, q ∈ W) : Anchoring saw U where
  names := fun y hy => hw.names y (h y hy)
  trans := fun y hy => hw.trans y (h y hy)
  rule := fun y hy => hw.rule y (h y hy)

/-- **A cut of a closed holding is closed.** Where the writers followed the
rule, a part closed downward holds what the writers of its records held: a
writer names every record it holds that is live there, and every other record
it holds is under one of those. -/
theorem closedOver_of_down {saw : Record → List Record} {S W : List Record} (hW : NoCycle W)
    (hw : Anchoring saw W) (hc : ClosedOver saw W) (h : Down S W) : ClosedOver saw S := by
  intro y hy x hx
  have hyW := h.part y hy
  have hV : ∀ q ∈ saw y, q ∈ W := hc y hyW
  have named : ∀ b ∈ saw y, liveK (saw y) none b = true → b ∈ S := fun b hb hl =>
    h.names y hy b (hV b hb) (hw.rule y hyW b hb (live_iff_liveK.2 ⟨none, hl⟩))
  cases hl : liveK (saw y) none x with
  | true => exact named x hx hl
  | false =>
    obtain ⟨b, hb, hlb⟩ := live_above (hW.part hV) none x rfl hl
    exact h.below (hV x hx) (named b (mem_above.1 hb).1 hlb) (above_sub hV hb)

/-- **The rule carries to a cut.** A part closed downward of a closed holding
whose writers followed the rule has anchored names, so its live records read
alone read every key as it does (`anchored_slice`). `Anchored` of the whole
would not do: it is about what is live now. -/
theorem anchored_of_down {saw : Record → List Record} {S W : List Record} (ha : Addressed W)
    (hW : NoCycle W) (hw : Anchoring saw W) (hc : ClosedOver saw W) (h : Down S W) : Anchored S :=
  anchored_of_written (fun a haS b hbS => ha a (h.part a haS) b (h.part b hbS)) (hw.part h.part)
    (closedOver_of_down hW hw hc h)

/-! ## The boundaries

**Convex is more than is needed.** `a ← b ← c`, all setting one key, and `c`
also names `a`. The part `{a, c}` is witnessed and not convex: `b` lies between. -/

def lv.a : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default,
    sets := some ["status"] }
def lv.b : Record :=
  { id := 2, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["status"] }
def lv.c : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [2, 1], emitted := default,
    sets := some ["status"] }

theorem convex_not_needed :
    Witnessed [lv.a, lv.c] [lv.a, lv.b, lv.c] (some "status") ∧ Witnessed [lv.a, lv.c] [lv.a, lv.b, lv.c] none ∧
      ¬Convex [lv.a, lv.c] [lv.a, lv.b, lv.c] := by
  unfold Witnessed Convex
  decide +kernel

/-! **Payload keys alone do not give the heads.** `t` sets `title`; `s1` and
`s2` set `status`, one after the other. Each record names every live record
under it that shares a payload key with it, and the live records `{t, s2}`,
read alone, take `t` for a head. -/

def hd.t : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default,
    sets := some ["title"] }
def hd.s1 : Record :=
  { id := 2, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["status"] }
def hd.s2 : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [2], emitted := default,
    sets := some ["status"] }

theorem heads_need_env :
    AnchoredKeys [hd.t, hd.s1, hd.s2] ∧ liveOf [hd.t, hd.s1, hd.s2] = [hd.t, hd.s2] ∧
      liveEnv [hd.t, hd.s2] hd.t = true ∧ liveEnv [hd.t, hd.s1, hd.s2] hd.t = false := by
  unfold AnchoredKeys
  decide +kernel

/-! **A holder that is not closed.** `x` sets `a` and `b`; `y` sets `a`; `z`
sets `b`; `y2` sets `a`. Every writer followed the rule, and `y2`'s writer held
`z`. A holder of `x`, `y` and `y2` without `z` reads `x` as live for `b`, keeps
it, and from its live records alone reads `x` as live for `a` too. -/

def cl.x : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default,
    sets := some ["a", "b"] }
def cl.y : Record :=
  { id := 2, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["a"] }
def cl.z : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [2, 1], emitted := default,
    sets := some ["b"] }
def cl.y2 : Record :=
  { id := 4, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [3, 2], emitted := default,
    sets := some ["a"] }

/-- What each writer held. -/
def cl.saw (r : Record) : List Record :=
  if r.id = 2 then [cl.x] else if r.id = 3 then [cl.x, cl.y] else if r.id = 4 then [cl.x, cl.y, cl.z] else []

theorem closed_needed :
    Anchoring cl.saw [cl.x, cl.y, cl.y2] ∧ ¬ClosedOver cl.saw [cl.x, cl.y, cl.y2] ∧
      liveFor (liveOf [cl.x, cl.y, cl.y2]) "a" cl.x = true ∧ liveFor [cl.x, cl.y, cl.y2] "a" cl.x = false := by
  refine ⟨Anchoring.of_checks ?_ ?_ ?_, fun hc => ?_, ?_, ?_⟩
  · decide +kernel
  · decide +kernel
  · decide +kernel
  · have h : [cl.x, cl.y, cl.y2].all (fun y => (cl.saw y).all fun x => [cl.x, cl.y, cl.y2].contains x) = true :=
      all_eq_true.2 fun y hy => all_eq_true.2 fun x hx => by simpa using hc y hy x hx
    rw [show [cl.x, cl.y, cl.y2].all (fun y => (cl.saw y).all fun x => [cl.x, cl.y, cl.y2].contains x) = false
      from by decide +kernel] at h
    cases h
  · decide +kernel
  · decide +kernel

/-! ## A thing's keyed state from a part -/

/-- **A witnessed part that holds the live records reads the thing's keyed
state as the whole does.** -/
theorem keyedIn_part {t : String} {xs ys : List Record} (hx : Addressed xs) (hy : Addressed ys)
    (hs : ∀ r ∈ recordsOf t ys, r ∈ recordsOf t xs)
    (hl : ∀ r ∈ recordsOf t xs, live (recordsOf t xs) r = true → r ∈ recordsOf t ys)
    (hw : ∀ o, Witnessed (recordsOf t ys) (recordsOf t xs) o) :
    keyedIn t ys = keyedIn t xs := by
  have ax := addressed_recordsOf hx t
  have ay := addressed_recordsOf hy t
  have he : ∀ o, ∀ a ∈ recordsOf t ys, liveK (recordsOf t ys) o a = liveK (recordsOf t xs) o a :=
    fun o => (witnessed_iff hs o).1 (hw o)
  have hlive : ∀ a ∈ recordsOf t ys, live (recordsOf t ys) a = live (recordsOf t xs) a := by
    intro a ha
    apply Bool.eq_iff_iff.2
    simp only [live_iff_liveK]
    constructor
    · rintro ⟨o, h⟩; exact ⟨o, by rw [← he o a ha]; exact h⟩
    · rintro ⟨o, h⟩; exact ⟨o, by rw [he o a ha]; exact h⟩
  have key : ∀ (pU pW : Record → Bool), (∀ a ∈ recordsOf t ys, pU a = pW a) →
      (∀ a ∈ recordsOf t xs, pW a = true → a ∈ recordsOf t ys) →
      ∀ r, r ∈ (recordsOf t ys).filter pU ↔ r ∈ (recordsOf t xs).filter pW := by
    intro pU pW h1 h2 r
    constructor
    · intro h
      obtain ⟨hr, hp⟩ := mem_filter.1 h
      exact mem_filter.2 ⟨hs r hr, by rw [← h1 r hr]; exact hp⟩
    · intro h
      obtain ⟨hr, hp⟩ := mem_filter.1 h
      have := h2 r hr hp
      exact mem_filter.2 ⟨this, by rw [h1 r this]; exact hp⟩
  have hk : liveKeys (recordsOf t ys) = liveKeys (recordsOf t xs) := by
    unfold liveKeys
    apply canonSet_eq
    intro k
    simp only [mem_flatMap]
    constructor
    · rintro ⟨r, hr, hk⟩; exact ⟨r, (key _ _ hlive hl r).1 hr, hk⟩
    · rintro ⟨r, hr, hk⟩; exact ⟨r, (key _ _ hlive hl r).2 hr, hk⟩
  have hh : (stateIn t ys).heads = (stateIn t xs).heads := by
    have sy : (stateIn t ys).heads.Pairwise Newer := (LatestBy.inv_of ay).heads_sorted
    have sx : (stateIn t xs).heads.Pairwise Newer := (LatestBy.inv_of ax).heads_sorted
    apply sorted_ext sy sx
    intro r
    rw [mem_stateIn_heads_iff hy, mem_stateIn_heads_iff hx]
    have := key (liveEnv (recordsOf t ys)) (liveEnv (recordsOf t xs))
      (fun a ha => by rw [← liveK_none, ← liveK_none]; exact he none a ha)
      (fun a ha h => hl a ha (by simp [live, h])) r
    exact ⟨fun ⟨a, b⟩ => mem_filter.1 (this.1 (mem_filter.2 ⟨a, b⟩)),
      fun ⟨a, b⟩ => mem_filter.1 (this.2 (mem_filter.2 ⟨a, b⟩))⟩
  unfold keyedIn
  simp only
  rw [show (ys.filter (·.isOf t)) = recordsOf t ys from rfl, show (xs.filter (·.isOf t)) = recordsOf t xs from rfl,
    hk, hh]
  congr 1
  · exact map_congr_left fun k _ => by
      congr 1
      apply orderHeads_set_decides (addressed_filter ay _) (addressed_filter ax _)
      exact key _ _ (fun a ha => by rw [← liveK_some, ← liveK_some]; exact he (some k) a ha)
        (fun a ha h => hl a ha (live_iff_liveK.2 ⟨some k, h⟩))
  · apply orderHeads_set_decides (addressed_filter ay _) (addressed_filter ax _)
    exact key _ _ (fun a ha => by rw [hlive a ha])
      (fun a ha h => hl a ha (by simp only [Bool.and_eq_true] at h; exact h.2))

/-- A convex part that holds the live records reads the keyed state. -/
theorem keyedIn_convex {t : String} {xs ys : List Record} (hx : Addressed xs) (hy : Addressed ys)
    (hc : NoCycle (recordsOf t xs))
    (hs : ∀ r ∈ recordsOf t ys, r ∈ recordsOf t xs)
    (hl : ∀ r ∈ recordsOf t xs, live (recordsOf t xs) r = true → r ∈ recordsOf t ys)
    (hv : Convex (recordsOf t ys) (recordsOf t xs)) :
    keyedIn t ys = keyedIn t xs :=
  keyedIn_part hx hy hs hl fun o =>
    witnessed_of_convex hc hv o fun a ha h => hl a ha (live_iff_liveK.2 ⟨o, h⟩)

/-- **With anchored writers the live records of a thing are its slice**: read
alone, they give its whole keyed state, heads included. -/
theorem keyedIn_live {t : String} {xs : List Record} (hx : Addressed xs)
    (hc : NoCycle (recordsOf t xs)) (ha : Anchored (recordsOf t xs)) :
    keyedIn t (liveOf (recordsOf t xs)) = keyedIn t xs := by
  have e : recordsOf t (liveOf (recordsOf t xs)) = liveOf (recordsOf t xs) :=
    filter_eq_self.2 fun a h => (mem_filter.1 (mem_filter.1 h).1).2
  apply keyedIn_part hx (addressed_filter (addressed_recordsOf hx t) _)
  · rw [e]; exact fun r hr => (mem_filter.1 hr).1
  · rw [e]; exact fun r hr h => mem_filter.2 ⟨hr, h⟩
  · rw [e]; exact anchored_witnessed hc ha

/-! ## Behind settling and admission

The laws above are about the records a reader folds. A reader folds what its
places hold after settling the forms and admitting by `prev` (`realm`). A part
of the forms gives a part of the folded records when it owes each record of the
thing it holds at all three things (`KeyedPart`): every form of it, every
record that erases it, and that it refuses it when the whole does. It owes
nothing for the `prev` targets of a record the whole admits: holding fewer of
them never refuses it (`entitled_fewer`). A record the whole refuses is another
matter, since a part that lacks the target the whole refuses it for admits it,
and its envelope then sets its keys; a part that holds what such a record names
in `prev` refuses it too (`KeyedPart.of_targets`, in `Laws.KeyedSpecSlice`). -/

structure KeyedPart (may : Record → Record → Bool) (t : String) (S W : List Record) : Prop where
  sub : ∀ r ∈ S, r ∈ W
  forms : ∀ a ∈ S, a.isOf t = true → ∀ b ∈ W, b.id = a.id → b ∈ S
  erasers : ∀ a ∈ S, a.isOf t = true → ∀ e ∈ W, e.erases.contains a.id = true → e ∈ S
  admitted : ∀ r ∈ settle W, r.isOf t = true → (∃ a ∈ S, a.id = r.id) →
    entitled may (settle S) r = true → entitled may (settle W) r = true

/-- The part keeps, for a record of the thing it holds, what the whole keeps. -/
theorem pick_part {may : Record → Record → Bool} {t : String} {S W : List Record} (hF : Forms W)
    (h : KeyedPart may t S W) {a : Record} (ha : a ∈ S) (hat : a.isOf t = true) :
    pick S a.id = pick W a.id := by
  unfold pick
  have hforms : ∀ r, r ∈ S.filter (·.id == a.id) ↔ r ∈ W.filter (·.id == a.id) := fun r => by
    simp only [mem_filter]
    constructor
    · exact fun ⟨hr, e⟩ => ⟨h.sub r hr, e⟩
    · rintro ⟨hr, e⟩
      exact ⟨h.forms a ha hat r hr (by simpa using e), e⟩
  have hFS : Forms (S.filter (·.id == a.id)) := fun x hx y hy e =>
    hF x (h.sub x (mem_filter.1 hx).1) y (h.sub y (mem_filter.1 hy).1) e
  have he : erasedIn S a.id = erasedIn W a.id := by
    unfold erasedIn
    apply Bool.eq_iff_iff.2
    simp only [any_eq_true]
    exact ⟨fun ⟨e, he, p⟩ => ⟨e, h.sub e he, p⟩, fun ⟨e, he, p⟩ => ⟨e, h.erasers a ha hat e he p, p⟩⟩
  rw [baseForm_eq hFS (fun x hx => by simpa using (mem_filter.1 hx).2) hforms, he]

/-- A form of a settled record of the thing is of the thing. -/
theorem form_isOf {t : String} {W : List Record} (hF : Forms W) {r a : Record} (hr : r ∈ settle W)
    (ht : r.isOf t = true) (ha : a ∈ W) (e : a.id = r.id) : a.isOf t = true := by
  obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
  obtain ⟨b, hb, hbi, hrb⟩ := pick_form hp
  have : b.id = a.id := by rw [e, ← hrb.id]
  rw [← isOf_env (hF b hb a ha this).1, ← isOf_env hrb]; exact ht

/-- The part settles a record of the thing exactly when the whole does and the
part holds a form of it. -/
theorem settle_part {may : Record → Record → Bool} {t : String} {S W : List Record} (hF : Forms W)
    (h : KeyedPart may t S W) {r : Record} (ht : r.isOf t = true) :
    r ∈ settle S ↔ r ∈ settle W ∧ ∃ a ∈ S, a.id = r.id := by
  constructor
  · intro hr
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    obtain ⟨a, ha, hai, hra⟩ := pick_form hp
    have hat : a.isOf t = true := by rw [← isOf_env hra]; exact ht
    subst hai
    exact ⟨mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, h.sub a ha, rfl⟩, by rw [← pick_part hF h ha hat]; exact hp⟩,
      a, ha, (pick_id hp).symm⟩
  · rintro ⟨hr, a, ha, e⟩
    have hat := form_isOf hF hr ht (h.sub a ha) e
    obtain ⟨i, -, hp⟩ := mem_filterMap.1 hr
    have hi : i = a.id := by rw [e, pick_id hp]
    subst hi
    exact mem_filterMap.2 ⟨a.id, mem_idsOf.2 ⟨a, ha, rfl⟩, by rw [pick_part hF h ha hat]; exact hp⟩

/-- Two records kept for one id, in a part and in the whole, share an envelope. -/
theorem settle_env_part {S W : List Record} (hF : Forms W) (hs : ∀ r ∈ S, r ∈ W) {q q' : Record}
    (hq : q ∈ settle W) (hq' : q' ∈ settle S) (e : q.id = q'.id) : SameEnvelope q q' := by
  obtain ⟨j, -, hj⟩ := mem_filterMap.1 hq
  obtain ⟨b, hb, -, hqb⟩ := pick_form hj
  obtain ⟨j', -, hj'⟩ := mem_filterMap.1 hq'
  obtain ⟨b', hb', -, hqb'⟩ := pick_form hj'
  have hbb : b.id = b'.id := by rw [← hqb.id, ← hqb'.id]; exact e
  exact hqb.trans ((hF b hb b' (hs b' hb') hbb).1.trans hqb'.symm)

/-- **Holding less never refuses.** A record the whole admits, any part admits:
admission asks only about the `prev` targets held. -/
theorem entitled_fewer {may : Record → Record → Bool} (hm : EnvOnly may) {S W : List Record}
    (hF : Forms W) (hs : ∀ r ∈ S, r ∈ W) {r : Record} (he : entitled may (settle W) r = true) :
    entitled may (settle S) r = true := by
  unfold entitled at he ⊢
  simp only [all_eq_true] at he ⊢
  intro p hp q' hq'
  by_cases e : q'.id = p
  · obtain ⟨j, -, hj⟩ := mem_filterMap.1 hq'
    obtain ⟨b, hb, -, hqb⟩ := pick_form hj
    have hbW := hs b hb
    obtain ⟨q, hq⟩ := pick_some hbW
    have hqW : q ∈ settle W := mem_filterMap.2 ⟨b.id, mem_idsOf.2 ⟨b, hbW, rfl⟩, hq⟩
    have hid : q.id = q'.id := by rw [pick_id hq, hqb.id]
    have se := settle_env_part hF hs hqW hq' hid
    have := he p hp q hqW
    rw [← hid, ← shares_env (SameEnvelope.refl r) se, ← hm r r q q' (SameEnvelope.refl r) se]
    exact this
  · simp [e]

/-- The part folds a record of the thing exactly when the whole folds it and
the part holds a form of it. -/
theorem realm_part {may : Record → Record → Bool} (hm : EnvOnly may) {t : String} {S W : List Record}
    (hF : Forms W) (h : KeyedPart may t S W) {r : Record} (ht : r.isOf t = true) :
    r ∈ realm may S ↔ r ∈ realm may W ∧ ∃ a ∈ S, a.id = r.id := by
  unfold realm
  constructor
  · intro hr
    obtain ⟨hw, ha⟩ := (settle_part hF h ht).1 (mem_admit.1 hr).1
    exact ⟨mem_admit.2 ⟨hw, h.admitted r hw ht ha (mem_admit.1 hr).2⟩, ha⟩
  · rintro ⟨hr, ha⟩
    obtain ⟨hw, he⟩ := mem_admit.1 hr
    exact mem_admit.2 ⟨(settle_part hF h ht).2 ⟨hw, ha⟩, entitled_fewer hm hF h.sub he⟩

/-- **The keyed slice.** Behind settling and admission, a part that holds a form
of every live record of the thing, and whose records of the thing are witnessed,
reads the thing's keyed state as a reader of everything does. -/
theorem keyedIn_slice {may : Record → Record → Bool} (hm : EnvOnly may) {t : String}
    {S W : List Record} (hF : Forms W) (h : KeyedPart may t S W)
    (hl : ∀ r ∈ recordsOf t (realm may W), live (recordsOf t (realm may W)) r = true → ∃ a ∈ S, a.id = r.id)
    (hw : ∀ o, Witnessed (recordsOf t (realm may S)) (recordsOf t (realm may W)) o) :
    keyedIn t (realm may S) = keyedIn t (realm may W) := by
  apply keyedIn_part (admit_distinct may (settle_distinct W)).addressed
    (admit_distinct may (settle_distinct S)).addressed _ _ hw
  · intro r hr
    obtain ⟨hrS, ht⟩ := mem_filter.1 hr
    exact mem_filter.2 ⟨((realm_part hm hF h ht).1 hrS).1, ht⟩
  · intro r hr hlr
    obtain ⟨hrW, ht⟩ := mem_filter.1 hr
    exact mem_filter.2 ⟨(realm_part hm hF h ht).2 ⟨hrW, hl r hr hlr⟩, ht⟩

/-- Witnessed is about which records are held. -/
theorem Witnessed.congr {U U' W : List Record} (h : ∀ r, r ∈ U ↔ r ∈ U') {o : Option String}
    (hw : Witnessed U W o) : Witnessed U' W o := by
  intro a ha hs hd
  obtain ⟨b, hb, hsb⟩ := hw a ((h a).2 ha) hs hd
  exact ⟨b, (above_congr h a b).1 hb, hsb⟩

/-- **With anchored writers the slice is the live records and what erases
them.** A part that holds, of the thing, exactly the forms of its live records,
with their erasers, reads its keyed state as a reader of everything does. -/
theorem keyedIn_live_slice {may : Record → Record → Bool} (hm : EnvOnly may) {t : String}
    {S W : List Record} (hF : Forms W) (h : KeyedPart may t S W)
    (hc : NoCycle (recordsOf t (realm may W))) (ha : Anchored (recordsOf t (realm may W)))
    (hl : ∀ r ∈ recordsOf t (realm may W), live (recordsOf t (realm may W)) r = true → ∃ a ∈ S, a.id = r.id)
    (ho : ∀ r ∈ recordsOf t (realm may W), (∃ a ∈ S, a.id = r.id) → live (recordsOf t (realm may W)) r = true) :
    keyedIn t (realm may S) = keyedIn t (realm may W) := by
  apply keyedIn_slice hm hF h hl
  intro o
  apply (anchored_witnessed hc ha o).congr
  intro r
  constructor
  · intro hr
    obtain ⟨hrW, hlr⟩ := mem_filter.1 hr
    have ht := (mem_filter.1 hrW).2
    exact mem_filter.2 ⟨(realm_part hm hF h ht).2 ⟨(mem_filter.1 hrW).1, hl r hrW hlr⟩, ht⟩
  · intro hr
    obtain ⟨hrS, ht⟩ := mem_filter.1 hr
    obtain ⟨hrW, hex⟩ := (realm_part hm hF h ht).1 hrS
    have hrt : r ∈ recordsOf t (realm may W) := mem_filter.2 ⟨hrW, ht⟩
    exact mem_filter.2 ⟨hrt, ho r hrt hex⟩

/-- A part that holds the same records of the thing is witnessed. -/
theorem witnessed_same {U W : List Record} (h : ∀ r, r ∈ U ↔ r ∈ W) (o : Option String) :
    Witnessed U W o := by
  intro a ha hs hd
  obtain ⟨b, hb, hsb⟩ := (dead_iff hs).1 hd
  exact ⟨b, (above_congr h a b).2 hb, hsb⟩

/-- **`thing.slice` answers the keyed state too.** A slice holds every record
of the thing, so nothing in it lacks a witness. -/
theorem keyedIn_thing_slice {may : Record → Record → Bool} (hm : EnvOnly may) {t : String}
    {S W : List Record} (hF : Forms W) (h : Slice t S W) :
    keyedIn t (realm may S) = keyedIn t (realm may W) := by
  have key : ∀ r, r ∈ recordsOf t (realm may S) ↔ r ∈ recordsOf t (realm may W) := by
    intro r
    unfold realm
    constructor
    · intro hr
      obtain ⟨hr, ht⟩ := mem_filter.1 hr
      obtain ⟨hs, he⟩ := mem_admit.1 hr
      have hw := (settle_slice hF h ht).1 hs
      exact mem_filter.2 ⟨mem_admit.2 ⟨hw, by rw [← entitled_slice hm hF h hw ht]; exact he⟩, ht⟩
    · intro hr
      obtain ⟨hr, ht⟩ := mem_filter.1 hr
      obtain ⟨hw, he⟩ := mem_admit.1 hr
      exact mem_filter.2 ⟨mem_admit.2 ⟨(settle_slice hF h ht).2 hw, by rw [entitled_slice hm hF h hw ht]; exact he⟩, ht⟩
  exact keyedIn_part (admit_distinct may (settle_distinct W)).addressed
    (admit_distinct may (settle_distinct S)).addressed (fun r hr => (key r).1 hr)
    (fun r hr _ => (key r).2 hr) fun o => witnessed_same key o

/-! ## What does not carry over from `thing.slice`

**A `prev` target is a setter.** `thing.slice` sends the records a member
names in `prev`, and their envelopes are enough there. Under `keys` an
envelope still sets its keys. `d` sets `status`; `x` sets `note` and names
`d`; `y1`, `y2`, `y3` set `status` one after another from `d`. Every record
names every live record under it. The live records `{x, y3}` read alone give
the thing's keyed state. Add the records they name, `d` and `y2`, and `d`
reads as a live setter of `status`: nothing held shows `y3` above it. -/

def tg.d : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default,
    sets := some ["status"] }
def tg.x : Record :=
  { id := 2, kind := "task", observer := "sl:y", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["note"] }
def tg.y1 : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["status"] }
def tg.y2 : Record :=
  { id := 4, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [3], emitted := default,
    sets := some ["status"] }
def tg.y3 : Record :=
  { id := 5, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [4], emitted := default,
    sets := some ["status"] }
def tg.all : List Record := [tg.d, tg.x, tg.y1, tg.y2, tg.y3]

theorem target_needs_witness :
    Anchored tg.all ∧ liveOf tg.all = [tg.x, tg.y3] ∧
      keyedIn "k1" [tg.x, tg.y3] = keyedIn "k1" tg.all ∧
      keyedIn "k1" [tg.d, tg.x, tg.y2, tg.y3] ≠ keyedIn "k1" tg.all := by
  unfold Anchored
  decide +kernel

/-! **Erasers are for what is shown.** `e`, of another thing, erases the one
record of `k1`. Without it the part reads that record's payload as here. -/

def er.x : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", value := "v", prev := [], emitted := default,
    sets := some ["status"] }
def er.e : Record :=
  { id := 2, kind := "note", observer := "sl:x", of := some "k2", value := "", prev := [], emitted := default,
    erases := [1] }

theorem keyed_slice_needs_erasers :
    keyedIn "k1" (realm (fun _ _ => true) [er.x]) ≠ keyedIn "k1" (realm (fun _ _ => true) [er.x, er.e]) := by
  decide +kernel

end Loops

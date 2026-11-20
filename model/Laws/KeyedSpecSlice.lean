import Laws.LiveSlice
import Laws.SpecSlice
import Laws.KeyedSlots

/-!
# What a part must hold to answer a slot that follows `keys`

`spec.slice` answers slots that read records as written or by heads. A slot
that follows `keys` reads each thing it selects by its live setters, records
the spec need not take, so a spec slice is not enough for it
(`keyed_slot_needs_live`).

A part *covers* a thing when its records of the thing are records of the
whole, hold the thing's live records, and are witnessed for every key
(`Covers`): then it reads the thing's keyed state as the whole does
(`keyedIn_part`), and with it everything a keyed slot asks of the thing
(`Covers.thingEv`, `Covers.soleSetter`, `Covers.liveIn`, `Covers.thingKeys`).

A part answers every slot that follows `keys` when it covers each thing, or
neither reader can select the thing: the part holds no record of it the spec
takes, and the whole has no head of it the spec takes (`KeyedCover`,
`Slot.keyed_cover`). With a spec slice beside it, every slot of the spec is
answered (`Spec.run_slice_keyed`).
-/

namespace Loops

open List

/-! ## A covered thing -/

/-- The part `ys` reads thing `t` as `xs` does: its records of `t` are records
of `xs`, hold `t`'s live records, and are witnessed for every key. -/
structure Covers (t : String) (ys xs : List Record) : Prop where
  sub : ∀ r ∈ recordsOf t ys, r ∈ recordsOf t xs
  live : ∀ r ∈ recordsOf t xs, live (recordsOf t xs) r = true → r ∈ recordsOf t ys
  witnessed : ∀ o, Witnessed (recordsOf t ys) (recordsOf t xs) o

section Covered
variable {t : String} {ys xs : List Record}

theorem Covers.liveK (h : Covers t ys xs) (o : Option String) :
    ∀ a ∈ recordsOf t ys, liveK (recordsOf t ys) o a = liveK (recordsOf t xs) o a :=
  (witnessed_iff h.sub o).1 (h.witnessed o)

/-- A filter that agrees on the part, and whose records in the whole are live,
holds the same records in the part and in the whole. -/
theorem Covers.mem_filter (h : Covers t ys xs) {p q : Record → Bool}
    (h₁ : ∀ a ∈ recordsOf t ys, p a = q a)
    (h₂ : ∀ a ∈ recordsOf t xs, q a = true → Loops.live (recordsOf t xs) a = true) (r : Record) :
    r ∈ (recordsOf t ys).filter p ↔ r ∈ (recordsOf t xs).filter q := by
  constructor
  · intro hr
    obtain ⟨hr, hp⟩ := List.mem_filter.1 hr
    exact List.mem_filter.2 ⟨h.sub r hr, by rw [← h₁ r hr]; exact hp⟩
  · intro hr
    obtain ⟨hr, hq⟩ := List.mem_filter.1 hr
    have := h.live r hr (h₂ r hr hq)
    exact List.mem_filter.2 ⟨this, by rw [h₁ r this]; exact hq⟩

theorem Covers.mem_liveFor (h : Covers t ys xs) (f : String) (r : Record) :
    r ∈ (recordsOf t ys).filter (liveFor (recordsOf t ys) f) ↔
      r ∈ (recordsOf t xs).filter (liveFor (recordsOf t xs) f) :=
  h.mem_filter (fun a ha => h.liveK (some f) a ha)
    (fun _ _ hq => live_iff_liveK.2 ⟨some f, hq⟩) r

theorem Covers.keyedIn (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys) :
    keyedIn t ys = keyedIn t xs :=
  keyedIn_part hx hy h.sub h.live h.witnessed

theorem Covers.heads (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys) :
    (stateIn t ys).heads = (stateIn t xs).heads :=
  congrArg Keyed.heads (h.keyedIn hx hy)

theorem Covers.setters (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys) (f : String) :
    setters (recordsOf t ys) f = setters (recordsOf t xs) f :=
  orderHeads_set_decides (addressed_filter (addressed_recordsOf hy t) _)
    (addressed_filter (addressed_recordsOf hx t) _) (h.mem_liveFor f)

theorem Covers.liveIn (h : Covers t ys xs) (r : Record) : r ∈ liveIn ys t ↔ r ∈ liveIn xs t := by
  unfold Loops.liveIn
  refine h.mem_filter (fun a ha => ?_) (fun _ _ hq => hq) r
  apply Bool.eq_iff_iff.2
  simp only [live_iff_liveK]
  constructor
  · rintro ⟨o, ho⟩; exact ⟨o, by rw [← h.liveK o a ha]; exact ho⟩
  · rintro ⟨o, ho⟩; exact ⟨o, by rw [h.liveK o a ha]; exact ho⟩

theorem Covers.onThing (h : Covers t ys xs) (c : Cond) :
    c.onThing (recordsOf t ys) = c.onThing (recordsOf t xs) := by
  unfold Cond.onThing
  split
  · rfl
  · rename_i f _
    apply Tri.agree_congr
    intro a
    simp only [List.mem_map]
    constructor
    · rintro ⟨r, hr, e⟩; exact ⟨r, (h.mem_liveFor f r).1 hr, e⟩
    · rintro ⟨r, hr, e⟩; exact ⟨r, (h.mem_liveFor f r).2 hr, e⟩

theorem Covers.thingTest (h : Covers t ys xs) (cs : List Cond) :
    thingTest cs (recordsOf t ys) = thingTest cs (recordsOf t xs) := by
  unfold Loops.thingTest
  have : (fun (c : Cond) (v : Tri) => (c.onThing (recordsOf t ys)).and v) =
      fun c v => (c.onThing (recordsOf t xs)).and v := by
    funext c v; rw [h.onThing c]
  rw [this]

/-- Both readers select a covered thing alike. -/
theorem Covers.thingEv (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys)
    (sel : Record → Bool) (cs : List Cond) : thingEv sel cs ys t = thingEv sel cs xs t := by
  unfold Loops.thingEv
  rw [h.heads hx hy, h.thingTest cs]

theorem Covers.soleSetter (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys) (f : String) :
    soleSetter ys f t = soleSetter xs f t := by
  unfold Loops.soleSetter
  rw [h.setters hx hy f]

theorem Covers.thingKeys (h : Covers t ys xs) (hx : Addressed xs) (hy : Addressed ys) (b : By)
    (sel : Record → Bool) (cs : List Cond) : b.thingKeys sel cs ys t = b.thingKeys sel cs xs t := by
  unfold By.thingKeys
  cases b with
  | payload f => simp only; rw [h.setters hx hy f]
  | _ => simp only; rw [h.heads hx hy]

end Covered

/-! ## The things a slot keeps -/

section Slots
variable {ys xs : List Record}

theorem mem_thingsOf {zs : List Record} {t : String} : t ∈ thingsOf zs ↔ ∃ r ∈ zs, r.isOf t = true := by
  simp only [thingsOf, mem_canonSet, List.mem_map, isOf_iff]

/-- A thing a reader selects, or cannot answer for, has a head, and a part
that covers the thing holds it. -/
theorem thingEv_in {t : String} (hx : Addressed xs) (c : Covers t ys xs) {sel : Record → Bool}
    {cs : List Cond} (h : thingEv sel cs xs t ≠ .no) : t ∈ thingsOf ys := by
  unfold thingEv at h
  split at h
  · rename_i hany
    obtain ⟨r, hr, -⟩ := List.any_eq_true.1 hany
    obtain ⟨hrt, hl⟩ := (mem_stateIn_heads_iff hx).1 hr
    have := c.live r hrt (by simp [Loops.live, hl])
    exact mem_thingsOf.2 ⟨r, (List.mem_filter.1 this).1, (List.mem_filter.1 this).2⟩
  · exact absurd rfl h

/-- A reader selects no thing none of whose heads the spec takes. -/
theorem thingEv_untaken {zs : List Record} {t : String} {sel : Record → Bool} (cs : List Cond)
    (h : ∀ r ∈ (stateIn t zs).heads, sel r = false) : thingEv sel cs zs t = .no := by
  unfold thingEv
  split
  · rename_i hany
    obtain ⟨r, hr, hc⟩ := List.any_eq_true.1 hany
    simp [chosen, h r hr] at hc
  · rfl

variable {evy evx : String → Tri}

/-- Where every thing is covered or kept by neither reader, the two readers
keep the same things. -/
theorem things_filter
    (hc : ∀ t, Covers t ys xs ∨ (evy t = .no ∧ evx t = .no))
    (he : ∀ t, Covers t ys xs → evy t = evx t)
    (hin : ∀ t, Covers t ys xs → evx t ≠ .no → t ∈ thingsOf ys)
    (v : Tri) (hv : v ≠ .no) :
    (thingsOf ys).filter (evy · == v) = (thingsOf xs).filter (evx · == v) := by
  apply canon_ext List.filter_sublist List.filter_sublist
  intro t
  simp only [List.mem_filter, beq_iff_eq]
  constructor
  · rintro ⟨ht, e⟩
    rcases hc t with c | ⟨n, -⟩
    · refine ⟨?_, by rw [← he t c]; exact e⟩
      obtain ⟨r, hr, hrt⟩ := mem_thingsOf.1 ht
      exact mem_thingsOf.2 ⟨r, (List.mem_filter.1 (c.sub r (List.mem_filter.2 ⟨hr, hrt⟩))).1, hrt⟩
    · rw [n] at e; exact absurd e.symm hv
  · rintro ⟨-, e⟩
    rcases hc t with c | ⟨-, n⟩
    · exact ⟨hin t c (by rw [e]; exact hv), by rw [he t c]; exact e⟩
    · rw [n] at e; exact absurd e.symm hv

theorem filterMap_congr_mem {α β} {f g : α → Option β} {l : List α} (h : ∀ a ∈ l, f a = g a) :
    l.filterMap f = l.filterMap g := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    rw [List.filterMap_cons, List.filterMap_cons, h a List.mem_cons_self,
      ih fun b hb => h b (List.mem_cons_of_mem _ hb)]

theorem Acc.overThings_part {σ} (f : Acc σ) {py px : String → Option Record} {ty tx : List String}
    (hyes : ty.filter (evy · == .yes) = tx.filter (evx · == .yes))
    (hunk : ty.filter (evy · == .unknown) = tx.filter (evx · == .unknown))
    (hp : ∀ t ∈ tx.filter (evx · == .yes), py t = px t) :
    f.overThings py evy ty = f.overThings px evx tx := by
  unfold Acc.overThings
  simp only [hyes, hunk]
  have e : (tx.filter (evx · == .yes)).filter (fun t => !(py t).any f.readable) =
      (tx.filter (evx · == .yes)).filter (fun t => !(px t).any f.readable) :=
    List.filter_congr fun t ht => by rw [hp t ht]
  rw [filterMap_congr_mem hp, e]

/-- **A slot that follows `keys` reads the same from a part** that covers every
thing either reader keeps. -/
theorem keyedValue_part (hx : Addressed xs) (hy : Addressed ys) (ty : FoldType)
    (hc : ∀ t, Covers t ys xs ∨ (evy t = .no ∧ evx t = .no))
    (he : ∀ t, Covers t ys xs → evy t = evx t)
    (hin : ∀ t, Covers t ys xs → evx t ≠ .no → t ∈ thingsOf ys) :
    keyedValue ty ys evy = keyedValue ty xs evx := by
  have hyes := things_filter hc he hin .yes (by decide)
  have hunk := things_filter hc he hin .unknown (by decide)
  have cov : ∀ t ∈ (thingsOf xs).filter (evx · == .yes), Covers t ys xs := by
    intro t ht
    have e : evx t = .yes := by simpa using (List.mem_filter.1 ht).2
    rcases hc t with c | ⟨-, n⟩
    · exact c
    · rw [n] at e; cases e
  unfold keyedValue
  cases ty with
  | count => simp only [hyes, hunk]
  | sum f _ => simp only; rw [Acc.overThings_part _ hyes hunk fun t ht => (cov t ht).soleSetter hx hy f]
  | min f _ => simp only; rw [Acc.overThings_part _ hyes hunk fun t ht => (cov t ht).soleSetter hx hy f]
  | max f _ => simp only; rw [Acc.overThings_part _ hyes hunk fun t ht => (cov t ht).soleSetter hx hy f]
  | latestBy =>
    simp only [hyes, hunk]
    rw [List.map_congr_left fun t ht => by rw [(cov t ht).keyedIn hx hy]]
  | collect _ limit =>
    have sub : ∀ {zs : List Record} {ts : List String}, Addressed zs →
        Addressed (ts.flatMap (liveIn zs)) := fun hz a ha b hb =>
      hz a (by obtain ⟨_, _, ha⟩ := List.mem_flatMap.1 ha; exact liveIn_sub ha) b
        (by obtain ⟨_, _, hb⟩ := List.mem_flatMap.1 hb; exact liveIn_sub hb)
    simp only [hyes, hunk]
    rw [newestFirst_eq (sub hy) (sub hx) fun r => by
      simp only [List.mem_flatMap]
      constructor
      · rintro ⟨t, ht, hr⟩; exact ⟨t, ht, ((cov t ht).liveIn r).1 hr⟩
      · rintro ⟨t, ht, hr⟩; exact ⟨t, ht, ((cov t ht).liveIn r).2 hr⟩]

/-- Latest-by following `keys` is the keyed slot over the things `thingEv` selects. -/
theorem keyedOf_as_value (sel : Record → Bool) (cs : List Cond) (zs : List Record) :
    keyedOf sel cs zs = keyedValue (.latestBy .keys) zs (thingEv sel cs zs) := by
  unfold keyedOf keyedValue
  simp only
  have key : ∀ (L : List String) (v : Tri), v ≠ .no →
      ((L.filterMap fun t =>
          if (keyedIn t zs).heads.any (chosen sel cs) then
            some (t, keyedIn t zs, thingTest cs (zs.filter (·.isOf t))) else none).filter (·.2.2 == v)).map
          (fun x => x.1) = L.filter (thingEv sel cs zs · == v) := by
    intro L v hv
    induction L with
    | nil => rfl
    | cons t L ih =>
      rw [List.filterMap_cons, List.filter_cons]
      by_cases hch : (keyedIn t zs).heads.any (chosen sel cs) = true
      · have hev : thingEv sel cs zs t = thingTest cs (zs.filter (·.isOf t)) := by
          unfold thingEv; rw [← keyedIn_heads]; simp only [hch, ↓reduceIte]
        simp only [hch, ite_true, List.filter_cons, hev]
        split
        · simp only [List.map_cons, ih]
        · exact ih
      · have hev : thingEv sel cs zs t = .no := by
          unfold thingEv; rw [← keyedIn_heads]; simp [hch]
        have hne : (Tri.no == v) = false := by cases v <;> first | rfl | exact absurd rfl hv
        simp only [hch, hev, hne]
        exact ih
  have keyed : ∀ (L : List String),
      ((L.filterMap fun t =>
          if (keyedIn t zs).heads.any (chosen sel cs) then
            some (t, keyedIn t zs, thingTest cs (zs.filter (·.isOf t))) else none).filter
            (·.2.2 == .yes)).map (fun x => (x.1, x.2.1.view)) =
        (L.filter (thingEv sel cs zs · == .yes)).map fun t => (t, (keyedIn t zs).view) := by
    intro L
    induction L with
    | nil => rfl
    | cons t L ih =>
      rw [List.filterMap_cons, List.filter_cons]
      by_cases hch : (keyedIn t zs).heads.any (chosen sel cs) = true
      · have hev : thingEv sel cs zs t = thingTest cs (zs.filter (·.isOf t)) := by
          unfold thingEv; rw [← keyedIn_heads]; simp only [hch, ↓reduceIte]
        simp only [hch, ite_true, List.filter_cons, hev]
        split
        · simp only [List.map_cons, ih]
        · exact ih
      · have hev : thingEv sel cs zs t = .no := by
          unfold thingEv; rw [← keyedIn_heads]; simp [hch]
        simp only [hch, hev]
        exact ih
  congr 1
  · exact keyed _
  · rw [← key _ .unknown (by decide), List.length_map]

/-- A slot without `by` that follows `keys`, over a part. -/
theorem Slot.plain_keyed (hx : Addressed xs) (hy : Addressed ys) (s : Slot) (sel : Record → Bool)
    (hk : s.type.follow = some .keys)
    (hc : ∀ t, Covers t ys xs ∨
      (thingEv sel s.tests.conds ys t = .no ∧ thingEv sel s.tests.conds xs t = .no)) :
    s.plain sel ys = s.plain sel xs := by
  have he := fun t (c : Covers t ys xs) => c.thingEv hx hy sel s.tests.conds
  have hin := fun t (c : Covers t ys xs) (h : thingEv sel s.tests.conds xs t ≠ .no) => thingEv_in hx c h
  unfold Slot.plain
  split
  all_goals dsimp only
  all_goals rename_i heq
  all_goals rw [heq] at hk
  all_goals simp only [FoldType.follow] at hk
  · subst hk; simp only [ite_true]; exact keyedValue_part hx hy _ hc he hin
  · subst hk; simp only [ite_true]; exact keyedValue_part hx hy _ hc he hin
  · subst hk; simp only [ite_true]; exact keyedValue_part hx hy _ hc he hin
  · subst hk; simp only [ite_true]; exact keyedValue_part hx hy _ hc he hin
  · cases hk
  · rw [keyedOf_as_value, keyedOf_as_value]; exact keyedValue_part hx hy _ hc he hin
  · subst hk; simp only [ite_true]; exact keyedValue_part hx hy _ hc he hin

theorem flatMap_congr_mem {α β} {f g : α → List β} {l : List α} (h : ∀ a ∈ l, f a = g a) :
    l.flatMap f = l.flatMap g := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    rw [List.flatMap_cons, List.flatMap_cons, h a List.mem_cons_self,
      ih fun b hb => h b (List.mem_cons_of_mem _ hb)]

theorem filter_and {α} (L : List α) (p q : α → Bool) :
    L.filter (fun a => p a && q a) = (L.filter p).filter q := by
  rw [List.filter_filter]
  exact List.filter_congr fun a _ => Bool.and_comm _ _

/-- A slot with `by` that follows `keys`, over a part. -/
theorem Slot.keyedGrouped_part (hx : Addressed xs) (hy : Addressed ys) (s : Slot) (b : By)
    (sel : Record → Bool)
    (hc : ∀ t, Covers t ys xs ∨
      (thingEv sel s.tests.conds ys t = .no ∧ thingEv sel s.tests.conds xs t = .no)) :
    s.keyedGrouped b sel ys = s.keyedGrouped b sel xs := by
  have he := fun t (c : Covers t ys xs) => c.thingEv hx hy sel s.tests.conds
  have hin := fun t (c : Covers t ys xs) (h : thingEv sel s.tests.conds xs t ≠ .no) => thingEv_in hx c h
  have hyes := things_filter hc he hin .yes (by decide)
  have hunk := things_filter hc he hin .unknown (by decide)
  have cov : ∀ t ∈ (thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes), Covers t ys xs := by
    intro t ht
    have e : thingEv sel s.tests.conds xs t = .yes := by simpa using (List.mem_filter.1 ht).2
    rcases hc t with c | ⟨-, n⟩
    · exact c
    · rw [n] at e; cases e
  have hkeys := fun t ht => (cov t ht).thingKeys hx hy b sel s.tests.conds
  unfold Slot.keyedGrouped
  simp only [filter_and]
  rw [hyes, hunk]
  have e1 : ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).flatMap
        (fun t => (b.thingKeys sel s.tests.conds ys t).getD []) =
      ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).flatMap
        (fun t => (b.thingKeys sel s.tests.conds xs t).getD []) :=
    flatMap_congr_mem fun t ht => by rw [hkeys t ht]
  have e2 : ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).filter
        (fun t => (b.thingKeys sel s.tests.conds ys t).isNone) =
      ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).filter
        (fun t => (b.thingKeys sel s.tests.conds xs t).isNone) :=
    List.filter_congr fun t ht => by rw [hkeys t ht]
  have e3 : ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).filter
        (fun t => b.thingKeys sel s.tests.conds ys t == some []) =
      ((thingsOf xs).filter (thingEv sel s.tests.conds xs · == .yes)).filter
        (fun t => b.thingKeys sel s.tests.conds xs t == some []) :=
    List.filter_congr fun t ht => by rw [hkeys t ht]
  rw [e1, e2, e3]
  congr 1
  apply List.map_congr_left
  intro g _
  congr 2
  apply keyedValue_part hx hy
  · intro t
    rcases hc t with c | ⟨ny, nx⟩
    · exact .inl c
    · exact .inr ⟨by simp [ny], by simp [nx]⟩
  · intro t c
    rw [he t c, c.thingKeys hx hy b sel s.tests.conds]
  · intro t c h
    refine hin t c fun n => h ?_
    simp [n]

/-! ## The cover -/

/-- What a part `ys` of the folded records `xs` must be for the slots that
follow `keys`: it covers each thing, or neither reader can select the thing,
since the part holds no record of it that `sel` takes and the whole no head of
it that `sel` takes. -/
structure KeyedCover (sel : Record → Bool) (ys xs : List Record) : Prop where
  things : ∀ t, Covers t ys xs ∨
    ((∀ r ∈ recordsOf t ys, sel r = false) ∧ ∀ r ∈ (stateIn t xs).heads, sel r = false)

/-- **Every slot that follows `keys` gives the same value over the part.** -/
theorem Slot.keyed_cover {sel : Record → Bool} (hx : Addressed xs) (hy : Addressed ys)
    (h : KeyedCover sel ys xs) (s : Slot) (hk : s.type.follow = some .keys) :
    s.run sel ys = s.run sel xs := by
  have hc : ∀ t, Covers t ys xs ∨
      (thingEv sel s.tests.conds ys t = .no ∧ thingEv sel s.tests.conds xs t = .no) := by
    intro t
    rcases h.things t with c | ⟨hy', hx'⟩
    · exact .inl c
    · exact .inr ⟨thingEv_untaken _ fun r hr => hy' r ((mem_stateIn_heads_iff hy).1 hr).1,
        thingEv_untaken _ hx'⟩
  unfold Slot.run
  split
  · exact s.plain_keyed hx hy sel hk hc
  · simp only [hk, ite_true]
    exact s.keyedGrouped_part hx hy _ sel hc

end Slots

/-! ## The spec, behind settling and admission -/

section Realm
variable {may : Record → Record → Bool} {S W : List Record}

/-- A part of the forms held covers a thing when it owes each record of the
thing it holds its forms, its erasers and the whole's refusal (`KeyedPart`),
holds a form of every live record, and is witnessed. -/
theorem covers_of_keyedPart (hm : EnvOnly may) {t : String} (hF : Forms W) (h : KeyedPart may t S W)
    (hl : ∀ r ∈ recordsOf t (realm may W), live (recordsOf t (realm may W)) r = true → ∃ a ∈ S, a.id = r.id)
    (hw : ∀ o, Witnessed (recordsOf t (realm may S)) (recordsOf t (realm may W)) o) :
    Covers t (realm may S) (realm may W) where
  sub := fun r hr => by
    obtain ⟨hrS, ht⟩ := List.mem_filter.1 hr
    exact List.mem_filter.2 ⟨((realm_part hm hF h ht).1 hrS).1, ht⟩
  live := fun r hr hlr => by
    obtain ⟨hrW, ht⟩ := List.mem_filter.1 hr
    exact List.mem_filter.2 ⟨(realm_part hm hF h ht).2 ⟨hrW, hl r hr hlr⟩, ht⟩
  witnessed := hw

/-- With anchored names, the forms of a thing's live records, and nothing else
of the thing, cover it. -/
theorem covers_of_live (hm : EnvOnly may) {t : String} (hF : Forms W) (h : KeyedPart may t S W)
    (hc : NoCycle (recordsOf t (realm may W))) (ha : Anchored (recordsOf t (realm may W)))
    (hl : ∀ r ∈ recordsOf t (realm may W), live (recordsOf t (realm may W)) r = true → ∃ a ∈ S, a.id = r.id)
    (ho : ∀ r ∈ recordsOf t (realm may W), (∃ a ∈ S, a.id = r.id) → live (recordsOf t (realm may W)) r = true) :
    Covers t (realm may S) (realm may W) := by
  apply covers_of_keyedPart hm hF h hl
  intro o
  apply (anchored_witnessed hc ha o).congr
  intro r
  constructor
  · intro hr
    obtain ⟨hrW, hlr⟩ := List.mem_filter.1 hr
    have ht := (List.mem_filter.1 hrW).2
    exact List.mem_filter.2 ⟨(realm_part hm hF h ht).2 ⟨(List.mem_filter.1 hrW).1, hl r hrW hlr⟩, ht⟩
  · intro hr
    obtain ⟨hrS, ht⟩ := List.mem_filter.1 hr
    obtain ⟨hrW, hex⟩ := (realm_part hm hF h ht).1 hrS
    have hrt : r ∈ recordsOf t (realm may W) := List.mem_filter.2 ⟨hrW, ht⟩
    exact List.mem_filter.2 ⟨hrt, ho r hrt hex⟩

/-- **A refused record is owed its `prev` targets.** A part owes each record of
the thing it holds every form of it and what erases it, and either the whole
admits the record or the part holds what the record names in `prev`: it then
refuses the record exactly when the whole does. A spec slice can bring a record
of a covered thing that the whole refuses, as a target, and it brings the
targets only of the records the spec takes (`refused_needs_targets`). -/
theorem KeyedPart.of_targets (hm : EnvOnly may) {t : String} (hF : Forms W)
    (sub : ∀ r ∈ S, r ∈ W)
    (forms : ∀ a ∈ S, a.isOf t = true → ∀ b ∈ W, b.id = a.id → b ∈ S)
    (erasers : ∀ a ∈ S, a.isOf t = true → ∀ e ∈ W, e.erases.contains a.id = true → e ∈ S)
    (targets : ∀ r ∈ settle W, r.isOf t = true → (∃ a ∈ S, a.id = r.id) →
      entitled may (settle W) r = true ∨ ∀ q ∈ W, q.id ∈ r.prev → q ∈ S) :
    KeyedPart may t S W where
  sub := sub
  forms := forms
  erasers := erasers
  admitted := fun r hr ht ha he => by
    rcases targets r hr ht ha with h | h
    · exact h
    · rw [← entitled_part hm hF sub (SameEnvelope.refl r) h]; exact he

/-- **A cut refuses what the whole refuses.** A part closed downward holds what
each of its records names in `prev`, so it holds the target a record is
refused for. With every form of the thing's records it holds and what erases
them, it is a part the slice laws apply to, and nothing is asked about
admission. -/
theorem KeyedPart.of_down (hm : EnvOnly may) {t : String} (hF : Forms W) (h : Down S W)
    (forms : ∀ a ∈ S, a.isOf t = true → ∀ b ∈ W, b.id = a.id → b ∈ S)
    (erasers : ∀ a ∈ S, a.isOf t = true → ∀ e ∈ W, e.erases.contains a.id = true → e ∈ S) :
    KeyedPart may t S W :=
  KeyedPart.of_targets hm hF h.part forms erasers fun r hr _ ⟨a, ha, e⟩ => .inr fun q hq hqr => by
    obtain ⟨i, -, hp⟩ := List.mem_filterMap.1 hr
    obtain ⟨b, hb, -, hrb⟩ := pick_form hp
    have hba := (hF b hb a (h.part a ha) (by rw [e, ← hrb.id])).1
    exact h.names a ha q hq (by rw [← hba.prev, ← hrb.prev]; exact hqr)

/-- A part that holds no form of a record of the thing that the spec takes
folds no record of the thing that the spec takes. -/
theorem untaken_of_forms (sp : Spec) {t : String}
    (h : ∀ a ∈ S, a.isOf t = true → sp.takes a = false) :
    ∀ r ∈ recordsOf t (realm may S), sp.takes r = false := by
  intro r hr
  obtain ⟨hrS, ht⟩ := List.mem_filter.1 hr
  obtain ⟨a, ha, he⟩ := settle_form (mem_admit.1 hrS).1
  rw [sp.takes_env he]
  exact h a ha (by rw [← isOf_env he]; exact ht)

/-- **A spec whose slots all follow `keys`** is answered by a part whose folded
records cover every thing either reader could select. -/
theorem Spec.run_keyed (sp : Spec) (hk : ∀ s ∈ sp.slots, s.type.follow = some .keys)
    (hc : KeyedCover sp.takes (realm may S) (realm may W)) :
    sp.run (settle S) may = sp.run (settle W) may := by
  unfold Spec.run
  exact List.map_congr_left fun s hs => by
    rw [show admit may (settle S) = realm may S from rfl, show admit may (settle W) = realm may W from rfl,
      Slot.keyed_cover (realm_addressed may W) (realm_addressed may S) hc s (hk s hs)]

/-- **Any spec.** A spec slice answers the slots that read records as written
or by heads; with the cover beside it, the slots that follow `keys` are
answered too. -/
theorem Spec.run_slice_keyed {sp : Spec} (hm : EnvOnly may) (hF : Forms W) (h : SpecSlice sp S W)
    (hc : KeyedCover sp.takes (realm may S) (realm may W)) :
    sp.run (settle S) may = sp.run (settle W) may := by
  unfold Spec.run
  exact List.map_congr_left fun s _ => by
    rw [show admit may (settle S) = realm may S from rfl, show admit may (settle W) = realm may W from rfl]
    by_cases hk : s.type.follow = some .keys
    · rw [Slot.keyed_cover (realm_addressed may W) (realm_addressed may S) hc s hk]
    · rw [Slot.run_slice hm hF h s hk]

end Realm

/-! ## A spec slice is not enough

`m`, a note, sets `title`; `n`, a note, sets `status` from it; `h`, a task,
sets `est` from `n`. The spec takes tasks. Its spec slice holds `h` and the
record `h` names, `n`, and need not hold `m`. A reader of everything lists `m`
as the live setter of `title`; the slice reads no `title` at all. -/

def ks.spec : Spec :=
  { vertex := "v", sources := [{ store := ".", kinds := ["task"] }],
    slots := [{ name := "state", type := .latestBy .keys }] }

def ks.m : Record :=
  { id := 1, kind := "note", observer := "sl:x", of := some "k1", value := "", prev := [], emitted := default,
    sets := some ["title"] }
def ks.n : Record :=
  { id := 2, kind := "note", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["status"] }
def ks.h : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [2], emitted := default,
    sets := some ["est"] }

theorem keyed_slot_needs_live :
    SpecSlice ks.spec [ks.n, ks.h] [ks.m, ks.n, ks.h] ∧
      ks.spec.run (settle [ks.n, ks.h]) ≠ ks.spec.run (settle [ks.m, ks.n, ks.h]) := by
  refine ⟨⟨?_, ?_, ?_, ?_, ?_, ?_⟩, ?_⟩ <;> decide +kernel

/-! ## A refused record needs what it names

`y` is a note of another thing. `f`, a note of `k1`, names `y`, so a reader of
everything refuses `f`. `g`, a task of `k1`, names `f`. The spec takes tasks:
its spec slice holds `g` and the record `g` names, `f`, and need not hold `y`.
It holds the one live record of `k1`, `g`. Without `y` it admits `f` and lists
it as the live setter of `pri`; a reader of everything reads no `pri` at all. -/

def rf.y : Record :=
  { id := 1, kind := "note", observer := "sl:x", of := some "k2", value := "", prev := [], emitted := default,
    sets := some ["title"] }
def rf.f : Record :=
  { id := 2, kind := "note", observer := "sl:x", of := some "k1", value := "", prev := [1], emitted := default,
    sets := some ["pri"] }
def rf.g : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", value := "", prev := [2], emitted := default,
    sets := some ["est"] }

theorem refused_needs_targets :
    SpecSlice ks.spec [rf.f, rf.g] [rf.y, rf.f, rf.g] ∧
      ks.spec.run (settle [rf.f, rf.g]) ≠ ks.spec.run (settle [rf.y, rf.f, rf.g]) := by
  refine ⟨⟨?_, ?_, ?_, ?_, ?_, ?_⟩, ?_⟩ <;> decide +kernel

end Loops

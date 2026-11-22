import Laws.SpecText
import Loops.SpecHash
import Laws.Spec

/-! Identity of a written spec, independent of object member order. -/
namespace Loops
open List

theorem Rules.read_text (v : Rules) : Rules.ofJson? (.str v.text) = some v := by
  cases v with
  | mk n h => simp [Rules.ofJson?, Rules.text, natOf?_natDigits, h]

/-- The wire reader accepts precisely the positive version's own spelling. -/
theorem Rules.ofJson?_eq_some_iff {j : Json} {v : Rules} :
    Rules.ofJson? j = some v ↔ j = .str v.text := by
  constructor
  · intro h
    cases j <;> try contradiction
    rename_i s
    cases hn : natOf? s.toList with
    | none => simp [Rules.ofJson?, hn] at h
    | some n =>
      by_cases hp : 0 < n <;> simp [Rules.ofJson?, hn, hp] at h
      subst v
      exact congrArg Json.str (String.toList_inj.1 (by simpa [Rules.text] using natDigits_natOf? hn))
  · rintro rfl
    exact Rules.read_text _

private theorem digits_no_colon (n : Nat) : ':' ∉ natDigits n := by
  intro h
  obtain ⟨d, hd, e⟩ := mem_revDigits (List.mem_reverse.1 h)
  have hc := congrArg Char.toNat e
  rw [digitChar_toNat hd] at hc
  change 58 = 48 + d at hc
  omega

private theorem delim_inj {a b x y : List Char} (ha : ':' ∉ a) (hb : ':' ∉ b)
    (h : a ++ ':' :: x = b ++ ':' :: y) : a = b ∧ x = y := by
  induction a generalizing b with
  | nil =>
    cases b with
    | nil => exact ⟨rfl, (List.cons.inj h).2⟩
    | cons d b => exact False.elim (hb ((List.cons.inj h).1 ▸ List.mem_cons_self))
  | cons c a ih =>
    cases b with
    | nil => exact False.elim (ha ((List.cons.inj h).1 ▸ List.mem_cons_self))
    | cons d b =>
      have hd := List.cons.inj h
      obtain ⟨rfl, rfl⟩ := ih (fun hm => ha (mem_cons_of_mem _ hm)) (fun hm => hb (mem_cons_of_mem _ hm)) hd.2
      exact ⟨by rw [hd.1], rfl⟩

/-- Version and JCS text can both be recovered from the framed bytes. -/
theorem specPreimageOf_eq_iff {v w : Rules} {j k : Json} :
    specPreimageOf v j = specPreimageOf w k ↔ v = w ∧ j.canon = k.canon := by
  constructor
  · intro h
    simp only [specPreimageOf, ← utf8_append] at h
    have e := congrArg String.toList (utf8_injective h)
    simp only [specLabel, Rules.text, String.toList_append, String.toList_ofList, append_assoc,
      List.append_right_inj] at e
    obtain ⟨hd, ht⟩ := delim_inj (digits_no_colon _) (digits_no_colon _) e
    have hv : v.text = w.text := congrArg String.ofList hd
    have ver := congrArg (fun s => Rules.ofJson? (.str s)) hv
    simpa [Rules.read_text] using And.intro ver (String.toList_inj.1 ht)
  · rintro ⟨rfl, h⟩
    simp [specPreimageOf, h]

theorem specPreimage_ne_commit (v : Rules) (j p : Json) {salt : List UInt8} (hs : salt.length = 16) :
    specPreimageOf v j ≠ salt ++ utf8 p.canon := by
  intro h
  have hlabel : utf8 (specLabel v) = utf8 "loops/spec-hash/" ++ utf8 ("v" ++ v.text ++ ":") := by
    rw [← utf8_append, specLabel, ← String.append_assoc, ← String.append_assoc]
    rfl
  have h16 : (utf8 "loops/spec-hash/").length = 16 := by rw [utf8_eq]; rfl
  have e := congrArg (List.drop 16) h
  rw [specPreimageOf, hlabel, List.append_assoc, List.drop_left' h16, List.drop_left' hs, ← utf8_append] at e
  have ht := congrArg String.toList (utf8_injective e)
  obtain ⟨c, cs, hc, hj⟩ := Json.canon_head p
  rw [hc] at ht
  have : c = 'v' := by simpa [String.toList_append] using (congrArg List.head? ht).symm
  subst c
  simp [jsonHead] at hj

theorem specPreimage_ne_result (v : Rules) (j : Json) (r : List (String × SlotValue)) :
    specPreimageOf v j ≠ resultPreimage r := by
  intro h
  simp only [specPreimageOf, specLabel, resultPreimage, utf8_append, List.append_assoc] at h
  have : (utf8 "loops/spec-hash/v" ++ (utf8 v.text ++ (utf8 ":" ++ utf8 j.canon)))[6]? =
      (utf8 resultLabel ++ resultBytes r)[6]? := congrArg (·[6]?) h
  rw [List.getElem?_append_left (by rw [utf8_eq]; decide),
      List.getElem?_append_left (by rw [utf8_eq]; decide), utf8_eq, utf8_eq] at this
  contradiction

/-! ## The three unordered parts of a parsed spec -/

/-- A total order compatible with the fixed order used by the parsed-form writer.
It is used only to compare parsed structures. JCS always uses UTF-16 order. -/
private def memberOrder (s : String) : List Nat :=
  let rank := match s with
    | "from" | "kind" | "follow" => 1
    | "fold" | "about" | "limit" => 2
    | "of" | "by" => 3
    | "where" => 4
    | _ => 0
  rank :: utf16 s

/-- Order the three kinds of object members a parsed spec stores as lists. -/
def Tests.orderMembers (ts : Tests) : Tests :=
  (ts.map fun (f, os) => (f, os.mergeSort fun a b => memberOrder a.key ≤ memberOrder b.key)).mergeSort
    fun a b => memberOrder a.1.name ≤ memberOrder b.1.name

def Slot.orderMembers (s : Slot) : Slot := { s with tests := s.tests.orderMembers }

def Spec.orderMembers (s : Spec) : Spec :=
  { s with slots := (s.slots.map Slot.orderMembers).mergeSort fun a b => memberOrder a.name ≤ memberOrder b.name }

/-- Preserves every array, and ignores exactly the three member-list orders. -/
def Spec.MemberEq (a b : Spec) : Prop := a.orderMembers = b.orderMembers

theorem Tests.orderMembers_conds (ts : Tests) : ts.orderMembers.conds ~ ts.conds := by
  unfold Tests.orderMembers Tests.conds
  apply (List.Perm.flatMap_right _ (List.mergeSort_perm _ _)).trans
  simp only [List.flatMap_map]
  induction ts with
  | nil => rfl
  | cons t ts ih => exact ((List.mergeSort_perm _ _).map (Cond.mk t.1)).append ih

private theorem Slot.run_conds_perm (s : Slot) (ts : Tests) (hp : ts.conds ~ s.tests.conds)
    (sel : Record → Bool) (xs : List Record) :
    ({s with tests := ts} : Slot).run sel xs = s.run sel xs := by
  have allEq := funext (Cond.all_perm hp)
  have envEq := funext (Cond.all_perm (hp.filter (·.field.onEnvelope)))
  have payEq := funext (Cond.all_perm (hp.filter (!·.field.onEnvelope)))
  have thingEq : thingTest ts.conds = thingTest s.tests.conds := by
    funext rs
    exact (hp.filter (!·.field.onEnvelope)).foldr_eq'
      (fun x _ y _ z => Tri.and_left_comm (y.onThing rs) (x.onThing rs) z) .yes
  have chosenEq : chosen sel ts.conds = chosen sel s.tests.conds := by
    funext r
    simp only [chosen, envEq]
  have thingEvEq : thingEv sel ts.conds = thingEv sel s.tests.conds := by
    funext rs t
    simp only [thingEv, chosenEq, thingEq]
  have keysEq (b : By) : b.thingKeys sel ts.conds = b.thingKeys sel s.tests.conds := by
    funext rs t
    simp only [By.thingKeys, chosenEq]
  have evEq : ({s with tests := ts} : Slot).ev sel = s.ev sel := by
    funext r
    simp only [Slot.ev, allEq]
  cases s with
  | mk name ty tests group =>
    simp only at *
    simp only [Slot.run, Slot.plain, Slot.grouped, Slot.keyedGrouped, evEq, Slot.ev, Slot.pool,
      latestOf, collectOf, keyedOf, payTest, allEq, chosenEq, payEq, thingEq, thingEvEq, keysEq]

private theorem Perm.resultText_eq {rs qs : List (String × SlotValue)} (h : rs ~ qs)
    (hn : (qs.map (·.1)).Nodup) : resultText rs = resultText qs := by
  apply canon_member_order (h.map _)
  simpa [List.map_map, Function.comp_def] using (h.map Prod.fst).nodup_iff.2 hn

theorem Spec.resultBytes_orderMembers (sp : Spec) (hn : (sp.slots.map (·.name)).Nodup)
    (xs : List Record) (may : Record → Record → Bool) :
    resultBytes (sp.orderMembers.run xs may) = resultBytes (sp.run xs may) := by
  apply congrArg utf8
  apply Perm.resultText_eq _ (by simpa [Spec.run_names] using hn)
  unfold Spec.run Spec.orderMembers
  apply ((List.mergeSort_perm _ _).map _).trans
  apply List.Perm.of_eq
  simp only [List.map_map, Function.comp_def, Slot.orderMembers]
  exact List.map_congr_left fun s _ => congrArg (Prod.mk s.name)
    (Slot.run_conds_perm s _ (Tests.orderMembers_conds _) _ _)

theorem Spec.memberEq_resultBytes {a b : Spec} (h : a.MemberEq b)
    (ha : (a.slots.map (·.name)).Nodup) (hb : (b.slots.map (·.name)).Nodup)
    (xs : List Record) (may : Record → Record → Bool) :
    resultBytes (a.run xs may) = resultBytes (b.run xs may) := by
  rw [← a.resultBytes_orderMembers ha, ← b.resultBytes_orderMembers hb, show a.orderMembers = b.orderMembers from h]

/-! ## Reordering preserves the written spec -/

private theorem canon_obj_map {α : Type} (xs : List α) (f g : α → String × Json)
    (h : ∀ x ∈ xs, (f x).1 = (g x).1 ∧ (f x).2.canon = (g x).2.canon) :
    (Json.obj (xs.map f)).canon = (Json.obj (xs.map g)).canon := by
  have hc : Json.canonMembers (xs.map f) = Json.canonMembers (xs.map g) := by
    simp only [canonMembers_eq, map_map]
    exact map_congr_left fun x hx => Prod.ext (h x hx).1 (h x hx).2
  simp only [Json.canon, hc]

private theorem Tests.canon_orderMembers {ts : Tests} (h : ts = [] ∨ ts.Fits) :
    ts.orderMembers.toJson.canon = ts.toJson.canon := by
  rcases h with rfl | ⟨_, hd, hf⟩
  · simp [Tests.orderMembers]
  let mapped := ts.map fun (f, os) => (f, os.mergeSort fun a b => memberOrder a.key ≤ memberOrder b.key)
  let fieldLe := fun (a b : Field × List Op) => decide (memberOrder a.1.name ≤ memberOrder b.1.name)
  have hfields := (mergeSort_perm mapped fieldLe).map (fun p => p.1.name)
  have hn : (mapped.map (fun p => p.1.name)).Nodup := by simpa [mapped, map_map, Function.comp_def] using hd
  let member := fun (p : Field × List Op) => (p.1.name, Json.obj (p.2.map Op.toJson))
  change (Json.obj ((mapped.mergeSort fieldLe).map member)).canon = _
  rw [canon_member_order ((mergeSort_perm mapped fieldLe).map member)
    (by simpa [map_map, member, Function.comp_def] using hfields.nodup_iff.2 hn)]
  unfold mapped Tests.toJson
  rw [map_map]
  apply canon_obj_map
  intro p hp
  exact ⟨rfl, canon_member_order ((mergeSort_perm _ _).map Op.toJson)
    (by rw [map_map]; exact
      ((mergeSort_perm p.2 (fun a b => decide (memberOrder a.key ≤ memberOrder b.key))).map Op.key).nodup_iff.2 (hf p hp).2.2.1)⟩

private theorem Slot.canon_orderMembers {s : Slot} (h : s.Fits) :
    s.orderMembers.toJson.2.canon = s.toJson.2.canon := by
  have hw := Tests.canon_orderMembers h.2.2.1
  have he : s.tests.orderMembers.isEmpty = s.tests.isEmpty := by
    apply Bool.eq_iff_iff.2
    simp only [List.isEmpty_iff_length_eq_zero, Tests.orderMembers, List.length_mergeSort, List.length_map]
  simp only [Slot.orderMembers, Slot.toJson, he, Json.canon, canonMembers_eq, map_append]
  cases e : s.tests.isEmpty <;> simp [hw]

private theorem Spec.canon_orderMembers {sp : Spec} (h : sp.Fits) :
    sp.orderMembers.toJson.canon = sp.toJson.canon := by
  have hn := h.2.2.2.1
  let slots := sp.slots.map Slot.orderMembers
  have hp := (mergeSort_perm slots (fun a b => decide (memberOrder a.name ≤ memberOrder b.name))).map Slot.toJson
  have name (s : Slot) : s.toJson.1 = s.name := rfl
  have hd : ((slots.mergeSort (fun a b => decide (memberOrder a.name ≤ memberOrder b.name))).map (·.name)).Nodup :=
    ((mergeSort_perm slots _).map Slot.name).nodup_iff.2 (by simpa [slots, map_map, Function.comp_def, Slot.orderMembers] using hn)
  have hf := canon_member_order hp (by simpa [map_map, Function.comp_def, name] using hd)
  have hm : (Json.obj (slots.map Slot.toJson)).canon = (Json.obj (sp.slots.map Slot.toJson)).canon := by
    unfold slots
    rw [map_map]
    exact canon_obj_map _ _ _ fun s hs => ⟨rfl, Slot.canon_orderMembers (h.2.2.2.2 s hs)⟩
  have he : (Json.obj (sp.orderMembers.slots.map Slot.toJson)).canon =
      (Json.obj (sp.slots.map Slot.toJson)).canon := hf.trans hm
  let members (fold : Json) := [("vertex", Json.str sp.vertex),
    ("from", Json.arr (sp.sources.map Source.toJson)), ("fold", fold)]
  have eqMembers : Json.canonMembers (members (.obj (sp.orderMembers.slots.map Slot.toJson))) =
      Json.canonMembers (members (.obj (sp.slots.map Slot.toJson))) := by
    simp only [members, Json.canonMembers, he]
  change (Json.obj (members _)).canon = (Json.obj (members _)).canon
  simp only [Json.canon, eqMembers]

theorem Spec.memberEq_preimage {a b : Spec} (ha : a.Fits) (hb : b.Fits) (h : a.MemberEq b) (v : Rules) :
    specPreimageOf v a.toJson = specPreimageOf v b.toJson := by
  apply specPreimageOf_eq_iff.2
  exact ⟨rfl, (Spec.canon_orderMembers ha).symm.trans ((congrArg (fun s : Spec => s.toJson.canon) h).trans
    (Spec.canon_orderMembers hb))⟩

/-- No property of the digest function is needed in the forward direction. -/
theorem Spec.memberEq_hash {a b : Spec} (ha : a.Fits) (hb : b.Fits) (h : a.MemberEq b)
    (H : Hash) (v : Rules) : a.hash H v = b.hash H v := by
  unfold Spec.hash specHashOf
  rw [Spec.memberEq_preimage ha hb h]

/-! ## Recovering the parsed spec from equal preimages -/

private abbrev OrderedJson := SpecText.Ordered memberOrder

/-- Sorting distinct names selects a single representative of their permutation class. -/
private theorem ordered_map_obj {α : Type} (xs : List α) (f : α → String × Json)
    (hn : (xs.map (fun x => (f x).1)).Nodup) (hv : ∀ x ∈ xs, OrderedJson (f x).2) :
    OrderedJson (.obj ((xs.mergeSort (fun a b => memberOrder (f a).1 ≤ memberOrder (f b).1)).map f)) := by
  let key := fun x => memberOrder (f x).1
  let le := fun a b => decide (key a ≤ key b)
  have sorted := List.pairwise_mergeSort (le := le)
    (fun _ _ _ h k => decide_eq_true (List.le_trans (of_decide_eq_true h) (of_decide_eq_true k)))
    (fun a b => by simpa [le] using List.le_total (key a) (key b)) xs
  have unique := ((mergeSort_perm xs le).map (fun x => (f x).1)).nodup_iff.2 hn
  apply SpecText.Ordered.obj
  · apply pairwise_map.2
    apply (pairwise_and_iff.2 ⟨sorted, pairwise_map.1 unique⟩).imp
    rintro a b ⟨hab, hne⟩
    exact Or.resolve_right (List.le_iff_lt_or_eq.1 (of_decide_eq_true hab)) fun eq =>
      hne (utf16_inj (List.cons.inj eq).2)
  · intro m hm
    obtain ⟨x, hx, rfl⟩ := mem_map.1 hm
    exact hv x (mem_mergeSort.1 hx)

private theorem arg_json {ty : FieldType} {a : Arg} (h : a.Fits ty ∨ a.Bounds ty) :
    OrderedJson a.toJson := by
  cases a with
  | time c => exact .str _
  | val v =>
    cases v with
    | str _ => exact .str _
    | int _ => exact .int _
    | bool _ => exact .bool _
    | number _ => cases ty <;> simp [Arg.Fits, Arg.Bounds] at h

private theorem op_json {ty : FieldType} {o : Op} (h : o.Fits ty) : OrderedJson o.toJson.2 := by
  cases o with
  | is _ | not _ => exact arg_json (.inl h)
  | atLeast _ | below _ => exact arg_json (.inr h)
  | present | absent => exact .bool _

private theorem Tests.orderedJson {ts : Tests} (hf : ts = [] ∨ ts.Fits) :
    OrderedJson ts.orderMembers.toJson := by
  rcases hf with rfl | ⟨_, hd, hall⟩
  · simp only [Tests.orderMembers, map_nil, mergeSort_nil, Tests.toJson]
    exact .obj .nil (by simp)
  unfold Tests.orderMembers Tests.toJson
  apply ordered_map_obj
  · simpa [map_map, Function.comp_def] using hd
  · intro t ht
    obtain ⟨p, hp, rfl⟩ := mem_map.1 ht
    exact ordered_map_obj p.2 Op.toJson (hall p hp).2.2.1 (fun o ho => op_json ((hall p hp).2.2.2 o ho))

private theorem Tests.fits_orderMembers {ts : Tests} (hf : ts.Fits) : ts.orderMembers.Fits := by
  obtain ⟨hne, hd, hall⟩ := hf
  let ordered := ts.map fun (f, ops) => (f, ops.mergeSort fun a b => memberOrder a.key ≤ memberOrder b.key)
  have hp := mergeSort_perm ordered (fun a b => decide (memberOrder a.1.name ≤ memberOrder b.1.name))
  refine ⟨fun e => hne (by simpa [Tests.orderMembers] using congrArg List.length e), ?_, ?_⟩
  · exact (hp.map (fun p => p.1.name)).nodup_iff.2 (by simpa [ordered, map_map, Function.comp_def] using hd)
  · intro t ht
    obtain ⟨p, h, rfl⟩ := mem_map.1 (hp.mem_iff.1 ht)
    obtain ⟨hf, hne, hd, ho⟩ := hall p h
    have perm := mergeSort_perm p.2 (fun a b => decide (memberOrder a.key ≤ memberOrder b.key))
    exact ⟨hf, fun e => hne ((perm.symm.trans (.of_eq e)).eq_nil), (perm.map Op.key).nodup_iff.2 hd,
      fun o hm => ho o (perm.mem_iff.1 hm)⟩

private theorem Slot.fits_orderMembers {s : Slot} (h : s.Fits) : s.orderMembers.Fits := by
  refine ⟨h.1, h.2.1, ?_, h.2.2.2⟩
  rcases h.2.2.1 with he | hf
  · left; simp [Slot.orderMembers, he, Tests.orderMembers]
  · exact Or.inr (Tests.fits_orderMembers hf)

private theorem Spec.fits_orderMembers {s : Spec} (h : s.Fits) : s.orderMembers.Fits := by
  have hp := mergeSort_perm (s.slots.map Slot.orderMembers) (fun a b => decide (memberOrder a.name ≤ memberOrder b.name))
  refine ⟨h.1, h.2.1, h.2.2.1, ?_, ?_⟩
  · exact (hp.map Slot.name).nodup_iff.2 (by simpa [map_map, Function.comp_def, Slot.orderMembers] using h.2.2.2.1)
  · intro slot hm
    obtain ⟨x, hx, rfl⟩ := mem_map.1 (hp.mem_iff.1 hm)
    exact Slot.fits_orderMembers (h.2.2.2.2 x hx)

private theorem ordered_obj {ms : List (String × Json)}
    (hn : (ms.map Prod.fst).Pairwise (fun a b => memberOrder a < memberOrder b))
    (hv : ∀ v ∈ ms.map Prod.snd, OrderedJson v) : OrderedJson (.obj ms) :=
  .obj (by rwa [pairwise_map] at hn) (fun m hm => hv _ (mem_map_of_mem hm))

private theorem Source.orderedJson (s : Source) : OrderedJson s.toJson := by
  have hk : OrderedJson (.arr (s.kinds.map Json.str)) := .arr fun j hj => by
    obtain ⟨_, _, rfl⟩ := mem_map.1 hj
    exact .str _
  cases he : s.kinds.isEmpty <;> cases ha : s.about <;> cases ho : s.of <;>
    apply ordered_obj <;>
    simp +decide [he, ha, ho, SpecText.Ordered.str, hk]

private theorem Slot.orderedJson {s : Slot} (hs : s.Fits) : OrderedJson s.orderMembers.toJson.2 := by
  have hw := Tests.orderedJson hs.2.2.1
  rcases s with ⟨name, ty, tests, group⟩
  change OrderedJson tests.orderMembers.toJson at hw
  cases ty
  case' count f | sum _ f | min _ f | max _ f => cases f
  case' collect f lim => cases f <;> cases lim
  all_goals
    cases he : tests.orderMembers.isEmpty <;> cases group <;>
      apply ordered_obj <;>
      simp +decide [Slot.orderMembers, he, followJson, byJson, SpecText.Ordered.bool, SpecText.Ordered.str, SpecText.Ordered.int, hw]

private theorem Spec.orderedJson {s : Spec} (hs : s.Fits) : OrderedJson s.orderMembers.toJson := by
  have sources : OrderedJson (.arr (s.sources.map Source.toJson)) := .arr fun j hj => by
    obtain ⟨_, _, rfl⟩ := mem_map.1 hj
    exact Source.orderedJson _
  have slots : OrderedJson (.obj (s.orderMembers.slots.map Slot.toJson)) := by
    unfold Spec.orderMembers
    apply ordered_map_obj
    · simpa [map_map, Function.comp_def, Slot.orderMembers, show ∀ s : Slot, s.toJson.1 = s.name from fun _ => rfl] using hs.2.2.2.1
    · intro slot hm
      obtain ⟨x, hx, rfl⟩ := mem_map.1 hm
      exact Slot.orderedJson (hs.2.2.2.2 x hx)
  apply ordered_obj
  · change Pairwise _ ["vertex", "from", "fold"]; decide
  · change OrderedJson (.arr (s.orderMembers.sources.map Source.toJson)) at sources
    simp [SpecText.Ordered.str, sources, slots]

/-- Exactly the version and the parsed spec up to object member order are in
the preimage. The hypotheses characterize the image of the spec parser. -/
theorem Spec.preimage_eq_iff {a b : Spec} (ha : a.Fits) (hb : b.Fits) {v w : Rules} :
    specPreimageOf v a.toJson = specPreimageOf w b.toJson ↔ v = w ∧ a.MemberEq b := by
  constructor
  · intro h
    obtain ⟨ver, eqn⟩ := specPreimageOf_eq_iff.1 h
    have canon := (Spec.canon_orderMembers ha).trans (eqn.trans (Spec.canon_orderMembers hb).symm)
    have eqj := SpecText.Ordered.canon_inj (Spec.orderedJson ha) (Spec.orderedJson hb) canon
    exact ⟨ver, Except.ok.inj ((Spec.ofJson_toJson (Spec.fits_orderMembers ha)).symm.trans
      ((congrArg Spec.ofJson eqj).trans (Spec.ofJson_toJson (Spec.fits_orderMembers hb))))⟩
  · rintro ⟨rfl, eqn⟩
    exact Spec.memberEq_preimage ha hb eqn _

end Loops

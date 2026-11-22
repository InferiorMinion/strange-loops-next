import Loops.Tick
import Laws.SpecHash
import Laws.Payload

namespace Loops
open List

theorem recordIdOf?_idText (id : RecordId) : recordIdOf? (.str (idText id)) = some id := by
  simp [recordIdOf?, idText, String.toList_append, unbase32_base32, idDigest_length, idDigest_bytes_roundtrip]

private theorem mapM_writes {α β ε} (f : α → Except ε β) (g : α → β) {xs : List α}
    (h : ∀ x ∈ xs, f x = .ok (g x)) : xs.mapM f = .ok (xs.map g) := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
    rw [mapM_cons, h _ mem_cons_self, ih (fun y hy => h y (mem_cons_of_mem _ hy))]
    rfl

private theorem ids_read (p : String) (xs : List (RecordId × Shown)) :
    ResultReader.ids p (idsJson xs) = .ok (unshownIds xs) := by
  unfold ResultReader.ids idsJson unshownIds
  simp only [zipIdx_map, List.mapM_map]
  rw [mapM_writes _ (fun x => (x.1.1, Shown.here)) (by
    intro x _; simp [recordIdOf?_idText, ResultReader.required]; rfl)]
  apply congrArg Except.ok
  simpa only [map_map, Function.comp_def] using
    congrArg (List.map (fun h : RecordId × Shown => (h.1, Shown.here))) (zipIdx_map_fst 0 xs)

private theorem readNamed {α β : Type} (f : String → Json → Except String β) (write : α → Json)
    (norm : α → β) (xs : List (String × α)) (p : String) (hn : (xs.map Prod.fst).Nodup)
    (h : ∀ x ∈ xs, ∀ q, f q (write x.2) = .ok (norm x.2)) :
    ResultReader.named f p (.obj (xs.map fun x => (x.1, write x.2))) =
      .ok (xs.map fun x => (x.1, norm x.2)) := by
  simp only [ResultReader.named, ResultReader.object, map_map, Function.comp_def, hn, ↓reduceIte,
    pure, Except.pure, bind, Except.bind, List.mapM_map]
  apply mapM_writes
  intro x hx
  simp [h x hx]

/-- The constructor present at this position in a result for the given fold. -/
def GroupValue.FitsFold : GroupValue → FoldType → Prop
  | .nat _, .count _ | .int _, .sum .. | .opt _, .min .. | .opt _, .max .. | .heads .., .collect .. => True
  | _, _ => False

private theorem readNat (n : Nat) (p : String) : ResultReader.nat p (intJson n) = .ok n := by
  simp [ResultReader.nat, intJson, intText, natOf?_natDigits, ResultReader.required]

private theorem readInt (i : Int) (p : String) : ResultReader.integer p (intJson i) = .ok i := by
  simp [ResultReader.integer, intJson, intOf?_intText, ResultReader.required]

private theorem readOpt (i : Option Int) (p : String) :
    ResultReader.optInt p (match i with | some v => intJson v | none => .null) = .ok i := by
  cases i with
  | none => rfl
  | some v => change some <$> ResultReader.integer p (intJson v) = _; rw [readInt]; rfl

private theorem readSome (v : Int) (p : String) : ResultReader.optInt p (intJson v) = .ok (some v) :=
  readOpt (some v) p

private theorem readNone (p : String) : ResultReader.optInt p .null = .ok none := rfl

private theorem readGroup (g : GroupValue) (ty : FoldType) (p : String) (h : g.FitsFold ty) :
    ResultReader.group ty p g.result = .ok g.unshown := by
  cases ty <;> cases g <;> simp only [GroupValue.FitsFold] at h <;> try contradiction
  case' min.opt _ _ r | max.opt _ _ r => rcases r with ⟨_ | v, u⟩
  all_goals
    simp [ResultReader.group, GroupValue.result, GroupValue.unshown, ResultReader.fixed, ResultReader.object,
      ResultReader.get, List.lookup, readNat, readInt, readSome, readNone, ids_read, bind, Except.bind, pure, Except.pure]

/-- A slot's value has its stated fold shape, with distinct names in its maps. -/
def SlotValue.FitsSlot (v : SlotValue) (s : Slot) : Prop :=
  match s.group, s.type, v with
  | some _, ty, .groups gs .. => (gs.map Prod.fst).Nodup ∧ ∀ g ∈ gs, g.2.FitsFold ty
  | none, .latestBy .prev, .things ts _ => (ts.map Prod.fst).Nodup
  | none, .latestBy .keys, .keyed ts _ => (ts.map Prod.fst).Nodup ∧ ∀ t ∈ ts, (t.2.keys.map Prod.fst).Nodup
  | none, .count _, .nat _ | none, .sum .., .int _ | none, .min .., .opt _ | none, .max .., .opt _
  | none, .collect .., .heads .. => True
  | _, _, _ => False

private theorem readKeyed (k : KeyedView) (p : String) (h : (k.keys.map Prod.fst).Nodup) :
    ResultReader.keyed p k.result = .ok k.unshown := by
  have keys := fun q => readNamed ResultReader.ids idsJson unshownIds k.keys q h (fun _ _ path => ids_read path _)
  simp [ResultReader.keyed, KeyedView.result, KeyedView.unshown, ResultReader.fixed, ResultReader.object,
    ResultReader.get, List.lookup, ids_read, keys, bind, Except.bind, pure, Except.pure]

theorem SlotValue.read_result (v : SlotValue) (s : Slot) (p : String) (h : v.FitsSlot s) :
    ResultReader.slot s p v.result = .ok v.unshown := by
  cases s with
  | mk name ty tests grp =>
    cases grp with
    | some b =>
      cases v <;> try (cases ty <;> contradiction)
      rename_i gs u m
      have hf : (gs.map Prod.fst).Nodup ∧ ∀ g ∈ gs, g.2.FitsFold ty := by cases ty <;> exact h
      have groups := fun q => readNamed (ResultReader.group ty) GroupValue.result GroupValue.unshown gs q hf.1
        (fun g hg path => readGroup _ _ path (hf.2 g hg))
      simp [ResultReader.slot, SlotValue.result, SlotValue.unshown, ResultReader.fixed, ResultReader.object,
        ResultReader.get, List.lookup, groups, readNat, bind, Except.bind, pure, Except.pure]
    | none =>
      cases ty <;> cases v <;> try contradiction
      case latestBy.things f ts u =>
        cases f <;> try contradiction
        have things := fun q => readNamed ResultReader.ids idsJson unshownIds ts q h (fun _ _ path => ids_read path _)
        simp [ResultReader.slot, SlotValue.result, SlotValue.unshown, ResultReader.fixed, ResultReader.object,
          ResultReader.get, List.lookup, things, readNat, bind, Except.bind, pure, Except.pure]
      case latestBy.keyed f ts u =>
        cases f <;> try contradiction
        have things := fun q => readNamed ResultReader.keyed KeyedView.result KeyedView.unshown ts q h.1
          (fun t ht path => readKeyed _ path (h.2 t ht))
        simp [ResultReader.slot, SlotValue.result, SlotValue.unshown, ResultReader.fixed, ResultReader.object,
          ResultReader.get, List.lookup, things, readNat, bind, Except.bind, pure, Except.pure]
      case' min.opt _ _ r | max.opt _ _ r => rcases r with ⟨_ | v, u⟩
      all_goals first
        | solve | cases ‹Follow› <;> contradiction
        | simp [ResultReader.slot, ResultReader.group, SlotValue.result, GroupValue.result, SlotValue.unshown,
            ResultReader.fixed, ResultReader.object, ResultReader.get, List.lookup, ids_read, readSome, readNone, readNat,
            readInt, bind, Except.bind, pure, Except.pure]

private theorem lookup_of_pair {α : Type} {xs : List (String × α)} (hn : (xs.map Prod.fst).Nodup)
    {k : String} {v : α} (hm : (k,v) ∈ xs) : xs.lookup k = some v := by
  induction xs with
  | nil => contradiction
  | cons x xs ih =>
    rcases mem_cons.1 hm with rfl | ht
    · simp
    · have ne : k ≠ x.1 := fun e => (List.nodup_cons.1 hn).1 (e ▸ mem_map_of_mem (f := Prod.fst) ht)
      simpa [List.lookup, show (k == x.1) = false by simpa using ne] using ih (List.nodup_cons.1 hn).2 ht

theorem readResult_written (s : Spec) (values : Slot → SlotValue) (hn : (s.slots.map Slot.name).Nodup)
    (hv : ∀ a ∈ s.slots, (values a).FitsSlot a) :
    let r := s.slots.map fun a => (a.name, values a)
    readResult s (resultJson r) = .ok (unshownResult r) := by
  let kvs := s.slots.map fun a => (a.name, (values a).result)
  have keys : kvs.map Prod.fst = s.slots.map Slot.name := by simp [kvs, map_map, Function.comp_def]
  have pair (a : Slot) (ha : a ∈ s.slots) : kvs.lookup a.name = some (values a).result :=
    lookup_of_pair (keys ▸ hn) (mem_map_of_mem (f := fun a => (a.name, (values a).result)) ha)
  have extra : kvs.find? (fun kv => !(s.slots.map Slot.name).contains kv.1) = none := by
    apply find?_eq_none.2; intro kv hk; simp [← keys, mem_map_of_mem (f := Prod.fst) hk]
  have miss : (s.slots.map Slot.name).find? (fun k => (kvs.lookup k).isNone) = none := by
    apply find?_eq_none.2; intro k hk
    obtain ⟨a, ha, rfl⟩ := mem_map.1 hk
    simp [pair a ha]
  have fixed : ResultReader.fixed "result" (s.slots.map Slot.name) (.obj kvs) = .ok kvs := by
    simp only [ResultReader.fixed, ResultReader.object, keys, hn, ↓reduceIte, extra, miss, bind, Except.bind, pure, Except.pure]
  dsimp only
  unfold readResult resultJson unshownResult
  simp only [map_map]
  change (do
    let xs ← ResultReader.fixed "result" (s.slots.map Slot.name) (.obj kvs)
    s.slots.mapM fun (a : Slot) => do return (a.name, ← ResultReader.get "result" xs a.name (ResultReader.slot a))) = _
  rw [fixed]
  apply mapM_writes
  intro a ha
  simp [ResultReader.get, pair a ha, SlotValue.read_result _ _ _ (hv a ha)]; rfl

private theorem ids_written (p : String) (xs : List RecordId) (hs : xs.Pairwise (· < ·)) :
    TickReader.ids p (tickIds xs) = .ok xs := by
  have hd := hs.imp (fun {a b} h => Fin.ne_of_lt h)
  have readAll : ((xs.map (.str ∘ idText)).zipIdx.mapM (TickReader.idAt p)) = .ok xs := by
    simp only [zipIdx_map, List.mapM_map]
    rw [mapM_writes _ (fun x => x.1) (by intro x _; simp [Function.comp_def, TickReader.idAt, recordIdOf?_idText]; rfl)]
    simp
  simp only [TickReader.ids, tickIds, readAll, bind, Except.bind]
  simp [show xs.Nodup from hd, hs]; rfl

private theorem hash_written (p pre : String) (bs : List UInt8) (h : bs.length = 32) :
    TickReader.hashText p pre (.str (pre ++ base32 bs)) = .ok (pre ++ base32 bs) := by
  have good := (byteText_eq_true_iff 32 (base32 bs)).2 ⟨bs, h, rfl⟩
  have preOk := isPrefixOf_iff_prefix.2 (prefix_append pre.toList (base32 bs).toList)
  simp [TickReader.hashText, String.toList_append, ← String.length_toList, good, preOk]; rfl

theorem TickPayload.read_members (p : TickPayload) (H : Hash)
    (hs : (H (specPreimageOf p.rules p.spec.toJson)).length = 32)
    (hr : (H (resultPreimage p.result)).length = 32) (hc : p.covers.Pairwise (· < ·))
    (hh : p.withheld.Pairwise (· < ·)) (hi : ∀ i ∈ p.withheld, i ∈ p.covers) :
    readTickMembers H (p.toJson H) = .ok ⟨p.rules, p.spec.toJson, p.covers, p.withheld, resultJson p.result⟩ := by
  have sub : p.withheld.all p.covers.contains = true := by simpa using hi
  have a := hash_written "spec-hash" "spec:" (H (specPreimageOf p.rules p.spec.toJson)) hs
  have b := hash_written "result-hash" "res:" (H (resultPreimage p.result)) hr
  change TickReader.hashText "spec-hash" "spec:" (.str (specHashOf H p.rules p.spec.toJson)) = .ok (specHashOf H p.rules p.spec.toJson) at a
  change TickReader.hashText "result-hash" "res:" (.str (resultHashOf H (resultJson p.result))) = .ok (resultHashOf H (resultJson p.result)) at b
  have empty (e : p.withheld.isEmpty = true) : p.withheld = [] := List.isEmpty_iff.1 e
  cases he : p.withheld.isEmpty
  all_goals
    simp only [readTickMembers, TickPayload.toJson, TickMembers.toJson, he, Bool.false_eq_true, ↓reduceIte, List.cons_append,
      List.nil_append, List.map_cons, List.map_nil]
    simp +decide only [TickReader.requiredNames, TickReader.firstRepeated, TickReader.field, List.lookup, List.find?_cons,
      List.find?_nil, bind, Except.bind]
    simp [TickReader.values, TickReader.withheldIds, Rules.read_text, ids_written _ _ hc, ids_written _ _ hh, he, sub, a, b,
      bind, Except.bind, pure, Except.pure]
    simp [Spec.toJson]
    try exact empty he

private theorem readHeld_tickIds (xs : List RecordId) :
    readHeld (tickIds xs) = .many (xs.map (Val.str ∘ idText)) := by
  simp [tickIds, readHeld, filterMap_map, Function.comp_def, payloadMember?, payloadScalar?, Option.filter, Val.nameable]

/-- The full projection onto a held record. These are exactly the fields a
spec can see, including the difference between absent and present empty. -/
theorem TickPayload.record_fields (p : TickPayload) (H : Hash) (env : Env) (salt : List UInt8) (r : Record) :
    ((make env salt (p.toJson H)).readInto r).fields =
      [("rules", .one (.str p.rules.text)), ("spec", .other), ("spec-hash", .one (.str (p.spec.hash H p.rules))),
       ("covers", .many (p.covers.map (Val.str ∘ idText))), ("result", .other),
       ("result-hash", .one (.str (resultHash H p.result)))] ++
      (if p.withheld.isEmpty then [] else [("withheld", .many (p.withheld.map (Val.str ∘ idText)))]) := by
  cases he : p.withheld.isEmpty <;>
    simp [Stored.read_opening, TickPayload.toJson, TickMembers.toJson, he, payloadFields, readHeld_tickIds] <;> exact ⟨rfl,rfl,rfl,rfl,rfl⟩

/-- The writer's result is presented in spec slot order. Its only lossy
projection while reading the writer's JSON is the visibility on listed ids. -/
theorem TickPayload.read_written (H : Hash)
    (v : Rules) (s : Spec) (hf : s.Fits) (covers withheld : List RecordId)
    (hc : covers.Pairwise (· < ·)) (hh : withheld.Pairwise (· < ·))
    (hi : ∀ i ∈ withheld, i ∈ covers) (values : Slot → SlotValue)
    (hv : ∀ a ∈ s.slots, (values a).FitsSlot a)
    (hs : (H (specPreimageOf v s.toJson)).length = 32)
    (hr : (H (resultPreimage (s.slots.map fun a => (a.name, values a)))).length = 32) :
    let r := s.slots.map fun a => (a.name, values a)
    let p : TickPayload := ⟨v, s, covers, withheld, r⟩
    readTickPayload H v (p.toJson H) = .ok {p with result := unshownResult r} := by
  dsimp only
  let p : TickPayload := ⟨v, s, covers, withheld, s.slots.map fun a => (a.name, values a)⟩
  rw [readTickPayload, TickPayload.read_members p H hs hr hc hh hi]
  dsimp only [p]
  simp [Except.mapError, bind, Except.bind, TickMembers.readPayload, TickMembers.readSpec, TickMembers.readSlots, Spec.ofJson_toJson hf,
    readResult_written s values hf.2.2.2.1 hv, pure, Except.pure]

end Loops

import Laws.Tick

/-! A successful read re-encodes to the input's canonical text, including when
its objects arrive in a different order from the writer's. -/
namespace Loops
open List

private theorem bound {ε α β} {r : Except ε α} {f : α → Except ε β} {b}
    (h : (r >>= f) = .ok b) : ∃ a, r = .ok a ∧ f a = .ok b := by
  cases r with
  | error => contradiction
  | ok a => exact ⟨_, rfl, h⟩

private theorem mapped {ε α ε'} {r : Except ε α} {f : ε → ε'} {a}
    (h : r.mapError f = .ok a) : r = .ok a := by cases r <;> simp_all [Except.mapError]

private theorem mappedList {ε α β γ} {xs : List α} {ys : List β} {f : α → Except ε β}
    (h : xs.mapM f = .ok ys) (a : α → γ) (b : β → γ)
    (agree : ∀ x ∈ xs, ∀ y, f x = .ok y → a x = b y) : xs.map a = ys.map b := by
  induction xs generalizing ys with
  | nil => cases h; rfl
  | cons x xs ih =>
    rw [mapM_cons] at h
    obtain ⟨y, hy, h⟩ := bound h
    obtain ⟨tail, ht, h⟩ := bound h
    cases h
    simp only [map_cons, agree x mem_cons_self y hy, ih ht (fun z hz => agree z (mem_cons_of_mem _ hz))]

/-- Compare two objects by looking up each written member in the input.
Member values need only have the same canonical text. -/
private theorem objectText {xs ys : List (String × Json)}
    (hx : (xs.map Prod.fst).Nodup) (hy : (ys.map Prod.fst).Nodup)
    (keys : ∀ x ∈ xs, x.1 ∈ ys.map Prod.fst)
    (field : ∀ y ∈ ys, ∃ v, xs.lookup y.1 = some v ∧ v.canon = y.2.canon) :
    (Json.obj xs).canon = (Json.obj ys).canon := by
  let inj (ms : List (String × Json)) (hm : (ms.map Prod.fst).Nodup) :
      ∀ a ∈ Json.canonMembers ms, ∀ b ∈ Json.canonMembers ms, utf16 a.1 = utf16 b.1 → a = b := by
    intro a ha b hb he
    apply eq_of_mem_of_nodup_map (f := Prod.fst) _ a ha b hb (utf16_inj he)
    simpa [canonMembers_eq, map_map, Function.comp_def] using hm
  have eqs : sortBy (fun kv => utf16 kv.1) (Json.canonMembers xs) =
      sortBy (fun kv => utf16 kv.1) (Json.canonMembers ys) := by
    apply sortBy_ext
    intro kv
    rw [mem_sortBy (inj xs hx), mem_sortBy (inj ys hy), canonMembers_eq, canonMembers_eq]
    constructor
    · intro h
      obtain ⟨x, hxm, rfl⟩ := mem_map.1 h
      obtain ⟨y, hym, e⟩ := mem_map.1 (keys x hxm)
      obtain ⟨v, hv, eq⟩ := field y hym
      have he := eq_of_mem_of_nodup_map hx (y.1, v) (lookup_mem hv) x hxm e
      exact mem_map.2 ⟨y, hym, Prod.ext e (eq.symm.trans (congrArg (fun x : String × Json => x.2.canon) he))⟩
    · intro h
      obtain ⟨y, hym, rfl⟩ := mem_map.1 h
      obtain ⟨v, hv, eq⟩ := field y hym
      exact mem_map.2 ⟨_, lookup_mem hv, Prod.ext rfl eq⟩
  simp only [Json.canon, eqs]

private theorem fixedFacts {p names j xs} (h : ResultReader.fixed p names j = .ok xs) :
    j = .obj xs ∧ (xs.map Prod.fst).Nodup ∧ ∀ x ∈ xs, x.1 ∈ names := by
  obtain ⟨ms, hm, hf⟩ := bound h
  unfold ResultReader.object at hm
  split at hm <;> try contradiction
  split at hm <;> try contradiction
  cases hm
  cases he : ms.find? (fun kv => !names.contains kv.1) with
  | some kv => rw [he] at hf; contradiction
  | none =>
    rw [he] at hf
    split at hf <;> try contradiction
    cases hf
    exact ⟨rfl, by assumption, by simpa using find?_eq_none.1 he⟩

private theorem getFact {α} {p xs k f} {a : α} (h : ResultReader.get p xs k f = .ok a) :
    ∃ v, xs.lookup k = some v ∧ f (ResultReader.member p k) v = .ok a := by
  unfold ResultReader.get at h
  split at h <;> first | contradiction | exact ⟨_, by assumption, h⟩

private theorem recordId_repr {j id} (h : recordIdOf? j = some id) : j = .str (idText id) := by
  unfold recordIdOf? at h
  split at h <;> try contradiction
  rename_i s
  split at h <;> try contradiction
  rename_i cs he
  cases hb : unbase32 (String.ofList cs) with
  | none => simp [hb] at h
  | some bs =>
    by_cases hl : bs.length = 32 <;> simp [hb, hl] at h
    subst id
    apply congrArg Json.str
    apply String.toList_inj.1
    simp [idText, bytes_idDigest bs hl, (unbase32_eq_some_iff _ _).1 hb, String.toList_append, he]

private theorem nat_repr {j p n} (h : ResultReader.nat p j = .ok n) : j = intJson n := by
  unfold ResultReader.nat at h
  split at h <;> try contradiction
  rename_i s
  cases ht : natOf? s.toList <;> simp [ResultReader.required, ht] at h
  subst n
  exact congrArg Json.str (String.toList_inj.1 (by simpa [intJson, intText] using natDigits_natOf? ht))

private theorem int_repr {j p n} (h : ResultReader.integer p j = .ok n) : j = intJson n := by
  unfold ResultReader.integer at h
  split at h <;> try contradiction
  rename_i s
  cases ht : intOf? s <;> simp [ResultReader.required, ht] at h
  subst n
  exact congrArg Json.str (intOf?_eq_some_iff.1 ht)

private theorem opt_repr {j p n} (h : ResultReader.optInt p j = .ok n) :
    j = n.elim .null intJson := by
  unfold ResultReader.optInt at h
  split at h
  · cases h; rfl
  · cases hi : ResultReader.integer p j <;> simp [hi, Functor.map, Except.map] at h
    cases h; exact int_repr hi

private theorem ids_repr {j p ids} (h : ResultReader.ids p j = .ok ids) : j = idsJson ids := by
  unfold ResultReader.ids at h
  split at h <;> try contradiction
  rename_i xs
  apply congrArg Json.arr
  rw [← zipIdx_map_fst 0 xs]
  apply mappedList h
  intro x _ y ok
  obtain ⟨id, hi, hy⟩ := bound ok
  cases hy
  cases he : recordIdOf? x.1 <;> simp [ResultReader.required, he] at hi
  subst id
  exact recordId_repr he

private def AtMember (xs : List (String × Json)) (k : String) (j : Json) : Prop :=
  ∃ raw, xs.lookup k = some raw ∧ raw.canon = j.canon

private theorem getCanon {α} {p xs k f} {a : α} {j : Json} (h : ResultReader.get p xs k f = .ok a)
    (rep : ∀ raw, f (ResultReader.member p k) raw = .ok a → raw.canon = j.canon) : AtMember xs k j := by
  obtain ⟨v, hv, ok⟩ := getFact h
  exact ⟨_, hv, rep v ok⟩

private theorem fixedText {p names j xs} {ys : List (String × Json)} (h : ResultReader.fixed p names j = .ok xs)
    (hy : (ys.map Prod.fst).Nodup) (names' : ∀ k ∈ names, k ∈ ys.map Prod.fst)
    (fields : ∀ y ∈ ys, AtMember xs y.1 y.2) : j.canon = (Json.obj ys).canon := by
  obtain ⟨rfl, hx, keys⟩ := fixedFacts h
  exact objectText hx hy (fun x hx => names' _ (keys x hx)) fields

private theorem namedText {α} {reader : String → Json → Except String α} {writer : α → Json} {p j xs}
    (h : ResultReader.named reader p j = .ok xs)
    (rep : ∀ {q raw v}, reader q raw = .ok v → raw.canon = (writer v).canon) :
    j.canon = (Json.obj (xs.map fun x => (x.1, writer x.2))).canon := by
  obtain ⟨ms, ho, hm⟩ := bound h
  unfold ResultReader.object at ho
  split at ho <;> try contradiction
  split at ho <;> try contradiction
  cases ho
  have hs := mappedList hm (fun x => (x.1, x.2.canon)) (fun x => (x.1, (writer x.2).canon)) (by
    intro x _ y ok
    obtain ⟨v, hf, done⟩ := bound ok
    cases done
    exact Prod.ext rfl (rep hf))
  simp only [Json.canon, canonMembers_eq, map_map, Function.comp_def, hs]

private theorem groupText {ty p j g} (h : ResultReader.group ty p j = .ok g) : j.canon = g.result.canon := by
  cases ty <;> obtain ⟨xs, hf, h⟩ := bound h <;> obtain ⟨u, hu, h⟩ := bound h
  case latestBy => contradiction
  all_goals
    obtain ⟨v, hv, h⟩ := bound h
    cases h
    apply fixedText hf (by simp only [map_cons, map_nil]; decide) (by simp only [map_cons, map_nil, mem_cons, not_mem_nil, or_false]; grind)
    intro y hy
    simp only [mem_cons, not_mem_nil, or_false] at hy
    rcases hy with rfl | rfl
    all_goals first
      | exact getCanon hu (fun _ h => congrArg Json.canon (nat_repr h))
      | exact getCanon hv (fun _ h => congrArg Json.canon (nat_repr h))
      | exact getCanon hv (fun _ h => congrArg Json.canon (int_repr h))
      | exact getCanon hv (fun _ h => congrArg Json.canon (ids_repr h))
      | cases v <;> exact getCanon hv (fun _ h => congrArg Json.canon (opt_repr h))

private theorem keyedText {p j v} (h : ResultReader.keyed p j = .ok v) : j.canon = v.result.canon := by
  obtain ⟨xs, hf, h⟩ := bound h
  obtain ⟨hs, hh, h⟩ := bound h
  obtain ⟨ks, hk, h⟩ := bound h
  obtain ⟨ws, hw, h⟩ := bound h
  cases h
  apply fixedText hf (by simp only [map_cons, map_nil]; decide) (by simp only [map_cons, map_nil, mem_cons, not_mem_nil, or_false]; grind)
  intro y hy
  simp only [mem_cons, not_mem_nil, or_false] at hy
  rcases hy with rfl | rfl | rfl
  · exact getCanon hh (fun _ h => congrArg Json.canon (ids_repr h))
  · exact getCanon hk (fun _ h => namedText h (fun h => congrArg Json.canon (ids_repr h)))
  · exact getCanon hw (fun _ h => congrArg Json.canon (ids_repr h))

private theorem slotText {s p j v} (h : ResultReader.slot s p j = .ok v) : j.canon = v.result.canon := by
  unfold ResultReader.slot at h
  split at h
  · obtain ⟨xs, hf, h⟩ := bound h
    obtain ⟨gs, hg, h⟩ := bound h
    obtain ⟨m, hm, h⟩ := bound h
    obtain ⟨u, hu, h⟩ := bound h
    cases h
    apply fixedText hf (by simp only [map_cons, map_nil]; decide) (by simp)
    intro y hy
    simp only [mem_cons, not_mem_nil, or_false] at hy
    rcases hy with rfl | rfl | rfl
    · exact getCanon hg (fun _ h => namedText h groupText)
    · exact getCanon hm (fun _ h => congrArg Json.canon (nat_repr h))
    · exact getCanon hu (fun _ h => congrArg Json.canon (nat_repr h))
  · split at h
    · obtain ⟨xs, hf, h⟩ := bound h
      obtain ⟨u, hu, h⟩ := bound h
      split at h
      all_goals
        obtain ⟨things, ht, h⟩ := bound h
        cases h
        apply fixedText hf (by simp only [map_cons, map_nil]; decide) (by simp)
        intro y hy
        simp only [mem_cons, not_mem_nil, or_false] at hy
        rcases hy with rfl | rfl
        · first
          | exact getCanon ht (fun _ h => namedText h (fun ok => congrArg Json.canon (ids_repr ok)))
          | exact getCanon ht (fun _ h => namedText h keyedText)
        · exact getCanon hu (fun _ h => congrArg Json.canon (nat_repr h))
    · obtain ⟨g, hg, h⟩ := bound h
      cases g <;> cases h <;> exact groupText hg

private theorem eachMapped {ε α β} {xs : List α} {ys : List β} {f : α → Except ε β}
    (h : xs.mapM f = .ok ys) : ∀ y ∈ ys, ∃ x ∈ xs, f x = .ok y := by
  induction xs generalizing ys with
  | nil => cases h; simp
  | cons x xs ih =>
    rw [mapM_cons] at h
    obtain ⟨b, hb, h⟩ := bound h
    obtain ⟨bs, hbs, h⟩ := bound h
    cases h
    intro y hy
    rcases mem_cons.1 hy with rfl | hy
    · exact ⟨_, mem_cons_self, hb⟩
    · obtain ⟨a, ha, hf⟩ := ih hbs y hy
      exact ⟨_, mem_cons_of_mem _ ha, hf⟩

/-- Successful shape decoding preserves the whole JSON object's canonical text. -/
theorem readResult_text {s j r} (hn : (s.slots.map Slot.name).Nodup) (h : readResult s j = .ok r) :
    j.canon = resultText r := by
  obtain ⟨xs, hf, hr⟩ := bound h
  have names := mappedList hr Slot.name Prod.fst (by
    intro a _ b hb
    obtain ⟨val, _, done⟩ := bound hb
    cases done; rfl)
  have keys : ((resultMembers r).map Prod.fst) = s.slots.map Slot.name := by
    simpa [resultMembers, map_map, Function.comp_def] using names.symm
  apply fixedText (ys := resultMembers r) hf (keys ▸ hn) (fun _ hk => keys ▸ hk)
  intro y hy
  obtain ⟨b, hb, rfl⟩ := mem_map.1 hy
  obtain ⟨a, _, ok⟩ := eachMapped hr _ hb
  obtain ⟨v, hv, done⟩ := bound ok
  cases done
  exact getCanon hv (fun _ h => slotText h)

private theorem tickIds_repr {p j xs} (h : TickReader.ids p j = .ok xs) : j = tickIds xs := by
  unfold TickReader.ids at h
  split at h <;> try contradiction
  rename_i js
  obtain ⟨ids, hm, h⟩ := bound h
  by_cases hd : ids.Nodup <;> by_cases hs : ids.Pairwise (· < ·) <;>
    simp [hd, hs, refuse, bind, Except.bind, pure, Except.pure] at h
  subst xs
  apply congrArg Json.arr
  rw [← zipIdx_map_fst 0 js]
  apply mappedList hm
  intro x _ y ok
  unfold TickReader.idAt at ok
  split at ok <;> try contradiction
  cases ok
  exact recordId_repr (by assumption)

private theorem hash_repr {p pre j s} (h : TickReader.hashText p pre j = .ok s) : j = .str s := by
  unfold TickReader.hashText at h
  split at h <;> try contradiction
  split at h <;> try contradiction
  cases h; rfl

private def optWithheld (xs : List RecordId) : Option Json :=
  if xs.isEmpty then none else some (tickIds xs)

private theorem withheld_repr {covers : List RecordId} {raw xs}
    (h : TickReader.withheldIds covers raw = .ok xs) : raw = optWithheld xs := by
  cases raw with
  | none => cases h; rfl
  | some j =>
    obtain ⟨ids, hi, h⟩ := bound h
    cases he : ids.isEmpty <;> cases hs : ids.all covers.contains <;>
      simp [he, hs, refuse, bind, Except.bind, pure, Except.pure] at h
    subst xs
    simp [optWithheld, he, tickIds_repr hi]

private theorem values_repr {H rv sp sh cv r rh w m}
    (h : TickReader.values H rv sp sh cv r rh w = .ok m) :
    rv = .str m.rules.text ∧ sp = m.spec ∧ sh = .str (specHashOf H m.rules sp) ∧
    cv = tickIds m.covers ∧ r = m.result ∧ rh = .str (resultHashOf H r) ∧ w = optWithheld m.withheld := by
  unfold TickReader.values at h
  cases ver : Rules.ofJson? rv with
  | none => rw [ver] at h; contradiction
  | some v =>
    rw [ver] at h
    cases sp <;> try contradiction
    rename_i fields
    obtain ⟨cs, hc, h⟩ := bound h
    obtain ⟨ws, hw, h⟩ := bound h
    obtain ⟨sHash, hs, h⟩ := bound h
    obtain ⟨rHash, hr, h⟩ := bound h
    by_cases es : sHash = specHashOf H v (.obj fields) <;> by_cases er : rHash = resultHashOf H r <;>
      simp [es, er, refuse, bind, Except.bind, pure, Except.pure] at h
    subst m
    exact ⟨Rules.ofJson?_eq_some_iff.1 ver, rfl, es ▸ hash_repr hs, tickIds_repr hc, rfl,
      er ▸ hash_repr hr, withheld_repr hw⟩

private theorem noRepeats {ks seen : List String} (h : TickReader.firstRepeated seen ks = none) :
    ks.Nodup ∧ ∀ k ∈ ks, k ∉ seen := by
  induction ks generalizing seen with
  | nil => simp
  | cons k ks ih =>
    unfold TickReader.firstRepeated at h
    split at h <;> try contradiction
    rename_i hn
    obtain ⟨hd, fresh⟩ := ih h
    simp at hn
    exact ⟨nodup_cons.2 ⟨fun hk => fresh k hk mem_cons_self, hd⟩, fun a ha => by
      rcases mem_cons.1 ha with rfl | h
      · exact hn
      · exact fun hm => fresh a h (mem_cons_of_mem _ hm)⟩

private theorem atField {xs k v} (h : TickReader.field xs k = .ok v) : AtMember xs k v := by
  unfold TickReader.field at h
  split at h <;> try contradiction
  cases h; exact ⟨_, by assumption, rfl⟩

/-- Re-encoding needs no knowledge of the version's spec or result language. -/
theorem readTickMembers_text {H j m} (h : readTickMembers H j = .ok m) :
    j.canon = (m.toJson H).canon := by
  cases j <;> try contradiction
  rename_i xs
  unfold readTickMembers at h
  cases dup : TickReader.firstRepeated [] (xs.map Prod.fst) with
  | some k => simp only [dup] at h; contradiction
  | none =>
    simp only [dup] at h
    cases extra : xs.find? (fun kv => !(TickReader.requiredNames ++ ["withheld"]).contains kv.1) with
    | some e => simp only [extra] at h; contradiction
    | none =>
      simp only [extra] at h
      obtain ⟨rv, hv, h⟩ := bound h
      obtain ⟨sp, hs, h⟩ := bound h
      obtain ⟨sh, hsh, h⟩ := bound h
      obtain ⟨cv, hc, h⟩ := bound h
      obtain ⟨r, hr, h⟩ := bound h
      obtain ⟨rh, hrh, h⟩ := bound h
      obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, hw⟩ := values_repr h
      have keys : ∀ x ∈ xs, x.1 ∈ TickReader.requiredNames ++ ["withheld"] := by
        intro x hx
        have he := find?_eq_none.1 extra x hx
        simpa only [Bool.not_eq_true', Bool.not_eq_false, contains_iff_mem] using he
      cases empty : m.withheld.isEmpty
      all_goals
        simp only [optWithheld, empty, Bool.false_eq_true, ↓reduceIte] at hw
        simp only [TickMembers.toJson, empty, Bool.false_eq_true, ↓reduceIte, cons_append, nil_append]
        apply objectText (noRepeats dup).1 (by simp only [map_cons, map_nil]; decide)
        · intro x hx
          have key := keys x hx
          simp only [TickReader.requiredNames, cons_append, nil_append, mem_cons, not_mem_nil, or_false] at key
          simp only [map_cons, map_nil, mem_cons, not_mem_nil, or_false]
          first
          | exact key
          | have ne : x.1 ≠ "withheld" := by simpa [ne_comm] using lookup_eq_none_iff.1 hw x hx
            grind
        · intro y hy
          simp only [mem_cons, not_mem_nil, or_false] at hy
          rcases hy with rfl | rfl | rfl | rfl | rfl | tail
          case' inr.inr.inr.inr.inr =>
            first | subst y | rcases tail with rfl | rfl
          all_goals first
            | exact atField hv | exact atField hs | exact atField hsh | exact atField hc
            | exact atField hr | exact atField hrh | exact ⟨_, hw, rfl⟩

/-- Every accepted payload has the JCS text of its typed writer. No assumption
on the digest, nor any consistency between lists and counts, is needed. -/
theorem readTickPayload_text {H current j p} (h : readTickPayload H current j = .ok p) :
    j.canon = (p.toJson H).canon := by
  obtain ⟨m, hm, hp⟩ := bound h
  have raw := readTickMembers_text (mapped hm)
  unfold TickMembers.readPayload at hp
  by_cases same : m.rules = current
  case neg => simp [same, bind, Except.bind, throw, throwThe, MonadExceptOf.throw] at hp
  simp only [show (m.rules != current) = false by simp [same], Bool.false_eq_true, ↓reduceIte] at hp
  obtain ⟨s, hs, hp⟩ := bound hp
  have parsed := Spec.ofJson_fits (mapped hs)
  by_cases sc : m.spec.canon = s.toJson.canon
  · simp only [show (m.spec.canon != s.toJson.canon) = false by simp [sc], Bool.false_eq_true, ↓reduceIte] at hp
    obtain ⟨r, hr, done⟩ := bound hp
    cases done
    have rc := readResult_text parsed.2.2.2.1 (mapped hr)
    rw [raw]
    change m.result.canon = (resultJson r).canon at rc
    simp only [TickPayload.toJson, TickMembers.toJson, Json.canon, canonMembers_eq, map_append, map_cons, map_nil,
      specHashOf, specPreimageOf, resultHashOf, sc, rc]
  · simp [sc, bind, Except.bind, throw, throwThe, MonadExceptOf.throw] at hp

end Loops

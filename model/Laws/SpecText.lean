import Laws.Result

/-! JCS is injective on the strings, integers, booleans, arrays and objects
a parsed spec can write, when every object uses a fixed strict member order. That order need not be JCS's order. A delimiter after a
number matters: the text of 1 is a proper prefix of the text of 10. -/
namespace Loops.SpecText
open List

abbrev Chars := List Char

def delimiter (c : Char) : Bool := c == ',' || c == ']' || c == '}'
def boundary (xs : Chars) : Prop := xs.head?.all delimiter = true

/-- One representation per member order; the scalar grammar used by parsed specs. -/
inductive Ordered (key : String → List Nat) : Json → Prop
  | str (s : String) : Ordered key (.str s)
  | int (n : Int) : Ordered key (.int n)
  | bool (b : Bool) : Ordered key (.bool b)
  | arr {xs} : (∀ x ∈ xs, Ordered key x) → Ordered key (.arr xs)
  | obj {ms} : ms.Pairwise (fun a b => key a.1 < key b.1) →
      (∀ m ∈ ms, Ordered key m.2) → Ordered key (.obj ms)

def separated (f : α → Chars) (close : Char) (xs : List α) (rest : Chars) :=
  [','].intercalate (xs.map f) ++ close :: rest

theorem splitElementsMap (g : α → β) {as bs : List α} {close : Char} (hc : delimiter close) (hnc : close ≠ ',')
    {f : α → Chars} (starts : ∀ a, ∃ c cs, f a = c :: cs ∧ c ≠ close)
    (read : ∀ a ∈ as, ∀ b ∈ bs, ∀ {X Y : Chars}, boundary X → boundary Y →
      f a ++ X = f b ++ Y → g a = g b ∧ X = Y)
    {X Y : Chars} (eqn : separated f close as X = separated f close bs Y) :
    as.map g = bs.map g ∧ X = Y := by
  induction as generalizing bs with
  | nil =>
    cases bs with
    | nil => exact ⟨rfl, (List.cons.inj eqn).2⟩
    | cons b bs =>
      obtain ⟨c, cs, he, hn⟩ := starts b
      simp [separated, intercalate_comma, he] at eqn
      exact False.elim (hn eqn.1.symm)
  | cons a as ih =>
    cases bs with
    | nil =>
      obtain ⟨c, cs, he, hn⟩ := starts a
      simp [separated, intercalate_comma, he] at eqn
      exact False.elim (hn eqn.1)
    | cons b bs =>
      let tail (xs : List α) (rest : Chars) :=
        (xs.map f).flatMap (fun s => ',' :: s) ++ close :: rest
      have bound (xs : List α) (rest : Chars) : boundary (tail xs rest) := by
        cases xs with
        | nil => exact hc
        | cons x xs => rfl
      simp only [separated, map_cons, intercalate_comma, append_assoc] at eqn
      obtain ⟨eq, ht⟩ := read a mem_cons_self b mem_cons_self (bound as X) (bound bs Y) eqn
      have hrest : separated f close as X = separated f close bs Y := by
        cases as <;> cases bs <;> simp [tail, separated, intercalate_comma, hnc, Ne.symm hnc] at ht ⊢ <;> exact ht
      obtain ⟨et, rfl⟩ := ih (fun a ha b hb => read a (mem_cons_of_mem _ ha) b (mem_cons_of_mem _ hb)) hrest
      exact ⟨by simp only [map_cons, eq, et], rfl⟩

private theorem splitElements {as bs : List α} {close : Char} (hc : delimiter close) (hnc : close ≠ ',')
    {f : α → Chars} (starts : ∀ a, ∃ c cs, f a = c :: cs ∧ c ≠ close)
    (read : ∀ a ∈ as, ∀ b ∈ bs, ∀ {X Y : Chars}, boundary X → boundary Y →
      f a ++ X = f b ++ Y → a = b ∧ X = Y)
    {X Y : Chars} (eqn : separated f close as X = separated f close bs Y) :
    as = bs ∧ X = Y := by
  simpa using splitElementsMap id hc hnc starts read eqn

theorem primitive_boundary {a b X Y : Chars}
    (ha : ∀ c ∈ a, !delimiter c) (hb : ∀ c ∈ b, !delimiter c)
    (hx : boundary X) (hy : boundary Y) (h : a ++ X = b ++ Y) : a = b ∧ X = Y := by
  have stop {xs : Chars} (hb : boundary xs) : xs.takeWhile (fun c => !delimiter c) = [] := by
    cases xs <;> simp_all [boundary]
  have heads := congrArg (takeWhile (fun c => !delimiter c)) h
  rw [takeWhile_append_of_pos ha, takeWhile_append_of_pos hb, stop hx, stop hy, append_nil, append_nil] at heads
  subst b
  exact ⟨rfl, List.append_cancel_left h⟩

private theorem digit_safe {c : Char} (hc : c.isDigit) : (!delimiter c) = true := by
  simp [delimiter]
  exact ⟨⟨fun e => by subst c; contradiction, fun e => by subst c; contradiction⟩, fun e => by subst c; contradiction⟩

private theorem canon_pos (n : Nat) : (Json.int (.ofNat n)).canon.toList = Nat.toDigits 10 n := by
  change n.repr.toList = _
  rw [Nat.toList_repr]

private theorem canon_neg (n : Nat) : (Json.int (.negSucc n)).canon.toList = '-' :: Nat.toDigits 10 (n+1) := by
  change ("-" ++ (n+1).repr).toList = _
  simp [String.toList_append]

private theorem digit_ten {c : Char} {n : Nat} (h : c ∈ Nat.toDigits 10 n) : c.isDigit :=
  Nat.isDigit_of_mem_toDigits (by decide : 0 < 10) (by decide) h

theorem int_chars (i : Int) : ∀ c ∈ (Json.int i).canon.toList, !delimiter c := by
  cases i with
  | ofNat n => rw [canon_pos]; exact fun _ h => digit_safe (digit_ten h)
  | negSucc n =>
    rw [canon_neg]
    simp only [mem_cons, forall_eq_or_imp]
    exact ⟨by decide, fun _ h => digit_safe (digit_ten h)⟩

private theorem int_inj {i j : Int} (h : (Json.int i).canon.toList = (Json.int j).canon.toList) : i = j := by
  have inj {m n : Nat} (e : Nat.toDigits 10 m = Nat.toDigits 10 n) : m = n := by
    simpa using congrArg (fun ds => Nat.ofDigitChars 10 ds 0) e
  have minus {n : Nat} (h : '-' ∈ Nat.toDigits 10 n) : False := by have := digit_ten h; contradiction
  cases i <;> cases j
  · exact congrArg Int.ofNat (inj (by simpa only [canon_pos] using h))
  · rw [canon_pos, canon_neg] at h
    exact False.elim (minus (h ▸ mem_cons_self))
  · rw [canon_neg, canon_pos] at h
    exact False.elim (minus (h ▸ mem_cons_self))
  · rw [canon_neg, canon_neg] at h
    exact congrArg Int.negSucc (Nat.succ.inj (inj (List.cons.inj h).2))

private def headKind (s : Chars) : Nat :=
  match s.head? with
  | some '"' => 1 | some '[' => 2 | some '{' => 3 | some 't' | some 'f' => 4 | _ => 0

private def kind : Json → Nat
  | .str _ => 1 | .arr _ => 2 | .obj _ => 3 | .bool _ => 4 | _ => 0

private theorem headKind_int (i : Int) (xs : Chars) : headKind ((Json.int i).canon.toList ++ xs) = 0 := by
  cases i with
  | negSucc n => rw [canon_neg]; rfl
  | ofNat n =>
    obtain ⟨c, cs, he, hd⟩ := natText_head n
    change headKind ((toString n).toList ++ xs) = _
    rw [he]
    simp only [headKind, cons_append, head?_cons]
    split <;> simp_all

private theorem headKind_ordered {key j} (h : Ordered key j) (xs : Chars) :
    headKind (j.canon.toList ++ xs) = kind j := by
  cases h with
  | bool b => cases b <;> rfl
  | int i => exact headKind_int i xs
  | str s => rw [canon_str]; rfl
  | arr ys => rw [canon_arr]; rfl
  | obj _ _ => simp only [Json.canon, String.toList_append]; rfl

theorem value_start (v : Json) (close : Char) (hc : delimiter close) :
    ∃ c cs, v.canon.toList = c :: cs ∧ c ≠ close := by
  obtain ⟨c, cs, he, h⟩ := Json.canon_head v
  refine ⟨c, cs, he, ?_⟩
  rintro rfl
  simp only [delimiter, Bool.or_eq_true, beq_iff_eq] at hc
  rcases hc with (rfl | rfl) | rfl <;> simp [jsonHead] at h

def sortedMembers (ms : List (String × Json)) := sortBy (fun kv : String × Json => utf16 kv.1) ms

private theorem sortedMembers_perm {key : String → List Nat} {ms : List (String × Json)}
    (h : ms.Pairwise (fun a b => key a.1 < key b.1)) : sortedMembers ms ~ ms := by
  have hd : (ms.map Prod.fst).Nodup := pairwise_map.2 (h.imp fun {a b} lt eq => by rw [eq] at lt; exact (List.lt_irrefl _ lt))
  exact sortBy_perm (fun a ha b hb eq => eq_of_mem_of_nodup_map hd a ha b hb (utf16_inj eq))
    (h.imp (fun {a b} lt eq => by subst b; exact List.lt_irrefl _ lt))

private theorem object_text {key : String → List Nat} {ms : List (String × Json)}
    (h : ms.Pairwise (fun a b => key a.1 < key b.1)) (X : Chars) :
    (Json.obj ms).canon.toList ++ X = '{' :: separated memberText '}' (sortedMembers ms) X := by
  have hp := sortedMembers_perm h
  have hn : (ms.map Prod.fst).Nodup := pairwise_map.2 (h.imp fun {a b} lt eq => by rw [eq] at lt; exact (List.lt_irrefl _ lt))
  rw [canon_member_order hp.symm hn, canon_obj (show Sorted (fun kv => utf16 kv.1) (sortedMembers ms) from sortBy_sorted _ _)]
  simp [separated]

theorem quoted_prefix {s t : String} {X Y : Chars}
    (h : (Json.str s).canon.toList ++ X = (Json.str t).canon.toList ++ Y) : s = t ∧ X = Y := by
  have e := Json.canon_prefix _ _ (show (Json.str s).Plain from trivial)
    (show (Json.str t).Plain from trivial) h
  exact ⟨Json.str.inj e.1, e.2⟩

private theorem ordered_prefix {key a} (ha : Ordered key a) {b} (hb : Ordered key b) {X Y : Chars}
    (hx : boundary X) (hy : boundary Y) (h : a.canon.toList ++ X = b.canon.toList ++ Y) :
    a = b ∧ X = Y := by
  induction ha generalizing b X Y with
  | str s =>
    have k := (headKind_ordered (.str (key := key) s) X).symm.trans ((congrArg headKind h).trans (headKind_ordered hb Y))
    cases hb <;> try (simp [kind] at k; done)
    obtain ⟨rfl, rfl⟩ := quoted_prefix h
    exact ⟨rfl, rfl⟩
  | int i =>
    have k := (headKind_int i X).symm.trans ((congrArg headKind h).trans (headKind_ordered hb Y))
    cases hb <;> try (simp [kind] at k; done)
    obtain ⟨hv, rfl⟩ := primitive_boundary (int_chars _) (int_chars _) hx hy h
    exact ⟨congrArg Json.int (int_inj hv), rfl⟩
  | bool v =>
    have k := (headKind_ordered (.bool (key := key) v) X).symm.trans ((congrArg headKind h).trans (headKind_ordered hb Y))
    cases hb <;> try (simp [kind] at k; done)
    rename_i w
    cases v <;> cases w <;> simp_all [Json.canon]
  | arr ha ih =>
    have k := (headKind_ordered (.arr ha) X).symm.trans ((congrArg headKind h).trans (headKind_ordered hb Y))
    cases hb <;> try (simp [kind] at k; done)
    rename_i bs hbs
    simp only [canon_arr, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨rfl, rfl⟩ := splitElements (close := ']') (by decide) (by decide) (fun j => value_start j _ (by decide))
      (fun a ha b hb => ih a ha (hbs b hb)) h
    exact ⟨rfl, rfl⟩
  | @obj ms hs ha ih =>
    have k := (headKind_ordered (.obj hs ha) X).symm.trans ((congrArg headKind h).trans (headKind_ordered hb Y))
    cases hb <;> try (simp [kind] at k; done)
    rename_i ns ht hbs
    rw [object_text hs, object_text ht] at h
    have hp := sortedMembers_perm hs
    have hq := sortedMembers_perm ht
    obtain ⟨e, rfl⟩ := splitElements (f := memberText) (close := '}') (by decide) (by decide)
      (fun kv => ⟨_, _, (memberText_head kv).choose_spec, by decide⟩) (by
        intro a ham b hbm U V hu hv eqn
        rcases a with ⟨a, v⟩
        rcases b with ⟨b, w⟩
        simp only [memberText, cons_append, append_assoc, cons.injEq, true_and] at eqn
        obtain ⟨he, rest⟩ := escapes_prefix _ _ eqn
        obtain ⟨rfl, hrest⟩ := ih (a, v) (hp.mem_iff.1 ham) (hbs _ (hq.mem_iff.1 hbm)) hu hv (List.cons.inj rest).2
        exact ⟨Prod.ext (String.toList_inj.1 he) rfl, hrest⟩) (List.cons.inj h).2
    have perm := hp.symm.trans (e ▸ hq)
    exact ⟨congrArg Json.obj (perm.eq_of_pairwise (fun a b _ _ hab hba => False.elim (List.lt_asymm hab hba)) hs ht), rfl⟩

theorem Ordered.canon_inj {key a b} (ha : Ordered key a) (hb : Ordered key b) (h : a.canon = b.canon) :
    a = b := (ordered_prefix ha hb (X := []) (Y := []) rfl rfl (by rw [h])).1

end Loops.SpecText

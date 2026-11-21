import Loops.Result
import Laws.Record
import Laws.IdText
import Laws.Parse

/-!
# Laws of the canonical result (D7)

* **Integers are exact** (`result.exact`). The codec writes an integer of any
  size as its decimal digits, and reads back exactly the texts it writes:
  `intOf? s = some i` holds exactly when `s = intText i`. No integer is a
  binary64 number on the way, so none is rounded or capped.
* **`shown` is not in the bytes** (`result.shown`). Setting aside what is
  shown of every listed record never changes the canonical text, so two
  results that differ only there encode alike.
* **Nothing else is lost** (`result.canonical`). Two runs of one spec give
  the same canonical text, the same bytes and the same preimage of the hash
  exactly when their results agree with `shown` set aside. The only premise
  is that the spec names each slot once, which every spec the parser accepts
  does. Under it, JCS's sorting of the slots loses nothing; the maps are by
  thing, group key or payload key, which every slot already lists in UTF-16
  order, and the fixed members are written in JCS order.

* **A result hash is no commitment** (`result.no-commit`). The preimage is
  never a 16-byte salt and then a JSON value's canonical bytes, which is what a
  record's commitment hashes: the label is 21 bytes, its 17th byte is `h`, and
  no canonical JSON text starts with `h`.

The text is injective on the JSON the codec writes (`Json.canon_injective`):
`null`, text, arrays, and objects whose members are in UTF-16 order, each named
once. No other JSON is claimed here.

These hold for any hash `H`; the vectors use SHA-256. The hash itself is a
parameter: what these laws give is that the preimage determines the result,
not that `H` has no collisions.
-/

namespace Loops

open List

/-! ## Integers as text -/

theorem digitChar_toNat {d : Nat} (h : d < 10) : (digitChar d).toNat = 48 + d :=
  toNat_ofNat_valid (by omega)

theorem digitVal?_digitChar {d : Nat} (h : d < 10) : digitVal? (digitChar d) = some d := by
  unfold digitVal?
  rw [digitChar_toNat h]
  split
  · congr 1; omega
  · exfalso; omega

theorem digitVal?_some {c : Char} {d : Nat} (h : digitVal? c = some d) : d < 10 ∧ digitChar d = c := by
  unfold digitVal? at h
  split at h
  · rename_i hc
    cases h
    refine ⟨by omega, Char.toNat_inj.1 ?_⟩
    rw [digitChar_toNat (by omega)]; omega
  · cases h

theorem digitChar_zero : digitChar 0 = '0' := by decide

theorem digitChar_ne_zero {d : Nat} (h : d < 10) (hd : d ≠ 0) : digitChar d ≠ '0' := by
  intro e
  have := congrArg Char.toNat e
  rw [digitChar_toNat h] at this
  simp at this; omega

theorem revValue_cons (c : Char) (cs : List Char) :
    revValue (c :: cs) = (digitVal? c).bind fun d => (revValue cs).bind fun n => some (10 * n + d) := rfl

theorem revValue_cons_eq {c : Char} {cs : List Char} {n : Nat} (h : revValue (c :: cs) = some n) :
    ∃ d m, digitVal? c = some d ∧ revValue cs = some m ∧ n = 10 * m + d := by
  rw [revValue_cons] at h
  cases hd : digitVal? c with
  | none => simp [hd] at h
  | some d =>
    cases hm : revValue cs with
    | none => simp [hd, hm] at h
    | some m => simp only [hd, hm, Option.bind_some, Option.some.injEq] at h; exact ⟨d, m, rfl, rfl, h.symm⟩

theorem revDigits_small {n : Nat} (h : n < 10) : revDigits n = [digitChar n] := by
  rw [revDigits]; simp [h]

theorem revDigits_large {n : Nat} (h : ¬ n < 10) : revDigits n = digitChar (n % 10) :: revDigits (n / 10) := by
  rw [revDigits]; simp [h]

theorem revDigits_zero : revDigits 0 = ['0'] := by
  rw [revDigits_small (by omega), digitChar_zero]

theorem revValue_revDigits (n : Nat) : revValue (revDigits n) = some n := by
  by_cases h : n < 10
  · rw [revDigits_small h, revValue_cons, digitVal?_digitChar h]
    simp [revValue]
  · rw [revDigits_large h, revValue_cons, digitVal?_digitChar (Nat.mod_lt n (by omega)),
      revValue_revDigits (n / 10)]
    simp only [Option.bind_some, Option.some.injEq]
    omega
termination_by n
decreasing_by omega

/-- Digits least significant first whose most significant digit is not zero
are the only spelling of their value. -/
theorem revDigits_revValue : ∀ {cs : List Char} {n : Nat}, revValue cs = some n →
    cs.getLast? ≠ some '0' → cs ≠ [] → revDigits n = cs
  | [], _, _, _, h => absurd rfl h
  | [c], n, hv, hl, _ => by
    obtain ⟨d, m, hd, hm, rfl⟩ := revValue_cons_eq hv
    simp only [revValue, Option.some.injEq] at hm
    subst hm
    obtain ⟨hd10, hdc⟩ := digitVal?_some hd
    rw [Nat.mul_zero, Nat.zero_add, revDigits_small hd10, hdc]
  | c :: c' :: rest, n, hv, hl, _ => by
    obtain ⟨d, m, hd, hm, rfl⟩ := revValue_cons_eq hv
    obtain ⟨hd10, hdc⟩ := digitVal?_some hd
    have hl' : (c' :: rest).getLast? ≠ some '0' := by
      rwa [List.getLast?_cons_cons] at hl
    have ih := revDigits_revValue hm hl' (by simp)
    have hm0 : m ≠ 0 := by
      rintro rfl
      rw [revDigits_zero] at ih
      rw [← ih] at hl'
      simp at hl'
    rw [revDigits_large (by omega)]
    have e1 : (10 * m + d) % 10 = d := by omega
    have e2 : (10 * m + d) / 10 = m := by omega
    rw [e1, e2, ih, hdc]

theorem revDigits_ne_nil (n : Nat) : revDigits n ≠ [] := by
  by_cases h : n < 10
  · rw [revDigits_small h]; simp
  · rw [revDigits_large h]; simp

theorem revDigits_last {n : Nat} (h : n ≠ 0) : (revDigits n).getLast? ≠ some '0' := by
  by_cases h10 : n < 10
  · rw [revDigits_small h10]
    simp only [List.getLast?_singleton, ne_eq, Option.some.injEq]
    exact digitChar_ne_zero h10 h
  · rw [revDigits_large h10]
    obtain ⟨x, xs, e⟩ := List.exists_cons_of_ne_nil (revDigits_ne_nil (n / 10))
    have ih := revDigits_last (n := n / 10) (by omega)
    rw [e] at ih ⊢
    rwa [List.getLast?_cons_cons]
termination_by n
decreasing_by omega

theorem natDigits_zero : natDigits 0 = ['0'] := by simp [natDigits, revDigits_zero]

theorem natOf?_cons {c : Char} {cs : List Char} (hc : c ≠ '0') : natOf? (c :: cs) = revValue (c :: cs).reverse := by
  cases cs with
  | nil =>
    unfold natOf?
    split
    · rename_i h; simp at h
    · rename_i h; simp at h; exact absurd h hc
    · rename_i c' cs' _ h; simp at h; obtain ⟨rfl, rfl⟩ := h; simp [hc]
  | cons d ds => simp [natOf?, hc]

theorem mem_revDigits {n : Nat} {c : Char} (h : c ∈ revDigits n) : ∃ d < 10, c = digitChar d := by
  by_cases h10 : n < 10
  · rw [revDigits_small h10] at h
    simp at h; exact ⟨n, h10, h⟩
  · rw [revDigits_large h10] at h
    rcases List.mem_cons.1 h with h | h
    · exact ⟨n % 10, Nat.mod_lt n (by omega), h⟩
    · exact mem_revDigits h
termination_by n
decreasing_by omega

theorem minus_not_mem_natDigits (n : Nat) : '-' ∉ natDigits n := by
  intro h
  obtain ⟨d, hd, e⟩ := mem_revDigits (List.mem_reverse.1 h)
  have := congrArg Char.toNat e
  rw [digitChar_toNat hd] at this
  simp at this; omega

theorem natOf?_natDigits (n : Nat) : natOf? (natDigits n) = some n := by
  by_cases h0 : n = 0
  · subst h0; rw [natDigits_zero]; rfl
  · have hl := revDigits_last h0
    unfold natDigits
    obtain ⟨c, cs, e⟩ : ∃ c cs, (revDigits n).reverse = c :: cs :=
      List.exists_cons_of_ne_nil (by simpa using revDigits_ne_nil n)
    have hc : c ≠ '0' := by
      intro hc; apply hl
      rw [← List.reverse_reverse (revDigits n), e, hc]; simp [List.getLast?_reverse]
    rw [e]
    have hv : revValue (c :: cs).reverse = some n := by rw [← e, List.reverse_reverse]; exact revValue_revDigits n
    rw [natOf?_cons hc]; exact hv

theorem natDigits_natOf? {cs : List Char} {n : Nat} (h : natOf? cs = some n) : cs = natDigits n := by
  match cs, h with
  | c :: cs, h =>
    by_cases hc : c = '0'
    · subst hc
      cases cs with
      | nil => simp only [natOf?, Option.some.injEq] at h; subst h; rw [natDigits_zero]
      | cons d ds => simp [natOf?] at h
    · have hv : revValue (c :: cs).reverse = some n := by rwa [natOf?_cons hc] at h
      have := revDigits_revValue hv (by simpa [List.getLast?_reverse] using hc) (by simp)
      simp [natDigits, this]

/-- **The codec reads back exactly the texts it writes.** An integer's text
reads as that integer, and a text that reads as an integer is that integer's
text: no other spelling is accepted. -/
theorem intOf?_eq_some_iff {s : String} {i : Int} : intOf? s = some i ↔ s = intText i := by
  constructor
  · intro h
    unfold intOf? at h
    split at h
    · rename_i cs hs
      split at h
      · rename_i n hn
        cases h
        rw [← String.ofList_toList (s := s), hs, natDigits_natOf? hn]; rfl
      · cases h
    · rename_i cs hs
      obtain ⟨n, hn, rfl⟩ := Option.map_eq_some_iff.1 h
      rw [← String.ofList_toList (s := s), natDigits_natOf? hn]; rfl
  · rintro rfl
    cases i with
    | ofNat n =>
      unfold intOf?
      simp only [intText, String.toList_ofList]
      split
      · rename_i cs hs
        exact absurd (hs ▸ List.mem_cons_self) (minus_not_mem_natDigits n)
      · simp [natOf?_natDigits]
    | negSucc n =>
      simp [intOf?, intText, natOf?_natDigits]

theorem intOf?_intText (i : Int) : intOf? (intText i) = some i := intOf?_eq_some_iff.2 rfl

theorem intText_injective {a b : Int} (h : intText a = intText b) : a = b := by
  have := intOf?_intText a
  rw [h, intOf?_intText] at this
  exact (Option.some.inj this).symm

/-! ## Canonical text is injective on the JSON the codec writes

The codec writes `null`, text, arrays and objects whose members are already in
UTF-16 order, each named once (`Json.Plain`). On those, `Json.canon` is
injective, and more: no canonical text is a proper prefix of another, so a
reader can tell where each value ends. That is the whole argument: a string's
escapes each decode to one character, and an array's or object's elements are
separated where the previous element's text ends. -/

mutual
/-- JSON the result codec writes: `null`, text, arrays of such, and objects of
such whose members are in UTF-16 order of their names, each named once. -/
def Json.Plain : Json → Prop
  | .null => True
  | .str _ => True
  | .arr xs => Json.PlainAll xs
  | .obj kvs => Sorted (fun kv : String × Json => utf16 kv.1) kvs ∧ Json.PlainMembers kvs
  | .bool _ => False
  | .int _ => False
  | .number _ => False

def Json.PlainAll : List Json → Prop
  | [] => True
  | x :: xs => x.Plain ∧ Json.PlainAll xs

def Json.PlainMembers : List (String × Json) → Prop
  | [] => True
  | (_, v) :: kvs => v.Plain ∧ Json.PlainMembers kvs
end

theorem Json.plainAll_iff : ∀ {xs : List Json}, Json.PlainAll xs ↔ ∀ x ∈ xs, x.Plain
  | [] => by simp [Json.PlainAll]
  | x :: xs => by simp [Json.PlainAll, Json.plainAll_iff (xs := xs)]

theorem Json.plainMembers_iff : ∀ {kvs : List (String × Json)}, Json.PlainMembers kvs ↔ ∀ kv ∈ kvs, kv.2.Plain
  | [] => by simp [Json.PlainMembers]
  | (k, v) :: kvs => by simp [Json.PlainMembers, Json.plainMembers_iff (kvs := kvs)]

/-! ### Sorting what is sorted -/

theorem insertBy_last {α : Type} {key : α → List Nat} {x : α} :
    ∀ {l : List α}, (∀ y ∈ l, lexLt (key y) (key x) = true) → insertBy lexLt key x l = l ++ [x]
  | [], _ => rfl
  | y :: ys, h => by
    have hyx := h y mem_cons_self
    have h1 : lexLt (key x) (key y) = false := lexLt_asymm hyx
    have h2 : key x ≠ key y := by intro e; rw [e, lexLt_irrefl] at hyx; simp at hyx
    simp only [insertBy, h1, h2, Bool.false_eq_true, ite_false, cons_append,
      insertBy_last (fun z hz => h z (mem_cons_of_mem y hz))]

theorem foldl_insertBy_sorted {α : Type} {key : α → List Nat} :
    ∀ {l acc : List α}, Sorted key (acc ++ l) → l.foldl (fun acc x => insertBy lexLt key x acc) acc = acc ++ l
  | [], acc, _ => by simp
  | x :: xs, acc, h => by
    have hx : ∀ y ∈ acc, lexLt (key y) (key x) = true := fun y hy =>
      (pairwise_append.1 h).2.2 y hy x mem_cons_self
    simp only [foldl_cons, insertBy_last hx]
    rw [foldl_insertBy_sorted (by simpa using h)]
    simp

/-- A list already in order sorts to itself. -/
theorem sortBy_of_sorted {α : Type} {key : α → List Nat} {l : List α} (h : Sorted key l) : sortBy key l = l := by
  unfold sortBy
  exact foldl_insertBy_sorted (acc := []) (by simpa using h)

theorem sorted_map_snd {α β : Type} {f : α → β} {l : List (String × α)}
    (h : Sorted (fun p : String × α => utf16 p.1) l) :
    Sorted (fun p : String × β => utf16 p.1) (l.map fun p => (p.1, f p.2)) := by
  unfold Sorted at *
  rw [pairwise_map]; exact h

/-! ### Escapes decode -/

/-- A hexadecimal digit's value, for the lower-case digits `hexDigit` writes. -/
def hexVal (c : Char) : Nat := if c.toNat < 58 then c.toNat - 48 else c.toNat - 87

/-- One character of a string's text, as JCS escapes it, and what follows. -/
def unescape : List Char → Option (Char × List Char)
  | '\\' :: '"' :: r => some ('"', r)
  | '\\' :: '\\' :: r => some ('\\', r)
  | '\\' :: 'b' :: r => some ('\x08', r)
  | '\\' :: 'f' :: r => some ('\x0c', r)
  | '\\' :: 'n' :: r => some ('\n', r)
  | '\\' :: 'r' :: r => some ('\r', r)
  | '\\' :: 't' :: r => some ('\t', r)
  | '\\' :: 'u' :: '0' :: '0' :: h :: l :: r => some (Char.ofNat (16 * hexVal h + hexVal l), r)
  | c :: r => some (c, r)
  | [] => none

theorem hexVal_hexDigit : ∀ k, k < 16 → hexVal (hexDigit k) = k := by decide

theorem unescape_plain {c : Char} (h : c ≠ '\\') (r : List Char) : unescape (c :: r) = some (c, r) := by
  unfold unescape
  split <;> simp_all

/-- Each character's escape decodes to that character, whatever follows. -/
theorem unescape_escape (c : Char) (X : List Char) : unescape ((escapeChar c).toList ++ X) = some (c, X) := by
  unfold escapeChar
  split
  · rfl
  · rfl
  · rfl
  · rfl
  · rfl
  · rfl
  · rfl
  · rename_i h1 h2 h3 h4 h5 h6 h7
    split
    · rename_i hc
      have hd : c.toNat / 16 < 16 := by omega
      have hm : c.toNat % 16 < 16 := Nat.mod_lt _ (by omega)
      simp only [String.toList_append, String.toList_singleton, Char.toString]
      show unescape ('\\' :: 'u' :: '0' :: '0' :: hexDigit (c.toNat / 16) :: hexDigit (c.toNat % 16) :: X) = _
      simp only [unescape, hexVal_hexDigit _ hd, hexVal_hexDigit _ hm]
      have : 16 * (c.toNat / 16) + c.toNat % 16 = c.toNat := by omega
      rw [this, Char.ofNat_toNat]
    · simp only [Char.toString, String.toList_singleton, singleton_append]
      exact unescape_plain h2 X

theorem escape_ne_nil (c : Char) : (escapeChar c).toList ≠ [] := by
  intro h
  have := unescape_escape c []
  rw [h] at this
  simp [unescape] at this

/-- No escape starts with a quote, so a string's text ends at its first quote. -/
theorem escape_head (c : Char) : ∃ d r, (escapeChar c).toList = d :: r ∧ d ≠ '"' := by
  obtain ⟨d, r, e⟩ := List.exists_cons_of_ne_nil (escape_ne_nil c)
  refine ⟨d, r, e, ?_⟩
  rintro rfl
  have h := unescape_escape c []
  rw [e, List.append_nil, unescape_plain (by decide) r] at h
  simp only [Option.some.injEq, Prod.mk.injEq] at h
  obtain ⟨rfl, rfl⟩ := h
  have : (escapeChar '"').toList = ['\\', '"'] := rfl
  rw [this] at e; simp at e

theorem escapes_prefix : ∀ (s t : List Char) {X Y : List Char},
    s.flatMap (fun c => (escapeChar c).toList) ++ '"' :: X = t.flatMap (fun c => (escapeChar c).toList) ++ '"' :: Y →
      s = t ∧ X = Y
  | [], [], X, Y, h => by simpa using h
  | [], d :: t, X, Y, h => by
    obtain ⟨e, r, he, hne⟩ := escape_head d
    simp only [flatMap_nil, nil_append, flatMap_cons, he, cons_append, cons.injEq] at h
    exact absurd h.1.symm hne
  | c :: s, [], X, Y, h => by
    obtain ⟨e, r, he, hne⟩ := escape_head c
    simp only [flatMap_nil, nil_append, flatMap_cons, he, cons_append, cons.injEq] at h
    exact absurd h.1 hne
  | c :: s, d :: t, X, Y, h => by
    simp only [flatMap_cons, append_assoc] at h
    have h' := congrArg unescape h
    rw [unescape_escape, unescape_escape] at h'
    simp only [Option.some.injEq, Prod.mk.injEq] at h'
    obtain ⟨rfl, h''⟩ := h'
    obtain ⟨rfl, hx⟩ := escapes_prefix s t h''
    exact ⟨rfl, hx⟩

/-! ### Canonical text, character by character -/

/-- A member's canonical text: its quoted name, `:`, its value's text. -/
def memberText (kv : String × Json) : List Char :=
  '"' :: kv.1.toList.flatMap (fun c => (escapeChar c).toList) ++ '"' :: ':' :: kv.2.canon.toList

theorem quote_toList (s : String) :
    (quote s).toList = '"' :: s.toList.flatMap (fun c => (escapeChar c).toList) ++ ['"'] := by
  simp [quote, flatMap_map]

theorem canon_null : (Json.null).canon.toList = ['n', 'u', 'l', 'l'] := rfl

theorem canon_str (s : String) :
    (Json.str s).canon.toList = '"' :: s.toList.flatMap (fun c => (escapeChar c).toList) ++ ['"'] := by
  simp only [Json.canon]; exact quote_toList s

theorem canonAll_eq : ∀ xs : List Json, Json.canonAll xs = xs.map Json.canon
  | [] => rfl
  | x :: xs => by simp [Json.canonAll, canonAll_eq xs]

theorem canon_arr (xs : List Json) :
    (Json.arr xs).canon.toList = '[' :: [','].intercalate (xs.map fun x => x.canon.toList) ++ [']'] := by
  simp only [Json.canon, canonAll_eq, String.toList_append, String.toList_intercalate, map_map]
  rfl

theorem canon_obj {kvs : List (String × Json)} (h : Sorted (fun kv : String × Json => utf16 kv.1) kvs) :
    (Json.obj kvs).canon.toList = '{' :: [','].intercalate (kvs.map memberText) ++ ['}'] := by
  have hs : Sorted (fun kv : String × String => utf16 kv.1) (kvs.map fun kv => (kv.1, kv.2.canon)) :=
    sorted_map_snd h
  simp only [Json.canon, canonMembers_eq, sortBy_of_sorted hs, String.toList_append,
    String.toList_intercalate, map_map]
  simp [Function.comp_def, quote_toList]
  rfl

theorem intercalate_comma (a : List Char) (l : List (List Char)) :
    [','].intercalate (a :: l) = a ++ l.flatMap (fun b => ',' :: b) := by
  induction l generalizing a with
  | nil => simp
  | cons b l ih => rw [intercalate_cons_cons, ih]; simp

/-- A plain value's text starts with `n`, `"`, `[` or `{`: never with what
separates or closes a list. -/
theorem canon_head {a : Json} (h : a.Plain) :
    ∃ c r, a.canon.toList = c :: r ∧ c ≠ ',' ∧ c ≠ ']' ∧ c ≠ '}' := by
  match a, h with
  | .null, _ => exact ⟨_, _, canon_null, by decide, by decide, by decide⟩
  | .str s, _ => exact ⟨_, _, by rw [canon_str]; rfl, by decide, by decide, by decide⟩
  | .arr xs, _ => exact ⟨_, _, by rw [canon_arr]; rfl, by decide, by decide, by decide⟩
  | .obj kvs, h => exact ⟨_, _, by rw [canon_obj h.1]; rfl, by decide, by decide, by decide⟩

theorem memberText_head (kv : String × Json) : ∃ r, memberText kv = '"' :: r := ⟨_, rfl⟩

mutual
/-- **No canonical text is a proper prefix of another**, on plain JSON, and a
canonical text is the text of one value only. -/
theorem Json.canon_prefix : ∀ (a b : Json), a.Plain → b.Plain → ∀ {X Y : List Char},
    a.canon.toList ++ X = b.canon.toList ++ Y → a = b ∧ X = Y
  | .null, .null, _, _, X, Y, h => ⟨rfl, by simpa [canon_null] using h⟩
  | .null, .str t, _, _, _, _, h => by simp [canon_null, canon_str] at h
  | .null, .arr ys, _, _, _, _, h => by simp [canon_null, canon_arr] at h
  | .null, .obj lvs, _, hb, _, _, h => by simp [canon_null, canon_obj hb.1] at h
  | .str s, .null, _, _, _, _, h => by simp [canon_null, canon_str] at h
  | .str s, .str t, _, _, X, Y, h => by
    simp only [canon_str, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨hs, hx⟩ := escapes_prefix _ _ h
    exact ⟨by rw [String.toList_inj.1 hs], hx⟩
  | .str s, .arr ys, _, _, _, _, h => by simp [canon_str, canon_arr] at h
  | .str s, .obj lvs, _, hb, _, _, h => by simp [canon_str, canon_obj hb.1] at h
  | .arr xs, .null, _, _, _, _, h => by simp [canon_null, canon_arr] at h
  | .arr xs, .str t, _, _, _, _, h => by simp [canon_str, canon_arr] at h
  | .arr xs, .arr ys, ha, hb, X, Y, h => by
    simp only [canon_arr, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨e, hx⟩ := Json.elems_prefix xs ys ha hb h
    exact ⟨by rw [e], hx⟩
  | .arr xs, .obj lvs, _, hb, _, _, h => by simp [canon_arr, canon_obj hb.1] at h
  | .obj kvs, .null, ha, _, _, _, h => by simp [canon_null, canon_obj ha.1] at h
  | .obj kvs, .str t, ha, _, _, _, h => by simp [canon_str, canon_obj ha.1] at h
  | .obj kvs, .arr ys, ha, _, _, _, h => by simp [canon_arr, canon_obj ha.1] at h
  | .obj kvs, .obj lvs, ha, hb, X, Y, h => by
    simp only [canon_obj ha.1, canon_obj hb.1, cons_append, append_assoc, cons.injEq,
      true_and] at h
    obtain ⟨e, hx⟩ := Json.members_prefix kvs lvs ha.2 hb.2 h
    exact ⟨by rw [e], hx⟩
  | .bool _, _, ha, _, _, _, _ => ha.elim
  | .int _, _, ha, _, _, _, _ => ha.elim
  | .number _, _, ha, _, _, _, _ => ha.elim
  | .null, .bool _, _, hb, _, _, _ => hb.elim
  | .null, .int _, _, hb, _, _, _ => hb.elim
  | .null, .number _, _, hb, _, _, _ => hb.elim
  | .str _, .bool _, _, hb, _, _, _ => hb.elim
  | .str _, .int _, _, hb, _, _, _ => hb.elim
  | .str _, .number _, _, hb, _, _, _ => hb.elim
  | .arr _, .bool _, _, hb, _, _, _ => hb.elim
  | .arr _, .int _, _, hb, _, _, _ => hb.elim
  | .arr _, .number _, _, hb, _, _, _ => hb.elim
  | .obj _, .bool _, _, hb, _, _, _ => hb.elim
  | .obj _, .int _, _, hb, _, _, _ => hb.elim
  | .obj _, .number _, _, hb, _, _, _ => hb.elim

/-- An array's elements, up to its closing bracket. -/
theorem Json.elems_prefix : ∀ (xs ys : List Json), Json.PlainAll xs → Json.PlainAll ys → ∀ {X Y : List Char},
    [','].intercalate (xs.map fun x => x.canon.toList) ++ ']' :: X =
      [','].intercalate (ys.map fun x => x.canon.toList) ++ ']' :: Y → xs = ys ∧ X = Y
  | [], [], _, _, X, Y, h => by simpa using h
  | [], y :: ys, _, hb, _, _, h => by
    obtain ⟨c, r, e, _, hc, _⟩ := canon_head hb.1
    simp only [map_nil, intercalate_nil, nil_append, map_cons, intercalate_comma, e, cons_append,
      cons.injEq] at h
    exact absurd h.1.symm hc
  | x :: xs, [], ha, _, _, _, h => by
    obtain ⟨c, r, e, _, hc, _⟩ := canon_head ha.1
    simp only [map_nil, intercalate_nil, nil_append, map_cons, intercalate_comma, e, cons_append,
      cons.injEq] at h
    exact absurd h.1 hc
  | x :: xs, y :: ys, ha, hb, X, Y, h => by
    simp only [map_cons, intercalate_comma, append_assoc] at h
    obtain ⟨rfl, h'⟩ := Json.canon_prefix x y ha.1 hb.1 h
    obtain ⟨rfl, hx⟩ := Json.tail_prefix xs ys ha.2 hb.2 h'
    exact ⟨rfl, hx⟩

/-- The elements after an array's first, each after a comma. -/
theorem Json.tail_prefix : ∀ (xs ys : List Json), Json.PlainAll xs → Json.PlainAll ys → ∀ {X Y : List Char},
    (xs.map fun x => x.canon.toList).flatMap (fun b => ',' :: b) ++ ']' :: X =
      (ys.map fun x => x.canon.toList).flatMap (fun b => ',' :: b) ++ ']' :: Y → xs = ys ∧ X = Y
  | [], [], _, _, X, Y, h => by simpa using h
  | [], y :: ys, _, _, _, _, h => by simp at h
  | x :: xs, [], _, _, _, _, h => by simp at h
  | x :: xs, y :: ys, ha, hb, X, Y, h => by
    simp only [map_cons, flatMap_cons, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨rfl, h'⟩ := Json.canon_prefix x y ha.1 hb.1 h
    obtain ⟨rfl, hx⟩ := Json.tail_prefix xs ys ha.2 hb.2 h'
    exact ⟨rfl, hx⟩

/-- An object's members, up to its closing brace. -/
theorem Json.members_prefix : ∀ (kvs lvs : List (String × Json)), Json.PlainMembers kvs →
    Json.PlainMembers lvs → ∀ {X Y : List Char},
    [','].intercalate (kvs.map memberText) ++ '}' :: X = [','].intercalate (lvs.map memberText) ++ '}' :: Y →
      kvs = lvs ∧ X = Y
  | [], [], _, _, X, Y, h => by simpa using h
  | [], lv :: lvs, _, _, _, _, h => by
    obtain ⟨r, e⟩ := memberText_head lv
    simp [intercalate_comma, e] at h
  | kv :: kvs, [], _, _, _, _, h => by
    obtain ⟨r, e⟩ := memberText_head kv
    simp [intercalate_comma, e] at h
  | (k, v) :: kvs, (l, w) :: lvs, ha, hb, X, Y, h => by
    simp only [map_cons, intercalate_comma, append_assoc, memberText, cons_append, cons.injEq, true_and] at h
    obtain ⟨hk, h1⟩ := escapes_prefix _ _ h
    simp only [cons.injEq, true_and] at h1
    obtain ⟨rfl, h2⟩ := Json.canon_prefix v w ha.1 hb.1 h1
    obtain ⟨rfl, hx⟩ := Json.mtail_prefix kvs lvs ha.2 hb.2 h2
    exact ⟨by rw [String.toList_inj.1 hk], hx⟩

/-- The members after an object's first, each after a comma. -/
theorem Json.mtail_prefix : ∀ (kvs lvs : List (String × Json)), Json.PlainMembers kvs →
    Json.PlainMembers lvs → ∀ {X Y : List Char},
    (kvs.map memberText).flatMap (fun b => ',' :: b) ++ '}' :: X =
      (lvs.map memberText).flatMap (fun b => ',' :: b) ++ '}' :: Y → kvs = lvs ∧ X = Y
  | [], [], _, _, X, Y, h => by simpa using h
  | [], lv :: lvs, _, _, _, _, h => by simp at h
  | kv :: kvs, [], _, _, _, _, h => by simp at h
  | (k, v) :: kvs, (l, w) :: lvs, ha, hb, X, Y, h => by
    simp only [map_cons, flatMap_cons, memberText, cons_append, append_assoc, cons.injEq, true_and] at h
    obtain ⟨hk, h1⟩ := escapes_prefix _ _ h
    simp only [cons.injEq, true_and] at h1
    obtain ⟨rfl, h2⟩ := Json.canon_prefix v w ha.1 hb.1 h1
    obtain ⟨rfl, hx⟩ := Json.mtail_prefix kvs lvs ha.2 hb.2 h2
    exact ⟨by rw [String.toList_inj.1 hk], hx⟩
end

/-- **Canonical text is injective on plain JSON**: two values the codec could
write with the same canonical text are the same value. -/
theorem Json.canon_injective {a b : Json} (ha : a.Plain) (hb : b.Plain) (h : a.canon = b.canon) : a = b :=
  (Json.canon_prefix a b ha hb (X := []) (Y := []) (by rw [h])).1

/-! ## What a result writes -/

/-- A map's members in UTF-16 order of their keys, each once. -/
abbrev KeyOrdered {α : Type} (l : List (String × α)) : Prop := Sorted (fun p : String × α => utf16 p.1) l

/-- A slot value whose maps, by thing, group key or payload key, are in UTF-16
order of their keys: what every slot writes (`Slot.run_ordered`). -/
def SlotValue.Ordered : SlotValue → Prop
  | .things ts _ => KeyOrdered ts
  | .keyed ts _ => KeyOrdered ts ∧ ∀ p ∈ ts, KeyOrdered p.2.keys
  | .groups gs _ _ => KeyOrdered gs
  | _ => True

/-- Which fold a group's value is of: count, sum, min or max, or a list. -/
def GroupValue.tag : GroupValue → Nat
  | .nat _ => 0
  | .int _ => 1
  | .opt _ => 2
  | .heads _ _ => 3

/-- Two slot values of one fold: the same shape, and with `by`, groups of the
same fold. Two runs of one slot always are (`Slot.run_like`). -/
def SlotValue.Like : SlotValue → SlotValue → Prop
  | .nat _, .nat _ => True
  | .int _, .int _ => True
  | .opt _, .opt _ => True
  | .heads _ _, .heads _ _ => True
  | .things _ _, .things _ _ => True
  | .keyed _ _, .keyed _ _ => True
  | .groups gs _ _, .groups hs _ _ => ∃ t, (∀ p ∈ gs, p.2.tag = t) ∧ ∀ p ∈ hs, p.2.tag = t
  | _, _ => False

/-! ### The JSON is plain -/

theorem sorted_two {α : Type} {k₁ k₂ : String} {a b : α} (h : lexLt (utf16 k₁) (utf16 k₂) = true) :
    KeyOrdered [(k₁, a), (k₂, b)] := by
  simp [KeyOrdered, Sorted, h]

theorem sorted_three {α : Type} {k₁ k₂ k₃ : String} {a b c : α} (h₁ : lexLt (utf16 k₁) (utf16 k₂) = true)
    (h₂ : lexLt (utf16 k₂) (utf16 k₃) = true) : KeyOrdered [(k₁, a), (k₂, b), (k₃, c)] := by
  simp [KeyOrdered, Sorted, h₁, h₂, lexLt_trans h₁ h₂]

theorem idsJson_plain (hs : List (RecordId × Shown)) : (idsJson hs).Plain := by
  simp only [idsJson, Json.Plain, Json.plainAll_iff, mem_map]
  rintro _ ⟨h, _, rfl⟩; trivial

theorem GroupValue.result_plain (g : GroupValue) : g.result.Plain := by
  cases g with
  | nat r => exact ⟨sorted_two (by decide), trivial, trivial, trivial⟩
  | int r => exact ⟨sorted_two (by decide), trivial, trivial, trivial⟩
  | opt r =>
    refine ⟨sorted_two (by decide), trivial, ?_, trivial⟩
    cases r.value <;> trivial
  | heads hs u => exact ⟨sorted_two (by decide), idsJson_plain hs, trivial, trivial⟩

theorem idMap_plain {ts : List (String × List (RecordId × Shown))} (h : KeyOrdered ts) :
    (Json.obj (ts.map fun p => (p.1, idsJson p.2))).Plain := by
  refine ⟨sorted_map_snd h, Json.plainMembers_iff.2 ?_⟩
  simp only [mem_map]
  rintro _ ⟨p, _, rfl⟩; exact idsJson_plain _

theorem KeyedView.result_plain {k : KeyedView} (h : KeyOrdered k.keys) : k.result.Plain :=
  ⟨sorted_three (by decide) (by decide), idsJson_plain _, idMap_plain h, idsJson_plain _, trivial⟩

theorem SlotValue.result_plain {v : SlotValue} (h : v.Ordered) : v.result.Plain := by
  cases v with
  | nat r => exact GroupValue.result_plain (.nat r)
  | int r => exact GroupValue.result_plain (.int r)
  | opt r => exact GroupValue.result_plain (.opt r)
  | heads hs u => exact GroupValue.result_plain (.heads hs u)
  | things ts u => exact ⟨sorted_two (by decide), idMap_plain h, trivial, trivial⟩
  | keyed ts u =>
    refine ⟨sorted_two (by decide), ⟨sorted_map_snd h.1, Json.plainMembers_iff.2 ?_⟩, trivial, trivial⟩
    simp only [mem_map]
    rintro _ ⟨p, hp, rfl⟩; exact KeyedView.result_plain (h.2 p hp)
  | groups gs u m =>
    refine ⟨sorted_three (by decide) (by decide), ⟨sorted_map_snd h, Json.plainMembers_iff.2 ?_⟩,
      trivial, trivial, trivial⟩
    simp only [mem_map]
    rintro _ ⟨p, _, rfl⟩; exact GroupValue.result_plain _

/-! ### `shown` is not written -/

theorem idsJson_unshown (hs : List (RecordId × Shown)) : idsJson (unshownIds hs) = idsJson hs := by
  simp [idsJson, unshownIds, map_map, Function.comp_def]

theorem GroupValue.result_unshown (g : GroupValue) : g.unshown.result = g.result := by
  cases g <;> simp [GroupValue.unshown, GroupValue.result, idsJson_unshown]

theorem KeyedView.result_unshown (k : KeyedView) : k.unshown.result = k.result := by
  simp [KeyedView.unshown, KeyedView.result, idsJson_unshown, map_map, Function.comp_def]

/-- **A slot's canonical value does not read `shown`.** -/
theorem SlotValue.result_unshown (v : SlotValue) : v.unshown.result = v.result := by
  cases v with
  | nat r => rfl
  | int r => rfl
  | opt r => rfl
  | heads hs u => simp [SlotValue.unshown, SlotValue.result, GroupValue.result, idsJson_unshown]
  | things ts u => simp [SlotValue.unshown, SlotValue.result, idsJson_unshown, map_map, Function.comp_def]
  | keyed ts u => simp [SlotValue.unshown, SlotValue.result, KeyedView.result_unshown, map_map, Function.comp_def]
  | groups gs u m => simp [SlotValue.unshown, SlotValue.result, GroupValue.result_unshown, map_map, Function.comp_def]

/-- **Setting `shown` aside leaves the canonical text as it was**, in every slot. -/
theorem resultText_unshown (r : List (String × SlotValue)) : resultText (unshownResult r) = resultText r := by
  simp [resultText, resultJson, unshownResult, map_map, Function.comp_def, SlotValue.result_unshown]

/-- Two results that differ only in what is shown have the same text, bytes,
preimage and hash, for any hash. -/
theorem resultHash_of_unshown (H : Hash) {r q : List (String × SlotValue)}
    (h : unshownResult r = unshownResult q) :
    resultText r = resultText q ∧ resultBytes r = resultBytes q ∧ resultPreimage r = resultPreimage q ∧
      resultHash H r = resultHash H q := by
  have ht : resultText r = resultText q := by rw [← resultText_unshown r, h, resultText_unshown]
  simp [resultHash, resultPreimage, resultBytes, ht]

/-! ### Nothing else is lost -/

theorem intJson_eq_iff {a b : Int} : intJson a = intJson b ↔ a = b :=
  ⟨fun h => intText_injective (Json.str.inj h), fun h => h ▸ rfl⟩

theorem idsJson_eq_iff {hs gs : List (RecordId × Shown)} : idsJson hs = idsJson gs ↔ unshownIds hs = unshownIds gs := by
  constructor
  · intro h
    simp only [idsJson, Json.arr.injEq] at h
    induction hs generalizing gs with
    | nil => cases gs <;> simp_all [unshownIds]
    | cons x xs ih =>
      cases gs with
      | nil => simp at h
      | cons y ys =>
        simp only [map_cons, cons.injEq, Json.str.injEq] at h
        have := ih h.2
        simp only [unshownIds, map_cons, cons.injEq, Prod.mk.injEq, and_true] at this ⊢
        exact ⟨idText_injective h.1, this⟩
  · intro h; rw [← idsJson_unshown hs, h, idsJson_unshown]

/-- Two maps written alike, members by key, hold values written alike, key by key. -/
theorem map_snd_inj {α β γ : Type} {F : α → β} {G : α → γ} :
    ∀ {ts us : List (String × α)}, (∀ a ∈ ts, ∀ b ∈ us, F a.2 = F b.2 → G a.2 = G b.2) →
      ts.map (fun p => (p.1, F p.2)) = us.map (fun p => (p.1, F p.2)) →
        ts.map (fun p => (p.1, G p.2)) = us.map (fun p => (p.1, G p.2))
  | [], [], _, _ => rfl
  | [], _ :: _, _, h => by simp at h
  | _ :: _, [], _, h => by simp at h
  | a :: ts, b :: us, hi, h => by
    simp only [map_cons, cons.injEq, Prod.mk.injEq] at h ⊢
    exact ⟨⟨h.1.1, hi a mem_cons_self b mem_cons_self h.1.2⟩,
      map_snd_inj (fun x hx y hy => hi x (mem_cons_of_mem a hx) y (mem_cons_of_mem b hy)) h.2⟩

theorem GroupValue.result_inj {a b : GroupValue} (ht : a.tag = b.tag) (h : a.result = b.result) :
    a.unshown = b.unshown := by
  cases a <;> cases b <;> simp only [GroupValue.tag] at ht <;> try omega
  all_goals simp only [GroupValue.result, Json.obj.injEq, cons.injEq, Prod.mk.injEq, true_and, and_true,
    intJson_eq_iff, idsJson_eq_iff, Int.natCast_inj, GroupValue.unshown, GroupValue.nat.injEq,
    GroupValue.int.injEq, GroupValue.opt.injEq, GroupValue.heads.injEq] at h ⊢
  · rename_i r q; cases r; cases q; simp_all
  · rename_i r q; cases r; cases q; simp_all
  · rename_i r q
    obtain ⟨v, u⟩ := r; obtain ⟨w, u'⟩ := q
    cases v <;> cases w <;> simp only [intJson_eq_iff] at h
    · simp_all
    · simp [intJson] at h
    · simp [intJson] at h
    · simp_all
  · exact h

theorem KeyedView.result_inj {k k' : KeyedView} (h : k.result = k'.result) : k.unshown = k'.unshown := by
  simp only [KeyedView.result, Json.obj.injEq, cons.injEq, Prod.mk.injEq, true_and, and_true,
    idsJson_eq_iff] at h
  obtain ⟨h1, h2, h3⟩ := h
  have hk := map_snd_inj (G := unshownIds) (fun a _ b _ e => idsJson_eq_iff.1 e) h2
  simp only [KeyedView.unshown]
  rw [h1, h3, hk]

/-- **A slot's canonical value is lost nowhere but `shown`**: two values of
one fold written alike are alike once `shown` is set aside. -/
theorem SlotValue.result_inj {v w : SlotValue} (hl : v.Like w) (h : v.result = w.result) :
    v.unshown = w.unshown := by
  cases v <;> cases w <;> simp only [SlotValue.Like] at hl
  case nat.nat r q =>
    have := GroupValue.result_inj (a := .nat r) (b := .nat q) rfl h
    simpa [GroupValue.unshown, SlotValue.unshown] using this
  case int.int r q =>
    have := GroupValue.result_inj (a := .int r) (b := .int q) rfl h
    simpa [GroupValue.unshown, SlotValue.unshown] using this
  case opt.opt r q =>
    have := GroupValue.result_inj (a := .opt r) (b := .opt q) rfl h
    simpa [GroupValue.unshown, SlotValue.unshown] using this
  case heads.heads hs u gs u' =>
    have := GroupValue.result_inj (a := .heads hs u) (b := .heads gs u') rfl h
    simpa [GroupValue.unshown, SlotValue.unshown] using this
  case things.things ts u us u' =>
    simp only [SlotValue.result, Json.obj.injEq, cons.injEq, Prod.mk.injEq, true_and, and_true,
      intJson_eq_iff, Int.natCast_inj] at h
    simp only [SlotValue.unshown, SlotValue.things.injEq]
    exact ⟨map_snd_inj (fun a _ b _ e => idsJson_eq_iff.1 e) h.1, h.2⟩
  case keyed.keyed ts u us u' =>
    simp only [SlotValue.result, Json.obj.injEq, cons.injEq, Prod.mk.injEq, true_and, and_true,
      intJson_eq_iff, Int.natCast_inj] at h
    simp only [SlotValue.unshown, SlotValue.keyed.injEq]
    exact ⟨map_snd_inj (fun a _ b _ e => KeyedView.result_inj e) h.1, h.2⟩
  case groups.groups gs u m hs u' m' =>
    obtain ⟨t, hg, hh⟩ := hl
    simp only [SlotValue.result, Json.obj.injEq, cons.injEq, Prod.mk.injEq, true_and, and_true,
      intJson_eq_iff, Int.natCast_inj] at h
    simp only [SlotValue.unshown, SlotValue.groups.injEq]
    exact ⟨map_snd_inj (fun a ha b hb e => GroupValue.result_inj ((hg a ha).trans (hh b hb).symm) e) h.1,
      h.2.2, h.2.1⟩

/-! ## What a spec's run gives -/

theorem keyOrdered_of_sublist {α : Type} {l : List (String × α)} {ks : List String}
    (h : (l.map Prod.fst).Sublist (canonSet ks)) : KeyOrdered l := by
  have hs : (canonSet ks).Pairwise fun a b => lexLt (utf16 a) (utf16 b) = true := sortBy_sorted utf16 ks
  have := hs.sublist h
  unfold KeyOrdered Sorted
  rwa [pairwise_map] at this

theorem keyOrdered_canonSet {α : Type} (ks : List String) (f : String → α) :
    KeyOrdered ((canonSet ks).map fun g => (g, f g)) :=
  keyOrdered_of_sublist (ks := ks) (by simp [Function.comp_def])

/-- A `filterMap` that keys what it keeps by what it read keeps a sublist of the keys. -/
theorem filterMap_key_sublist {α β : Type} {f : α → Option (α × β)} (hf : ∀ a q, f a = some q → q.1 = a) :
    ∀ l : List α, ((l.filterMap f).map Prod.fst).Sublist l
  | [] => by simp
  | a :: l => by
    cases h : f a with
    | none => rw [filterMap_cons_none h]; exact (filterMap_key_sublist hf l).cons _
    | some q => rw [filterMap_cons_some h, map_cons, hf a q h]; exact (filterMap_key_sublist hf l).cons_cons _

/-- A `filterMap` that keeps each key keeps a sublist of the keys. -/
theorem filterMap_fst_sublist {α β γ : Type} {f : α × β → Option (α × γ)} (hf : ∀ p q, f p = some q → q.1 = p.1) :
    ∀ l : List (α × β), ((l.filterMap f).map Prod.fst).Sublist (l.map Prod.fst)
  | [] => by simp
  | p :: l => by
    simp only [filterMap_cons, map_cons]
    cases h : f p with
    | none => exact (filterMap_fst_sublist hf l).cons _
    | some q => rw [map_cons, hf p q h]; exact (filterMap_fst_sublist hf l).cons_cons _

theorem keyedIn_view_ordered (t : String) (xs : List Record) : KeyOrdered (keyedIn t xs).view.keys := by
  simp only [Keyed.view, keyedIn, liveKeys, map_map, Function.comp_def]
  exact keyOrdered_canonSet _ _

theorem latestOf_ordered (sel : Record → Bool) (cs : List Cond) (xs : List Record) : (latestOf sel cs xs).Ordered := by
  simp only [latestOf, SlotValue.Ordered]
  apply keyOrdered_of_sublist (ks := xs.map Record.thing)
  refine (filterMap_fst_sublist ?_ _).trans ?_
  · intro p q h; split at h
    · cases h
    · cases h; rfl
  · simp [perThing, thingsOf, map_map, Function.comp_def]

theorem keyedOf_ordered (sel : Record → Bool) (cs : List Cond) (xs : List Record) : (keyedOf sel cs xs).Ordered := by
  simp only [keyedOf, SlotValue.Ordered]
  refine ⟨keyOrdered_of_sublist (ks := xs.map Record.thing) ?_, ?_⟩
  · have hk := filterMap_key_sublist (f := fun t =>
        if (keyedIn t xs).heads.any (chosen sel cs) then
          some (t, keyedIn t xs, thingTest cs (xs.filter (·.isOf t))) else none)
      (by
        intro a q h
        split at h
        · cases h; rfl
        · cases h) (thingsOf xs)
    refine Sublist.trans ?_ hk
    rw [map_map]
    have e : ∀ l : List (String × Keyed × Tri),
        l.map (Prod.fst ∘ fun x => match x with | (t, k, _) => (t, k.view)) = l.map Prod.fst := by
      intro l; apply map_congr_left; rintro ⟨t, k, e⟩ _; rfl
    rw [e]
    exact Sublist.map _ filter_sublist
  · intro p hp
    simp only [mem_map, mem_filter, mem_filterMap] at hp
    obtain ⟨⟨t, k, e⟩, ⟨⟨t', _, ht'⟩, _⟩, rfl⟩ := hp
    split at ht'
    · simp only [Option.some.injEq, Prod.mk.injEq] at ht'
      obtain ⟨rfl, rfl, rfl⟩ := ht'
      exact keyedIn_view_ordered t' xs
    · cases ht'

theorem keyedValue_ordered (ty : FoldType) (xs : List Record) (ev : String → Tri) :
    (keyedValue ty xs ev).Ordered := by
  cases ty with
  | latestBy f =>
    simp only [keyedValue, SlotValue.Ordered]
    refine ⟨keyOrdered_of_sublist (ks := xs.map Record.thing) ?_, ?_⟩
    · rw [map_map]; simp only [Function.comp_def, map_id']; exact filter_sublist
    · intro p hp
      obtain ⟨t, _, rfl⟩ := mem_map.1 hp
      exact keyedIn_view_ordered t xs
  | _ => simp [keyedValue, SlotValue.Ordered]

theorem Slot.plain_ordered (s : Slot) (sel : Record → Bool) (xs : List Record) : (s.plain sel xs).Ordered := by
  unfold Slot.plain
  split
  · split
    · exact keyedValue_ordered _ _ _
    · trivial
  · split
    · exact keyedValue_ordered _ _ _
    · trivial
  · split
    · exact keyedValue_ordered _ _ _
    · trivial
  · split
    · exact keyedValue_ordered _ _ _
    · trivial
  · exact latestOf_ordered _ _ _
  · exact keyedOf_ordered _ _ _
  · split
    · exact keyedValue_ordered _ _ _
    · simp [collectOf, SlotValue.Ordered]

/-- **Every slot writes its maps in UTF-16 order of their keys**: by thing, by
group key, and a keyed thing's payload keys. -/
theorem Slot.run_ordered (s : Slot) (sel : Record → Bool) (xs : List Record) : (s.run sel xs).Ordered := by
  unfold Slot.run
  split
  · exact s.plain_ordered sel xs
  · split
    · simp only [Slot.keyedGrouped, SlotValue.Ordered]; exact keyOrdered_canonSet _ _
    · simp only [Slot.grouped, groupKeys, SlotValue.Ordered]; exact keyOrdered_canonSet _ _

/-- Which fold a slot's groups hold: count, sum, min or max, or a list. -/
def FoldType.tag : FoldType → Nat
  | .count _ => 0
  | .sum _ _ => 1
  | .min _ _ | .max _ _ => 2
  | .latestBy _ | .collect _ _ => 3

theorem keyedValue_tag (ty : FoldType) (xs : List Record) (ev : String → Tri) :
    (keyedValue ty xs ev).group.tag = ty.tag := by
  cases ty <;> rfl

theorem Slot.plain_tag (s : Slot) (sel : Record → Bool) (xs : List Record) : (s.plain sel xs).group.tag = s.type.tag := by
  unfold Slot.plain
  split <;> rename_i h <;> rw [h] <;> (try split) <;>
    first
    | exact keyedValue_tag _ _ _
    | rfl

theorem Slot.plain_like (s : Slot) (sel sel' : Record → Bool) (xs ys : List Record) :
    (s.plain sel xs).Like (s.plain sel' ys) := by
  unfold Slot.plain
  split
  all_goals first
    | (split <;> simp [keyedValue, collectOf, SlotValue.Like])
    | simp [latestOf, keyedOf, SlotValue.Like]

/-- **Two runs of one slot are of one fold**: the same shape, and with `by`,
groups of the same fold. -/
theorem Slot.run_like (s : Slot) (sel sel' : Record → Bool) (xs ys : List Record) :
    (s.run sel xs).Like (s.run sel' ys) := by
  unfold Slot.run
  split
  · exact s.plain_like sel sel' xs ys
  · split
    · simp only [Slot.keyedGrouped, SlotValue.Like]
      refine ⟨s.type.tag, ?_, ?_⟩ <;>
      · intro p hp
        obtain ⟨g, _, rfl⟩ := mem_map.1 hp
        exact keyedValue_tag _ _ _
    · simp only [Slot.grouped, SlotValue.Like]
      refine ⟨s.type.tag, ?_, ?_⟩ <;>
      · intro p hp
        obtain ⟨g, _, rfl⟩ := mem_map.1 hp
        exact Slot.plain_tag _ _ _

/-! ## The whole result -/

theorem sortBy_perm {α : Type} {key : α → List Nat} {l : List α}
    (inj : ∀ a ∈ l, ∀ b ∈ l, key a = key b → a = b) (hl : l.Nodup) : sortBy key l ~ l := by
  have hs : (sortBy key l).Nodup := (sortBy_sorted key l).imp fun h e => by
    subst e; rw [lexLt_irrefl] at h; exact absurd h (by simp)
  exact (perm_ext_iff_of_nodup hs hl).2 (mem_sortBy inj)

/-- The members a result writes: each slot's name and canonical value. -/
abbrev resultMembers (r : List (String × SlotValue)) : List (String × Json) := r.map fun p => (p.1, p.2.result)

theorem resultMembers_inj {r : List (String × SlotValue)} (hr : (r.map (·.1)).Nodup) :
    ∀ a ∈ resultMembers r, ∀ b ∈ resultMembers r, utf16 a.1 = utf16 b.1 → a = b := by
  intro a ha b hb e
  have hn : ((resultMembers r).map (·.1)).Nodup := by simpa [resultMembers, map_map, Function.comp_def] using hr
  exact eq_of_mem_of_nodup_map hn a ha b hb (utf16_inj e)

theorem resultText_sorted {r : List (String × SlotValue)} (hr : (r.map (·.1)).Nodup) :
    resultText r = (Json.obj (sortBy (fun kv : String × Json => utf16 kv.1) (resultMembers r))).canon := by
  have hn : ((resultMembers r).map (·.1)).Nodup := by simpa [resultMembers, map_map, Function.comp_def] using hr
  exact canon_member_order (sortBy_perm (resultMembers_inj hr) (Pairwise.of_map (·.1) (fun _ _ hne e => hne (by rw [e])) hn)).symm hn

theorem sorted_members_plain {r : List (String × SlotValue)} (hr : (r.map (·.1)).Nodup)
    (ho : ∀ p ∈ r, p.2.Ordered) :
    (Json.obj (sortBy (fun kv : String × Json => utf16 kv.1) (resultMembers r))).Plain := by
  refine ⟨sortBy_sorted _ _, Json.plainMembers_iff.2 ?_⟩
  intro kv hkv
  rw [mem_sortBy (resultMembers_inj hr)] at hkv
  obtain ⟨p, hp, rfl⟩ := mem_map.1 hkv
  exact SlotValue.result_plain (ho p hp)

/-- **Two results have the same canonical text exactly when they write the
same slots, each with the same canonical value.** Each names its slots once,
and its maps are in UTF-16 order, as every run's are (`Slot.run_ordered`). -/
theorem resultText_eq_iff {r q : List (String × SlotValue)} (hr : (r.map (·.1)).Nodup)
    (hq : (q.map (·.1)).Nodup) (or : ∀ p ∈ r, p.2.Ordered) (oq : ∀ p ∈ q, p.2.Ordered) :
    resultText r = resultText q ↔ ∀ m, m ∈ resultMembers r ↔ m ∈ resultMembers q := by
  rw [resultText_sorted hr, resultText_sorted hq]
  constructor
  · intro h m
    have e := Json.canon_injective (sorted_members_plain hr or) (sorted_members_plain hq oq) h
    simp only [Json.obj.injEq] at e
    rw [← mem_sortBy (resultMembers_inj hr), e, mem_sortBy (resultMembers_inj hq)]
  · intro h
    have e : sortBy (fun kv : String × Json => utf16 kv.1) (resultMembers r) =
        sortBy (fun kv : String × Json => utf16 kv.1) (resultMembers q) :=
      sortBy_ext fun m => by rw [mem_sortBy (resultMembers_inj hr), mem_sortBy (resultMembers_inj hq)]; exact h m
    rw [e]

theorem Spec.run_names (sp : Spec) (xs : List Record) (may : Record → Record → Bool) :
    (sp.run xs may).map (·.1) = sp.slots.map (·.name) := by
  simp [Spec.run, map_map, Function.comp_def]

/-- **Two runs of one spec have the same canonical text exactly when their
results agree once `shown` is set aside.** Whatever the records and whatever
the store's rule, the text loses `shown` and nothing else. The spec names each
slot once, as every spec the parser accepts does (`Spec.ofJson_fits`). -/
theorem Spec.resultText_run_iff {sp : Spec} (hn : (sp.slots.map (·.name)).Nodup) (xs ys : List Record)
    (may may' : Record → Record → Bool) :
    resultText (sp.run xs may) = resultText (sp.run ys may') ↔
      unshownResult (sp.run xs may) = unshownResult (sp.run ys may') := by
  constructor
  · intro h
    have ord : ∀ zs may'', ∀ p ∈ sp.run zs may'', p.2.Ordered := by
      intro zs m p hp
      simp only [Spec.run, mem_map] at hp
      obtain ⟨s, _, rfl⟩ := hp
      exact Slot.run_ordered _ _ _
    have hm := (resultText_eq_iff (by rw [Spec.run_names]; exact hn) (by rw [Spec.run_names]; exact hn)
      (ord xs may) (ord ys may')).1 h
    simp only [unshownResult, Spec.run, map_map]
    apply map_congr_left
    intro s hs
    have h1 : (s.name, (s.run sp.takes (admit may xs)).result) ∈ resultMembers (sp.run xs may) := by
      simp only [resultMembers, Spec.run, map_map, mem_map]; exact ⟨s, hs, rfl⟩
    have h2 := (hm _).1 h1
    simp only [resultMembers, Spec.run, map_map, mem_map, Function.comp_def, Prod.mk.injEq] at h2
    obtain ⟨s', hs', hname, hres⟩ := h2
    have : s' = s := eq_of_mem_of_nodup_map hn s' hs' s hs hname
    subst this
    simp only [Function.comp_def, Prod.mk.injEq, true_and]
    exact SlotValue.result_inj (Slot.run_like _ _ _ _ _) hres.symm
  · intro h
    exact (resultHash_of_unshown (fun _ => []) h).1

/-! ## Bytes and preimage -/

theorem toList_loop (bs : ByteArray) (i : Nat) (r : List UInt8) :
    ByteArray.toList.loop bs i r = r.reverse ++ bs.data.toList.drop i := by
  rw [ByteArray.toList.loop]
  split
  · rename_i h
    rw [toList_loop bs (i + 1)]
    have hi : i < bs.data.toList.length := by simpa using h
    rw [List.drop_eq_getElem_cons hi]
    simp [ByteArray.get!, getElem!_pos bs.data i (by simpa using h)]
  · rename_i h
    have : bs.data.toList.length ≤ i := by simp; omega
    simp [List.drop_eq_nil_of_le this]
termination_by bs.size - i

/-- UTF-8 tells texts apart: the bytes are the text. -/
theorem utf8_injective {a b : String} (h : utf8 a = utf8 b) : a = b := by
  have e : ∀ s : String, utf8 s = s.toByteArray.data.toList := fun s => by
    simp [utf8, String.toUTF8, ByteArray.toList, toList_loop]
  rw [e, e] at h
  have : a.toByteArray = b.toByteArray := by
    cases ha : a.toByteArray; cases hb : b.toByteArray
    rw [ha, hb] at h
    simp only at h
    rw [Array.toList_inj.1 h]
  exact String.toByteArray_inj.1 this

/-- Two runs of one spec have the same bytes exactly when their results agree
once `shown` is set aside. -/
theorem Spec.resultBytes_run_iff {sp : Spec} (hn : (sp.slots.map (·.name)).Nodup) (xs ys : List Record)
    (may may' : Record → Record → Bool) :
    resultBytes (sp.run xs may) = resultBytes (sp.run ys may') ↔
      unshownResult (sp.run xs may) = unshownResult (sp.run ys may') := by
  rw [← Spec.resultText_run_iff hn]
  exact ⟨utf8_injective, fun h => by simp [resultBytes, h]⟩

/-- Two runs of one spec give the hash the same preimage exactly when their
results agree once `shown` is set aside: two runs that differ anywhere else
share a hash only where the hash collides. -/
theorem Spec.resultPreimage_run_iff {sp : Spec} (hn : (sp.slots.map (·.name)).Nodup) (xs ys : List Record)
    (may may' : Record → Record → Bool) :
    resultPreimage (sp.run xs may) = resultPreimage (sp.run ys may') ↔
      unshownResult (sp.run xs may) = unshownResult (sp.run ys may') := by
  rw [← Spec.resultBytes_run_iff hn]
  exact ⟨fun h => List.append_cancel_left h, fun h => by simp [resultPreimage, h]⟩

/-- For a spec the parser accepts, with no premise: two runs give the hash the
same preimage exactly when their results agree once `shown` is set aside. -/
theorem Spec.resultPreimage_parsed_iff {j : Json} {sp : Spec} (h : Spec.ofJson j = .ok sp) (xs ys : List Record)
    (may may' : Record → Record → Bool) :
    resultPreimage (sp.run xs may) = resultPreimage (sp.run ys may') ↔
      unshownResult (sp.run xs may) = unshownResult (sp.run ys may') :=
  Spec.resultPreimage_run_iff (Spec.ofJson_fits h).2.2.2.1 xs ys may may'

/-! ## A result hash is no commitment

A commitment hashes a 16-byte salt and then a payload's canonical bytes
(`commitOf`). If the label were 16 bytes, a record whose salt is the label and
whose payload is the canonical result would commit to the result's own
preimage. The label is 21 bytes, so the preimage's 17th byte is `h`, and no
canonical JSON text starts with `h`. -/

/-- What a canonical JSON text can start with. -/
def jsonHead (c : Char) : Prop :=
  c = 'n' ∨ c = 't' ∨ c = 'f' ∨ c = '"' ∨ c = '[' ∨ c = '{' ∨ c = '-' ∨ c.isDigit

theorem natText_head (m : Nat) : ∃ c cs, (toString m).toList = c :: cs ∧ c.isDigit := by
  rw [Nat.toString_eq_ofList_toDigits, String.toList_ofList]
  obtain ⟨c, cs, e⟩ := List.exists_cons_of_ne_nil (Nat.toDigits_ne_nil (n := m) (b := 10))
  exact ⟨c, cs, e, Nat.isDigit_of_mem_toDigits (by decide) (by decide) (e ▸ mem_cons_self)⟩

theorem natText_take_head (m k : Nat) (hk : 0 < k) (rest : List Char) :
    ∃ c cs, (toString m).toList.take k ++ rest = c :: cs ∧ jsonHead c := by
  obtain ⟨c, cs, e, hc⟩ := natText_head m
  obtain ⟨k', rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
  rw [e]
  exact ⟨c, _, rfl, by simp [jsonHead, hc]⟩

theorem natText_append_head (m : Nat) (rest : List Char) :
    ∃ c cs, (toString m).toList ++ rest = c :: cs ∧ jsonHead c := by
  obtain ⟨c, cs, e, hc⟩ := natText_head m
  rw [e]
  exact ⟨c, _, rfl, by simp [jsonHead, hc]⟩

theorem toNat_pos_of_window {p : Int} (h : (decide (0 < p) && decide (p ≤ 21)) = true) : 0 < p.toNat := by
  simp only [Bool.and_eq_true, decide_eq_true_eq] at h; omega

theorem minus_toList : ("-" : String).toList = ['-'] := rfl

theorem zeroPoint_toList : ("0." : String).toList = ['0', '.'] := rfl

set_option maxHeartbeats 4000000 in
/-- A number's canonical text starts with `-` or a digit, or is `null`. -/
theorem Number.canon_head (n : Number) : ∃ c cs, n.canon.toList = c :: cs ∧ jsonHead c := by
  unfold Number.canon
  split
  · exact ⟨'n', _, rfl, Or.inl rfl⟩
  split
  · exact ⟨'0', _, rfl, by simp [jsonHead]⟩
  simp only [Id.run, pure]
  repeat' split
  all_goals simp only [String.toList_append, String.toList_ofList, String.empty_append, List.append_assoc,
    minus_toList, zeroPoint_toList, List.cons_append, List.nil_append, String.toList_empty]
  all_goals first
    | exact natText_append_head _ _
    | exact natText_take_head _ _ (by decide) _
    | (refine natText_take_head _ _ (toNat_pos_of_window ?_) _; assumption)
    | exact ⟨'-', _, rfl, by simp [jsonHead]⟩
    | exact ⟨'0', _, rfl, by simp [jsonHead]⟩

/-- **No canonical JSON text starts with `h`**: it starts with `n`, `t`, `f`,
a quote, a bracket, a brace, `-` or a digit. -/
theorem Json.canon_head (j : Json) : ∃ c cs, j.canon.toList = c :: cs ∧ jsonHead c := by
  cases j with
  | null => exact ⟨'n', _, rfl, by simp [jsonHead]⟩
  | bool b =>
    cases b
    · exact ⟨'f', _, rfl, by simp [jsonHead]⟩
    · exact ⟨'t', _, rfl, by simp [jsonHead]⟩
  | int i =>
    cases i with
    | ofNat m =>
      simp only [Json.canon]
      show ∃ c cs, (Nat.repr m).toList = c :: cs ∧ jsonHead c
      rw [← Nat.toString_eq_repr]
      simpa using natText_append_head m []
    | negSucc m =>
      simp only [Json.canon]
      show ∃ c cs, ("-" ++ Nat.repr (m + 1)).toList = c :: cs ∧ jsonHead c
      exact ⟨'-', (Nat.repr (m + 1)).toList, by rw [String.toList_append, minus_toList]; rfl,
        by simp [jsonHead]⟩
  | number n => simp only [Json.canon]; exact Number.canon_head n
  | str s => exact ⟨'"', _, by rw [canon_str]; rfl, by simp [jsonHead]⟩
  | arr xs => exact ⟨'[', _, by rw [canon_arr]; rfl, by simp [jsonHead]⟩
  | obj kvs => exact ⟨'{', _, by simp only [Json.canon, String.toList_append]; rfl, by simp [jsonHead]⟩

theorem utf8_eq (s : String) : utf8 s = s.toByteArray.data.toList := by
  simp [utf8, String.toUTF8, ByteArray.toList, toList_loop]

theorem utf8_append (a b : String) : utf8 (a ++ b) = utf8 a ++ utf8 b := by
  rw [utf8_eq, utf8_eq, utf8_eq, String.toByteArray_append, ByteArray.toList_data_append]

theorem resultLabel_split : utf8 resultLabel = utf8 "loops/result-has" ++ utf8 "h/v1:" := by
  rw [← utf8_append]; rfl

theorem utf8_length_16 : (utf8 "loops/result-has").length = 16 := by rw [utf8_eq]; rfl

/-- **A result hash's preimage is no commitment's**: for no 16-byte salt and no
JSON value is it the salt followed by the value's canonical bytes. So no
record commits to the preimage of a result hash, whatever its salt and payload.
The result hash and commitment also have distinct text prefixes, `res:` and `h:`. -/
theorem resultPreimage_ne_commit (r : List (String × SlotValue)) {salt : List UInt8} (hs : salt.length = 16)
    (payload : Json) : resultPreimage r ≠ salt ++ utf8 payload.canon := by
  intro h
  rw [resultPreimage, resultLabel_split, List.append_assoc] at h
  have hd := congrArg (List.drop 16) h
  rw [List.drop_left' utf8_length_16, List.drop_left' hs, resultBytes, ← utf8_append] at hd
  have e := congrArg String.toList (utf8_injective hd)
  obtain ⟨c, cs, hc, hj⟩ := Json.canon_head payload
  rw [hc, String.toList_append] at e
  have : c = 'h' := by
    have := congrArg List.head? e
    simpa using this.symm
  subst this
  simp [jsonHead] at hj

end Loops

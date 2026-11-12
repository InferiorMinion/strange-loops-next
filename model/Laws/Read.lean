import Loops.Read
import Laws.Write

/-!
# Laws of reading an envelope's shape

* **An unreadable field is refused.** An envelope with any field the reader
  could not read, `unreadable` in its place, is refused by `Env.problem`
  (`Env.problem_unreadable`). So an accepted write's envelope holds no
  unreadable field (`Written.no_unreadable`): the empty text can be hashed on
  the way to a refusal, but never reaches an accepted or stored record.
* **The reader puts it there.** `Read.envOf` reads a required field left
  out, or a field of the wrong JSON type, as `unreadable`, so the envelope it
  reads is refused (`Read.envOf_missing`, `Read.envOf_mistyped`,
  `Read.envOf_set_mistyped`).
* **A salt that is not base32hex text is not 16 bytes** (`Read.salt_length`),
  and under a good envelope that is the writer's refusal
  (`writeProblem_salt`).

Which code is reported, and its place in the order, follow from `Env.problem`
checking fields one after another; the vectors pin them.
-/

namespace Loops

/-- An envelope with any unreadable field is refused. -/
theorem Env.problem_unreadable {e : Env}
    (h : e.kind = unreadable ∨ e.observer = unreadable ∨ e.emitted = unreadable ∨
      e.occurred = some unreadable ∨ unreadable ∈ e.about ∨ e.of = some unreadable ∨
      unreadable ∈ e.prev ∨ e.under = some unreadable ∨ unreadable ∈ e.erases) :
    e.problem ≠ none := by
  obtain ⟨hk, ho, he, hs, hr⟩ := unreadable_refused
  intro hn
  unfold Env.problem at hn
  repeat (split at hn; · cases hn)
  rename_i h1 h2 h3 h4 h5 h6 h7 h8 h9 _
  rcases h with h | h | h | h | h | h | h | h | h
  · exact h1 (h ▸ hk)
  · exact h2 (by simp [h, ho])
  · exact h3 (by simp [h, he])
  · exact h4 (by simp [h, hs])
  · have : ¬ e.about.all typedRef = true := fun hall => by
      have := List.all_eq_true.1 hall _ h; simp [ho] at this
    exact h5 (by simpa using this)
  · exact h6 (by simp [h, ho])
  · have : ¬ e.prev.all recordId = true := fun hall => by
      have := List.all_eq_true.1 hall _ h; simp [hr] at this
    exact h7 (by simpa using this)
  · exact h8 (by simp [h, hr])
  · have : ¬ e.erases.all recordId = true := fun hall => by
      have := List.all_eq_true.1 hall _ h; simp [hr] at this
    exact h9 (by simpa using this)

/-- An accepted write's envelope holds no unreadable field. -/
theorem Written.no_unreadable (w : Written) :
    ¬(w.env.kind = unreadable ∨ w.env.observer = unreadable ∨ w.env.emitted = unreadable ∨
      w.env.occurred = some unreadable ∨ unreadable ∈ w.env.about ∨ w.env.of = some unreadable ∨
      unreadable ∈ w.env.prev ∨ w.env.under = some unreadable ∨ unreadable ∈ w.env.erases) :=
  fun h => Env.problem_unreadable h w.ok.env

namespace Read

theorem textOr_unreadable {j : Json} (h : ∀ s, j ≠ .str s) : textOr j = unreadable := by
  cases j <;> simp_all [textOr]

/-- A required field left out is refused. -/
theorem envOf_missing {kvs : List (String × Json)}
    (h : kvs.lookup "kind" = none ∨ kvs.lookup "observer" = none ∨ kvs.lookup "emitted" = none) :
    (envOf kvs).problem ≠ none := by
  apply Env.problem_unreadable
  rcases h with h | h | h <;> simp [envOf, h]

/-- A field that holds one text, given a value of another JSON type, is refused. -/
theorem envOf_mistyped {kvs : List (String × Json)} {k : String} {j : Json}
    (hk : k ∈ ["kind", "observer", "emitted", "occurred", "of", "under"])
    (hj : kvs.lookup k = some j) (h : ∀ s, j ≠ .str s) : (envOf kvs).problem ≠ none := by
  apply Env.problem_unreadable
  have ht := textOr_unreadable h
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl | rfl | rfl | rfl <;> simp [envOf, hj, ht]

/-- A set given a value that is not an array, or an array with a member that
is not text, is refused. -/
theorem envOf_set_mistyped {kvs : List (String × Json)} {k : String} {j : Json}
    (hk : k ∈ ["about", "prev", "erases"]) (hj : kvs.lookup k = some j)
    (h : (∀ xs, j ≠ .arr xs) ∨ ∃ xs x, j = .arr xs ∧ x ∈ xs ∧ ∀ s, x ≠ .str s) :
    (envOf kvs).problem ≠ none := by
  apply Env.problem_unreadable
  have hu : unreadable ∈ textsOr j := by
    rcases h with h | ⟨xs, x, rfl, hx, hs⟩
    · cases j <;> simp_all [textsOr]
    · exact List.mem_map.2 ⟨x, hx, textOr_unreadable hs⟩
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl <;> simp [envOf, hj, hu]

/-- A salt of 16 bytes was written as base32hex text of 16 bytes. -/
theorem salt_length {j : Option Json} (h : (salt j).length = 16) :
    ∃ s bs, j = some (.str s) ∧ Loops.unbase32 s = some bs ∧ bs.length = 16 := by
  match j, h with
  | some (.str s), h =>
    cases hb : Loops.unbase32 s with
    | none => simp [salt, hb] at h
    | some bs => exact ⟨s, bs, rfl, hb, by simpa [salt, hb] using h⟩

end Read

/-- Under a good envelope, a salt that is not 16 bytes is the writer's refusal. -/
theorem writeProblem_salt {env : Env} {salt : List UInt8} {payload : Json}
    (he : env.problem = none) (hs : salt.length ≠ 16) :
    writeProblem env salt payload = some "salt-not-16-bytes" := by
  simp [writeProblem, he, hs]

end Loops

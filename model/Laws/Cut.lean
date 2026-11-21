import Laws.LiveSlice

/-!
# Which parts closed downward a time names

A cut is a part closed downward (`Down`, in `Laws.LiveSlice`). A time names a
*prefix*, the records emitted by then, and a prefix need not be closed
downward: a record can name one with a later `emitted`. Two parts closed
downward stand beside every part.

* **Down** (`downOf_down`, `downOf_least`): a part with everything held under
  it is the least part closed downward that holds it.
* **Held back** (`heldOf_down`, `heldOf_greatest`): the records of a part with
  everything held under them in it are the greatest part closed downward
  inside it.
* **Prefixes** (`prefix_down_iff`): every prefix is closed downward exactly
  when no record names a record with a later `emitted`.
* **Held back is the prefix of another clock** (`heldOf_prefix`): a record is
  held back at a time exactly when it and everything held under it were
  emitted by then.
* **A prefix that is not closed downward reads wrongly**
  (`prefix_reads_wrongly`): it can hold the record that replaced another and
  read the other as live.
-/

namespace Loops

open List

variable {W P U : List Record}

/-! ## Down: a part and what it reaches -/

/-- The records of `W` in `P`, or under a record of `P`. -/
def downOf (W P : List Record) : List Record :=
  W.filter fun a => P.contains a || P.any fun b => (above W a).contains b

theorem mem_downOf {a : Record} : a ∈ downOf W P ↔ a ∈ W ∧ (a ∈ P ∨ ∃ b ∈ P, b ∈ above W a) := by
  simp [downOf]

theorem downOf_holds (hP : ∀ r ∈ P, r ∈ W) : ∀ r ∈ P, r ∈ downOf W P :=
  fun r hr => mem_downOf.2 ⟨hP r hr, .inl hr⟩

/-- **Down is closed downward.** -/
theorem downOf_down (W P : List Record) : Down (downOf W P) W where
  part := fun _ hr => (mem_downOf.1 hr).1
  names := fun y hy x hx hxy => by
    have hya : y ∈ above W x := above_of_names (mem_downOf.1 hy).1 hxy
    rcases (mem_downOf.1 hy).2 with h | ⟨b, hb, hby⟩
    · exact mem_downOf.2 ⟨hx, .inr ⟨y, h, hya⟩⟩
    · exact mem_downOf.2 ⟨hx, .inr ⟨b, hb, above_trans hya hby⟩⟩

/-- **And the least such part that holds `P`.** -/
theorem downOf_least (hU : Down U W) (hPU : ∀ r ∈ P, r ∈ U) : ∀ r ∈ downOf W P, r ∈ U := fun r hr => by
  rcases (mem_downOf.1 hr).2 with h | ⟨b, hb, hbr⟩
  · exact hPU r h
  · exact hU.below (mem_downOf.1 hr).1 (hPU b hb) hbr

/-! ## Held back: the records whose whole ancestry is in the part -/

/-- The records of `P` with every record of `W` under them in `P`. -/
def heldOf (W P : List Record) : List Record :=
  P.filter fun a => W.all fun c => !(above W c).contains a || P.contains c

theorem mem_heldOf {a : Record} : a ∈ heldOf W P ↔ a ∈ P ∧ ∀ c ∈ W, a ∈ above W c → c ∈ P := by
  simp only [heldOf, mem_filter, all_eq_true, Bool.or_eq_true, Bool.not_eq_true', contains_iff_mem]
  refine and_congr_right fun _ => forall_congr' fun c => forall_congr' fun _ => ?_
  by_cases h : a ∈ above W c <;> simp [h]

/-- **Held back is closed downward.** -/
theorem heldOf_down (hP : ∀ r ∈ P, r ∈ W) : Down (heldOf W P) W where
  part := fun r hr => hP r (mem_heldOf.1 hr).1
  names := fun y hy x hx hxy => by
    obtain ⟨hyP, hall⟩ := mem_heldOf.1 hy
    have hya : y ∈ above W x := above_of_names (hP y hyP) hxy
    exact mem_heldOf.2 ⟨hall x hx hya, fun c hc hxc => hall c hc (above_trans hxc hya)⟩

/-- **And the greatest such part inside `P`.** -/
theorem heldOf_greatest (hU : Down U W) (hUP : ∀ r ∈ U, r ∈ P) : ∀ r ∈ U, r ∈ heldOf W P :=
  fun r hr => mem_heldOf.2 ⟨hUP r hr, fun c hc hrc => hUP c (hU.below hc hr hrc)⟩

/-! ## Prefixes -/

/-- The prefix at `T`: the records emitted by `T`. -/
def upTo (W : List Record) (T : Moment) : List Record :=
  W.filter fun a => decide (a.emitted.moment ≤ T)

theorem mem_upTo {T : Moment} {a : Record} : a ∈ upTo W T ↔ a ∈ W ∧ a.emitted.moment ≤ T := by
  simp [upTo]

/-- No record names a record with a later `emitted`. -/
def NamesEarlier (W : List Record) : Prop :=
  ∀ y ∈ W, ∀ x ∈ W, x.id ∈ y.prev → x.emitted.moment ≤ y.emitted.moment

/-- **Every prefix is closed downward exactly when no record names a later one.** -/
theorem prefix_down_iff : (∀ T, Down (upTo W T) W) ↔ NamesEarlier W := by
  constructor
  · intro h y hy x hx hxy
    exact (mem_upTo.1 ((h y.emitted.moment).names y (mem_upTo.2 ⟨hy, List.le_refl _⟩) x hx hxy)).2
  · intro h T
    exact ⟨fun _ hr => (mem_upTo.1 hr).1, fun y hy x hx hxy =>
      mem_upTo.2 ⟨hx, List.le_trans (h y (mem_upTo.1 hy).1 x hx hxy) (mem_upTo.1 hy).2⟩⟩

/-- **Held back is the prefix of another clock**: a record's own `emitted` or
the latest under it, whichever is later. It is what a writer that never emits
earlier than what it names would have written. -/
theorem heldOf_prefix {T : Moment} {a : Record} :
    a ∈ heldOf W (upTo W T) ↔
      a ∈ W ∧ a.emitted.moment ≤ T ∧ ∀ c ∈ W, a ∈ above W c → c.emitted.moment ≤ T := by
  rw [mem_heldOf, mem_upTo, and_assoc]
  exact and_congr_right fun _ => and_congr_right fun _ =>
    ⟨fun h c hc hac => (mem_upTo.1 (h c hc hac)).2, fun h c hc hac => mem_upTo.2 ⟨hc, h c hc hac⟩⟩

/-! **A prefix that is not closed downward reads wrongly.** `a ← b ← c`, where
`a` and `c` set `status` and the writer of `b` had a clock five seconds fast.
The prefix at one second is `a` and `c`. It holds `c`, which replaced `a`, and
reads `a` as live for `status`: the path from `c` down to `a` goes through `b`.
Down at that time is all three, and reads as the whole does. -/

def sk.a : Record :=
  { id := 1, kind := "task", observer := "sl:x", of := some "k1", prev := [],
    emitted := ⟨"1970-01-01T00:00:00Z", [0], by decide⟩, sets := some ["status"] }
def sk.b : Record :=
  { id := 2, kind := "task", observer := "sl:x", of := some "k1", prev := [1],
    emitted := ⟨"1970-01-01T00:00:05Z", [5], by decide⟩, sets := some ["title"] }
def sk.c : Record :=
  { id := 3, kind := "task", observer := "sl:x", of := some "k1", prev := [2],
    emitted := ⟨"1970-01-01T00:00:01Z", [1], by decide⟩, sets := some ["status"] }
def sk.all : List Record := [sk.a, sk.b, sk.c]

theorem prefix_reads_wrongly :
    upTo sk.all [1] = [sk.a, sk.c] ∧ ¬Down (upTo sk.all [1]) sk.all ∧
      liveFor (upTo sk.all [1]) "status" sk.a = true ∧ liveFor sk.all "status" sk.a = false ∧
      downOf sk.all (upTo sk.all [1]) = sk.all := by
  refine ⟨by decide +kernel, fun h => ?_, by decide +kernel, by decide +kernel, by decide +kernel⟩
  exact absurd (h.names sk.c (by decide +kernel) sk.b (by decide +kernel) (by decide +kernel)) (by decide +kernel)

end Loops

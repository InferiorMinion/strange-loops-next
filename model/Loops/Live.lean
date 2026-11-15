import Loops.Fold

/-!
# Live setters: a thing read by its keys

A record's `sets` says which payload keys it sets. A record without `sets` is
read whole and sets every key; one with `"sets": []` sets none and is a pure
join. Every record sets the envelope, whatever its `sets`.

* **Above.** A record is above another over `prev` when it names it, or names
  a record above it, through the records of their thing the reader holds and
  admitted. A record the reader does not hold breaks the chain: a reader
  cannot see past it.
* **Live for a key.** A setter of `k` is live for `k` when no record above it
  sets `k`. Live for the envelope is a head: nothing above it at all.
* **Live.** A record is live when it is live for some key, the envelope among
  them. So every head is live, a pure join too, and a whole-read emit is live
  until a whole-read emit is above it.

The keyed state of a thing lists, for each key some live record declares, the
key's live setters, whole-read ones included; the live whole-read emits; and
the thing's heads. Each list is newest first. It is worked out where heads are,
over the admitted records of the thing, before any slot selects.
-/

namespace Loops

/-! ## Keys a record sets -/

/-- A record is read whole when it has no `sets`. -/
def Record.whole (r : Record) : Bool := r.sets.isNone

/-- The keys a record declares: its `sets`, none when it is read whole. -/
def Record.declares (r : Record) : List String := r.sets.getD []

/-- A record sets key `k` when it declares `k`, or when it is read whole. -/
def Record.setsKey (r : Record) (k : String) : Bool :=
  match r.sets with
  | none => true
  | some ks => ks.contains k

/-! ## Above, over `prev` -/

/-- `q` names an id of `ids` in its `prev`. -/
def namesIn (ids : List RecordId) (q : Record) : Bool := q.prev.any ids.contains

/-- `ids`, with the id of every record of `rs` that names an id it holds added,
one at a time, until no record of `rs` names one it lacks. -/
def upward (rs : List Record) (ids : List RecordId) : List RecordId :=
  match _hf : rs.find? (fun q => !ids.contains q.id && namesIn ids q) with
  | none => ids
  | some q => upward rs (q.id :: ids)
termination_by (rs.filter fun q => !ids.contains q.id).length
decreasing_by
  have hq := List.mem_of_find?_eq_some _hf
  have hp := List.find?_some _hf
  simp only [Bool.and_eq_true, Bool.not_eq_true'] at hp
  have e : (rs.filter fun x => !(q.id :: ids).contains x.id) =
      (rs.filter fun x => !ids.contains x.id).filter fun x => x.id != q.id := by
    rw [List.filter_filter]
    congr 1; funext x
    by_cases hx : x.id = q.id <;> simp [hx]
  show (List.filter (fun x => !(q.id :: ids).contains x.id) rs).length <
    (rs.filter fun q => !ids.contains q.id).length
  rw [e]
  apply List.length_filter_lt_length_iff_exists.2
  exact ⟨q, List.mem_filter.2 ⟨hq, by simpa using hp.1⟩, by simp⟩

/-- The records of `rs` above `r` over `prev`. -/
def above (rs : List Record) (r : Record) : List Record := rs.filter (namesIn (upward rs [r.id]))

/-! ## Liveness -/

/-- A setter of `k` is live for `k` when no record above it sets `k`. -/
def liveFor (rs : List Record) (k : String) (r : Record) : Bool :=
  r.setsKey k && !(above rs r).any (·.setsKey k)

/-- Live for the envelope, which every record sets: nothing is above it. It is a head. -/
def liveEnv (rs : List Record) (r : Record) : Bool := (above rs r).isEmpty

/-- Live for some key, the envelope among them. A whole-read emit sets every
key, so it is live for some key exactly when no whole-read emit is above it;
a record with `sets`, when it is a head or live for a key it declares. -/
def live (rs : List Record) (r : Record) : Bool :=
  liveEnv rs r ||
    match r.sets with
    | none => !(above rs r).any Record.whole
    | some ks => ks.any fun k => liveFor rs k r

/-! ## A thing's keyed state -/

/-- What a slot that follows `keys` reads of a thing: for each key some live
record declares, that key's live setters, whole-read ones included; the live
whole-read emits; and the heads. Each list is newest first. -/
structure Keyed where
  keys : List (String × List Record)
  whole : List Record
  heads : List Record
  deriving Repr, DecidableEq, Inhabited

/-- The keys some live record of `rs` declares, in canonical order. -/
def liveKeys (rs : List Record) : List String :=
  canonSet ((rs.filter (live rs)).flatMap Record.declares)

/-- Thing `t`'s keyed state over the records a reader holds: liveness over the
records of `t`, beside `t`'s heads as `stateIn` works them out. -/
def keyedIn (t : String) (xs : List Record) : Keyed :=
  let rs := xs.filter (·.isOf t)
  { keys := (liveKeys rs).map fun k => (k, orderHeads (rs.filter (liveFor rs k)))
    whole := orderHeads (rs.filter fun r => r.whole && live rs r)
    heads := (stateIn t xs).heads }

end Loops

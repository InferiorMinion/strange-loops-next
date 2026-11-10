import Loops.Json
import Loops.Base32
import Loops.Occurred

/-!
# Envelope

What the core reads about a record. `about` and `prev` are sets: their
canonical form is sorted by UTF-16 code units with repeats removed, and an
empty set is left out, so the same set always gives the same bytes. `erases`
is a set too: the records whose payloads this one erases, whatever its kind.

`of` names the one thing a record is an emit of: a typed reference, a record
id or a natural key. It is optional and, like `occurred` and `under`, left out
when absent, so a record without it keeps its bytes. Only a root leaves it
out: a record with `prev` states `of` (`of-missing`). `about` names the things
a record mentions; it never says what the record is an emit of.
-/

namespace Loops

structure Env where
  kind     : String
  observer : String
  emitted  : String
  occurred : Option String := none
  about    : List String := []
  of       : Option String := none
  prev     : List String := []
  under    : Option String := none
  erases   : List String := []
  deriving Repr, Inhabited

def Env.toJson (e : Env) : Json :=
  let opt (k : String) : Option String → List (String × Json)
    | some v => [(k, .str v)]
    | none => []
  let set (k : String) (xs : List String) : List (String × Json) :=
    match canonSet xs with
    | [] => []
    | ys => [(k, .arr (ys.map .str))]
  .obj ([("kind", .str e.kind), ("observer", .str e.observer), ("emitted", .str e.emitted)] ++
    opt "occurred" e.occurred ++ set "about" e.about ++ opt "of" e.of ++ set "prev" e.prev ++ opt "under" e.under ++
    set "erases" e.erases)

/-! ## Validation -/

def isLowerAlnum (c : Char) : Bool := c.isLower || c.isDigit

/-- `prefix:value`, the prefix lower-case letters, digits and hyphens, starting with a letter. -/
def typedRef (s : String) : Bool :=
  match s.toList.span (· != ':') with
  | (p :: ps, ':' :: _ :: _) => p.isLower && ps.all (fun c => isLowerAlnum c || c == '-')
  | _ => false

/-- A record id: `r:` and canonical base32hex for exactly 32 bytes. -/
def recordId (s : String) : Bool :=
  match s.toList with
  | 'r' :: ':' :: rest => byteText 32 (String.ofList rest)
  | _ => false

/-- Check raw envelope keys before decoding values. Typed `Env` has no unknown fields. -/
def envelopeKeysProblem (kvs : List (String × Json)) : Option String :=
  if kvs.all (fun (k, _) => ["kind", "observer", "emitted", "occurred", "about", "of", "prev", "under", "erases"].contains k)
  then none else some "envelope-field-unknown"

/-- Check every raw envelope before decoding any envelope value.
Malformed shells stay for the decoder; an unknown key in any object wins. -/
def envelopesKeysProblem (raw : List Json) : Option String :=
  envelopeKeysProblem (raw.flatMap fun j => match j with
    | .obj kvs => kvs
    | _ => [])

/-- Inspect only each draft's envelope keys, not its label, salt or values. -/
def draftsKeysProblem (raw : List Json) : Option String :=
  envelopesKeysProblem (raw.flatMap fun j => match j with
    | .obj kvs => kvs.filterMap fun (k, v) => if k == "env" then some v else none
    | _ => [])

/-- Why a typed envelope is refused, if it is. -/
def Env.problem (e : Env) : Option String :=
  if e.kind.isEmpty then some "kind-empty"
  else if !typedRef e.observer then some "observer-not-typed"
  else if !validEmitted e.emitted then some "emitted-not-rfc3339"
  else if !(e.occurred.map fun s => (occurredSpan? s).isSome).getD true then some "occurred-not-span"
  else if !e.about.all typedRef then some "about-not-typed"
  else if !(e.of.map typedRef).getD true then some "of-not-typed"
  else if !e.prev.all recordId then some "prev-not-record-id"
  else if !(e.under.map recordId).getD true then some "under-not-record-id"
  else if !e.erases.all recordId then some "erases-not-record-id"
  else if !e.prev.isEmpty && e.of.isNone then some "of-missing"
  else none

end Loops

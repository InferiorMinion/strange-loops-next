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

/-- Check every raw envelope's keys before decoding any envelope value.
An unknown key in any object wins, whatever the other envelopes are. -/
def envelopesKeysProblem (raw : List Json) : Option String :=
  envelopeKeysProblem (raw.flatMap fun j => match j with
    | .obj kvs => kvs
    | _ => [])

/-- The codes of raw shape, in the order they are checked. -/
def shapeCodes : List String := ["envelope-field-unknown", "envelope-not-object"]

def Json.isObj : Json → Bool
  | .obj _ => true
  | _ => false

/-- Raw shape across every envelope: an unknown key in any object first, then
any envelope that is not an object (`envelope-not-object`). Both come before
any envelope value, label or salt is decoded. -/
def envelopesShapeProblem (raw : List Json) : Option String :=
  match envelopesKeysProblem raw with
  | some why => some why
  | none => if raw.all Json.isObj then none else some "envelope-not-object"

/-- Each draft's envelopes, every occurrence of its `env` member. A draft object
without one has an envelope that is not an object, which `null` stands for here.
A draft that is not an object has none: it is malformed input, not a shape refusal. -/
def draftEnvs (raw : List Json) : List Json :=
  raw.flatMap fun j => match j with
    | .obj kvs => match kvs.filterMap fun (k, v) => if k == "env" then some v else none with
      | [] => [.null]
      | envs => envs
    | _ => []

/-- Inspect only each draft's envelope shape, not its label, salt or values. -/
def draftsShapeProblem (raw : List Json) : Option String :=
  envelopesShapeProblem (draftEnvs raw)

/-- What the reader reads for a field it can't read as its JSON type: a
required field left out, a field holding the wrong JSON type, or a member of a
set that is not text. It is empty text, which the field's own check refuses
(`unreadable_refused`), so the refusal is the field's own code, in the field's
place in the order. It can be hashed on the way to a refusal: resolving a
batch's labels computes a labelled draft's id before writing checks it, and a
record input compares a later variant's id with the first's. It never reaches
an accepted or stored record (`Written.no_unreadable`). -/
def unreadable : String := ""

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

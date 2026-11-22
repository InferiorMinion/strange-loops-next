import Loops.Tick

namespace Loops

def coveredForms (covers : List RecordId) (forms : List Record) :=
  forms.filter fun r => covers.contains r.id

def tickMask (withheld : List RecordId) (r : Record) :=
  if withheld.contains r.id then r.withhold else r

def tickRun (sp : Spec) (covers withheld : List RecordId) (forms : List Record) :=
  sp.run (settle ((coveredForms covers forms).map (tickMask withheld)))

/-- Local access uses the whole holding, including erasers outside the claim. -/
def tickOpen (forms : List Record) (i : RecordId) :=
  (forms.any fun r => r.id == i && r.shown == .here) && !erasedIn forms i

def makeTick (sp : Spec) (forms : List Record) : TickPayload :=
  let covers := idsOf forms
  let withheld := covers.filter fun i => !tickOpen forms i
  ⟨Rules.current, sp, covers, withheld, tickRun sp covers withheld forms⟩

structure TickSubject where
  kind : String
  payload : Option Json

structure MissingInputs where
  records : Nat
  openings : Nat
  deriving Repr, DecidableEq

inductive ReplayIssue where
  | ignoredErasure
  | missing (counts : MissingInputs)
  deriving Repr, DecidableEq

inductive TodayReport where
  | same | another | unreadableSpec | noncanonicalSpec
  | unavailable (why : ReplayIssue)
  deriving Repr, DecidableEq

inductive TickAnswer where
  | notTick (fault : Refusal)
  | payloadUnheld | unreadableSpec | newer
  | older (today : TodayReport)
  | missing (counts : MissingInputs)
  | same | differs (claimed computed : String)
  deriving Repr, DecidableEq

inductive CoverClosure where
  | yes | no | unknown
  deriving Repr, DecidableEq

structure TickCheck where
  answer : TickAnswer
  closure : CoverClosure := .unknown
  takenOutside : Option Nat := none
  deriving Repr, DecidableEq

def tickClosure (covers : List RecordId) (forms : List Record) : CoverClosure :=
  if (coveredForms covers forms).any (fun r =>
      !(r.prev.all covers.contains && r.under.all covers.contains)) then .no
  else if covers.all (idsOf forms).contains then .yes else .unknown

def takenOutside (sp : Spec) (covers : List RecordId) (forms : List Record) : Nat :=
  ((realm (fun _ _ => true) forms).filter fun r => !covers.contains r.id && sp.takes r).length

def tickMissing (covers withheld : List RecordId) (forms : List Record) : MissingInputs :=
  let held := idsOf forms
  ⟨(covers.filter fun i => !held.contains i).length,
   (covers.filter fun i => held.contains i && !withheld.contains i && !tickOpen forms i).length⟩

def replayIssue (covers withheld : List RecordId) (forms : List Record) : Option ReplayIssue :=
  if (coveredForms covers forms).any (fun r => r.erases.any (fun i => covers.contains i && !withheld.contains i))
    then some .ignoredErasure
  else
    let missing := tickMissing covers withheld forms
    if missing == ⟨0, 0⟩ then none else some (.missing missing)

inductive TickComparison where
  | same | differs (claimed computed : String)
  deriving Repr, DecidableEq

/-- Both diagnostics and verdicts compare actual bytes before computing any hash details. -/
def replayTick (H : Hash) (m : TickMembers) (sp : Spec) (forms : List Record) :
    Except ReplayIssue TickComparison :=
  match replayIssue m.covers m.withheld forms with
  | some why => .error why
  | none =>
    let computed := tickRun sp m.covers m.withheld forms
    if utf8 m.result.canon == resultBytes computed then .ok .same
    else .ok (.differs (resultHashOf H m.result) (resultHash H computed))

def checkMembers (H : Hash) (current : Rules) (m : TickMembers) (forms : List Record) : TickCheck := Id.run do
  let closure := tickClosure m.covers forms
  if current.number < m.rules.number then return ⟨.newer, closure, none⟩
  let older := m.rules.number < current.number
  let sp ← match m.readSpec with
    | .ok sp => pure sp
    | .error (.invalid f) => return ⟨if older then .older .noncanonicalSpec else .notTick f, closure, none⟩
    | .error _ => return ⟨if older then .older .unreadableSpec else .unreadableSpec, closure, none⟩
  let taken := some (takenOutside sp m.covers forms)
  if older then
    let today := match replayTick H m sp forms with
      | .error why => TodayReport.unavailable why
      | .ok .same => .same
      | .ok (.differs ..) => .another
    return ⟨.older today, closure, taken⟩
  match m.readSlots sp with
  | .error f => return ⟨.notTick f, closure, taken⟩
  | .ok _ =>
    let answer := match replayTick H m sp forms with
      | .error .ignoredErasure => TickAnswer.notTick ⟨"tick-withheld-erased", "withheld"⟩
      | .error (.missing counts) => .missing counts
      | .ok .same => .same
      | .ok (.differs claimed computed) => .differs claimed computed
    return ⟨answer, closure, taken⟩

def checkTick (H : Hash) (current : Rules) (subject : TickSubject) (forms : List Record) : TickCheck :=
  if subject.kind != "tick" then { answer := .notTick ⟨"tick-kind", "kind"⟩ }
  else match subject.payload with
    | none => { answer := .payloadUnheld }
    | some j => match readTickMembers H j with
      | .error f => { answer := .notTick f }
      | .ok m => checkMembers H current m forms

end Loops

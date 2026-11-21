import Gen.Scenario
import Gen.Spec
import Gen.Realm

namespace Gen.RulingVectors

open Loops Loops.Lens
open Gen.ScenarioVectors (jShownIds jSlot rotations)

private def lines (xs : List Line) : J := .arr (xs.map fun x => .str (String.ofList x))
private def sections (xs : List Section) : J := .arr (xs.map fun x =>
  .obj [("id", .str (String.ofList x.id)), ("body", lines x.body)])
private def outcomeJ : Outcome → J
  | .accepted i b => .obj [("accepted", .str (String.ofList i)), ("body", lines b)]
  | .unknown i => .obj [("unknown", .str (String.ofList i))]
  | .absent i => .obj [("absent", .str (String.ofList i))]

private def scenario (name thing : String) (spj : Json) (pool : List Record)
    (steps : List (String × List Record)) (base : List Section) (text : List Line)
    (laws : List String := []) : Except String J := do
  let sp ← match Spec.ofJson spj with
    | .ok sp => pure sp
    | .error r => throw s!"{r.code}: {r.path}"
  let some os := edit base text | throw "lens edit does not parse"
  let result (forms : List Record) : J :=
    let settled := settle forms
    let admitted := admit (fun _ _ => true) settled
    let state := stateIn thing admitted
    .obj [("settled", jShownIds settled),
      ("admitted", .arr (admitted.map (.str ∘ idText ∘ (·.id)))),
      ("heads", .arr (state.heads.map (.str ∘ idText ∘ (·.id)))),
      ("winner", state.winner.map (fun r => J.str (idText r.id)) |>.getD .null),
      ("slots", .obj ((sp.run settled).map fun (n, v) => (n, jSlot v)))]
  let mut results : List J := []
  for (_, forms) in steps do
    let expected := result forms
    if !(rotations forms).all (fun rs => (result rs).render 0 == expected.render 0) then
      throw s!"scenario/{name}: arrival orders disagree"
    results := results ++ [expected]
  let input : J := .obj [
    ("spec", Gen.Records.ofJson spj), ("thing", .str thing),
    ("records", .arr (pool.map jRecord)),
    ("steps", .arr (steps.map fun (label, forms) => .obj [
      ("name", .str label), ("forms", jShownIds forms),
      ("orders", .arr ((rotations forms).map fun rs => .arr (rs.map (.str ∘ idText ∘ (·.id)))))])),
    ("lens", .obj [("base", sections base), ("text", lines text)])]
  let expected : J := .obj [("steps", .arr results),
    ("lens", .obj [("rendered", lines (render base)), ("outcomes", .arr (os.map outcomeJ)),
      ("applied", sections (apply base os))])]
  pure (vector name "scenario" laws input expected)

private def countSpec : Json := Gen.SpecVectors.fromOf (.obj [("store", .str ".")])
private def r (id : RecordId) (thing : String) (t : Nat) (value : String) (prev : List RecordId := [])
    (ok : Reads t := by decide) : Record :=
  { Gen.RealmVectors.r id "note" (some thing) t prev (ok := ok) with fields := [("value", .one (.str value))] }

private def admissionCase : Except String J := do
  let a := r (fixtureId 52) "thing:A" 100 "base"
  let b := r (fixtureId 53) "thing:B" 200 "refused" [fixtureId 52]
  let c := r (fixtureId 0) "thing:A" 300 "also refused" [fixtureId 53]
  let base : List Section := [⟨"thing:A".toList, ["base".toList]⟩]
  scenario "gap01-gap06-admission" "thing:A" countSpec [a, b, c]
    [("before", [a]), ("after", [a, b, c])] base (render base)

private def linesCase : Except String J := do
  let a := r (fixtureId 0x91) "thing:A" 100 "one\r\ntwo\n"
  let b := r (fixtureId 0x92) "thing:A" 200 "\nnext\n" [fixtureId 0x91]
  let body (value : String) : List Line := (value.splitOn "\n").map String.toList
  let base : List Section := [⟨"thing:A".toList, body (← fixtureText a)⟩]
  let changed : List Section := [⟨"thing:A".toList, body (← fixtureText b)⟩]
  scenario "gap11-line-adapter" "thing:A" countSpec [a, b]
    [("before", [a]), ("after", [a, b])] base (render changed)

-- `by` settles once, over everything held: Y, in project y, erases X, in
-- project x. Settling within each group would leave X's payload readable in x,
-- where its eraser is not.
private def byErasureCase : Except String J := do
  let x : Record := { Gen.RealmVectors.r (fixtureId 0x3a) "task" (some "task:1") 100 (about := ["project:x"]) with
    fields := [("value", .one (.str "five")), ("n", .one (.int 5))] }
  let y : Record := { Gen.RealmVectors.r (fixtureId 0x3b) "task" (some "task:2") 200 (about := ["project:y"]) with
    fields := [("value", .one (.str "one")), ("n", .one (.int 1))], erases := [fixtureId 0x3a] }
  let spj : Json := .obj [("vertex", .str "per-project"), ("from", .arr [.obj [("store", .str ".")]]),
    ("fold", .obj [
      ("sum", .obj [("sum", .str "n"), ("by", .str "env.about:project")]),
      ("current", .obj [("sum", .str "n"), ("follow", .str "prev"), ("by", .str "env.about:project")]),
      ("count", .obj [("count", .bool true), ("by", .str "env.about:project")]),
      ("plain", .obj [("sum", .str "n")])])]
  let base : List Section := [⟨"task:1".toList, [(← fixtureText x).toList]⟩]
  scenario "by-01-erased-in-another-group" "task:1" spj [x, y]
    [("before", [x]), ("after", [x, y]), ("withheld-x", [x.withhold, y])] base (render base)
    ["spec.set", "by.after-admission", "by.accounting", "result.unread"]

-- Latest-by following `keys` reads what settling keeps. W is read whole; B
-- sets `status` from it. A place holding B withheld, or a bare erased copy of
-- B with no eraser, leaves B withheld; E erases it. In each, B keeps its
-- envelope and `sets`: it is still live for `status`, which is unknown, while
-- `title` is still read from W.
private def keyedSettleCase : Except String J := do
  let w : Record := { Gen.RealmVectors.r (fixtureId 0x1a1) "task" (some "task:1") 100 with
    fields := [("value", .one (.str "write the note")), ("status", .one (.str "open")), ("title", .one (.str "note"))] }
  let b : Record := { Gen.RealmVectors.r (fixtureId 0x1a2) "task" (some "task:1") 200 [fixtureId 0x1a1] with
    sets := some ["status"], fields := [("status", .one (.str "closed")), ("value", .one (.str ""))] }
  let e : Record := { Gen.RealmVectors.r (fixtureId 0x1a3) "erasure" none 300 with erases := [fixtureId 0x1a2] }
  let spj : Json := .obj [("vertex", .str "tasks"), ("from", .arr [.obj [("store", .str "."), ("kind", .arr [.str "task"])]]),
    ("fold", .obj [
      ("state", .obj [("latest-by", .str "of"), ("follow", .str "keys")]),
      ("closed", .obj [("latest-by", .str "of"), ("follow", .str "keys"),
        ("where", .obj [("status", .obj [("is", .str "closed")])])]),
      ("note", .obj [("latest-by", .str "of"), ("follow", .str "keys"),
        ("where", .obj [("title", .obj [("is", .str "note")])])])])]
  -- the lens reads the head, B, before and after the last step
  let base : List Section := [⟨"task:1".toList, [(← fixtureText b).toList]⟩]
  scenario "keys-01-settled-setter" "task:1" spj [w, b, e]
    [("before", [w]), ("set", [w, b]), ("withheld", [w, b.withhold]), ("erased-alone", [w, b.erase]),
      ("erased", [w, b, e])] base (render base)
    ["spec.set", "keyed.live", "keyed.fold", "keyed.select", "settle.erased", "result.unread"]

def vectors : Except String (List (String × J)) := do
  return [("gap01-gap06-admission", ← admissionCase), ("gap11-line-adapter", ← linesCase),
    ("by-01-erased-in-another-group", ← byErasureCase), ("keys-01-settled-setter", ← keyedSettleCase)]

end Gen.RulingVectors

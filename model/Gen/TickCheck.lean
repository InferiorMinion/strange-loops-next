import Gen.TickPayload
import Loops.TickCheck

namespace Gen.TickCheckVectors
open Loops

private def missingFields (m : MissingInputs) : List (String × J) :=
  [("missing-records", .num m.records), ("missing-openings", .num m.openings)]

private def todayJson : TodayReport → J
  | .same => .str "the same result"
  | .another => .str "another result"
  | .unreadableSpec => .obj [("not-available", .str "this binary cannot read the spec")]
  | .noncanonicalSpec => .obj [("not-available", .str "spec is not in today's parsed form")]
  | .unavailable .ignoredErasure => .obj [("not-available", .str "tick-withheld-erased")]
  | .unavailable (.missing m) => .obj ([("not-available", .str "missing inputs")] ++ missingFields m)

private def checkJson (c : TickCheck) : J :=
  let fields := match c.answer with
    | .notTick f => [("answer", J.str "not a tick"), ("refused", .str f.code), ("path", .str f.path)]
    | .payloadUnheld => [("answer", .str "cannot check"), ("reason", .str "the tick's payload is not held")]
    | .unreadableSpec => [("answer", .str "cannot check"), ("reason", .str "this binary cannot read the spec")]
    | .newer => [("answer", .str "cannot check"), ("reason", .str "made under newer rules")]
    | .older today => [("answer", .str "cannot check"), ("reason", .str "made under older rules"),
        ("today", todayJson today)]
    | .missing m => [("answer", .str "cannot check"), ("reason", .str "missing inputs")] ++ missingFields m
    | .same => [("answer", .str "same")]
    | .differs claimed computed => [("answer", .str "differs"), ("claimed-hash", .str claimed),
        ("computed-hash", .str computed)]
  let closure := match c.closure with | .yes => "yes" | .no => "no" | .unknown => "unknown"
  .obj (fields ++ [("closure", .str closure), ("taken-not-covered", c.takenOutside.elim .null J.num)])

/-- An opening in a fixture is projected through the ordinary stored-payload reader. -/
private def readForm (path : String) (j : Json) : Except String Record := do
  let kvs ← Read.object path j
  if (kvs.lookup "opening").isNone then return ← Read.record path j
  let parts ← Read.members? path ["record", "opening"] j
  let r ← Read.record (Read.dot path "record") (← Read.req path parts "record")
  let payload ← Read.req path parts "opening"
  return (make {Records.note with kind := r.kind} (List.replicate 16 0) payload).readInto r

private def forms (path : String) (j : Json) := Read.each path readForm j

def answer (input : Json) : Except String J := do
  let kvs ← Read.object "input" input
  let op ← Read.text "input.op" (← Read.req "input" kvs "op")
  match op with
  | "make-and-check" =>
    let kvs ← Read.members? "input" ["op", "spec", "writer", "checker", "fresh"] input
    let sp ← (Spec.ofJson (← Read.req "input" kvs "spec")).mapError fun f => s!"{f.code} at {f.path}"
    let xs ← forms "input.writer" (← Read.req "input" kvs "writer")
    let ys := (← Read.opt kvs "checker" (forms "input.checker")).getD xs
    let fresh ← Read.opt kvs "fresh" (fun j => match j with | .bool b => pure b | _ => throw "input.fresh: expected boolean")
    let p := (makeTick sp xs).toJson sha256
    let checked := checkJson (checkTick sha256 Rules.current ⟨"tick", some p⟩ ys)
    return .obj ([("payload", Records.ofJson p), ("check", checked)] ++
      if fresh.getD false then [("fresh", Records.ofJson ((makeTick sp ys).toJson sha256))] else [])
  | "check-tick" =>
    let kvs ← Read.members? "input" ["op", "current", "subject", "checker"] input
    let some current := Rules.ofJson? (← Read.req "input" kvs "current") | throw "input.current: invalid version"
    let subject ← Read.members? "input.subject" ["kind", "payload"] (← Read.req "input" kvs "subject")
    let kind ← Read.text "input.subject.kind" (← Read.req "input.subject" subject "kind")
    let ys ← forms "input.checker" (← Read.req "input" kvs "checker")
    return checkJson (checkTick sha256 current ⟨kind, subject.lookup "payload"⟩ ys)
  | _ => TickPayloadVectors.answer input

private def folded (slots : List (String × Json)) : Json :=
  .obj [("vertex", .str "backups"), ("from", .arr [.obj [("store", .str "lab"), ("kind", .arr [.str "backup"])]]),
    ("fold", .obj slots)]

private def parse (j : Json) : Except String Spec :=
  (Spec.ofJson j).mapError fun f => s!"{f.code} at {f.path}"

private def makeInput (sp : Spec) (writer : List J) (checker : Option (List J)) (fresh : Bool) : J :=
  .obj ([("op", .str "make-and-check"), ("spec", Records.ofJson sp.toJson), ("writer", .arr writer)] ++
    checker.toList.map (fun fs => ("checker", .arr fs)) ++ if fresh then [("fresh", .bool true)] else [])

private def checkInput (subject : TickSubject) (ys : List Record) (current : Rules) : J :=
  .obj [("op", .str "check-tick"), ("current", .str current.text),
    ("subject", .obj ([("kind", .str subject.kind)] ++ subject.payload.toList.map fun j => ("payload", Records.ofJson j))),
    ("checker", .arr (ys.map jRecord))]

private def cases : Except String (List (String × List String × J)) := do
  let sp ← parse Tick.backups
  let xs := TickPayloadVectors.backups
  let first := xs.head!
  let follow ← parse (folded [("runs", .obj [("count", .bool true), ("follow", .str "prev")]),
    ("mb", .obj [("sum", .str "mb"), ("follow", .str "prev")])])
  let count ← parse (folded [("runs", .obj [("count", .bool true)])])
  let minSpec ← parse (folded [("minimum", .obj [("min", .str "n")])])
  let seventh := {first with
    id := 7, fields := [("mb", .one (.int 4)), ("svc", .one (.str "pihole"))],
    occurred := some (claim "2026-11-17"), about := ["svc:pihole"]}
  let retired := {seventh with kind := "retired", of := some (idText first.id), prev := [first.id]}
  let eraser := {seventh with kind := "retention", erases := [first.id]}
  let xsHidden := first.withhold :: xs.tail!
  let xsErased := first.erase :: xs.tail!
  let p := makeTick sp xs
  let marked := makeTick sp (xs ++ [eraser])
  let omitted := {marked with withheld := []}
  let wrong := {p with result := p.result.map fun (k,v) => (k, match v with
    | SlotValue.int n => SlotValue.int {n with value := n.value + 1} | other => other)}
  let shorter := {p with result := p.result.map fun (k,v) => (k, match v with
    | SlotValue.heads ids u => SlotValue.heads (ids.take 5) u | other => other)}
  let two : Rules := ⟨2, by decide⟩
  let raw := fun (v : Rules) (s r : Json) => (TickMembers.mk v s p.covers p.withheld r).toJson sha256
  let future := folded [("next", .obj [("future", .bool true)])]
  let made := fun (name : String) (laws : List String) (spec : Spec) (writer : List Record)
      (checker : Option (List Record)) (fresh : Bool) =>
    (name, laws, makeInput spec (writer.map jRecord) (checker.map (·.map jRecord)) fresh)
  let check := fun name laws (v : Rules) (j : Json) (ys : List Record) =>
    (name, laws, checkInput ⟨"tick", some j⟩ ys v)
  let now := Rules.current
  let ordinary := ["tick.made"]
  let inputs := ["tick.inputs"]
  let hidden := ["tick.withheld"]
  let old := ["tick.inputs"]
  let nums := [ {first with fields := [("n", .one (.int 9))]}, {seventh with fields := []},
    {seventh with id := 8, fields := [("n", .one (.str "absent"))]} ]
  let oldMin := {makeTick minSpec nums with result := [("minimum", .opt ⟨some 0, 0⟩)]}
  let revision := {seventh with of := some "svc:other", prev := [first.id]}
  let grant := {retired with under := some first.id}
  let weekly ← parse (.obj [("vertex", .str "weekly"), ("from", .arr [.obj [("store", .str "lab")]]),
    ("fold", .obj [("records", .obj [("count", .bool true)]), ("covering-first", .obj [("count", .bool true),
      ("where", .obj [("covers", .obj [("is", .str (idText first.id))])])])])])
  let nightly := fun (id : RecordId) (records : List Record) =>
    let r : Record := {id, kind := "tick", observer := "sl:laptop", prev := [], emitted := stampAt 0}
    J.obj [("record", jRecord r), ("opening", Records.ofJson ((makeTick sp records).toJson sha256))]
  let nested := [nightly 16 (xs.take 3), nightly 17 (xs.drop 3), jRecord {first with id := 18, kind := "note", fields := []}]
  let drift := match p.toJson sha256 with
    | .obj kvs => Json.obj (kvs.map fun (k,v) => (k, if k == "rules" then .str two.text else v))
    | j => j
  let alternateHash := "res:" ++ base32 (sha256 (utf8 "loops/result-hash/v2:" ++ resultBytes p.result))
  let setResultHash := fun (v : Rules) (hash : String) => match ({p with rules := v}).toJson sha256 with
    | .obj ms => Json.obj (ms.map fun (k,j) => (k, if k == "result-hash" then .str hash else j))
    | j => j
  let unreadable := raw now future (resultJson p.result)
  let noncanonical := match sp.toJson with | .obj kvs => Json.obj (("note", .str "kept") :: kvs) | j => j
  return [
    made "six-made" ordinary sp xs none false,
    made "make-nontaken" ordinary sp (xs ++ [{first with id := 7, kind := "note"}]) none false,
    made "make-erased" hidden sp xsErased (some xs) false,
    made "six-reordered" (ordinary ++ inputs) sp xs (some (xs.reverse ++ [first])) true,
    made "late-backup" ordinary sp xs (some (xs ++ [seventh])) true,
    made "retired-successor" ordinary follow (xs ++ [retired]) none false,
    made "late-retired" ordinary follow xs (some (xs ++ [retired])) true,
    made "admit-inside" inputs sp [revision] (some [revision, first]) true,
    made "outside-erased-taken" ordinary sp xs
      (some (xs ++ [seventh, {eraser with id := 8, erases := [seventh.id]}])) false,
    made "taken-after-admit" ordinary sp [first] (some [first, revision]) false,
    made "withheld-backup" hidden sp xsHidden none false,
    made "withheld-open-at-checker" hidden sp xsHidden (some (xs ++ [first.withhold, first])) false,
    made "covered-erasure" (ordinary ++ hidden) sp (xs ++ [eraser]) none false,
    check "covered-eraser-omitted" hidden now (omitted.toJson sha256) (xs ++ [eraser]),
    check "covered-eraser-omitted-partial" hidden now (omitted.toJson sha256) (eraser :: xs.drop 2),
    made "eraser-outside" hidden sp xsErased none false,
    check "eraser-outside-no-opening" inputs now (p.toJson sha256) xsErased,
    check "eraser-outside-retained" inputs now (p.toJson sha256) (xs ++ [eraser]),
    check "missing-records" inputs now (p.toJson sha256) (xs.take 4),
    check "missing-opening" inputs now (p.toJson sha256) xsHidden,
    check "missing-both" inputs now (p.toJson sha256) (xsHidden.take 5 ++ [first.withhold]),
    made "count-open-required" inputs count xs (some xsHidden) false,
    ("own-payload-unheld", ordinary, checkInput ⟨"tick", none⟩ xs now),
    ("wrong-kind", ordinary, checkInput ⟨"note", some (p.toJson sha256)⟩ xs now),
    ("wrong-kind-unheld", ordinary, checkInput ⟨"note", none⟩ xs now),
    ("weekly-one-level", ["tick.level"], makeInput weekly nested none false),
    ("weekly-inner-records-held", ["tick.level"], makeInput weekly nested (some (nested ++ xs.map jRecord)) false),
    check "different-result" inputs now (wrong.toJson sha256) xs,
    check "different-unreconciled" inputs now (shorter.toJson sha256) xs,
    check "check-padded-count" ["tick.canonical"] now
      (raw now count.toJson (.obj [("runs", .obj [("unread", .str "0"), ("value", .str "06")])])) xs,
    made "closed-cover" ordinary follow [first, grant] none false,
    made "open-prev" ordinary follow [retired] none false,
    made "open-under" ordinary follow [{retired with prev := [], under := some first.id}] none false,
    made "closure-unknown" inputs sp xs (some []) false,
    made "open-prev-partial" inputs follow [retired, xs[1]!.withhold] (some [retired]) false,
    check "older-same-today" old two (p.toJson sha256) xs,
    check "older-different-today" old two (oldMin.toJson sha256) nums,
    check "older-missing-today" old two (p.toJson sha256) xsHidden,
    check "older-ignored-erasure" old two (omitted.toJson sha256) [eraser],
    check "older-unreadable-today" old two unreadable xs,
    check "older-noncanonical-today" old two (raw now noncanonical (resultJson p.result)) xs,
    check "older-foreign-result" old two (raw now sp.toJson (.arr [])) xs,
    check "check-newer-result-label" old now (setResultHash two alternateHash) xs,
    check "check-older-result-mismatch" old two (setResultHash now alternateHash) xs,
    check "check-newer-hash-spelling" old now (setResultHash two "res:A") xs,
    check "newer-rules" old now (raw two future (.arr [])) [],
    check "newer-ten-under-two" old two (raw ⟨10, by decide⟩ sp.toJson (resultJson p.result)) xs,
    check "check-hash-before-version" inputs now drift xs,
    check "unreadable-spec" inputs now unreadable xs,
    check "check-noncanonical-spec" inputs now (raw now noncanonical (resultJson p.result)) xs,
    check "check-invalid-result" inputs now (raw now sp.toJson (.arr [])) (xs ++ [seventh])]

def vectors : Except String (List (String × J)) := do
  let earlier ← TickPayloadVectors.vectors
  let checks ← (← cases).mapM fun (name, laws, input) => do
    return (name, vector name "tick" laws input (← answer (← Read.parseJson (input.render 0))))
  return earlier ++ checks

end Gen.TickCheckVectors

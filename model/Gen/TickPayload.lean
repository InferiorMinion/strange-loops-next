import Gen.Result
import Gen.Payload
import Gen.Tick
import Loops.TickCheck

namespace Gen.TickPayloadVectors
open Loops

private def fault (f : Refusal) : J := .obj [("refused", .str f.code), ("path", .str f.path)]

private def readBack (j : Json) : Except String J := do
  match readTickPayload sha256 Rules.current j with
  | .error (.invalid f) => return fault f
  | .error .unreadableSpec => return .obj [("cannot-read", .str "spec")]
  | .error (.otherRules r) => return .obj [("other-rules", .str r.text)]
  | .ok p =>
    let text := (p.toJson sha256).canon
    unless text == j.canon do throw "reading changed the payload's canonical bytes"
    let canon ← Read.parseJson text
    let .ok again := readTickPayload sha256 Rules.current canon | throw "JCS payload did not read"
    unless again.bytes sha256 == p.bytes sha256 do throw "second reading changed the bytes"
    return .obj [("payload", Records.ofJson (p.toJson sha256)), ("jcs", .str text)]

private def parseSpec (j : Json) : Except String Spec :=
  (Spec.ofJson j).mapError fun f => s!"{f.code} at {f.path}"

def answer (input : Json) : Except String J := do
  let kvs ← Read.object "input" input
  let op ← Read.text "input.op" (← Read.req "input" kvs "op")
  match op with
  | "payload-text" =>
    let kvs ← Read.members? "input" ["op", "text"] input
    let text ← Read.text "input.text" (← Read.req "input" kvs "text")
    readBack (← Read.parseJson text)
  | "payload" | "members" =>
    let kvs ← Read.members? "input" ["op", "payload"] input
    let j ← Read.req "input" kvs "payload"
    if op == "payload" then return ← readBack j
    match readTickMembers sha256 Rules.current j with
    | .error f => return fault f
    | .ok m => return .obj [("rules", .str m.rules.text), ("spec", Records.ofJson m.spec),
        ("result", Records.ofJson m.result), ("covers", Records.ofJson (tickIds m.covers)),
        ("withheld", Records.ofJson (tickIds m.withheld))]
  | "write-payload" =>
    let kvs ← Read.members? "input" ["op", "spec", "records"] input
    let s ← parseSpec (← Read.req "input" kvs "spec")
    let records ← Read.records "input.records" (← Read.req "input" kvs "records")
    let p := makeTick s records
    let j := p.toJson sha256
    let .ok q := readTickPayload sha256 Rules.current j | throw "written payload refused"
    unless q.spec == s && q.covers == p.covers && q.withheld == p.withheld &&
        q.result == unshownResult p.result do throw "written result did not read back in normal form"
    readBack j
  | "fields" =>
    let kvs ← Read.members? "input" ["op", "payload"] input
    Payload.answer (.obj [("payload", ← Read.req "input" kvs "payload")])
  | "read-ticks" =>
    let kvs ← Read.members? "input" ["op", "spec", "ticks"] input
    let s ← parseSpec (← Read.req "input" kvs "spec")
    let xs ← Read.each "input.ticks" (fun p j => do
      let fields ← Read.members? p ["record", "payload"] j
      let envelope ← Read.record (Read.dot p "record") (← Read.req p fields "record")
      let payload ← Read.req p fields "payload"
      return (make { Records.note with kind := "tick" } (List.replicate 16 0) payload).readInto envelope)
      (← Read.req "input" kvs "ticks")
    return Records.ofJson (resultJson (s.run xs))
  | _ => Tick.answer input

private def backup (id : RecordId) (service : String) (mb : Int) (night : Claim) : Record :=
  { id, kind := "backup", observer := "sl:laptop", prev := [], about := ["svc:" ++ service],
    emitted := stamp "2026-11-21T20:00:00Z", occurred := some night,
    fields := [("svc", .one (.str service)), ("mb", .one (.int mb))] }

def backups : List Record := [
  backup 1 "jellyfin" 412 (claim "2026-11-16"), backup 2 "pihole" 3 (claim "2026-11-16"),
  backup 3 "gitea" 88 (claim "2026-11-16"), backup 4 "jellyfin" 419 (claim "2026-11-17"),
  backup 5 "pihole" 10 (claim "2026-11-17"), backup 6 "gitea" 95 (claim "2026-11-17")]

private def writeInput (spec : Json) (xs : List Record) : Except String Json := do
  let records ← xs.mapM fun r => Read.parseJson ((jRecord r).render 0)
  return .obj [("op", .str "write-payload"), ("spec", spec), ("records", .arr records)]

private def change (j : Json) (k : String) (v : Json) : Json :=
  match j with
  | .obj kvs => .obj (kvs.map fun p => if p.1 == k then (k, v) else p)
  | x => x

private def drop (j : Json) (k : String) : Json :=
  match j with | .obj kvs => .obj (kvs.filter (·.1 != k)) | x => x

private def add (j : Json) (k : String) (v : Json) : Json :=
  match j with | .obj kvs => .obj (kvs ++ [(k,v)]) | x => x

private def specChange (j : Json) (v : Rules) (spec : Json) :=
  change (change j "spec" spec) "spec-hash" (.str (specHashOf sha256 v spec))

private def resultChange (j r : Json) :=
  change (change j "result" r) "result-hash" (.str (resultHashOf sha256 r))

private def raw (op : String) (j : Json) := Json.obj [("op", .str op), ("payload", j)]

private def tickSpec : Json := .obj [("vertex", .str "ticks"),
  ("from", .arr [.obj [("store", .str "lab"), ("kind", .arr [.str "tick"] )]]),
  ("fold", .obj [("ticks", .obj [("count", .bool true)]),
    ("jellyfin", .obj [("count", .bool true), ("where", .obj [("covers", .obj [("is", .str (idText 1))])])]),
    ("by-record", .obj [("count", .bool true), ("by", .str "covers")]),
    ("by-spec", .obj [("count", .bool true), ("by", .str "spec-hash")]),
    ("total", .obj [("sum", .str "result")])])]

private def tickRecord (id : RecordId) (p : TickPayload) : Except String Json := do
  let r : Record := { id, kind := "tick", observer := "sl:laptop", emitted := stampAt 0, prev := [] }
  return .obj [("record", ← Read.parseJson ((jRecord r).render 0)), ("payload", p.toJson sha256)]

private def cases (backupSpec : Json) : Except String (List (String × String × Json)) := do
  let s ← parseSpec backupSpec
  let p := makeTick s backups
  let j := p.toJson sha256
  let hidden := makeTick s (backups.map fun rec => if rec.id == 1 then rec.withhold else rec)
  let a := makeTick s (backups.take 3)
  let b := makeTick s (backups.drop 3)
  let jelly := makeTick s (backups.filter fun r => r.get "svc" == some (.one (.str "jellyfin")))
  let two : Rules := ⟨2, by decide⟩
  let unreadable := change s.toJson "fold" (.obj [("next", .obj [("new-fold", .bool true)])])
  let empty := change s.toJson "fold" (.obj [])
  let newest := specChange (change j "rules" (.str "2")) two unreadable
  let sampleResult := resultJson p.result
  let unknownId := tickIds [20]
  let r := fun name (payload : Json) => (name, "tick.canonical",
    if payload.keysDistinct then raw "payload" payload else
      .obj [("op", .str "payload-text"), ("text", .str ((Records.ofJson payload).render 0))])
  let limited := change s.toJson "fold" (.obj [("runs", .obj [("count", .bool true)]),
    ("mb", .obj [("sum", .str "mb")]), ("list", .obj [("collect", .bool true), ("limit", .int 1)])])
  let accepting := [
    ("six-backups", "tick.canonical", ← writeInput backupSpec backups),
    ("empty-cover", "tick.canonical", ← writeInput backupSpec []),
    ("empty-fold", "tick.canonical", ← writeInput empty backups),
    r "unreconciled-result" (resultChange (specChange j p.rules limited) (change sampleResult "list"
      (.obj [("heads", tickIds [20, 20, 1]), ("unread", .str "9007199254740993")]))),
    ("newer-rules-members", "tick.canonical", raw "members" (resultChange newest (.arr [.str "future"]))),
    ("tick-field-reading", "tick.record", raw "fields" (hidden.toJson sha256)),
    ("spec-reads-ticks", "tick.record", .obj [("op", .str "read-ticks"), ("spec", tickSpec),
      ("ticks", .arr [← tickRecord 16 a, ← tickRecord 17 b, ← tickRecord 18 jelly])])]
  let reverseMembers : Json → Json := fun j => match j with
    | .obj xs => .obj xs.reverse | x => x
  let groupSpec := change s.toJson "fold" (.obj [("small", .obj [("min", .str "mb"), ("by", .str "svc")])])
  let groupPayload := (makeTick (← parseSpec groupSpec) backups).toJson sha256
  let groupResult := fun g m => Json.obj [("small", .obj [("groups", g), ("ungrouped", m), ("unread", .str "0")])]
  let keyedSpec := change s.toJson "fold" (.obj [("state", .obj [("latest-by", .str "of"), ("follow", .str "keys")])])
  let keyedPayload := (makeTick (← parseSpec keyedSpec) []).toJson sha256
  let badKeyed := Json.obj [("state", .obj [("unread", .str "0"), ("things", .obj [("task:1", .obj
    [("heads", .arr []), ("whole", .arr []), ("keys", .obj [("name", .null)])])])])]
  let otherLabel := "res:" ++ base32 (sha256 (utf8 "loops/result-hash/v2:" ++ utf8 sampleResult.canon))
  let newLabel := change (({p with rules := two}).toJson sha256) "result-hash" (.str otherLabel)
  let repeatedSource := Json.obj [("store", .str "lab"), ("kind", .arr [.str "backup"]), ("store", .str "elsewhere")]
  let repeatedSpec := change s.toJson "from" (.arr [repeatedSource])
  let repeatedGroup := groupResult (.obj [("db.one", .obj [("unread", .str "0"), ("value", .null), ("value", .str "3")])]) (.str "0")
  let extra := [
    ("write-nontaken", "tick.made", ← writeInput backupSpec (backups ++ [{backups.head! with id := 7, kind := "note"}])),
    ("write-erased", "tick.withheld", ← writeInput backupSpec (backups.head!.erase :: backups.tail!)),
    r "spec-repeated-source" (change j "spec" repeatedSpec),
    r "result-repeated-group-value" (resultChange groupPayload repeatedGroup),
    r "newer-result-label" newLabel,
    ("newer-result-label-members", "tick.canonical", raw "members" newLabel),
    r "current-wrong-result-label" (change j "result-hash" (.str otherLabel)),
    r "reordered-payload" (reverseMembers (change j "result" (reverseMembers sampleResult))),
    r "newer-rules-deferred" (resultChange newest (.arr [.str "future"])),
    ("shape-grouped-min", "tick.canonical", ← writeInput groupSpec backups),
    r "result-groups-not-object" (resultChange groupPayload (groupResult (.arr []) (.str "0"))),
    r "result-negative-ungrouped" (resultChange groupPayload (groupResult (.obj []) (.str "-1"))),
    r "result-key-ids-not-array" (resultChange keyedPayload badKeyed),
    r "result-repeated-slot" (resultChange j (add sampleResult "mb" (.obj [("unread", .str "0"), ("value", .str "1")])))]
  let faults := [
    r "kept-note" (specChange j p.rules (add s.toJson "note" (.str "kept"))),
    r "omitted-fold" (specChange ((makeTick (← parseSpec empty) []).toJson sha256) p.rules (drop empty "fold")),
    r "unreadable-spec-fitting-hash" (specChange j p.rules unreadable),
    r "unreadable-spec-wrong-hash" (change j "spec" unreadable),
    r "rules-label-drift" (change j "rules" (.str "2")),
    r "spec-not-object" (specChange j p.rules (.arr [])),
    r "not-object" (.arr []), r "unknown-member" (add j "extra" .null),
    r "duplicate-member" (add j "rules" (.str p.rules.text)),
    r "invalid-rules" (change j "rules" (.str "01")),
    r "spec-hash-spelling" (change j "spec-hash" (.str "spec:A")),
    r "result-hash-spelling" (change j "result-hash" (.str "res:A")),
    r "result-hash-mismatch" (change j "result-hash" (.str (resultHashOf sha256 (.obj [])))),
    r "withheld-empty" (add j "withheld" (.arr [])),
    r "withheld-outside" (add j "withheld" unknownId)]
  let missing := TickReader.requiredNames.map fun k => r ("missing-" ++ k) (drop j k)
  let ids := ["covers", "withheld"].flatMap fun k =>
    let set := fun v => if k == "covers" then change j k v else add j k v
    [r (k ++ "-not-array") (set .null), r (k ++ "-invalid-id") (set (.arr [.str "r:BAD"])),
     r (k ++ "-duplicate-id") (set (tickIds [1,1])), r (k ++ "-unsorted") (set (tickIds [2,1]))]
  let slot := fun k v => change sampleResult k v
  let shape := [
    ("result-not-object", .arr []), ("result-missing-slot", drop sampleResult "mb"),
    ("result-extra-slot", add sampleResult "extra" .null),
    ("result-missing-field", slot "mb" (.obj [("value", .str "1")])),
    ("result-extra-field", slot "runs" (.obj [("unread", .str "0"), ("value", .str "6"), ("extra", .null)])),
    ("result-integer-number", slot "mb" (.obj [("value", .int 0), ("unread", .str "0")])),
    ("result-decimal-spelling", slot "mb" (.obj [("value", .str "-0"), ("unread", .str "0")])),
    ("result-negative-count", slot "runs" (.obj [("value", .str "-1"), ("unread", .str "0")])),
    ("result-negative-zero-unread", slot "mb" (.obj [("value", .str "1027"), ("unread", .str "-0")])),
    ("result-negative-unread", slot "mb" (.obj [("value", .str "-1"), ("unread", .str "-1")])),
    ("result-disallowed-null", slot "mb" (.obj [("value", .null), ("unread", .str "0")])),
    ("result-id-spelling", slot "list" (.obj [("heads", .arr [.str "r:BAD"]), ("unread", .str "0")])),
    ("result-id-with-shown", slot "list" (.obj [("heads", .arr [.obj [("id", .str (idText 1)), ("shown", .str "here")]]),
      ("unread", .str "0")]))]
  let shapes ← (ResultVectors.runInputs.map fun old => old ++ ResultVectors.newCases)
  let roundtrips ← (shapes.filter fun c => ["r01-negative-past-2-53", "r03-nothing-read",
      "r04-grouped-ungrouped-unread", "r06-shown-withheld", "31-keys-example", "45-keys-by-payload"].contains c.name).mapM
    fun c => do return ("shape-" ++ c.name, "tick.canonical", ← writeInput c.spec c.records)
  return accepting ++ extra ++ roundtrips ++ faults ++ missing ++ ids ++ shape.map (fun (name, v) => r name (resultChange j v))

def vectors : Except String (List (String × J)) := do
  let identity ← Tick.vectors
  let payloads ← (← cases Tick.backups).mapM fun (name, law, input) => do
    return (name, vector name "tick" [law] (Records.ofJson input) (← answer input))
  return identity ++ payloads

end Gen.TickPayloadVectors

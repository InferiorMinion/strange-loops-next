import Gen.Records
import Gen.Encode
import Loops.SpecHash

/-! Inputs for the spec identity and window laws. No tick record is needed. -/
namespace Gen.Tick

open Loops

private def readSpec (j : Json) : Except String Spec :=
  (Spec.ofJson j).mapError fun r => s!"{r.code} at {r.path}"

private def readRules (j : Json) : Except String Rules :=
  match Rules.ofJson? j with
  | some v => pure v
  | none => throw "rules: expected canonical positive decimal text"

def answer (input : Json) : Except String J := do
  let obj ← Read.object "input" input
  let op ← Read.text "input.op" (← Read.req "input" obj "op")
  match op with
  | "hash" =>
    let kvs ← Read.members? "input" ["op", "samples"] input
    let samples ← Read.each "input.samples" (fun path j => do
      let kvs ← Read.members? path ["rules", "spec"] j
      let v ← readRules (← Read.req path kvs "rules")
      let raw ← Read.req path kvs "spec"
      let rawHash := specHashOf sha256 v raw
      let sp ← readSpec raw
      pure (J.obj [("parsed-jcs", .str sp.toJson.canon), ("spec-hash", .str (sp.hash sha256 v)),
        ("raw-member-hash", .str rawHash)])) (← Read.req "input" kvs "samples")
    pure (.arr samples)
  | "domains" =>
    let kvs ← Read.members? "input" ["op", "rules", "spec"] input
    let v ← readRules (← Read.req "input" kvs "rules")
    let sp ← readSpec (← Read.req "input" kvs "spec")
    let bytes := specPreimageOf v sp.toJson
    let some suffix := String.fromUTF8? (bytes.drop 16).toByteArray | throw "expected UTF-8"
    pure (.obj [("label", .str (specLabel v)), ("label-bytes", .arr ((utf8 (specLabel v)).map (.num ∘ UInt8.toNat))),
      ("after-salt", .str suffix), ("after-salt-is-json", .bool (Read.parseJson suffix).isOk),
      ("spec-byte-7", .num bytes[6]!.toNat), ("result-byte-7", .num (resultPreimage [])[6]!.toNat)])
  | "window" =>
    let kvs ← Read.members? "input" ["op", "spec", "records"] input
    let sp ← readSpec (← Read.req "input" kvs "spec")
    let xs ← Read.records "input.records" (← Read.req "input" kvs "records")
    pure (Records.ofJson (resultJson (sp.run xs)))
  | _ => throw s!"unknown tick fixture operation {op}"

private def sample (spec : Json) (rules := "1") : J :=
  .obj [("rules", .str rules), ("spec", Records.ofJson spec)]

private def spec (fold : List (String × Json)) (vertex := "backups") (stores := ["lab"])
    (kinds := ["backup"]) : Json :=
  .obj [("vertex", .str vertex),
    ("from", .arr (stores.map fun store => .obj [("store", .str store), ("kind", .arr (kinds.map .str))])),
    ("fold", .obj fold)]

private def backupSlots : List (String × Json) :=
  [("runs", .obj [("count", .bool true)]), ("mb", .obj [("sum", .str "mb")]),
   ("list", .obj [("collect", .bool true)])]

def backups := spec backupSlots

private def ordered (slots fields operators : Bool) : Json := Id.run do
  let turn {α} (reverse : Bool) (xs : List α) := if reverse then xs.reverse else xs
  let ops := turn operators [("at-least", Json.int 3), ("below", .int 12)]
  let fields := turn fields [("mb", Json.obj ops), ("svc", .obj [("present", .bool true)])]
  return spec (turn slots [("😀", .obj [("count", .bool true), ("where", .obj fields)]),
                     ("דּ", .obj [("collect", .bool true)])])

private def hashCases : List (String × List J) := [
  ("backups-hash", [sample backups]),
  ("member-order", [sample (ordered false false false), sample (ordered true false false),
                    sample (ordered false true false), sample (ordered false false true)]),
  ("source-order", [sample (spec backupSlots (stores := ["lab", "nas"])),
                    sample (spec backupSlots (stores := ["nas", "lab"]))]),
  ("kind-order", [sample (spec backupSlots (kinds := ["backup", "retry"])),
                  sample (spec backupSlots (kinds := ["retry", "backup"]))]),
  ("carried-note", [sample backups, sample (match backups with
    | .obj kvs => .obj (("note", .str "lab notes") :: kvs) | j => j)]),
  ("renamed-vertex", [sample backups, sample (spec backupSlots (vertex := "archive"))]),
  ("renamed-store", [sample backups, sample (spec backupSlots (stores := ["nas"]))]),
  ("rules-version", [sample backups, sample backups "2"])]

private def windowSlot (field : String) (follow := false) : Json :=
  .obj ([("count", .bool true), ("where", .obj [(field, .obj
    [("at-least", .str "2026-11-21"), ("below", .str "2026-11-22")])])] ++
    (if follow then [("follow", .str "prev")] else []))

private def backup (id : RecordId) (occurred : Option Claim) : Record :=
  { id, kind := "backup", observer := "sl:laptop", prev := [], occurred,
    emitted := stamp "2026-11-21T20:00:00Z" }

private def windowInput (slots : List (String × Json)) (xs : List Record) : J :=
  .obj [("op", .str "window"), ("spec", Records.ofJson (spec slots)),
    ("records", .arr (xs.map jRecord))]

private def cases : List (String × List String × J) := Id.run do
  let backfill := ([1, 2, 3, 4, 5, 6, 7] : List RecordId).map fun i => backup i
    (some (if i ≤ 3 then claim "2026-11-16" else claim "2026-11-17"))
  let first := backup (fixtureId 1) (some (claim "2026-11-21"))
  let later := { backup (fixtureId 2) (some (claim "2026-11-22")) with
    of := some (idText first.id), prev := [first.id] }
  return (hashCases.map fun (n, samples) =>
    (n, ["spec.hash"], J.obj [("op", .str "hash"), ("samples", .arr samples)])) ++ [
    ("preimage-domains", ["spec.no-commit", "spec.no-result"], .obj [
      ("op", .str "domains"), ("rules", .str "1"), ("spec", Records.ofJson backups)]),
    ("when-backfill", ["window.span"], windowInput
      (["env.occurred", "env.emitted", "env.when"].map fun f => (f, windowSlot f)) (backfill ++ [backup (fixtureId 8) none])),
    ("when-after-heads", ["window.heads"], windowInput
      [("all", windowSlot "env.when"), ("heads", windowSlot "env.when" true)] [first, later])]

def vectors : Except String (List (String × J)) := cases.mapM fun (name, laws, input) => do
  let expect ← answer (← Read.parseJson (input.render 0))
  pure (name, vector name "tick" laws input expect)

end Gen.Tick

import Gen.Spec

/-! Stored openings and the slots that read their top-level fields. The raw input
keeps nested values and numeric spellings which a reader projection forgets. -/
namespace Gen.Payload
open Loops

def scalar : Val → J
  | .str s => .obj [("text", .str s)]
  | .int i => .obj [("whole", .str (toString i))]
  | .number n => .obj [("number", .str n.canon)]
  | .bool b => .obj [("bool", .bool b)]

def held : HeldField → J
  | .one v => .obj [("one", scalar v)]
  | .many vs => .obj [("many", .arr (vs.map scalar))]
  | .other => .obj [("other", .bool true)]

def answer (input : Json) : Except String J := do
  let kvs ← Read.members? "input" ["payload"] input
  let p ← Read.req "input" kvs "payload"
  unless p.keysDistinct && p.inRange do throw "input.payload: invalid stored payload"
  let r := (make Records.note (List.replicate 16 0) p).readInto default
  return .obj [("fields", .obj ((sortBy (fun kv => utf16 kv.1) r.fields).map fun (k,v) => (k, held v)))]

def cases : List (String × String) := [
  ("01-empty-object", "{}"), ("02-top-null", "null"), ("03-top-boolean", "false"),
  ("04-top-text", "\"held\""), ("05-top-whole", "80"), ("06-top-fraction", "-1.5"),
  ("07-top-list", r#"[1,{"k":"inside"}]"#),
  ("08-scalars", r#"{"empty":"","txt":"abc","n":0,"__proto__":true,"10":false,"2":-7,"plus":4,"f":-1.5}"#),
  ("09-number-spellings", r#"{"one":1.0,"hundred":1e2,"neg":-2E0,"zero":-0,"tiny":5e-324,"ports":[80.0]}"#),
  ("10-safe-bounds", r#"{"max":9007199254740991,"min":-9007199254740991,"rounded":9007199254740991.1,"negative":-9007199254740991.1,"members":[9007199254740991.1,-9007199254740991.1,4503599627370495.5,-4503599627370495.5],"fraction":4503599627370495.5}"#),
  ("11-written-other", r#"{"null":null,"object":{},"labels":{"tier":"infra"},"list":[],"empty":""}"#),
  ("12-list-members", r#"{"mixed":["",false,80.0,true,-1,1.5,null,["nested"],{"x":1}]}"#),
  ("13-no-readable-members", r#"{"empty":[],"opaque":[null,{},["hidden"],1.5]}"#),
  ("14-repeats-and-collisions", r#"{"x":["1",1,true,"true","😀","דּ",1],"y":[1,"דּ","😀","true",true,1,"1"]}"#),
  ("15-routes-a", r#"{"routes":["svc:a"]}"#), ("16-routes-b", r#"{"routes":["svc:b"]}"#),
  ("17-routes-ab", r#"{"routes":["svc:a","svc:b"]}"#),
  ("18-envelope-names", r#"{"about":["svc:a"],"of":null,"sets":"ordinary"}"#)
]

def vectors : Except String (List (String × J)) := cases.mapM fun (name, raw) => do
  let input := J.obj [("payload", .raw raw)]
  let expect ← answer (← Read.parseJson (input.render 0))
  pure (name, vector name "payload" ["payload.read"] input expect)

/-- Each input record carries the original field object, including the members
that contribute no equality argument. Use actual protocol reader ids. -/
def row (id : RecordId) (thing fields : String) (prev : List RecordId := [])
    (sets : Option (List String) := none) : J :=
  let r : Record :=
    { id, kind := (thing.splitOn ":").head!, observer := "sl:kyle", prev,
      of := some thing, sets, emitted := stampAt 0 }
  match jRecord r with
  | .obj kvs => .obj (kvs ++ [("fields", .raw fields)])
  | j => j

def homelab := [
  row 1 "svc:grafana" r#"{"runs_on":"host:nuc1","watches":["svc:prometheus","svc:loki"]}"#,
  row 2 "svc:prometheus" r#"{"runs_on":"host:nuc1","watches":["svc:node-exporter"]}"#,
  row 3 "svc:loki" r#"{"runs_on":"host:nuc2","watches":[],"notes":null,"labels":{"tier":"infra"}}"#,
  row 4 "svc:caddy" r#"{"runs_on":"host:nuc2","ports":[80,443,8080]}"#,
  row 5 "backup:nuc1" r#"{"host":"nuc1","n":14}"#, row 6 "backup:nuc2" r#"{"host":"nuc2","n":9}"#,
  row 7 "backup:nas" r#"{"host":"nas","status":"failed"}"#, row 8 "backup:pi" r#"{"host":"pi","snapshots":3}"#,
  row 9 "setting:dns" r#"{"value":"1.1.1.1"}"#, row 10 "setting:ntp" r#"{"value":"pool.ntp.org"}"#,
  row 11 "setting:proxy" r#"{"enabled":false}"#]

abbrev Slots := List (String × Json)
def test (field op : String) (arg : Json) (fold := "count") (keys := false) : Json :=
  .obj ([(fold, .bool true), ("where", .obj [(field, .obj [(op,arg)])])] ++
    (if keys then [("follow", .str "keys")] else []))

def byField (field : String) (fold := "count") (keys := false) : Json :=
  .obj ([(fold, .bool true), ("by", .str field)] ++ (if keys then [("follow", .str "keys")] else []))

def keyedSlots : Slots :=
  [("a", test "routes" "is" (.str "svc:a") (keys := true)),
   ("b", test "routes" "is" (.str "svc:b") (keys := true)),
   ("c", test "routes" "is" (.str "svc:c") (keys := true)),
   ("present", test "routes" "present" (.bool true) (keys := true)),
   ("not-c", test "routes" "not" (.str "svc:c") (keys := true)),
   ("routes", byField "routes" (keys := true)),
   ("null", test "notes" "present" (.bool true) (keys := true)),
   ("omitted", test "missing" "absent" (.bool true) (keys := true)),
   ("undeclared", test "extra" "absent" (.bool true) (keys := true)),
   ("state", .obj [("latest-by", .str "of"), ("follow", .str "keys")])]

def fork (overlap : Bool) := [
  row 20 "svc:fork" "{}" [] (some []),
  row 21 "svc:fork" (if overlap then r#"{"routes":["svc:a","svc:b"],"notes":null,"extra":true}"#
    else r#"{"routes":["svc:a"],"notes":null,"extra":true}"#) [20] (some ["routes","notes","missing"]),
  row 22 "svc:fork" (if overlap then r#"{"routes":["svc:a"]}"# else r#"{"routes":["svc:b"]}"#)
    [20] (some ["routes"])]

def mixed : List J := [
  row 30 "svc:mixed" r#"{"x":[1,"1",true,"true","",false,1,"😀","דּ"],"n":2}"#,
  row 31 "svc:permuted" r#"{"x":["דּ","😀",1,false,"","true",true,"1",1],"n":[2]}"#,
  row 32 "svc:none" r#"{"x":[1.5,null,{},["nested"],9007199254740992],"n":null}"#,
  row 33 "svc:empty" r#"{"x":[],"n":{}}"#,
  jRecord ({ SpecVectors.task 34 "svc:hidden" 0 [] with kind := "svc" } : Record).withhold]

def runCases : List (String × List String × List J × Slots) := [
  ("pv3-written", ["where.fields"], homelab,
    [("missing",test "watches" "absent" (.bool true) "collect"), ("notes",test "notes" "present" (.bool true)),
     ("labels",test "labels" "present" (.bool true)), ("empty-about",test "env.about" "absent" (.bool true)),
     ("watches",test "watches" "present" (.bool true)), ("object-is",test "labels" "is" (.str "infra")),
     ("object-not",test "labels" "not" (.str "infra"))]),
  ("pv4-members", ["where.fields","where.complements"], homelab,
    [("is",test "watches" "is" (.str "svc:loki")), ("not",test "watches" "not" (.str "svc:loki")),
     ("who",test "watches" "is" (.str "svc:loki") "collect"), ("others",test "watches" "not" (.str "svc:loki") "collect")]),
  ("pv5-list-bounds", ["where.typed"], homelab, [("below",test "ports" "below" (.int 1024)),
    ("at-least",test "ports" "at-least" (.int 1024)), ("eighty",test "ports" "is" (.int 80))]),
  ("pv6-list-groups", ["by.accounting","by.partition"], homelab,
    [("count",byField "watches"), ("records",byField "watches" "collect")]),
  ("pv7-list-numeric", ["numeric.safe-whole"], homelab,
    ["sum","min","max"].map fun f => (f, .obj [(f,.str "ports")])),
  ("pv-lists-mixed", ["where.fields","by.accounting","numeric.safe-whole"], mixed,
    [("by-x",byField "x"), ("present",test "x" "present" (.bool true)), ("empty",test "x" "is" (.str "")),
     ("bool",test "x" "is" (.bool true)), ("nested",test "x" "is" (.str "nested")),
     ("current",.obj [("sum",.str "n"),("follow",.str "prev")]), ("below",test "n" "below" (.int 3)),
     ("by-n",byField "n")]),
  ("pv4-text-80", ["where.fields"], [row 40 "svc:scalar" r#"{"p":"80"}"#,
    row 41 "svc:text" r#"{"p":["80"]}"#, row 42 "svc:numeric" r#"{"p":[80.0]}"#],
    [("number",test "p" "is" (.int 80) "collect"), ("text",test "p" "is" (.str "80") "collect")]),
  ("pv6-key-80", ["by.accounting"], [row 43 "svc:caddy" r#"{"p":[80,"80"]}"#], [("ports",byField "p")]),
  ("pv-keys-disjoint", ["keyed.select","keyed.by"], fork false, keyedSlots),
  ("pv-keys-overlap", ["keyed.select","keyed.by"], fork true, keyedSlots)
]

def runVectors : Except String (List (String × J)) := runCases.mapM fun (name,laws,rows,slots) => do
  let spec := Json.obj [("vertex",.str name), ("from",.arr [.obj [("store",.str "."),("kind",.arr [.str "svc"])]]),
    ("fold", .obj slots)]
  let sp ← (Spec.ofJson spec).mapError (fun r => r.code)
  let orders := (List.range rows.length).map fun n => rows.drop n ++ rows.take n
  let answers ← orders.mapM fun order => do
    let rs ← order.mapM fun j => do Read.record "record" (← Read.parseJson (j.render 0))
    pure (J.obj ((sp.run rs).map fun (k,v) => (k,SpecVectors.jSlot v)))
  let expect := answers.headD .null
  unless answers.all (·.render 0 == expect.render 0) do throw s!"{name}: rotations disagree"
  pure (name, vector name "spec/run" laws (.obj [("spec", Records.ofJson spec),
    ("orders",.arr (orders.map J.arr))]) expect)

end Gen.Payload

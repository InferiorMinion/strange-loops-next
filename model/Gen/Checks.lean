import Gen.Read
import Gen.Encode
import Gen.ByteText
import Gen.Records
import Gen.Tick
import Loops.Tick

open Loops Gen

def ensure (ok : Bool) (name : String) : IO Unit :=
  unless ok do throw (IO.userError name)

def parsed (s : String) : IO Json :=
  match Read.parseJson s with
  | .ok j => pure j
  | .error e => throw (IO.userError e)

def refused {α} : Except String α → Bool
  | .error _ => true
  | .ok _ => false

def canonicalTextChecks : IO Unit := do
  -- The expected answers are independent of the decoder used by the vectors.
  for (n, pad) in [(16, 2), (32, 4), (64, 3)] do
    let ts := ByteTextVectors.texts n pad
    for (s, i) in ts.zipIdx do
      let accepted := i < 2
      -- Appending zero to the 32- or 64-byte encoding canonically encodes one
      -- extra byte. Only the fixed-width boundary must reject those texts.
      let canonical := accepted || (i == 3 && (n == 32 || n == 64))
      ensure ((Loops.unbase32 s).isSome == canonical) s!"R1 model decoder {n} bytes, case {i}"
      ensure ((Gen.Read.unbase32 s).isOk == canonical) s!"R1 fixture decoder {n} bytes, case {i}"
      ensure ((Gen.Read.bytes "bytes" (.str s)).isOk == canonical) s!"R1 fixture bytes {n}, case {i}"
      let prefixes := if n == 32 then ["r:", "h:", "k:"] else [""]
      for pre in prefixes do
        ensure ((Read.fixedBytes "bytes" n pre (.str (pre ++ s))).isOk == accepted)
          s!"R1 fixed boundary {pre}{n}, case {i}"
      if n == 32 then
        ensure (recordId ("r:" ++ s) == accepted) s!"R1 envelope id, case {i}"
        ensure ((Read.recordId "id" (.str ("r:" ++ s))).isOk == accepted) s!"R1 fixture id, case {i}"
        let e : Env := { kind := "note", observer := "o:test", emitted := "2026-10-21T00:00:00Z" }
        for (bad, code) in [({ e with of := some "t:x", prev := ["r:" ++ s] }, "prev-not-record-id"),
            ({ e with under := some ("r:" ++ s) }, "under-not-record-id"),
            ({ e with erases := ["r:" ++ s] }, "erases-not-record-id")] do
          ensure (bad.problem == if accepted then none else some code) s!"R1 {code}, case {i}"
    ensure (refused (Read.fixedBytes "bytes" n "k:" (.str ("h:" ++ ts.head!)))) s!"R1 prefix {n}"
    ensure (refused (Read.fixedBytes "bytes" n "" (.str (base32 (List.replicate (n - 1) 0)))))
      s!"R1 wrong decoded width {n}"
  let ids : List RecordId := [fixtureId 0, fixtureId 1, fixtureId 15, fixtureId 16,
    fixtureId 31, fixtureId 32, fixtureId 255, fixtureId 256,
    fixtureId (2 ^ 255), fixtureId (2 ^ 256 - 1)]
  for a in ids do
    for b in ids do
      ensure (decide (idText a < idText b) == decide (a < b))
        s!"R1 actual id text order {a.val}, {b.val}"
  let good := ["0000-01-01T00:00:00Z", "9999-12-31T23:59:59.999999999Z",
    "2000-02-29T12:34:56.5Z", "2026-10-21T00:00:00.000000001Z"]
  for s in good do
    let some t := instantParts? s.toList | throw (IO.userError s!"R4 refuses control {s}")
    ensure (t.text == s) s!"R4 parse/render {s}"
    ensure (instant? s.toList == some t.moment) s!"R4 actual parser moment {s}"
    ensure (validEmitted s && (Read.stampOf "emitted" (.str s)).isOk) s!"R4 emitted control {s}"
  let bad := ["2026-10-21T00:00:00+00:00", "2026-10-21T00:00:00-00:00",
    "2026-10-21T00:00:00+01:00", "2026-10-21T00:00:00-23:59",
    "2026-10-21T00:00:60Z", "2016-12-31T23:59:60Z", "2026-10-21T00:00:00.0Z",
    "2026-10-21T00:00:00.50Z", "2026-10-21T00:00:00.500Z",
    "2026-10-21T00:00:00.1234567891Z", "2026-10-21T00:00:00.Z"]
  let e : Env := { kind := "note", observer := "o:test", emitted := "2026-10-21T00:00:00Z" }
  for s in bad do
    ensure ((instant? s.toList).isNone && !validEmitted s) s!"R4 instant accepts {s}"
    ensure (refused (Read.stampOf "emitted" (.str s))) s!"R4 fixture emitted accepts {s}"
    ensure (({ e with emitted := s } : Env).problem == some "emitted-not-rfc3339") s!"R4 emitted code {s}"
    for occurred in [s, s ++ "/2027", "2026/" ++ s, s ++ "/.."] do
      ensure ((occurredSpan? occurred).isNone) s!"R4 occurred accepts {occurred}"
      ensure (refused (Read.claimOf "occurred" (.str occurred))) s!"R4 fixture occurred accepts {occurred}"
      ensure (({ e with occurred := some occurred } : Env).problem == some "occurred-not-span")
        s!"R4 occurred code {occurred}"
  -- Raw keys precede all value checks. The typed envelope keeps its existing rules.
  ensure (({ e with kind := "" } : Env).problem == some "kind-empty") "R6 typed kind rule changes"
  for fields in [[], [("kind", .str "")], [("kind", .int 7)], [("kind", .str "note")]] do
    let raw := Json.obj (fields ++ [("extension", .null)])
    ensure (match Read.env "env" raw with | .error why => why == "envelope-field-unknown" | _ => false)
      "R6 unknown key does not precede missing, empty or malformed values"
  ensure (!writeCodes.contains "envelope-field-unknown") "R6 raw shape leaked into typed writer codes"
  let unknown := Json.obj [("extra", .null)]
  let goodEnv := Json.obj [("kind", .str "note"), ("observer", .str "o:test"),
    ("emitted", .str "2026-10-21T00:00:00Z")]
  let shape := Json.obj [("env", unknown)]
  let malformed := [Json.int 7,
    .obj [("label", .int 7), ("env", goodEnv)],
    .obj [("env", goodEnv), ("salt", .int 7)],
    .obj [("env", .obj [("kind", .int 7)])]]
  for bad in malformed do
    for ds in [[bad, shape], [shape, bad]] do
      ensure (match Gen.Read.drafts "drafts" (.arr ds) with
        | .error why => why == "envelope-field-unknown" | _ => false)
        "R6 all-draft preflight must precede malformed shells, labels, values and salts"
  ensure (match Gen.Read.drafts "drafts" (.arr [.obj [("env", goodEnv), ("env", unknown)]]) with
    | .error why => why == "envelope-field-unknown" | _ => false)
    "R6 repeated draft env members must not hide unknown envelope keys"
  for es in [[goodEnv, unknown], [unknown, goodEnv], [.int 7, unknown]] do
    ensure (match Read.envs "envs" (.arr es) with
      | .error why => why == "envelope-field-unknown" | _ => false)
      "R6 all-envelope preflight must precede value decoding"
  -- Issue #1: an envelope that is not an object refuses after unknown keys and
  -- before any value; a field left out or of the wrong JSON type refuses with
  -- the field's own code.
  ensure (shapeCodes == ["envelope-field-unknown", "envelope-not-object"]) "shape codes changed order"
  ensure (shapeCodes.all (!writeCodes.contains ·)) "raw shape leaked into typed writer codes"
  let shapeIs {α : Type} (code : String) (r : Except String α) : Bool := match r with
    | .error why => why == code
    | _ => false
  for raw in [Json.int 7, .null, .str "note", .arr [goodEnv], .bool false] do
    ensure (shapeIs "envelope-not-object" (Read.env "env" raw)) "a nonobject envelope is not refused by name"
    for es in [[goodEnv, raw], [raw, goodEnv], [.obj [("kind", .int 7)], raw]] do
      ensure (shapeIs "envelope-not-object" (Read.envs "envs" (.arr es)))
        "a nonobject variant does not precede every value"
    for es in [[unknown, raw], [raw, unknown]] do
      ensure (shapeIs "envelope-field-unknown" (Read.envs "envs" (.arr es)))
        "an unknown key does not precede a nonobject envelope"
    for ds in [[.obj [("env", raw)]], [.obj [("env", goodEnv)], .obj [("env", raw)]]] do
      ensure (shapeIs "envelope-not-object" (Gen.Read.drafts "drafts" (.arr ds)))
        "a draft's nonobject envelope is not a shape refusal"
  ensure (shapeIs "envelope-field-unknown"
      (Read.env "env" (.obj [("kind", .str "a"), ("kind", .str "b"), ("extra", .null)])))
    "a repeated envelope member hides an unknown key"
  ensure (shapeIs "envelope-not-object" (Gen.Read.drafts "drafts" (.arr [.obj [("salt", .int 7)]])))
    "a draft without an envelope is not a shape refusal"
  ensure (shapeIs "envelope-field-unknown" (Gen.Read.drafts "drafts" (.arr [.obj [], .obj [("env", unknown)]])))
    "an unknown key does not precede a draft without an envelope"
  let set (k : String) (v : Option Json) : Json := match goodEnv with
    | .obj kvs => .obj ((kvs.filter (·.1 != k)) ++ (v.map (k, ·)).toList)
    | other => other
  let fields := [("kind", "kind-empty", true), ("observer", "observer-not-typed", true),
    ("emitted", "emitted-not-rfc3339", true), ("occurred", "occurred-not-span", false),
    ("about", "about-not-typed", false), ("of", "of-not-typed", false), ("prev", "prev-not-record-id", false),
    ("under", "under-not-record-id", false), ("erases", "erases-not-record-id", false),
    ("sets", "sets-not-keys", false)]
  for (k, code, required) in fields do
    let wrong := [Json.int 7, .null, .bool true, .obj [], .arr [.int 7]] ++
      (if ["about", "prev", "erases"].contains k then [.str "r:x", .arr [.str "topic:a", .null]]
       else if k == "sets" then [.str "title", .arr [.str "title", .null], .arr [.str "title", .str ""]]
       else [.arr []])
    for v in (if required then [none] else []) ++ wrong.map some do
      ensure (match Read.env "env" (set k v) with
        | .ok e => e.problem == some code
        | .error _ => false) s!"{k} left out or mistyped is not refused with {code}"
  -- `sets`: absent and empty differ, and an empty set is written
  for v in [Json.arr [], .arr [.str "title"], .arr [.str "@d", .str "title", .str "@d"]] do
    ensure (match Read.env "env" (set "sets" (some v)) with
      | .ok e => e.problem == none
      | .error _ => false) "a set of key names is refused"
  let absent : Env := { kind := "note", observer := "o:test", emitted := "2026-10-21T00:00:00Z" }
  let empty : Env := { absent with sets := some [] }
  ensure (empty.toJson.canon != absent.toJson.canon) "an empty sets is left out"
  ensure (empty.toJson.canon == "{\"emitted\":\"2026-10-21T00:00:00Z\",\"kind\":\"note\",\"observer\":\"o:test\",\"sets\":[]}")
    "an empty sets is not written as []"
  ensure (({ absent with sets := some ["title", "due", "title"] } : Env).toJson.canon ==
      ({ absent with sets := some ["due", "title"] } : Env).toJson.canon) "sets is not canonical"
  IO.println "R1/R4/R6 canonical text runtime checks pass"

/-- The answer `Records.recordAnswer` gives a record input written as text. -/
def recordAnswerOf (input : String) : IO (Except String String) := do
  pure ((Records.recordAnswer (← parsed input)).map (·.render 0))

def refusedWith (code : String) : Except String String → Bool
  | .ok s => s == (J.obj [("refused", .str code)]).render 0
  | .error _ => false

/-- A fault in the input: no answer at all, so never a protocol refusal. -/
def faulted : Except String String → Bool
  | .error _ => true
  | .ok _ => false

def rulingChecks : IO Unit := do
  -- D1: the public base boundary refuses a repeated id before it reads the text.
  let L (s : String) : Lens.Line := s.toList
  let one : Lens.Section := ⟨L "a", [L "one"]⟩
  let two : Lens.Section := ⟨L "a", [L "two"]⟩
  let other : Lens.Section := ⟨L "b", [L "B"]⟩
  for base in [[one, two], [one, other, two], [one, one]] do
    for text in [Lens.render [one], Lens.render base, [], [L "::: a", L "unclosed"], [L "stray"]] do
      ensure (match Lens.read base text with | .error why => why == "base-id-duplicate" | .ok _ => false) "D1 repeated base id is read"
  for base in [[], [one], [one, other]] do
    ensure (Lens.baseProblem base == none) "D1 distinct base is refused"
    for text in [Lens.render [two], Lens.render base, [], [L "stray"]] do
      ensure (match Lens.read base text with | .ok os => os == Lens.edit base text | .error _ => false) "D1 boundary changes an accepted edit"
  ensure (Lens.lookup (L "a") [one, two] == some [L "one"]) "D1 raw lookup no longer takes the first section"
  ensure (Lens.diff [one, two] [two] == [.accepted (L "a") [L "two"]]) "D1 raw diff no longer matches the first section"
  -- D2: code units are text only when surrogates pair.
  for units in [[0xD800], [0xDBFF], [0xDC00], [0xDFFF], [0xDC00, 0xD800], [0xD83D, 0x41], [0xDBFF, 0x41],
      [0x41, 0xD83D], [0xD83D, 0xD83D, 0xDE00], [0xD83D, 0xDE00, 0xDE00], [0x110000], [0x10000]] do
    ensure ((ofUtf16? units).isNone) s!"D2 lone surrogate reads as text: {units}"
  ensure (ofUtf16? [0xD83D, 0xDE00] == some [Char.ofNat 0x1F600]) "D2 pair does not join"
  ensure (ofUtf16? [0x41, 0xD7FF, 0xE000, 0xFFFF] ==
    some [Char.ofNat 0x41, Char.ofNat 0xD7FF, Char.ofNat 0xE000, Char.ofNat 0xFFFF]) "D2 scalar values refused"
  for s in ["", "plain", "雪😀", String.singleton (Char.ofNat 0x10FFFF)] do
    ensure (ofUtf16? (utf16 s) == some s.toList) s!"D2 a string's own code units do not read back: {s}"
  for text in ["\"\\ud800\"", "\"\\udc00\"", "\"\\udc00\\ud800\"", "\"\\ud83dx\"", "\"\\ud83d\\u0041\"",
      "{\"\\udfff\":0}", "[[\"\\ud83d\\ud83d\\ude00\"]]", "{\"a\":1,\"a\":2,\"b\":\"\\uD800\"}",
      "[\"\\ud800\\n\", \"\\udc00\"]"] do
    ensure (Read.textRefusal text == some "string-not-unicode") s!"D2 text is not refused by name: {text}"
    ensure (refused (Read.parseJson text)) s!"D2 text parses: {text}"
  -- a pair is text; text that is not JSON is not this refusal's, before or after the surrogate
  for text in ["\"\\ud83d\\ude00\"", "\"\\uD83D\\uDE00\"", "{\"a\" \"\\ud800\"}", "[1,]", "\"\\ud83d\\uZZZZ\"",
      "{\"a\":\"\\ud800\"", "\"\\ud800", "[\"\\ud800\",]", "\"\\ud800\" 1", "[\"\\udc00\", 01]"] do
    ensure (Read.textRefusal text == none) s!"D2 refusal claims text it does not own: {text}"
  ensure (match Read.parseJson "\"\\ud83d\\ude00\"" with | .ok (.str s) => s == "😀" | _ => false) "D2 pair changes"
  let env := "{\"kind\":\"note\",\"observer\":\"o:test\",\"emitted\":\"2026-10-21T00:00:00Z\"}"
  let noKind := "{\"kind\":\"\",\"observer\":\"o:test\",\"emitted\":\"2026-10-21T00:00:00Z\"}"
  let untyped := "{\"kind\":\"note\",\"observer\":\"test\",\"emitted\":\"2026-10-21T00:00:00Z\"}"
  let extra := "{\"kind\":\"note\",\"observer\":\"o:test\",\"emitted\":\"2026-10-21T00:00:00Z\",\"x\":0}"
  let zeros (n : Nat) := String.ofList (List.replicate n '0')
  let salt := zeros 26
  let input (envs payloads : String) (seed : Option String := none) (salt : String := salt) : String :=
    "{\"envs\":[" ++ envs ++ "]," ++ payloads ++ ",\"salt\":\"" ++ salt ++ "\"" ++
      (match seed with | some s => ",\"seed\":\"" ++ s ++ "\"" | none => "") ++ "}"
  let lone := "\"payload_text\":\"\\\"\\\\ud800\\\"\""
  for (envs, salt) in [(env, salt), (noKind, salt), (untyped ++ "," ++ noKind, salt), (env, zeros 24), (env, "!")] do
    ensure (refusedWith "string-not-unicode" (← recordAnswerOf (input envs lone (salt := salt))))
      "D2 a lone surrogate in a text payload does not precede envelope values and the salt"
  for envs in [extra, env ++ "," ++ extra, extra ++ "," ++ noKind] do
    ensure (refusedWith "envelope-field-unknown" (← recordAnswerOf (input envs lone)))
      "D2 a lone surrogate in a text payload precedes raw shape"
  ensure (refused (Read.parseJson (input env "\"payloads\":[\"\\ud800\"]")))
    "D2 a lone surrogate outside a text payload reads as a fixture"
  -- D3: overflow stays a number until the writer's number check, in its usual place.
  for payloads in ["\"payloads\":[1e999]", "\"payloads\":[{\"n\":[-1e999]}]",
      "\"payload_text\":\"1e999\"", "\"payload_text\":\"{\\\"n\\\":-1E400}\""] do
    ensure (refusedWith "number-out-of-range" (← recordAnswerOf (input env payloads))) "D3 overflow is not the writer's refusal"
    ensure (refusedWith "kind-empty" (← recordAnswerOf (input noKind payloads))) "D3 overflow precedes the envelope"
    ensure (refusedWith "salt-not-16-bytes" (← recordAnswerOf (input env payloads (salt := zeros 24)))) "D3 overflow precedes the salt"
    ensure (refusedWith "envelope-field-unknown" (← recordAnswerOf (input extra payloads))) "D3 overflow precedes shape"
  ensure (refusedWith "member-duplicate" (← recordAnswerOf (input env "\"payload_text\":\"{\\\"a\\\":1e999,\\\"a\\\":0}\"")))
    "D3 overflow precedes repeated members"
  -- D4: a signing seed is exactly 32 bytes, read only once the record is written.
  let written := "\"payloads\":[{}]"
  ensure (!(faulted (← recordAnswerOf (input env written (seed := some (zeros 52)))))) "D4 32-byte seed control fails"
  for seed in [some (zeros 50), some (zeros 53), some "", some (zeros 51 ++ "1"), some "!", none] do
    ensure (faulted (← recordAnswerOf (input env written (seed := seed)))) s!"D4 seed is padded, cut or assumed: {seed}"
    ensure (refusedWith "kind-empty" (← recordAnswerOf (input noKind written (seed := seed))))
      s!"D4 a refused record waits on its seed: {seed}"
  -- D5: the first envelope and first payload decide; a later variant never answers.
  let seed := some (zeros 52)
  ensure (refusedWith "kind-empty" (← recordAnswerOf (input (noKind ++ "," ++ env) written seed))) "D5 first refusal"
  ensure (refusedWith "kind-empty" (← recordAnswerOf (input (noKind ++ "," ++ untyped) written seed))) "D5 first of two refusals"
  ensure (refusedWith "observer-not-typed" (← recordAnswerOf (input (untyped ++ "," ++ noKind) written seed)))
    "D5 refusals are ranked across variants"
  ensure (refusedWith "number-out-of-range" (← recordAnswerOf (input (env ++ "," ++ noKind) "\"payloads\":[1e999,{}]" seed)))
    "D5 first pair"
  for (envs, payloads) in [(env ++ "," ++ noKind, written), (env ++ "," ++ untyped, written),
      (env, "\"payloads\":[{},9007199254740992]"), (env, "\"payloads\":[null,1e999]"),
      (env, "\"payloads\":[{\"a\":1},{\"a\":1,\"a\":2}]"), (env, "\"payloads\":[{\"a\":1},{\"a\":2}]")] do
    ensure (faulted (← recordAnswerOf (input envs payloads seed))) s!"D5 a later variant answers: {envs} {payloads}"
  ensure (!(faulted (← recordAnswerOf (input (env ++ "," ++ env) "\"payloads\":[{\"a\":1,\"b\":2},{\"b\":2,\"a\":1}]" seed))))
    "D5 agreeing variants control fails"
  -- D6: the four concrete helpers.
  let r : Record := { id := fixtureId 1, kind := "note", observer := "o:test", about := ["t:one"], prev := [], emitted := stampAt 0, fields := [("n", .one (.int 4)), ("est", .one (.int 7)), ("tag", .one (.str "x"))] }
  ensure ((Sum.on "est").step 5 r == 12) "D6 Sum.on control fails"
  for field in ["missing", "tag"] do
    ensure ((Sum.on field).step 5 r == 5) s!"D6 Sum.on moves on an unreadable field: {field}"
  ensure ((Sum.on "est").step 5 r.withhold == 5) "D6 Sum.on moves on a withheld record"
  ensure (Max.acc.readable r && Min.acc.readable r) "D6 extrema do not read a held safe whole n"
  for hidden in [r.withhold, r.erase] do
    ensure (!(Max.acc.readable hidden) && !(Min.acc.readable hidden)) "D6 extrema read a hidden record"
  let sp : Spec := { vertex := "v", sources := [{ store := ".", kinds := ["note"] }] }
  let vote := { r with id := fixtureId 2, kind := "vote" }
  ensure (sp.select [r, vote, r.withhold] == [r, r.withhold]) "D6 Spec.select is not the records the spec takes"
  ensure (sp.select [vote] == []) "D6 Spec.select keeps a record the spec does not take"
  -- An opening checks against the commitment of the record written with it, and no other salt or payload does.
  let salt : List UInt8 := (List.range 16).map (·.toUInt8)
  let payload := Json.obj [("status", .str "open"), ("est", .int 3)]
  let commit := (make { kind := "note", observer := "o:test", emitted := "2026-10-21T00:00:00Z" } salt payload).commit Gen.Records.H
  ensure (opens Gen.Records.H commit salt payload) "a written record's opening does not check against its commitment"
  ensure (!opens Gen.Records.H commit salt (.obj [("status", .str "open"), ("est", .int 4)])) "another payload opens the commitment"
  ensure (!opens Gen.Records.H commit (salt.map (· + 1)) payload) "another salt opens the commitment"
  ensure (!opens Gen.Records.H (commit ++ "0") salt payload) "the opening checks against another commitment"
  IO.println "D1-D6 ruling runtime checks pass"

/-- The result codec's integer reader (D7), which only laws use otherwise: it
reads back the text `intText` writes and nothing else. -/
def resultCodecChecks : IO Unit := do
  for i in [(0 : Int), 7, 10, 12, -12, 100, 9007199254740993, -18014398509481984,
      123456789012345678901234567890] do
    ensure (intOf? (intText i) == some i) s!"result integer {i} does not read back"
  for (text, i) in [("0", (0 : Int)), ("12", 12), ("-12", -12), ("9007199254740993", 9007199254740993)] do
    ensure (intOf? text == some i) s!"result integer text {text} misread"
  for text in ["", "-", "-0", "007", "00", "+1", "1:", ":", "1e3", " 1", "1 ", "1.0", "0x10", "--1"] do
    ensure (intOf? text == none) s!"result integer reader takes {text}"
  ensure (intText (-18014398509481984) == "-18014398509481984" && intText 0 == "0")
    "result integer text"
  ensure (resultLabel == "loops/result-hash/v1:" && resultLabel.utf8ByteSize == 21) "result label"
  let grouped (s : Shown) := SlotValue.groups [("g", .heads [(fixtureId 1, s)] 0)] 0 0
  ensure ((grouped .withheld).unshown == grouped .here) "unshown keeps a grouped head's visibility"

/-- Probe each projection boundary directly, including paths which a JSON
parser or the downstream grouping buckets could otherwise normalize first. -/
def payloadChecks : IO Unit := do
  for op in ["at-least", "below"] do
    ensure ((parseOp .refs "env.about" (op, .int 3)).isOk == false) "bound on envelope set"
    ensure ((parseOp .payload "ports" (op, .int 3)).isOk) "bound on payload field"
  let p ← parsed r#"{"opaque":null,"empty":[],"x":[2,3,2,"2",1.5,[9]]}"#
  let r : Record := { (default : Record) with fields := payloadFields p, id := fixtureId 1 }
  ensure ((payloadFields (.str "held")).isEmpty) "non-object field"
  ensure (r.get "opaque" == some .other) "opaque member dropped"
  ensure (payloadScalar? (.number (Number.ofScientific false 80 0)) == some (.int 80)) "binary64 whole"
  ensure (r.get "x" == some (.many [.int 2,.int 3,.int 2,.str "2"])) "immediate list members"
  let test (f : String) (op : Op) := (⟨.payload f, op⟩ : Cond).eval r
  ensure (test "opaque" .present == .yes && test "empty" .present == .yes) "written presence"
  ensure (test "x" (.is (.val (.int 3))) == .yes) "later list member"
  ensure (test "x" (.atLeast (.val (.int 3))) == .no) "list lower bound"
  ensure (test "x" (.below (.val (.int 3))) == .no) "list upper bound"
  ensure ((By.payload "x").keys r == some ["2","3"]) "distinct converted keys"
  ensure ((By.payload "x").keys r.withhold == none) "unavailable group key"
  ensure (r.num "x" == none) "list numeric read"
  let q := { r with id := fixtureId 2, fields := [("x", .many [.int 2])] }
  let c : Cond := ⟨.payload "x", .is (.val (.int 3))⟩
  ensure (c.onThing [r,q] == .unknown) "setter agreement"

def specIdentityChecks : IO Unit := do
  let hash := "spec:gs9soetlnq1fqp17th66fedrl0oecag0uf101fm5ov2c46ss73ag"
  let .ok sp := Spec.ofJson Tick.backups | throw (IO.userError "backups did not parse")
  ensure (sp.hash sha256 Rules.current == hash) "backups spec hash"
  for j in [Json.int 1, .str "", .str "0", .str "01", .str "+1", .str " 1", .str "١"] do
    ensure (Rules.ofJson? j).isNone "invalid rules version"
  let big := "123456789012345678901234567890123456789012345678901234567890"
  ensure (((Rules.ofJson? (.str big)).map Rules.text) == some big) "arbitrary-width rules version"
  let some ten := Rules.ofJson? (.str "10") | throw (IO.userError "version 10")
  let some two := Rules.ofJson? (.str "2") | throw (IO.userError "version 2")
  ensure (compare two ten == .lt && sp.hash sha256 two != hash) "numeric rules order and binding"
  -- Exercise the parser as well as the span projection on an unavailable payload.
  ensure (Field.ofName "env.when" == some (.env .when)) "window field name"
  ensure ((By.ofName "env.when").isNone) "window admitted as a by field"
  let day := claim "2026-11-21"
  let later := claim "2026-11-22"
  let record : Record :=
    { id := fixtureId 1, kind := "backup", observer := "sl:laptop", prev := [],
      emitted := stamp "2026-11-22T00:00:00Z", occurred := some day, shown := .withheld }
  for (op, expected) in [(Op.is (.time day), Tri.yes), (.not (.time day), .no),
      (.present, .yes), (.absent, .no), (.atLeast (.time day), .yes), (.below (.time later), .yes)] do
    ensure ((Cond.mk (.env .when) op).eval record == expected) "window operators on the occurrence span"
  ensure ((Cond.mk (.env .when) (.below (.time later))).eval {record with occurred := none} == .no)
    "window fallback at the excluded upper bound"

def tickPayloadChecks : IO Unit := do
  let .ok sp := Spec.ofJson Tick.backups | throw (IO.userError "backups spec")
  let records : List Record := []
  let p : TickPayload := ⟨Rules.current, sp, [], [], sp.run records⟩
  let j := p.toJson sha256
  let .ok decoded := readTickPayload sha256 Rules.current j | throw (IO.userError "empty-cover tick")
  ensure (decoded == p) "empty-cover tick payload"
  let other : Rules := ⟨2, by decide⟩
  let foreign := ({p with rules := other} : TickPayload).toJson sha256
  ensure (readTickMembers sha256 Rules.current foreign).isOk "foreign rules members"
  let repeated := Json.obj [("unknown", .arr [.null, .obj [("k", .null), ("k", .bool true)]])]
  unless readTickMembers sha256 Rules.current repeated matches .error ⟨"tick-member-duplicate", "unknown[1].k"⟩ do
    throw (IO.userError "nested duplicate lost its path before member validation")
  let alternate := "res:" ++ base32 (sha256 (utf8 "loops/result-hash/v2:" ++ resultBytes p.result))
  let changed := match foreign with
    | .obj ms => Json.obj (ms.map fun (k,v) => (k, if k == "result-hash" then .str alternate else v))
    | x => x
  ensure (readTickMembers sha256 Rules.current changed).isOk "newer result hash judged by today's label"
  unless readTickMembers sha256 other changed matches .error ⟨"tick-result-hash-mismatch", "result-hash"⟩ do
    throw (IO.userError "own-version result hash unchecked")
  ensure (match readTickPayload sha256 Rules.current foreign with
    | .error (.otherRules v) => v == other | _ => false) "foreign rules entered the slot reader"
  let s : Spec := { vertex := "limited", slots := [⟨"rows", .collect none (some 1), [], none⟩] }
  let claim : SlotValue := .heads [(1, .withheld), (1, .erased)] 7
  let .ok r := readResult s (resultJson [("rows", claim)]) | throw (IO.userError "repeated listed id or limit")
  ensure (r == [("rows", claim.unshown)]) "result read lost list entries or unread count"
  let wrong := Json.obj [("rows", .obj [("heads", .arr []), ("unread", .str "-1")])]
  ensure (refused (readResult s wrong)) "negative natural in result"
  IO.println "tick payload and result reader checks pass"

def tickCheckChecks : IO Unit := do
  let .ok sp := Spec.ofJson Tick.backups | throw (IO.userError "backups spec")
  let r : Record := { id := 1, kind := "backup", observer := "o:test", prev := [], emitted := stampAt 0 }
  let e := { r with id := 2, erases := [1] }
  -- The check stops on the missing opening. Exercise the claim itself: an
  -- eraser outside C must not act while settling the forms inside C.
  let opened := { r with fields := [("mb", .one (.int 4))] }
  ensure (resultBytes (tickRun sp [r.id] [] [opened, e]) == resultBytes (sp.run [opened]))
    "a claim settled an eraser outside its cover"
  let p := makeTick sp [r]
  let subject := fun (p : TickPayload) => TickSubject.mk "tick" (some (p.toJson sha256))
  ensure ((checkTick sha256 Rules.current (subject p) [r, e]).answer == .missing ⟨0, 1⟩)
    "a local eraser hides even an opening still in a form"
  let erased := makeTick sp [r, e]
  ensure (erased.withheld == [1]) "the writer missed a covered erasure"
  ensure ((checkTick sha256 Rules.current (subject {erased with withheld := []}) [e]).answer ==
    .notTick ⟨"tick-withheld-erased", "withheld"⟩) "missing records hid an eraser's evidence"
  let flat : Hash := fun _ => List.replicate 32 0
  let wrong := {p with result := sp.run []}
  let sameHash := resultHash flat p.result
  ensure ((checkTick flat Rules.current ⟨"tick", some (wrong.toJson flat)⟩ [r]).answer ==
    .differs sameHash sameHash) "the verdict compared digests instead of bytes"
  IO.println "tick claim and check runtime checks pass"

def main (args : List String) : IO Unit := do
  specIdentityChecks
  tickPayloadChecks
  tickCheckChecks
  payloadChecks
  canonicalTextChecks
  rulingChecks
  let [root] := args | throw (IO.userError "expected repository root")
  let table ← IO.FS.readFile (root ++ "/vectors/standard/es6-numbers.txt")
  let mut count := 0
  for line in table.splitOn "\n" do
    if line.isEmpty then continue
    let [hex, expected] := line.splitOn "," | throw (IO.userError "invalid number table")
    let some digits := hex.toList.mapM Read.hexVal | throw (IO.userError "invalid bit pattern")
    let bits := (digits.foldl (fun n d => 16 * n + d) 0).toUInt64
    let n : Number := ⟨bits⟩
    ensure (n.canon == expected) s!"canonical bits {hex}: {n.canon}, expected {expected}"
    let j ← parsed expected
    let actual := match j with
      | .int i => Number.ofScientific (i < 0) i.natAbs 0 |>.bits
      | .number x => x.bits
      | _ => 0
    ensure (actual == bits || (n.magnitude == 0 && actual == 0)) s!"decimal roundtrip {hex}"
    count := count + 1
  for (text, expected) in [("1.0", 1), ("1e2", 100), ("-0", 0), ("-0.0", 0),
      ("1e-400", 0), ("-1e-400", 0), ("9007199254740991", 9007199254740991),
      ("9007199254740990.5", 9007199254740990)] do
    let j ← parsed text
    ensure (match j with | .int i => i == expected | _ => false) s!"safe-whole normalization {text}"
  for text in ["01", "+1", "1.", "1e", "1e+", ".5", "NaN", "Infinity"] do
    ensure (refused (Read.parseJson text)) s!"invalid RFC number survives: {text}"
  -- Exact output integers use the same grammar, member order, and text decoder.
  for text in ["9007199254740993", "-9007199254740993",
      "123456789012345678901234567890123456789012345678901234567890", "-0"] do
    let some expected := text.toInt? | throw (IO.userError "invalid exact test integer")
    ensure (match Read.parseExactIntegers text with | .ok (.int i) => i == expected | _ => false)
      s!"exact output integer {text}"
  for text in ["9007199254740993.0", "9007199254740993e0", "1.0", "1e2",
      "1.5", "-0.0", "1e-999", "1e999", "-1e999"] do
    ensure (reprStr (Read.parseExactIntegers text) == reprStr (Read.parseJson text))
      s!"exact mode changes non-integer spelling {text}"
  for text in ["01", "+1", "1.", "1e", "1e+", ".5", "NaN", "Infinity",
      "{\"a\":1,}", "[1,]", "\"\\ud800\"", "\"\\udc00\""] do
    ensure (reprStr (Read.parseExactIntegers text) == reprStr (Read.parseJson text))
      s!"exact mode changes syntax refusal {text}"
  let text := "{\"雪\\ud83d\\ude00\":[9007199254740993],\"雪😀\":-9007199254740993}"
  ensure (match Read.parseExactIntegers text with
    | .ok (.obj [(k, .arr [.int i]), (k', .int j)]) =>
      k == "雪😀" && k' == k && i == 9007199254740993 && j == -9007199254740993
    | _ => false) "exact mode changes order, duplicates, or Unicode"
  let .ok rounded := Read.parseJson "9007199254740993" | throw (IO.userError "input integer fails to parse")
  ensure (match rounded with | .number n => n.bits == (Number.ofScientific false 9007199254740992 0).bits | _ => false)
    "input API stops rounding unsafe integers once to binary64"
  let env : Env := { kind := "note", observer := "o:test", emitted := timeText 0 }
  let salt := List.replicate 16 (0 : UInt8)
  for text in ["9007199254740992", "-9007199254740992", "9007199254740991.5",
      "-9007199254740991.5", "1e30", "-1e30", "1.7976931348623157e308", "-1.7976931348623157e308"] do
    let j ← parsed text
    ensure (!j.inRange) s!"layer0 admits unsafe magnitude {text}"
    ensure (writeProblem env salt j == some "number-out-of-range") s!"writer admits unsafe magnitude {text}"
    ensure (!(refused (Read.validateField "payload" j))) s!"payload decoder refuses {text}"
    ensure (refused ((parseBound .payload "bound" j).mapError (·.code))) s!"bound admits {text}"
    ensure (refused ((parseArg .payload "arg" j).mapError (·.code))) s!"argument admits {text}"
  for text in ["9007199254740991", "-9007199254740991", "9007199254740991.1",
      "-9007199254740991.1", "1.5", "-1.5", "1424953923781206.25", "5e-324", "-0"] do
    let j ← parsed text
    ensure j.inRange s!"layer0 refuses within-range value {text}"
    ensure (writeProblem env salt j == none) s!"writer refuses within-range value {text}"
  for j in [Json.int 9007199254740992, .int (-9007199254740992),
      .obj [("largest", .int 9007199254740991),
        ("deep", .arr [.obj [("n", .int (-9007199254740992))]])],
      .obj [("__proto__", .int 9007199254740992)]] do
    ensure (!j.inRange) "layer0 admits raw unsafe integer"
    ensure (writeProblem env salt j == some "number-out-of-range") "writer admits raw unsafe integer"
  for text in ["1e400", "-1e400"] do
    let j ← parsed text
    ensure (!j.inRange) s!"layer0 admits overflow {text}"
    ensure (writeProblem env salt j == some "number-out-of-range") s!"writer admits overflow {text}"
    ensure (refused (Read.validateField "payload" j)) s!"payload decoder admits {text}"
    ensure (refused ((parseBound .payload "bound" j).mapError (·.code))) s!"bound admits {text}"
    ensure (refused ((parseArg .payload "arg" j).mapError (·.code))) s!"argument admits {text}"
  for bits in [0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000000] do
    let j := Json.number ⟨bits⟩
    ensure (writeProblem env salt j == some "number-out-of-range") "writer admits nonfinite bits"
    ensure (refused (Read.validateField "payload" j)) "payload decoder admits nonfinite bits"
  ensure (firstRefusal ["number-out-of-range", "kind-empty"] == some "kind-empty")
    "global write priority changes"
  for (text, bits, expected) in [
      ("1424953923781206.25", 0x43143ff3c1cb0959, "1424953923781206.2"),
      ("-1424953923781206.25", 0xc3143ff3c1cb0959, "-1424953923781206.2")] do
    let j ← parsed text
    ensure (match j with | .number n => n.bits == bits && n.canon == expected | _ => false)
      s!"shortest midpoint tie {text}"
  for op in ["is", "not", "at-least", "below"] do
    for (text, accepted) in [("1.0", true), ("1e2", true), ("1.5", false),
        ("9007199254740992", false), ("-9007199254740992", false)] do
      let spec ← parsed ("{\"vertex\":\"x\",\"from\":[{\"store\":\".\"}],\"fold\":{\"c\":{\"count\":true,\"where\":{\"est\":{\"" ++ op ++ "\":" ++ text ++ "}}}}}")
      ensure (!(refused ((Spec.ofJson spec).mapError (·.code))) == accepted) s!"raw spec {op} {text}"
  let fraction ← parsed "1.5"
  ensure (fraction.inRange && writeProblem env salt fraction == none) "layer0 refuses finite fraction"
  ensure (refused ((parseArg .payload "arg" fraction).mapError (·.code))) "test accepts nonwhole argument"
  let base : Record := { id := fixtureId 1, kind := "note", observer := "o:test", about := ["t:one"], prev := [], emitted := stampAt 0, fields := [("n", .one (.int 1))] }
  let .obj kvs ← parsed (J.render 0 (jRecord base)) | throw (IO.userError "record is not an object")
  let changed (k : String) (v : Json) := Json.obj (kvs.map fun (name, old) => (name, if name == k then v else old))
  for (key, val) in [("value", Json.str ""), ("n", Json.int 0)] do
    ensure (refused (Read.record "record" (.obj ((key, val) :: kvs)))) s!"reader accepts flat {key}"
  let empty := { base with fields := [] }
  let .ok control := Read.record "record" (changed "fields" (.obj [("value", .str ""), ("n", .int 0)]))
    | throw (IO.userError "decoder refuses written empty text or zero")
  ensure (control.get "value" == some (.one (.str "")) && control.num "n" == some 0) "written empty text or zero lost"
  ensure (({ control with shown := .withheld } : Record).num "n" == none) "numeric field read from withheld record"
  ensure (empty.get "value" == none && empty.num "n" == none) "missing field reads as a default"
  let .ok fractional := Read.record "record" (changed "fields" (.obj [("n", fraction)]))
    | throw (IO.userError "decoder refuses nonwhole n")
  ensure (fractional.num "n" == none) "numeric slot reads a fraction"
  let unsafeInt := { base with fields := [("est", .one (.int 9007199254740992))] }
  ensure (unsafeInt.num "est" == none) "numeric field reads unsafe integer"
  let unsafeValue ← parsed "9007199254740992"
  let .ok unsafeRecord := Read.record "record" (changed "fields" (.obj [("est", unsafeValue)]))
    | throw (IO.userError "decoder refuses unsafe numeric field")
  ensure (unsafeRecord.num "est" == none) "numeric field reads unsafe whole"
  let unsafeSum := (Sum.on "est").select (fun _ => .yes) [unsafeRecord]
  ensure (unsafeSum.unread == 1) "unsafe numeric field does not count unread"
  let .ok unsafeN := Read.record "record" (changed "fields" (.obj [("n", unsafeValue)]))
    | throw (IO.userError "decoder refuses unsafe n")
  ensure (unsafeN.num "n" == none && ((Sum.on "n").select (fun _ => .yes) [unsafeN]).unread == 1)
    "unsafe n does not count unread"
  let sum := (Sum.on "n").select (fun _ => .yes) [base, fractional]
  ensure (sum.value == 1 && sum.unread == 1) "fraction does not count unread"
  let fixtureSum := Sum.acc.select (fun _ => .yes) [base, fractional]
  ensure (fixtureSum.value == 1 && fixtureSum.unread == 1) "fixture sum reads a fraction"
  ensure (!(Max.acc.readable fractional) && !(Min.acc.readable fractional)) "fixture extrema read a fraction"
  let maxHeads := LatestBy.maxHeads ⟨[], [base, fractional]⟩
  ensure (maxHeads.value == some 1 && maxHeads.unread == 1) "heads max reads a fraction"
  for about in [["untyped"], ["t:"], ["t:one", "bad"]] do
    ensure (refused (Read.record "record" (changed "about" (.arr (about.map .str))))) "untyped about survives"
  ensure (!(refused (Read.record "record" (.obj kvs)))) "typed about control fails"
  let sect : Lens.Section := ⟨"one".toList, ["body".toList]⟩
  ensure (Lens.parse (Lens.render [sect]) == some [sect]) "valid lens control fails"
  ensure ((Lens.parse (Lens.render [⟨[], []⟩])).isNone) "empty section id survives"
  ensure ((Lens.parse (Lens.render [sect, sect])).isNone) "duplicate section id survives"
  for of in ["untyped", "t:"] do
    ensure (refused (Read.record "record" (.obj (kvs ++ [("of", .str of)])))) "untyped of survives"
  ensure (!(refused (Read.record "record" (.obj (kvs ++ [("of", .str "t:a")]))))) "typed of control fails"
  -- `about` makes no head: a record that mentions two things is a head of its
  -- one thing, and its successor there supersedes it everywhere
  let a := { base with id := fixtureId 11, of := some "t:a", about := ["t:a", "t:b"] }
  let b := { base with id := fixtureId 12, of := some "t:a", prev := [fixtureId 11] }
  let hs := wholeHeads [a, b]
  ensure ((hs.map (·.id)) == [fixtureId 12]) "a superseded record that mentions another thing stays a head"
  ensure ((wholeHeads [a]).length == 1) "a record that mentions two things heads more than once"
  ensure (Latest.winner (Latest.fold [a, b]) == some b) "latest misses newest whole-view head"
  ensure (Latest.others (Latest.fold [a, b]) == []) "about makes a whole-view conflict"
  -- heads are per thing: a `prev` naming a record of another thing supersedes nothing
  let c := { base with id := fixtureId 13, of := some "t:b", prev := [fixtureId 11] }
  ensure (((wholeHeads [a, c]).map (·.id)) == [fixtureId 13, fixtureId 11])
    "global supersession removes another thing's head"
  resultCodecChecks
  IO.println s!"runtime checks pass; {count} RFC binary64 bit patterns pass"

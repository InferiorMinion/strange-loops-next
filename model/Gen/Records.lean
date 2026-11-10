import Loops
import Gen.Json
import Gen.Encode
import Gen.Ed25519
import Gen.Read

/-!
# Record vectors

Layer 0 end to end: canonical bytes, commitment, id, erasure and signature,
plus the envelopes the core refuses. Hashes are SHA-256 and signatures
Ed25519, both checked against their standards before anything is written.
-/

namespace Gen.Records

open Loops

def H : Hash := sha256

partial def ofJson : Json → J
  | .null => .null
  | .bool b => .bool b
  | .int i => .int i
  | .number n => .number n
  | .str s => .str s
  | .arr xs => .arr (xs.map ofJson)
  | .obj kvs => .obj (kvs.map fun (k, v) => (k, ofJson v))

/-- The envelope as a writer hands it over: sets in the writer's order. -/
def envInput (e : Env) : J :=
  let opt (k : String) : Option String → List (String × J)
    | some v => [(k, .str v)]
    | none => []
  let set (k : String) (xs : List String) : List (String × J) :=
    if xs.isEmpty then [] else [(k, .arr (xs.map .str))]
  .obj ([("kind", .str e.kind), ("observer", .str e.observer), ("emitted", .str e.emitted)] ++
    opt "occurred" e.occurred ++ set "about" e.about ++ opt "of" e.of ++ set "prev" e.prev ++
    opt "under" e.under ++ set "erases" e.erases)

def textBytes (s : String) : List UInt8 := s.toUTF8.toList

def seedFor (name : String) : List UInt8 := sha256 (textBytes ("loops vector key " ++ name))
def saltFor (name : String) : List UInt8 := (sha256 (textBytes ("loops vector salt " ++ name))).take 16

def keyText (seed : List UInt8) : String := "k:" ++ base32 (Ed25519.publicKey seed)

def lineageFor (name : String) : String := "sl:" ++ base32 (sha256 (textBytes ("genesis " ++ name)))

def kyle := lineageFor "kyle"
def wren := lineageFor "wren"

def storedJson (env : Env) (salt : List UInt8) (payload : Json) (seed : List UInt8) : Json :=
  let r := make env salt payload
  let id := r.id H
  .obj [("id", .str id), ("env", env.toJson), ("salt", .str (base32 salt)), ("payload", payload),
    ("sig", .obj [("key", .str (keyText seed)),
      ("ed25519", .str (base32 (Ed25519.sign seed (signingInput id))))])]

def erasedJson (env : Env) (salt : List UInt8) (payload : Json) (seed : List UInt8) : Json :=
  let r := make env salt payload
  let id := r.id H
  .obj [("id", .str id), ("env", env.toJson), ("commit", .str (r.commit H)),
    ("sig", .obj [("key", .str (keyText seed)),
      ("ed25519", .str (base32 (Ed25519.sign seed (signingInput id))))])]

structure Case where
  name : String
  laws : List String
  /-- Envelopes that must all give the same id (sets in different orders, repeats). -/
  envs : List Env
  /-- Payloads that must all give the same commitment (members in different orders). -/
  payloads : List Json
  key : String := "kyle"

def storedVector (c : Case) : Except String J := do
  let env := c.envs.headD default
  let payload := c.payloads.headD .null
  let salt := saltFor c.name
  let seed := seedFor c.key
  let r ← match write env salt payload with
    | .ok w => pure w.stored
    | .error why => throw s!"record/{c.name}: unexpectedly refused: {why}"
  let ids := c.envs.map fun e => idOf H e (r.commit H)
  let commits := c.payloads.map fun p => commitOf H salt p
  if !ids.all (· == r.id H) then throw s!"record/{c.name}: envelope variants disagree"
  if !commits.all (· == r.commit H) then throw s!"record/{c.name}: payload variants disagree"
  if (erase H r).id H != r.id H then throw s!"record/{c.name}: erasure changed the id"
  let id := r.id H
  return .obj [("vector", .str c.name), ("layer", .str "record"), ("laws", .arr (c.laws.map .str)),
    ("source", .str "model"),
    ("input", .obj [("envs", .arr (c.envs.map envInput)), ("payloads", .arr (c.payloads.map ofJson)),
      ("salt", .str (base32 salt)), ("seed", .str (base32 seed))]),
    ("expect", .obj [
      ("env_canonical", .str env.toJson.canon),
      ("payload_canonical", .str payload.canon),
      ("commit", .str (r.commit H)),
      ("id_preimage", .str (idPreimage env (r.commit H))),
      ("id", .str id),
      ("signing_input", .str ("loops/record/v1:" ++ id)),
      ("key", .str (keyText seed)),
      ("ed25519", .str (base32 (Ed25519.sign seed (signingInput id)))),
      ("line", .str (storedJson env salt payload seed).canon),
      ("erased_line", .str (erasedJson env salt payload seed).canon)])]

/-- A payload as written, members in the order given and repeats kept: the
only way a vector can carry a payload with a repeated member name. -/
partial def rawText : Json → String
  | .obj kvs => "{" ++ ",".intercalate (kvs.map fun (k, v) => quote k ++ ":" ++ rawText v) ++ "}"
  | .arr xs => "[" ++ ",".intercalate (xs.map rawText) ++ "]"
  | j => j.canon

def refusedVector (name : String) (env : Env) (payload : Json := .obj [])
    (salt : List UInt8 := saltFor name) : Except String J := do
  match write env salt payload with
  | .ok _ => throw s!"record/{name}: expected a refusal"
  | .error why =>
    return .obj [("vector", .str name), ("layer", .str "record"), ("laws", .arr [.str "record.refuse"]),
      ("source", .str "model"), ("input", .obj [("envs", .arr [envInput env]),
        ("salt", .str (base32 salt)), ("payload_text", .str (rawText payload))]),
      ("expect", .obj [("refused", .str why)])]

/-! ## The answer for a record input

`run` and `gen` both ask this. It lives here so that the vectors whose point
is the input as written (text payloads, several variants) take their `expect`
from the same function that later rereads them. -/

open Gen.Read in
/-- Raw envelope key refusals cross the fixture adapter as protocol refusals. -/
def envelopeOutcome (out : Dec J) : Dec J :=
  match out with
  | .error "envelope-field-unknown" => .ok (.obj [("refused", .str "envelope-field-unknown")])
  | other => other

/-- The refusal of a record input's text payload, if the protocol has one for it. -/
def payloadTextRefusal (kvs : List (String × Json)) : Option String :=
  match kvs.lookup "payload_text" with
  | some (.str t) => Gen.Read.textRefusal t
  | _ => none

open Gen.Read in
/-- `record`: one record written, or a text, raw shape or typed writer refusal.
The envelope comes in the writer's form, perhaps several ways (sets in other
orders, repeats), and the payload as JSON, perhaps several ways (members in
other orders), or as text when it repeats a member name or carries an escape.

The stages, in order:

1. Every envelope's raw keys (`envelope-field-unknown`).
2. A text payload with a lone surrogate (`string-not-unicode`), before any
   envelope value, the salt or the payload's members are looked at.
3. The first envelope and the first payload are written. Their refusal is the
   answer, whatever the other variants are.
4. Once those are written, every other variant must name the same record, and
   every other payload must be writable: a refused payload can share a
   commitment with a written one (a repeated member renders as its first, a
   nonfinite number as `null`), where a refused envelope never shares an id.
   A variant that fails is a fault in the input, not an answer.
5. Only then is the seed read. It is exactly 32 bytes; none is padded or cut. -/
def recordAnswerDecoded (input : Json) : Dec J := do
  let kvs ← members? "input" ["envs", "payloads", "payload_text", "salt", "seed"] input
  let shaped := Gen.Read.envs "input.envs" (← req "input" kvs "envs")
  if let .error "envelope-field-unknown" := shaped then throw "envelope-field-unknown"
  if let some why := payloadTextRefusal kvs then return .obj [("refused", .str why)]
  let envs ← shaped
  let payloads ← match kvs.lookup "payloads", kvs.lookup "payload_text" with
    | some ps, none => list "input.payloads" ps
    | none, some t => do
      match parseJson (← text "input.payload_text" t) with
      | .ok j => pure [j]
      | .error e => bad "input.payload_text" e
    | _, _ => bad "input" "takes one of \"payloads\" and \"payload_text\""
  let salt ← bytes "input.salt" (← req "input" kvs "salt")
  let some e := envs.head? | bad "input.envs" "no envelope given"
  let some payload := payloads.head? | bad "input.payloads" "no payload given"
  match write e salt payload with
  | .error why => pure (.obj [("refused", .str why)])
  | .ok w =>
    let r := w.stored
    let id := r.id H
    for (e', i) in envs.zip (List.range envs.length) do
      if idOf H e' (r.commit H) != id then bad (idx "input.envs" i) "names another record than input.envs[0]"
    for (p, i) in payloads.zip (List.range payloads.length) do
      if let some why := writeProblem e salt p then
        bad (idx "input.payloads" i) s!"is refused ({why}) where input.payloads[0] is written"
      if commitOf H salt p != r.commit H then bad (idx "input.payloads" i) "commits to another payload than input.payloads[0]"
    let seed ← bytes "input.seed" (← req "input" kvs "seed")
    if seed.length != 32 then bad "input.seed" s!"holds {seed.length} bytes; a signing seed is exactly 32"
    pure (.obj [
      ("env_canonical", .str e.toJson.canon),
      ("payload_canonical", .str payload.canon),
      ("commit", .str (r.commit H)),
      ("id_preimage", .str (idPreimage e (r.commit H))),
      ("id", .str id),
      ("signing_input", .str ("loops/record/v1:" ++ id)),
      ("key", .str (keyText seed)),
      ("ed25519", .str (base32 (Ed25519.sign seed (signingInput id)))),
      ("line", .str (storedJson e salt payload seed).canon),
      ("erased_line", .str (erasedJson e salt payload seed).canon)])

def recordAnswer (input : Json) : Gen.Read.Dec J := envelopeOutcome (recordAnswerDecoded input)

/-- A vector whose answer is computed from its input as written. `want` is the
refusal the case is there to pin, or `none` for a record that is written. -/
def answeredVector (name : String) (laws : List String) (input : J) (want : Option String) :
    Except String J := do
  let raw ← (Gen.Read.parseJson (input.render 0)).mapError (s!"record/{name}: input does not read back: " ++ ·)
  let expect ← (recordAnswer raw).mapError (s!"record/{name}: no answer: " ++ ·)
  let got := match expect with
    | .obj [("refused", .str why)] => some why
    | _ => none
  if got != want then throw s!"record/{name}: expected {want}, the model answers {got}"
  return vector name "record" laws input expect

/-! ## Cases -/

def note (emitted : String := "2026-10-21T09:30:00Z") : Env :=
  { kind := "note", observer := kyle, emitted }

def r1Id : String :=
  (make (note) (saltFor "01-minimal") (.obj [("body", .str "hello")])).id H

def grantId : String :=
  (make { kind := "grant", observer := kyle, emitted := "2026-10-20T12:00:00Z" }
    (saltFor "grant") (.obj [("to", .str wren)])).id H

def storedCases : List Case := [
  { name := "01-minimal", laws := ["record.id", "record.commit", "record.sign"],
    envs := [note], payloads := [.obj [("body", .str "hello")]] },
  { name := "02-full-envelope", laws := ["record.id", "record.sets"],
    envs := [{
      kind := "decision", observer := wren, emitted := "2026-10-21T08:15:02.25Z",
      occurred := some "2026-10-17T09:14:03Z/2026-10-17T09:18:13Z", about := ["topic:auth", "url:https://example.com/x"],
      of := some r1Id, prev := [r1Id], under := some grantId }],
    payloads := [.obj [("choice", .str "device flow"), ("why", .arr [.str "no browser on the box"])]],
    key := "wren" },
  { name := "03-sets-are-sets", laws := ["record.sets"],
    envs := [
      { note with about := ["topic:auth", "svc:api", "url:https://example.com/x"] },
      { note with about := ["url:https://example.com/x", "topic:auth", "svc:api"] },
      { note with about := ["svc:api", "topic:auth", "svc:api", "url:https://example.com/x", "topic:auth"] }],
    payloads := [.obj [("body", .str "same set, any order")]] },
  { name := "04-member-order", laws := ["record.member-order"],
    envs := [note],
    payloads := [
      .obj [("b", .int 2), ("a", .int 1), ("c", .obj [("y", .bool true), ("x", .null)])],
      .obj [("c", .obj [("x", .null), ("y", .bool true)]), ("a", .int 1), ("b", .int 2)]] },
  { name := "05-unicode", laws := ["record.canonical"],
    envs := [note],
    payloads := [.obj [
      ("text", .str "tab\there, quote \" and backslash \\, newline\nand \x01 control"),
      ("emoji", .str "😀 and ש"), ("line-sep", .str "a b"),
      (String.singleton (Char.ofNat 0x1F600), .str "astral key"),
      (String.singleton (Char.ofNat 0xFB33), .str "BMP key above the surrogates"),
      ("€", .str "euro key")]] },
  { name := "06-values", laws := ["record.canonical"],
    envs := [note],
    payloads := [.obj [("null", .null), ("t", .bool true), ("f", .bool false), ("zero", .int 0),
      ("neg", .int (-42)), ("max-safe", .int 9007199254740991), ("empty-arr", .arr []),
      ("empty-obj", .obj []), ("nested", .arr [.arr [.int 1, .arr []], .obj [("k", .str "")]])]] },
  { name := "07-payload-not-object", laws := ["record.canonical"],
    envs := [note], payloads := [.arr [.str "a payload can be any JSON value", .int 7]] },
  { name := "08-erasure", laws := ["record.erasure"],
    envs := [{
      kind := "note", observer := lineageFor "dev2", emitted := "2026-03-04T15:12:44Z",
      about := [r1Id] }],
    payloads := [.obj [("body", .str "Called the customer back at 555-0100")]], key := "dev2" },
  { name := "09-last-second", laws := ["record.emitted"],
    envs := [note "2016-12-31T23:59:59Z"], payloads := [.obj []] },
  -- `__proto__` is a member name like any other, though a JavaScript object
  -- assigned one sets its prototype instead
  { name := "10-proto-member", laws := ["record.canonical"],
    envs := [note],
    payloads := [.obj [("__proto__", .obj [("a", .int 1)]), ("b", .int 2)]] },
  { name := "11-erases", laws := ["record.id", "record.sets"],
    envs := [
      { note with of := some r1Id, prev := [r1Id], erases := [r1Id, grantId] },
      { note with of := some r1Id, prev := [r1Id], erases := [grantId, r1Id, grantId] }],
    payloads := [.obj [("body", .str "hello, without the number")]] },
  { name := "12-negative-exact-boundary", laws := ["record.canonical"],
    envs := [note], payloads := [.int (-9007199254740991)] },
  { name := "13-nine-digit-fraction", laws := ["record.emitted"],
    envs := [note "2026-10-21T00:00:00.000000001Z"], payloads := [.obj []] },
  -- `of` names a thing by a natural key; a root may state it
  { name := "14-of-natural-key", laws := ["record.id", "record.of"],
    envs := [{ note with of := some "task:auth-flow", about := ["topic:auth"] }],
    payloads := [.obj [("body", .str "start the auth flow")]] },
  { name := "38-native-shortest-tie", laws := ["record.canonical"],
    envs := [note], payloads := [.obj [
      ("positive", .number ⟨0x43143ff3c1cb0959⟩),
      ("negative", .number ⟨0xc3143ff3c1cb0959⟩)]] }
]

def refusedCases : List (String × Env) := [
  ("20-kind-empty", { note with kind := "" }),
  ("21-observer-untyped", { note with observer := "kyle" }),
  ("22-emitted-no-zone", note "2026-10-21T05:30:00"),
  ("23-emitted-lowercase", note "2026-10-21t05:30:00z"),
  ("24-emitted-no-such-day", note "2026-02-29T00:00:00Z"),
  ("25-emitted-refused-offset", note "2026-10-21T05:30:00+24:00"),
  ("26-about-untyped", { note with about := ["auth"] }),
  ("27-prev-not-record", { note with prev := ["topic:auth"] }),
  ("28-under-not-record", { note with under := some "r:short" }),
  ("33-erases-not-record", { note with erases := ["topic:auth"] }),
  ("34-occurred-duration", { note with occurred := some "2026-10-17T09:14/PT4M" }),
  ("35-observer-prefix-start", { note with observer := "1:x" }),
  ("36-observer-prefix-tail", { note with observer := "a!:x" }),
  ("40-of-untyped", { note with of := some "auth" }),
  ("41-of-missing", { note with prev := [r1Id] }),
  ("42-about-before-of", { note with about := ["auth"], of := some "auth" }),
  ("43-of-before-prev", { note with of := some "auth", prev := ["topic:auth"] }),
  ("44-erases-before-of-missing", { note with prev := [r1Id], erases := ["topic:auth"] })] ++
  ["+00:00", "-00:00", "+01:00", "-23:59"].zipIdx.map (fun (z, i) =>
    (s!"r4-offset-{i}", note ("2026-10-21T00:00:00" ++ z))) ++
  ["2026-10-21T00:00:60Z", "2016-12-31T23:59:60Z", "2026-10-21T00:00:00.0Z",
   "2026-10-21T00:00:00.50Z", "2026-10-21T00:00:00.500Z", "2026-10-21T00:00:00.1234567891Z", "2026-10-21T00:00:00.Z"].zipIdx.map
    (fun (s, i) => (s!"r4-emitted-{i}", note s)) ++
  ["2026-10-21T00:00:00+00:00", "2026-10-21T00:00:60Z", "2026-10-21T00:00:00.50Z",
   "2026-10-21T00:00:00.1234567891Z"].zipIdx.flatMap (fun (s, i) =>
    [(s!"r4-occurred-single-{i}", { note with occurred := some s }),
     (s!"r4-occurred-start-{i}", { note with occurred := some (s ++ "/2027") }),
     (s!"r4-occurred-stop-{i}", { note with occurred := some ("2026/" ++ s) }),
     (s!"r4-occurred-open-{i}", { note with occurred := some (s ++ "/..") })]) ++
  ["A", "w", "x", "y", "z", "1"].zipIdx.flatMap (fun (last, i) =>
    let bad := "r:" ++ String.ofList (List.replicate 51 '0') ++ last
    [(s!"r1-prev-{i}", { note with prev := [bad] }),
     (s!"r1-under-{i}", { note with under := some bad }),
     (s!"r1-erases-{i}", { note with erases := [bad] })]) ++ [
  ("r1-id-short", { note with prev := ["r:" ++ String.ofList (List.replicate 51 '0')] }),
  ("r1-id-long", { note with prev := ["r:" ++ String.ofList (List.replicate 53 '0')] })
]

/-- Unsafe magnitudes and repeated member names are refused at any depth under a good envelope. -/
def refusedPayloads : List (String × Json) := [
  ("29-number-out-of-range", .obj [("largest", .int 9007199254740991),
    ("deep", .arr [.obj [("n", .number (Number.ofScientific true 9007199254740992 0))]])]),
  ("32-proto-out-of-range", .obj [("__proto__", .number (Number.ofScientific false 9007199254740992 0))]),
  ("39-huge-out-of-range", .obj [
    ("positive", .number (Number.ofScientific false 1 30)),
    ("negative", .number (Number.ofScientific true 1 30)),
    ("max", .number ⟨0x7fefffffffffffff⟩),
    ("min", .number ⟨0xffefffffffffffff⟩)]),
  ("31-member-duplicate", .obj [("body", .str "hello"),
    ("meta", .obj [("tag", .str "a"), ("tag", .str "b")])])
]

/-- A salt that isn't 16 bytes. An empty salt would let anyone confirm a guess
about a payload from its commitment. -/
def shortSalt : List UInt8 := (saltFor "30-salt-short").take 15

/-! ## Inputs as written: text payloads and variants (D2, D3, D5) -/

def rawNote : J := envInput note

def withExtra (j : J) : J := match j with
  | .obj kvs => .obj (("extra", .null) :: kvs)
  | other => other

/-- A record input whose payload is text. -/
def textInput (name : String) (text : String) (env : J := rawNote)
    (salt : List UInt8 := saltFor name) (seed : Bool := false) : J :=
  .obj ([("envs", .arr [env]), ("salt", .str (base32 salt)), ("payload_text", .str text)] ++
    if seed then [("seed", J.str (base32 (seedFor "kyle")))] else [])

/-- A record input listing variants. -/
def variantInput (name : String) (envs : List J) (payloads : List J) : J :=
  .obj [("envs", .arr envs), ("payloads", .arr payloads), ("salt", .str (base32 (saltFor name)))]

def overflow (negative : Bool := false) : J := .number (Number.ofScientific negative 1 999)

/-- Lone surrogates in a text payload, wherever they stand, and what they precede and follow. -/
def unicodeCases : List (String × J × String) :=
  let t (name text : String) := (name, textInput name text, notUnicode)
  [ t "d2-lone-high" "{\"body\":\"\\ud800\"}",
    t "d2-lone-low" "{\"body\":\"\\udc00\"}",
    t "d2-low-then-high" "{\"body\":\"\\udc00\\ud800\"}",
    t "d2-high-then-text" "{\"body\":\"\\ud83dx\"}",
    t "d2-high-then-escape" "{\"body\":\"\\ud83d\\u0041\"}",
    t "d2-high-then-pair" "{\"body\":\"\\ud83d\\ud83d\\ude00\"}",
    t "d2-member-name" "{\"\\udc00\":1}",
    t "d2-nested" "[{\"a\":[\"\\ud800\"]}]",
    t "d2-before-member-duplicate" "{\"a\":1,\"a\":2,\"b\":\"\\ud800\"}",
    t "d2-before-number" "{\"n\":1e999,\"b\":\"\\ud800\"}",
    ("d2-before-salt", textInput "d2-before-salt" "\"\\ud800\"" (salt := shortSalt), notUnicode),
    ("d2-before-kind", textInput "d2-before-kind" "\"\\ud800\"" (env := envInput { note with kind := "" }), notUnicode),
    ("d2-shape-first", textInput "d2-shape-first" "\"\\ud800\"" (env := withExtra rawNote), "envelope-field-unknown") ]

/-- A number past binary64's range: the writer's number check refuses it, in its usual place. -/
def overflowCases : List (String × J × String) :=
  let t (name text code : String) := (name, textInput name text, code)
  [ t "d3-overflow" "{\"n\":1e999}" "number-out-of-range",
    t "d3-overflow-negative" "[-1e999]" "number-out-of-range",
    t "d3-overflow-exponent" "{\"deep\":[{\"n\":1.5E+400}]}" "number-out-of-range",
    ("d3-overflow-value", variantInput "d3-overflow-value" [rawNote] [.obj [("n", overflow)]], "number-out-of-range"),
    ("d3-overflow-value-negative", variantInput "d3-overflow-value-negative" [rawNote] [.arr [overflow true]],
      "number-out-of-range"),
    t "d3-member-duplicate-first" "{\"a\":1e999,\"a\":2}" "member-duplicate",
    ("d3-salt-first", textInput "d3-salt-first" "1e999" (salt := shortSalt), "salt-not-16-bytes"),
    ("d3-kind-first", textInput "d3-kind-first" "1e999" (env := envInput { note with kind := "" }), "kind-empty"),
    ("d3-shape-first", textInput "d3-shape-first" "1e999" (env := withExtra rawNote), "envelope-field-unknown") ]

/-- Variants that don't agree: the first envelope and first payload decide. -/
def variantCases : List (String × J × String) :=
  let noKind := envInput { note with kind := "" }
  let untyped := envInput { note with observer := "kyle" }
  let big : J := .number (Number.ofScientific false 9007199254740992 0)
  let v (name : String) (envs payloads : List J) (code : String) :=
    (name, variantInput name envs payloads, code)
  [ v "d5-refused-then-written" [noKind, rawNote] [.obj []] "kind-empty",
    v "d5-differently-refused-0" [untyped, noKind] [.obj []] "observer-not-typed",
    v "d5-differently-refused-1" [noKind, untyped] [.obj []] "kind-empty",
    v "d5-payload-refused-then-written" [rawNote] [big, .int 1] "number-out-of-range",
    v "d5-first-pair" [rawNote, noKind] [big, .int 1] "number-out-of-range",
    v "d5-envelope-before-payload" [noKind, rawNote] [big, .int 1] "kind-empty" ]

def writtenVectors : Except String (List (String × J)) := do
  let unicode ← unicodeCases.mapM fun (n, input, code) => do
    pure (n, ← answeredVector n ["record.unicode", "record.refuse"] input (some code))
  let paired ← answeredVector "d2-paired-escape" ["record.unicode", "record.canonical"]
    (textInput "d2-paired-escape" "{\"body\":\"\\ud83d\\ude00\",\"\\ud83d\\ude00\":\"\\u00e9\"}" (seed := true)) none
  let overflows ← overflowCases.mapM fun (n, input, code) => do
    pure (n, ← answeredVector n ["write.overflow", "record.refuse"] input (some code))
  let variants ← variantCases.mapM fun (n, input, code) => do
    pure (n, ← answeredVector n ["record.refuse"] input (some code))
  return unicode ++ [("d2-paired-escape", paired)] ++ overflows ++ variants

/-! ## Standards the reference code must meet before it writes anything -/

def selfCheck : Except String Unit := do
  let unknown := Json.obj [("extra", .null)]
  for fields in [[], [("kind", .int 7)]] do
    match Gen.Read.env "env" (.obj (fields ++ [("extra", .null)])) with
    | .error "envelope-field-unknown" => pure ()
    | _ => throw "R6 raw keys must precede envelope values"
  for envs in [[Json.int 7, unknown], [unknown, .int 7],
      [.obj [("kind", .int 7)], unknown], [unknown, .obj [("kind", .int 7)]]] do
    match Gen.Read.envs "envs" (.arr envs) with
    | .error "envelope-field-unknown" => pure ()
    | _ => throw "R6 every envelope variant must pass shape before value decoding"
  let check (what : String) (got want : String) : Except String Unit :=
    if got == want then pure () else throw s!"self-check failed: {what}: {got} ≠ {want}"
  check "sha256 abc" (hex (sha256 (textBytes "abc")))
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  check "sha256 empty" (hex (sha256 []))
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  check "sha256 two blocks" (hex (sha256 (textBytes "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")))
    "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
  check "sha512 abc" (hex (sha512 (textBytes "abc")))
    ("ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" ++
     "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f")
  -- RFC 8032 section 7.1, TEST 1 and TEST 2
  let s1 := unhex "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
  check "ed25519 test 1 key" (hex (Ed25519.publicKey s1))
    "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
  check "ed25519 test 1 sig" (hex (Ed25519.sign s1 []))
    ("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555" ++
     "fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
  let s2 := unhex "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
  check "ed25519 test 2 sig" (hex (Ed25519.sign s2 [0x72]))
    ("92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da" ++
     "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00")
  -- RFC 4648 section 10, lower-cased and unpadded
  check "base32hex" (", ".intercalate (["", "f", "fo", "foo", "foob", "fooba", "foobar"].map
      fun s => base32 (textBytes s)))
    ", co, cpng, cpnmu, cpnmuog, cpnmuoj1, cpnmuoj1e8"

def vectors : Except String (List (String × J)) := do
  selfCheck
  let stored ← storedCases.mapM fun c => do pure (c.name, ← storedVector c)
  let refused ← refusedCases.mapM fun (n, e) => do pure (n, ← refusedVector n e)
  let payloads ← refusedPayloads.mapM fun (n, p) => do pure (n, ← refusedVector n note p)
  let salt ← refusedVector "30-salt-short" note (salt := shortSalt)
  let envelopeFirst ← refusedVector "37-empty-salt-envelope-first" { note with kind := "" } (.obj []) []
  let unknown (empty : Bool) : String × J :=
    let name := if empty then "r6-unknown-before-kind" else "r6-unknown-field"
    let e := if empty then { note with kind := "" } else note
    let raw := match envInput e with | .obj kvs => J.obj (("extra", .null) :: kvs) | other => other
    (name, .obj [("vector", .str name), ("layer", .str "record"), ("laws", .arr [.str "record.refuse"]),
      ("source", .str "model"), ("input", .obj [("envs", .arr [raw]), ("salt", .str (base32 (saltFor name))),
      ("payloads", .arr [.obj []])]), ("expect", .obj [("refused", .str "envelope-field-unknown")])])
  let twoUnknown (reverse : Bool) : String × J :=
    let name := if reverse then "r6-two-envs-unknown-first" else "r6-two-envs-unknown-last"
    let raw := match envInput note with | .obj kvs => J.obj (("extra", .null) :: kvs) | other => other
    let envs := if reverse then [raw, envInput note] else [envInput note, raw]
    (name, vector name "record" ["record.refuse", "batch.refuse-order"]
      (.obj [("envs", .arr envs), ("salt", .str (base32 (saltFor name))), ("payloads", .arr [.obj []])])
      (.obj [("refused", .str "envelope-field-unknown")]))
  let rawCase (name : String) (envs : List J) : String × J :=
    (name, vector name "record" ["record.refuse", "batch.refuse-order"]
      (.obj [("envs", .arr envs), ("salt", .str (base32 (saltFor name))), ("payloads", .arr [.obj []])])
      (.obj [("refused", .str "envelope-field-unknown")]))
  let rawUnknown := match envInput note with
    | .obj kvs => J.obj (("extra", .null) :: kvs)
    | other => other
  let malformed := [("nonobject", J.int 7), ("kind-number", .obj [("kind", .int 7)])]
  let competing := malformed.flatMap fun (tag, raw) =>
    [rawCase s!"r6-{tag}-unknown-first" [rawUnknown, raw],
     rawCase s!"r6-{tag}-unknown-last" [raw, rawUnknown]]
  let singleton := [
    rawCase "r6-single-unknown-missing-kind" [.obj [("extra", .null)]],
    rawCase "r6-single-unknown-kind-number" [.obj [("kind", .int 7), ("extra", .null)]]]
  return stored ++ refused ++ payloads ++ [("30-salt-short", salt), ("37-empty-salt-envelope-first", envelopeFirst),
    unknown false, unknown true, twoUnknown false, twoUnknown true] ++ competing ++ singleton ++ (← writtenVectors)

end Gen.Records

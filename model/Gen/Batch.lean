import Loops
import Gen.Json
import Gen.Encode
import Gen.Records
import Gen.Read

/-!
# Batch vectors

A changeset whose records name each other by label. `input.drafts` lists
them; `expect.records` gives each one's id and canonical envelope, in the
order given, or `expect.refused`. Every rotation of the drafts must give the
same records, and each label's id must be its record's id with real ids in
place of labels (`Meaning`).
-/

namespace Gen.BatchVectors

open Loops Gen.Records

def draft (label : Option String) (name : String) (env : Env) (payload : Json) : Draft :=
  { label, env, salt := saltFor name, payload }

def at0 := "2026-10-24T09:00:00Z"

def decide1 := draft (some "@d") "batch-d"
  { kind := "decision", observer := kyle, emitted := at0, about := ["topic:auth"] }
  (.obj [("choice", .str "passkeys")])
def evidence := draft (some "@e") "batch-e"
  { kind := "evidence", observer := kyle, emitted := at0, about := ["topic:auth"] }
  (.obj [("source", .str "url:https://example.com/passkeys")])
def cites := draft none "batch-c"
  { kind := "cites", observer := kyle, emitted := at0, about := ["@d", "@e"] }
  (.obj [])
def revise := draft (some "@d2") "batch-d2"
  { kind := "decision", observer := kyle, emitted := at0, about := ["topic:auth"], of := some "@d",
    prev := ["@d"] }
  (.obj [("choice", .str "passkeys, with a recovery code")])
-- a decision already in the store, about the same topic
def heldAuth : Stored :=
  make
    { kind := "decision", observer := kyle, emitted := "2026-10-23T09:00:00Z",
      about := ["topic:auth"] }
    (saltFor "held-auth") (.obj [("choice", .str "device flow")])
def heldAuthId : String := heldAuth.id H
-- a revision of it, also in the store: not a root
def heldRevision : Stored :=
  make
    { kind := "decision", observer := kyle, emitted := "2026-10-23T10:00:00Z",
      about := ["topic:auth"], of := some heldAuthId, prev := [heldAuthId] }
    (saltFor "held-revision") (.obj [("choice", .str "device flow, with a fallback")])
def heldRevisionId : String := heldRevision.id H

/-- What the store holds besides the batch: each id with its `of` and `prev`. -/
def held : List (String × Held) :=
  [(heldAuthId, {}), (heldRevisionId, { of := some heldAuthId, prev := [heldAuthId] }), (r1Id, {})]

def outside := draft none "batch-o"
  { kind := "decision", observer := kyle, emitted := at0, about := ["@d2", "topic:auth"],
    of := some heldAuthId, prev := [heldAuthId] }
  (.obj [("choice", .str "passkeys")])
def unknownPrev := draft none "batch-u"
  { kind := "decision", observer := kyle, emitted := at0, about := ["topic:auth"], of := some grantId,
    prev := [grantId] }
  (.obj [])
def unrelated := draft none "batch-n"
  { kind := "note", observer := kyle, emitted := at0, about := ["topic:cache"], of := some "topic:cache",
    prev := ["@d"] }
  (.obj [])
-- `of` names a record the writer holds that is not a root: in the store, and
-- by label in the batch
def ofHeldRevision := draft none "batch-r"
  { kind := "decision", observer := kyle, emitted := at0, of := some heldRevisionId,
    prev := [heldRevisionId] }
  (.obj [])
def ofLabelRevision := draft none "batch-rl"
  { kind := "decision", observer := kyle, emitted := at0, of := some "@d2", prev := ["@d2"] }
  (.obj [])
-- a record of `@d` that names no `prev`: a second branch that joins it
def branch := draft none "batch-br"
  { kind := "decision", observer := kyle, emitted := at0, of := some "@d" }
  (.obj [("choice", .str "security keys")])
-- a revision of the revision: still of the root
def reviseAgain := draft none "batch-d3"
  { kind := "decision", observer := kyle, emitted := at0, of := some "@d", prev := ["@d2"] }
  (.obj [("choice", .str "passkeys, with two recovery codes")])
-- `prev` without `of`
def noOf := draft none "batch-no"
  { kind := "decision", observer := kyle, emitted := at0, prev := ["@d"] }
  (.obj [])

-- The bad envelope refuses before the out-of-range payload, in every draft order.
def badKind := draft none "batch-k"
  { kind := "", observer := kyle, emitted := at0, about := ["topic:auth"] } (.obj [])
def bigNumber := draft none "batch-big"
  { kind := "estimate", observer := kyle, emitted := at0, about := ["topic:auth"] }
  (.obj [("n", .number (Number.ofScientific false 9007199254740992 0))])
-- A number past binary64's range reaches the writer's number check like any other out of range.
def overflowNumber := draft none "batch-overflow"
  { kind := "estimate", observer := kyle, emitted := at0, about := ["topic:auth"] }
  (.obj [("n", .number (Number.ofScientific false 1 999)), ("low", .number (Number.ofScientific true 1 999))])
-- one `prev` the writer holds but of another thing, one it doesn't hold
def bothPrev := draft none "batch-p"
  { kind := "note", observer := kyle, emitted := at0, about := ["topic:cache"], of := some "topic:cache",
    prev := [heldAuthId, grantId] }
  (.obj [])

-- a revision that erases the record it replaces, and one naming a label in
-- `erases`, which only `about`, `of` and `prev` may use
def eraseHeld := draft none "batch-x"
  { kind := "decision", observer := kyle, emitted := at0, about := ["topic:auth"],
    of := some heldAuthId, prev := [heldAuthId], erases := [heldAuthId] }
  (.obj [("choice", .str "passkeys")])
def eraseLabel := draft none "batch-xl"
  { kind := "note", observer := kyle, emitted := at0, about := ["topic:auth"], erases := ["@d"] }
  (.obj [])

def cycleA := draft (some "@a") "batch-a"
  { kind := "note", observer := kyle, emitted := at0, of := some "@b", prev := ["@b"] } (.obj [])
def cycleB := draft (some "@b") "batch-b"
  { kind := "note", observer := kyle, emitted := at0, of := some "@a", prev := ["@a"] } (.obj [])

structure Case where
  name : String
  laws : List String
  drafts : List Draft

def cases : List Case := [
  ⟨"01-two-labels", ["batch.meaning"], [decide1, evidence, cites]⟩,
  ⟨"02-chain-and-real-ids", ["batch.meaning"], [decide1, revise, outside]⟩,
  ⟨"03-revise-and-erase", ["batch.meaning"], [eraseHeld]⟩,
  ⟨"04-held-self-anchor", ["batch.meaning"],
    [draft none "batch-anchor"
      { kind := "note", observer := kyle, emitted := at0, of := some r1Id, prev := [r1Id] }
      (.obj [])]⟩,
  ⟨"05-large-number-out-of-range", ["batch.refuse", "write.checked"], [bigNumber]⟩,
  ⟨"10-unknown-label", ["batch.refuse"], [cites]⟩,
  ⟨"11-duplicate-label", ["batch.refuse"], [decide1, { decide1 with salt := saltFor "batch-d-again" }]⟩,
  ⟨"12-cycle", ["batch.refuse"], [cycleA, cycleB]⟩,
  ⟨"13-prev-unknown", ["batch.refuse"], [unknownPrev]⟩,
  ⟨"14-prev-unrelated", ["batch.refuse"], [decide1, unrelated]⟩,
  ⟨"15-write-refused", ["batch.refuse", "write.checked"], [decide1, badKind, bigNumber]⟩,
  ⟨"16-refusal-order", ["batch.refuse"], [decide1, unrelated, unknownPrev, bothPrev]⟩,
  ⟨"17-label-in-erases", ["batch.refuse", "write.checked"], [decide1, eraseLabel]⟩,
  ⟨"23-overflow-out-of-range", ["batch.refuse", "write.overflow"], [decide1, overflowNumber]⟩,
  ⟨"24-overflow-after-envelope", ["batch.refuse", "write.overflow"], [overflowNumber, badKind]⟩,
  ⟨"25-overflow-before-prev", ["batch.refuse", "write.overflow"], [overflowNumber, unknownPrev]⟩,
  ⟨"26-label-in-of", ["batch.meaning"], [decide1, branch]⟩,
  ⟨"27-of-missing", ["batch.refuse", "write.checked"], [decide1, noOf]⟩,
  ⟨"28-of-not-root-held", ["batch.refuse"], [ofHeldRevision]⟩,
  ⟨"29-of-not-root-label", ["batch.refuse"], [decide1, revise, ofLabelRevision]⟩,
  ⟨"30-prev-unknown-before-of-not-root", ["batch.refuse"], [ofHeldRevision, unknownPrev]⟩,
  ⟨"31-of-not-root-before-prev-unrelated", ["batch.refuse"], [decide1, unrelated, ofHeldRevision]⟩,
  ⟨"32-chain-of-root", ["batch.meaning"], [decide1, revise, reviseAgain]⟩
]

def jDraft (d : Draft) : J :=
  .obj ((match d.label with
      | some l => [("label", J.str l)]
      | none => []) ++
    [("env", envInput d.env), ("salt", .str (base32 d.salt)), ("payload", ofJson d.payload)])

def rotations (xs : List Draft) : List (List Draft) :=
  (List.range xs.length).map fun i => xs.drop i ++ xs.take i

/-- Every label's id is its record's id with real ids in place of labels, and
there is exactly one record per draft. -/
def checkMeaning (ds : List Draft) (rs : List Stored) : Bool :=
  let σ := (ds.zip rs).filterMap fun (d, r) => d.label.map (·, r.id H)
  ds.length == rs.length && (ds.zip rs).all fun (d, r) =>
    match substEnv (fun l => σ.lookup l) d.env with
    | some e => idOf H e (d.commit H) == r.id H && e.toJson.canon == r.env.toJson.canon
    | none => false

def sortStr (xs : List String) : List String := sortBy utf16 xs

/-- The same batch listed differently: every rotation of its drafts, and each
with every draft's `prev` reversed (a set, so its order must not matter). -/
def variants (ds : List Draft) : List (List Draft) :=
  let rev := ds.map fun d => { d with env := { d.env with prev := d.env.prev.reverse } }
  rotations ds ++ rotations rev

/-- A held record: its id, its `of` when it has one, and its `prev`. -/
def jHeld : String × Held → J
  | (i, h) => .obj ([("id", .str i)] ++ (match h.of with
      | some o => [("of", J.str o)]
      | none => []) ++ [("prev", .arr (h.prev.map .str))])

def batchVector (c : Case) : Except String J := do
  let resolve := fun ds => (resolveBatch H (held.lookup ·) ds).map (·.map Written.stored)
  let expect ← match resolve c.drafts with
    | .ok rs =>
      let ids := sortStr (rs.map (·.id H))
      for ds in variants c.drafts do
        match resolve ds with
        | .ok rs' =>
          if !checkMeaning ds rs' then throw s!"batch/{c.name}: not the batch's meaning"
          if sortStr (rs'.map (·.id H)) != ids then
            throw s!"batch/{c.name}: draft order changed the records"
        | .error e => throw s!"batch/{c.name}: a reordering was refused: {e}"
      pure (J.obj [("records", .arr (rs.map fun r =>
        .obj [("id", .str (r.id H)), ("env_canonical", .str r.env.toJson.canon)]))])
    | .error why =>
      for ds in variants c.drafts do
        match resolve ds with
        | .ok _ => throw s!"batch/{c.name}: a reordering was accepted"
        | .error e => if e != why then
            throw s!"batch/{c.name}: a reordering was refused for {e}, not {why}"
      pure (J.obj [("refused", .str why)])
  return vector c.name "batch" c.laws
    (.obj [("held", .arr (held.map jHeld)),
      ("drafts", .arr (c.drafts.map jDraft))]) expect

/-- Raw inputs exercise the shape stage that typed drafts cannot represent. -/
def rawCases : List (String × List J) :=
  let unknown := match envInput decide1.env with
    | .obj kvs => J.obj (("extra", .null) :: kvs)
    | other => other
  let shape := J.obj [("env", unknown), ("salt", .str (base32 decide1.salt)), ("payload", .obj [])]
  let malformed (field : String) := match jDraft decide1 with
    | .obj kvs => J.obj (kvs.map fun (k, v) => if k == field then (k, .int 7) else (k, v))
    | other => other
  [("18-shape-before-duplicate-labels", [jDraft decide1, jDraft decide1, shape]),
   ("19-shape-before-values", [malformed "env", shape]),
   ("20-shape-before-label-values", [malformed "label", shape]),
   ("21-shape-before-salt", [malformed "salt", shape]),
   ("22-shape-before-kind", [jDraft badKind, shape])]

def rawVectors : Except String (List (String × J)) := do
  let mut out := []
  for (base, ds) in rawCases do
    for i in List.range ds.length do
      let rotated := ds.drop i ++ ds.take i
      let name := s!"{base}-{i}"
      let raw ← Gen.Read.parseJson ((J.arr rotated).render 0)
      match Gen.Read.drafts "input.drafts" raw with
      | .error "envelope-field-unknown" => pure ()
      | _ => throw s!"batch/{name}: raw shape did not precede labels and values"
      out := out ++ [(name, vector name "batch" ["batch.refuse", "batch.refuse-order"]
        (.obj [("held", .arr []), ("drafts", .arr rotated)])
        (.obj [("refused", .str "envelope-field-unknown")]))]
  return out

def vectors : Except String (List (String × J)) := do
  return (← cases.mapM fun c => do pure (c.name, ← batchVector c)) ++ (← rawVectors)

end Gen.BatchVectors

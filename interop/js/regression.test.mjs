import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { base32, base32Decode, fixedBytes, canonical, checkEnvelopeKeys, seal, writeText, writeRecord, problem, validEmitted, occurredSpan, signId, verifyId, commitOf, idOf, parse as parseText, wellFormed, unicodeProblem } from "./loops.mjs";
import { readVector } from "../answers.mjs";
import { parseSpecText, readJson, slotNames, slotsText } from "./spec.mjs";
import { resolveBatch } from "./batch.mjs";
import { heads, latest, latestBy, count, sum, min, max } from "./fold.mjs";
import { admit, realm } from "./realm.mjs";
import { parse, edit, diff, apply, baseProblem, render } from "./lens.mjs";
import { lensEdit } from "./scenario.mjs";
import { check as checkLens } from "./check-lens.mjs";
import { runSpec } from "./spec.mjs";
import { step } from "./scenario.mjs";

const id = (n) => `r:${base32(Uint8Array.from({ length: 32 }, () => n))}`;
const record = (n, of, prev = []) => ({
  id: id(n), kind: "note", ...(of !== undefined && { of }), prev, emitted: `2020-01-01T00:00:0${n}Z`, n,
});
const spec = { from: [{ store: "." }], fold: { count: { count: true } } };

test("latest uses the union of per-thing heads", () => {
  const a = record(1, "task:a");
  const b = record(2, "task:b");
  const c = record(3, "task:a", [a.id]);
  assert.deepEqual(heads([a, b, c]).map((r) => r.id), [c.id, b.id]);
  assert.deepEqual(latest([a, b, c]), { winner: c.id, others: [b.id] });
  assert.deepEqual(latest([]), { winner: null, others: [] });
});

// D8: one thing per record, `of` as written or the record's own id; `about`
// only mentions. A record about two things heads neither; a revision without
// `of` is of its own id, so it heads that and leaves its target's thing alone.
test("a record is of one thing, and about never makes a head", () => {
  const a = { ...record(1), about: ["task:a", "task:b"] };
  const b = { ...record(2, a.id, [a.id]), about: ["task:a"] };
  const c = { ...record(3, undefined, [a.id]), about: ["task:b"] };
  assert.deepEqual(Object.keys(latestBy([a, b])), [a.id]);
  assert.deepEqual(latestBy([a, b])[a.id].heads, [b.id]);
  assert.deepEqual(Object.keys(latestBy([a, c])).sort(), [a.id, c.id].sort());
  assert.deepEqual(heads([a, b, c]).map((r) => r.id), [c.id, b.id]);
  // The reader refuses c: its prev names a held record of another thing.
  assert.deepEqual(admit([a, b, c]).refused, [{ id: c.id, code: "prev-unrelated" }]);
});

test("spec and scenario admit once against all settled records", () => {
  const a = record(1, "task:a");
  const b = record(2, "task:b", [a.id]);
  const c = record(3, "task:a", [b.id]);
  const records = [a, b, c];
  assert.deepEqual(runSpec(spec, records).count, { value: 1, unread: 0 });
  const result = step(spec, "task:a", records);
  assert.deepEqual(result.admitted, [a.id]);
  assert.deepEqual(result.heads, [a.id]);
  assert.deepEqual(result.slots.count, { value: 1, unread: 0 });
});

const writerEnv = { kind: "note", observer: "o:test", emitted: "2026-10-21T00:00:00Z" };
const writerSalt = new Uint8Array(16);
const writerSeed = new Uint8Array(32);

test("record input delegates the active envelope and checks only siblings", () => {
  for (const writer of ["seal", "writeText"]) {
    for (const env of [{ extra: 1 }, { kind: 7, extra: 1 }]) {
      const input = { envs: [env], payloads: [null], salt: "!", seed: "!" };
      assert.throws(() => writeRecord(input, { writer }), /envelope-field-unknown/);
    }
    const guarded = { get kind() { throw new Error("value read"); } };
    for (const envs of [[guarded, { extra: 1 }], [{ extra: 1 }, guarded]]) {
      for (const envIndex of [0, 1]) {
        assert.throws(() => writeRecord({ envs, payloads: [null], salt: "!" }, { writer, envIndex }), /envelope-field-unknown/);
      }
    }
    // One shape check, not writeText -> seal rechecking the active envelope.
    let keyReads = 0;
    const env = new Proxy(writerEnv, { ownKeys(target) { keyReads++; return Reflect.ownKeys(target); } });
    writeRecord({ envs: [env], payloads: [null], salt: base32(writerSalt), seed: base32(writerSeed) }, { writer });
    assert.equal(keyReads, 1);
  }
});

test("an envelope's shape refuses by name: not an object, a field left out or of the wrong type", () => {
  const salt = base32(writerSalt), seed = base32(writerSeed);
  const refused = (envs, more = {}) => {
    try { writeRecord({ envs, payloads: [{}], salt, seed, ...more }); return null; }
    catch (e) { return e.message; }
  };
  for (const env of [7, null, "note", [writerEnv]]) {
    assert.equal(refused([env]), "envelope-not-object");
    assert.equal(refused([writerEnv, env]), "envelope-not-object");
    assert.equal(refused([env, { ...writerEnv, extra: 1 }]), "envelope-field-unknown");
    assert.equal(refused([{ ...writerEnv, extra: 1 }, env]), "envelope-field-unknown");
  }
  const { kind: _k, ...noKind } = writerEnv;
  assert.equal(refused([noKind]), "kind-empty");
  assert.equal(refused([{ ...noKind, observer: 7 }]), "kind-empty");
  const fields = { kind: "kind-empty", observer: "observer-not-typed", emitted: "emitted-not-rfc3339",
    occurred: "occurred-not-span", about: "about-not-typed", of: "of-not-typed", prev: "prev-not-record-id",
    under: "under-not-record-id", erases: "erases-not-record-id", sets: "sets-not-keys" };
  for (const [k, code] of Object.entries(fields)) {
    for (const v of [7, null, true, {}, [7]]) assert.equal(refused([{ ...writerEnv, [k]: v }]), code, `${k}: ${JSON.stringify(v)}`);
  }
  // A salt that is not base32hex text is a salt that is not 16 bytes.
  for (const s of [undefined, 7, "!"]) assert.equal(refused([writerEnv], { salt: s }), "salt-not-16-bytes");
  assert.equal(refused([noKind], { salt: 7 }), "kind-empty");
});

test("an unknown draft member is malformed input, after the shape stage", () => {
  const env = { kind: "note", observer: "o:t", emitted: "2026-10-21T00:00:00Z" };
  const salt = "0".repeat(26);
  const good = { env, salt, payload: null };
  assert.ok(resolveBatch([good], []).records);
  assert.throws(() => resolveBatch([{ ...good, extra: 0 }], []), /^Error: batch input: a draft has an unknown member$/);
  assert.deepEqual(resolveBatch([{ ...good, extra: 0 }, { ...good, env: { ...env, extra: 1 } }], []), { refused: "envelope-field-unknown" });
  assert.deepEqual(resolveBatch([{ ...good, extra: 0 }, { ...good, env: 7 }], []), { refused: "envelope-not-object" });
});

test("a record input with no envelope or no payload is malformed input", () => {
  const env = { kind: "note", observer: "o:t", emitted: "2026-10-21T00:00:00Z" };
  const salt = "0".repeat(26), seed = base32(writerSeed);
  for (const writer of ["seal", "writeText"]) {
    assert.throws(() => writeRecord({ envs: [], payloads: [null], salt, seed }, { writer }), /record input: no envelope given/);
    assert.throws(() => writeRecord({ envs: [env], payloads: [], salt, seed }, { writer }), /record input: no payload given/);
    // Raw shape still comes first.
    assert.throws(() => writeRecord({ envs: [{ ...env, extra: 1 }], payloads: [], salt, seed }, { writer }), /^Error: envelope-field-unknown$/);
  }
  // A text payload's lone surrogate comes before it, as in Lean.
  assert.throws(() => writeRecord({ envs: [], payload_text: '"\\ud800"', salt, seed }, { writer: "writeText" }), /^Error: string-not-unicode$/);
  assert.throws(() => writeRecord({ envs: [], payload_text: "null", salt, seed }, { writer: "writeText" }), /no envelope given/);
});

test("a label may stand in under, and takes part in cycles", () => {
  const salt = base32(writerSalt);
  const d = (label, env) => ({ ...(label && { label }), env: { ...writerEnv, ...env }, salt, payload: { label } });
  const { records } = resolveBatch([d("@g", { kind: "grant" }), d(null, { under: "@g" })], []);
  assert.equal(JSON.parse(records[1].env_canonical).under, records[0].id);
  assert.deepEqual(resolveBatch([d(null, { under: "@nowhere" })], []), { refused: "label-unknown" });
  assert.deepEqual(resolveBatch([d("@a", { under: "@b" }), d("@b", { under: "@a" })], []), { refused: "label-cycle" });
  assert.deepEqual(resolveBatch([d("@s", { under: "@s" })], []), { refused: "label-cycle" });
  // An `under` target is held, in the batch or in what is held, whatever it is.
  const target = records[0].id;
  assert.deepEqual(resolveBatch([d(null, { under: target })], []), { refused: "under-unknown" });
  assert.equal(resolveBatch([d(null, { under: target })], [{ id: target, prev: [] }]).records.length, 1);
  assert.equal(resolveBatch([d(null, { under: target }), d("@g", { kind: "grant" })], []).records.length, 2);
  // erases still takes no label: one there is no record id
  assert.deepEqual(resolveBatch([d("@g", {}), d(null, { erases: ["@g"] })], []), { refused: "erases-not-record-id" });
  // a draft's envelope left out or not an object is a shape refusal, before labels
  assert.deepEqual(resolveBatch([d("@a", {}), d("@a", {}), { salt, payload: null }], []), { refused: "envelope-not-object" });
});

test("base32hex exact image, width and byte order", () => {
  const alphabet = "0123456789abcdefghijklmnopqrstuv";
  for (const [n, pad] of [[16, 2], [32, 4], [64, 3]]) {
    for (const byte of [0, 1, 127, 255]) {
      const bytes = new Uint8Array(n).fill(byte);
      const text = base32(bytes);
      assert.deepEqual(base32Decode(text), bytes);
      assert.deepEqual(fixedBytes(text, "", n), bytes);
      for (let i = 1; i < 2 ** pad; i++) {
        const last = alphabet.indexOf(text.at(-1));
        assert.throws(() => base32Decode(text.slice(0, -1) + alphabet[last + i]));
      }
      for (const bad of [text + "0", text.slice(1), text + "=", text.slice(0, -1) + "w"]) assert.throws(() => fixedBytes(bad, "", n));
    }
    assert.throws(() => base32Decode(base32(new Uint8Array(n).fill(255)).toUpperCase()));
  }
  const ordered = Array.from({ length: 256 }, (_, i) => Uint8Array.from({ length: 32 }, (_, j) => j === 0 ? i : 255 - i));
  assert.deepEqual(ordered.map(base32), ordered.map(base32).toSorted());
  for (const field of ["prev", "under", "erases"]) {
    const bad = "r:" + "0".repeat(51) + "1";
    assert.equal(problem({ ...writerEnv, [field]: field === "under" ? bad : [bad] }), `${field}-not-record-id`);
  }
  const signed = signId(writerSeed, id(1));
  assert.equal(verifyId(signed, id(1)), true);
  assert.throws(() => verifyId({ ...signed, key: "x:" + signed.key.slice(2) }, id(1)));
  assert.throws(() => verifyId({ ...signed, ed25519: signed.ed25519.slice(0, -1) + "1" }, id(1)));
});

test("canonical UTC instants and envelope key priority", () => {
  for (const s of ["0000-02-29T00:00:00Z", "2024-02-29T23:59:59.123456789Z", "9999-12-31T23:59:59.5Z"]) assert.equal(validEmitted(s), true);
  for (const s of ["2024-01-01T00:00:00+00:00", "2024-01-01T00:00:60Z", "2024-01-01T00:00:00.0Z", "2024-01-01T00:00:00.50Z", "2024-01-01T00:00:00.1234567891Z", "2023-02-29T00:00:00Z"]) {
    assert.equal(validEmitted(s), false, s);
    assert.equal(problem({ ...writerEnv, emitted: s }), "emitted-not-rfc3339");
    for (const occurred of [s, `${s}/..`, `${s}/2025`, `2020/${s}`]) assert.equal(occurredSpan(occurred), null, occurred);
  }
  const env = { ...writerEnv, kind: "", unexpected: 1 };
  assert.throws(() => seal(env, writerSalt, {}, writerSeed), /envelope-field-unknown/);
  assert.throws(() => seal(JSON.parse('{"__proto__":0,"kind":""}'), writerSalt, {}, writerSeed), /envelope-field-unknown/);
  assert.throws(() => seal(Object.defineProperty({ ...writerEnv }, "hidden", { value: 1 }), writerSalt, {}, writerSeed), /envelope-field-unknown/);
  const guarded = { get kind() { throw new Error('value read'); }, unexpected: 1 };
  assert.throws(() => seal(guarded, writerSalt, {}, writerSeed), /envelope-field-unknown/);
  assert.throws(() => writeText(guarded, writerSalt, '{', writerSeed), /envelope-field-unknown/);
  const draft = { env, salt: base32(writerSalt), payload: {} };
  assert.deepEqual(resolveBatch([draft], []), { refused: "envelope-field-unknown" });
  assert.deepEqual(resolveBatch([{ ...draft, env: { ...env, about: ["@missing"] } }], []), { refused: "envelope-field-unknown" });
});

// D8: `of` is a known envelope key, so only a truly unknown key is a shape
// refusal, and that still comes before `of`'s value is read. Its value is one
// typed reference as text: a list holding one would pass a pattern test as
// its string form, so it is refused by type.
test("of is a known envelope key, checked by value after the shape", () => {
  const root = { ...writerEnv, of: "t:x" };
  assert.equal(problem(root), null);
  assert.deepEqual(Object.keys(JSON.parse(seal(root, writerSalt, {}, writerSeed).line).env).sort(), ["emitted", "kind", "observer", "of"]);
  assert.notEqual(seal(root, writerSalt, {}, writerSeed).id, seal(writerEnv, writerSalt, {}, writerSeed).id);
  for (const of of [5, ["t:x"], null, "", "x", "@label"]) {
    assert.equal(problem({ ...writerEnv, of }), "of-not-typed", JSON.stringify(of));
    assert.throws(() => seal({ ...writerEnv, of, extra: 1 }, writerSalt, {}, writerSeed), { message: "envelope-field-unknown" });
  }
  const guarded = { ...writerEnv, get of() { throw new Error("value read"); }, extra: 1 };
  assert.throws(() => seal(guarded, writerSalt, {}, writerSeed), { message: "envelope-field-unknown" });
  // Order: about, then of, then prev; of-missing after erases.
  assert.equal(problem({ ...writerEnv, about: ["x"], of: "x" }), "about-not-typed");
  assert.equal(problem({ ...writerEnv, of: "x", prev: ["x"] }), "of-not-typed");
  assert.equal(problem({ ...writerEnv, prev: [id(1)], erases: ["x"] }), "erases-not-record-id");
  assert.equal(problem({ ...writerEnv, prev: [id(1)] }), "of-missing");
  assert.equal(problem({ ...writerEnv, of: id(1), prev: [id(1)] }), null);
  // A batch: of-missing is a writer code, ranked before the salt.
  const draft = (env, salt = base32(writerSalt)) => ({ env, salt, payload: {} });
  assert.deepEqual(resolveBatch([draft({ ...writerEnv, prev: [id(1)] }, base32(new Uint8Array(15)))], [{ id: id(1), prev: [] }]), { refused: "of-missing" });
  assert.deepEqual(resolveBatch([draft({ ...writerEnv, of: "@nobody" })], []), { refused: "label-unknown" });
});

test("sets holds key names, canonical as about is, and an empty one is written", () => {
  const line = (env) => JSON.parse(seal(env, writerSalt, {}, writerSeed).line).env;
  const id = (env) => seal(env, writerSalt, {}, writerSeed).id;
  assert.deepEqual(line({ ...writerEnv, sets: ["title", "due", "title"] }).sets, ["due", "title"]);
  assert.equal(id({ ...writerEnv, sets: ["title", "due"] }), id({ ...writerEnv, sets: ["due", "title", "due"] }));
  // Absent and empty differ: an empty sets is written, and names another record.
  assert.deepEqual(line({ ...writerEnv, sets: [] }).sets, []);
  assert.equal("sets" in line(writerEnv), false);
  assert.notEqual(id({ ...writerEnv, sets: [] }), id(writerEnv));
  // Not an array, a member that is not text, or an empty name.
  for (const sets of [7, null, "title", {}, [7], [null], [["title"]], [""], ["title", ""]]) {
    assert.equal(problem({ ...writerEnv, sets }), "sets-not-keys", JSON.stringify(sets));
  }
  // After erases, before of-missing; nothing reads the payload.
  assert.equal(problem({ ...writerEnv, erases: ["x"], sets: [""] }), "erases-not-record-id");
  assert.equal(problem({ ...writerEnv, prev: [id(writerEnv)], sets: [7] }), "sets-not-keys");
  assert.equal(problem({ ...writerEnv, sets: ["status"] }), null);
  assert.ok(seal({ ...writerEnv, sets: ["status"] }, writerSalt, "done", writerSeed).id);
  // A key that looks like a label is a key: a batch neither replaces nor looks it up.
  const d = (label, env) => ({ ...(label && { label }), env: { ...writerEnv, ...env }, salt: base32(writerSalt), payload: {} });
  const { records } = resolveBatch([d("@t", {}), d(null, { of: "@t", prev: ["@t"], sets: ["@t", "@nowhere"] })], []);
  assert.deepEqual(JSON.parse(records[1].env_canonical).sets, ["@nowhere", "@t"]);
  assert.deepEqual(resolveBatch([d(null, { sets: [""] }), d(null, { erases: ["x"] })], []), { refused: "erases-not-record-id" });
});

test("raw shape precedes every variant value, batch label and seed", () => {
  const unread = { get kind() { throw new Error("value read"); } };
  const unknown = { ...writerEnv, extra: true };
  for (const envs of [[unread, unknown], [unknown, unread]]) {
    assert.throws(() => checkEnvelopeKeys(envs), /envelope-field-unknown/);
    const drafts = envs.map(env => ({
      env, get label() { throw new Error("label read"); },
      get salt() { throw new Error("salt read"); },
    }));
    assert.deepEqual(resolveBatch(drafts, []), { refused: "envelope-field-unknown" });
  }
  const seed = () => { throw new Error("seed read"); };
  assert.throws(() => seal({ ...writerEnv, kind: "" }, writerSalt, {}, seed), /kind-empty/);
  assert.throws(() => seal(writerEnv, writerSalt, {}, seed), /seed read/);
});

test("writers accept limits and within-range fractions after binary64 rounding", () => {
  for (const n of [Number.MAX_SAFE_INTEGER, -Number.MAX_SAFE_INTEGER, 9007199254740991.1,
    -9007199254740991.1, 1.5, -1.5, 1424953923781206.25, Number.MIN_VALUE, -0]) {
    const payload = JSON.parse(`{"deep":[{"n":${n}}],"__proto__":${n}}`);
    assert.deepEqual(JSON.parse(seal(writerEnv, writerSalt, payload, writerSeed).line).payload, payload);
    assert.deepEqual(writeText(writerEnv, writerSalt, JSON.stringify(payload), writerSeed),
      seal(writerEnv, writerSalt, payload, writerSeed));
  }
});

test("writers refuse unsafe magnitudes and nonfinite payloads with unchanged priority", () => {
  for (const n of [2 ** 53, -(2 ** 53), 9007199254740991.5, -9007199254740991.5,
    1e30, -1e30, Number.MAX_VALUE, -Number.MAX_VALUE, NaN, Infinity, -Infinity]) {
    assert.throws(() => seal(writerEnv, writerSalt, { deep: [{ n }] }, writerSeed),
      { message: "number-out-of-range" });
    assert.throws(() => seal({ ...writerEnv, kind: "" }, writerSalt, n, writerSeed),
      { message: "kind-empty" });
    assert.throws(() => seal(writerEnv, new Uint8Array(0), n, writerSeed),
      { message: "salt-not-16-bytes" });
  }
  for (const text of ["9007199254740991.5", "-9007199254740991.5", "1e30", "-1e30", "1e400", "-1e400"]) {
    assert.throws(() => writeText(writerEnv, writerSalt, text, writerSeed),
      { message: "number-out-of-range" });
  }
  for (const duplicate of ['{"n":1e30,"n":0}', '{"n":1e400,"n":0}']) {
    assert.throws(() => writeText(writerEnv, writerSalt, duplicate, writerSeed),
      { message: "member-duplicate" });
    assert.throws(() => writeText(writerEnv, new Uint8Array(0), duplicate, writerSeed),
      { message: "salt-not-16-bytes" });
    assert.throws(() => writeText({ ...writerEnv, kind: "" }, new Uint8Array(0), duplicate, writerSeed),
      { message: "kind-empty" });
  }
});

test("native canonicalization still handles all 4000 standards independently of writer range", () => {
  const rows = readFileSync(new URL("../../vectors/standard/es6-numbers.txt", import.meta.url), "utf8").trim().split("\n");
  assert.equal(rows.length, 4000);
  let outside = 0;
  for (const row of rows) {
    const [hex, expected] = row.split(",");
    const n = Buffer.from(hex.padStart(16, "0"), "hex").readDoubleBE();
    assert.equal(canonical(n), expected, hex);
    if (Math.abs(n) > Number.MAX_SAFE_INTEGER) outside++;
  }
  assert.ok(outside > 0, "the standard corpus includes out-of-range magnitudes");
});

test("numeric folds read safe whole values only", () => {
  const records = [1.0, 1.5, 2 ** 53, "2", true].map((n, i) => ({ ...record(i + 1, "task:a"), n }));
  for (const fold of [sum, min, max]) assert.deepEqual(fold(records), { value: 1, unread: 4 });
});

test("lens refuses empty and duplicate ids", () => {
  assert.equal(parse(["::: ", ":::"]), null);
  assert.equal(parse(["::: a", ":::", "::: a", ":::"]), null);
});

test("lens checker reports an unexpected null and checks the next vector", () => {
  const root = mkdtempSync(join(tmpdir(), "lens-check-"));
  try {
    const dir = join(root, "lens", "section");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "a.json"), JSON.stringify({ vector: "bad", input: { base: [], text: ["::: ", ":::"] }, expect: { parsed: [] } }));
    writeFileSync(join(dir, "b.json"), JSON.stringify({ vector: "next", input: { base: [], text: ["stray"] }, expect: { refused: true } }));
    const result = checkLens(root);
    assert.equal(result.files, 2);
    assert.equal(result.passed, 1);
    assert.equal(result.failures.length, 1);
    assert.match(result.failures[0], /lens\/section\/bad parsed/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// D1: the public base boundary.
test("lens refuses a base that repeats an id before it reads the text", () => {
  const one = { id: "a", body: ["one"] }, two = { id: "a", body: ["two"] }, other = { id: "b", body: ["B"] };
  for (const base of [[one, two], [one, other, two], [one, one]]) {
    assert.equal(baseProblem(base), "base-id-duplicate");
    for (const text of [render([one]), render(base), [], ["::: a", "unclosed"], ["stray"]]) {
      assert.deepEqual(edit(base, text), { refused: "base-id-duplicate" });
    }
    assert.throws(() => lensEdit({ base, text: render([one]) }), /base-id-duplicate/);
  }
  for (const base of [[], [one], [one, other]]) {
    assert.equal(baseProblem(base), null);
    assert.deepEqual(edit(base, ["stray"]), { parsed: null, refused: true });
    const parsed = [two];
    assert.deepEqual(edit(base, render(parsed)), {
      parsed, normalized: render(parsed), outcomes: diff(base, parsed), applied: apply(base, diff(base, parsed)),
    });
  }
  // The raw functions don't check, and take the first section with an id, as the model does.
  assert.deepEqual(diff([one, two], [two]), [{ accepted: "a", body: ["two"] }]);
  assert.deepEqual(diff([one, two], [one]), []);
  assert.deepEqual(diff([{ id: "a", body: [] }, { id: "a", body: [""] }], [{ id: "a", body: [] }]), []);
});

// D2: a lone surrogate is no text, in any API that reaches a hash.
const HIGH = "\ud83d", LOW = "\ude00";
const LONE = [HIGH, LOW, LOW + HIGH, HIGH + "x", "x" + HIGH, HIGH + HIGH + LOW, HIGH + LOW + LOW];
test("lone surrogates are refused by name before hashing, in values, member names and envelopes", () => {
  for (const s of ["", "plain", "雪" + HIGH + LOW, "\ud7ff\ue000\uffff", "\udbff\udfff"]) {
    assert.equal(wellFormed(s), true, JSON.stringify(s));
    assert.equal(wellFormed(s), s.isWellFormed());
    assert.equal(unicodeProblem({ [s]: [s] }), null);
    assert.equal(seal(writerEnv, writerSalt, { [s]: [s] }, writerSeed).id,
      writeText(writerEnv, writerSalt, JSON.stringify({ [s]: [s] }), writerSeed).id);
  }
  const refused = { message: "string-not-unicode" };
  for (const s of LONE) {
    assert.equal(wellFormed(s), false, JSON.stringify(s));
    assert.equal(s.isWellFormed(), false);
    for (const payload of [s, [s], { a: s }, { [s]: 1 }, { deep: [{ [s]: null }] }]) {
      assert.equal(unicodeProblem(payload), "string-not-unicode");
      assert.throws(() => seal(writerEnv, writerSalt, payload, writerSeed), refused);
      // JSON.stringify writes the surrogate as an escape; the raw code unit is refused the same way.
      for (const text of [JSON.stringify(payload), JSON.stringify(payload).replace(/\\u(d[89a-f][0-9a-f]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)))]) {
        assert.throws(() => writeText(writerEnv, writerSalt, text, writerSeed), refused);
        assert.throws(() => parseText(text), refused);
      }
      // Nothing reaches a hash another way.
      assert.throws(() => canonical(payload), refused);
      assert.throws(() => commitOf(writerSalt, payload), refused);
    }
    for (const env of [{ ...writerEnv, kind: s }, { ...writerEnv, observer: "o:" + s }, { ...writerEnv, about: ["t:" + s] },
      { ...writerEnv, of: "t:" + s }, { ...writerEnv, occurred: s }, { ...writerEnv, under: s }, { ...writerEnv, prev: [s] }, { ...writerEnv, erases: [s] }]) {
      assert.throws(() => seal(env, writerSalt, {}, writerSeed), refused);
      assert.throws(() => writeText(env, writerSalt, "{}", writerSeed), refused);
      assert.throws(() => idOf(env, "h:x"), refused);
      const draft = { env, salt: base32(writerSalt), payload: {} };
      assert.deepEqual(resolveBatch([draft], []), { refused: "string-not-unicode" });
    }
    assert.deepEqual(resolveBatch([{ env: writerEnv, salt: base32(writerSalt), payload: { a: [s] } }], []), { refused: "string-not-unicode" });
    assert.deepEqual(resolveBatch([{ label: "@" + s, env: writerEnv, salt: base32(writerSalt), payload: {} }], []), { refused: "string-not-unicode" });
    assert.deepEqual(resolveBatch([{ env: writerEnv, salt: base32(writerSalt), payload: {} }], [{ id: id(1), of: "t:" + s, prev: [] }]), { refused: "string-not-unicode" });
    // A vector is Unicode throughout, the members no implementation reads included.
    for (const vector of [{ layer: "fold/count", input: { orders: [[]] }, expect: s }, { layer: "x", input: {}, note: { [s]: 0 } }]) {
      assert.throws(() => readVector(JSON.stringify(vector)), /lone surrogate/);
    }
    assert.throws(() => parseSpecText(JSON.stringify({ vertex: s, from: [{ store: "." }] })), refused);
  }
  assert.deepEqual(readVector('{"layer":"x","input":{},"expect":"\\ud83d\\ude00"}').expect, HIGH + LOW);
});

test("a lone surrogate follows raw shape and precedes every value check", () => {
  const refused = { message: "string-not-unicode" };
  const lone = '"\\ud800"', repeated = '{"a":1,"a":2,"b":"\\udc00"}';
  for (const text of [lone, repeated, '{"n":1e999,"b":"\\ud800"}']) {
    assert.throws(() => writeText(writerEnv, writerSalt, text, writerSeed), refused);
    assert.throws(() => writeText({ ...writerEnv, kind: "" }, writerSalt, text, writerSeed), refused);
    assert.throws(() => writeText(writerEnv, new Uint8Array(15), text, writerSeed), refused);
    assert.throws(() => writeText(writerEnv, () => { throw new Error("salt read"); }, text, writerSeed), refused);
    assert.throws(() => writeText({ ...writerEnv, extra: 1 }, writerSalt, text, writerSeed), { message: "envelope-field-unknown" });
  }
  assert.throws(() => seal({ ...writerEnv, kind: "", extra: 1 }, writerSalt, "\ud800", writerSeed), { message: "envelope-field-unknown" });
  assert.throws(() => seal({ ...writerEnv, kind: "" }, new Uint8Array(15), ["\ud800"], writerSeed), refused);
  const draft = { env: { ...writerEnv, extra: 1 }, salt: base32(writerSalt), payload: "\ud800" };
  assert.deepEqual(resolveBatch([draft], []), { refused: "envelope-field-unknown" });
  assert.deepEqual(resolveBatch([{ ...draft, env: writerEnv, label: "@a" }, { ...draft, env: writerEnv, label: "@a" }], []),
    { refused: "string-not-unicode" });
  // Syntax first: text that is not JSON has no refusal code, wherever the surrogate or repeat stands.
  for (const text of ['{"a" "\\ud800"}', '{"a":"\\ud800"', '"\\ud800', '["\\ud800",]', '"\\ud800" 1', '{"a":1,"a":2,"b" 3}', '"\\ud83d\\uZZZZ"']) {
    assert.throws(() => parseText(text), (e) => !/^[a-z0-9-]+$/.test(e.message), text);
    assert.throws(() => writeText(writerEnv, writerSalt, text, writerSeed), (e) => !/^[a-z0-9-]+$/.test(e.message), text);
  }
  assert.throws(() => parseText('["\\ud800\\n", "\\udc00"]'), refused);
  assert.throws(() => parseText('{"a":1,"a":2}'), { message: "member-duplicate" });
  assert.equal(parseText('"\\ud83d\\ude00"'), "😀");
});

// D3: overflow stays a number until the writer's number check.
test("a number past binary64's range is the writer's number refusal, in its usual place", () => {
  const out = { message: "number-out-of-range" };
  for (const text of ["1e999", "-1e999", '{"n":[1.5E+400]}']) {
    assert.throws(() => writeText(writerEnv, writerSalt, text, writerSeed), out);
    assert.throws(() => writeText({ ...writerEnv, kind: "" }, writerSalt, text, writerSeed), { message: "kind-empty" });
    assert.throws(() => writeText(writerEnv, new Uint8Array(15), text, writerSeed), { message: "salt-not-16-bytes" });
    assert.throws(() => writeText({ ...writerEnv, extra: 1 }, writerSalt, text, writerSeed), { message: "envelope-field-unknown" });
  }
  assert.throws(() => writeText(writerEnv, writerSalt, '{"n":1e999,"n":0}', writerSeed), { message: "member-duplicate" });
  // A record input's value, written back as text, overflows again rather than becoming null.
  const input = { envs: [writerEnv], salt: base32(writerSalt), seed: base32(writerSeed) };
  for (const payload of [Infinity, -Infinity, { n: [Infinity] }, NaN]) {
    for (const writer of ["seal", "writeText"]) {
      assert.throws(() => writeRecord({ ...input, payloads: [payload] }, { writer }), out);
    }
  }
  const draft = (payload, env = writerEnv) => ({ env, salt: base32(writerSalt), payload });
  assert.deepEqual(resolveBatch([draft({ n: Infinity })], []), { refused: "number-out-of-range" });
  assert.deepEqual(resolveBatch([draft(-Infinity), draft({}, { ...writerEnv, kind: "" })], []), { refused: "kind-empty" });
  assert.deepEqual(resolveBatch([draft(Infinity, { ...writerEnv, of: id(9), prev: [id(9)] })], []), { refused: "number-out-of-range" });
});

// D4: a fixture's signing seed is exactly 32 bytes, read once the record is written.
test("a signing seed is exactly 32 bytes and is read last", () => {
  const input = { envs: [writerEnv], payloads: [{}], salt: base32(writerSalt) };
  assert.equal(writeRecord({ ...input, seed: base32(writerSeed) }).id, seal(writerEnv, writerSalt, {}, writerSeed).id);
  for (const seed of [base32(new Uint8Array(31)), base32(new Uint8Array(33)), "", base32(new Uint8Array(32)).slice(0, -1) + "1", "!", undefined]) {
    for (const writer of ["seal", "writeText"]) {
      assert.throws(() => writeRecord({ ...input, seed }, { writer }), (e) => !/^[a-z0-9-]+$/.test(e.message), String(seed));
      assert.throws(() => writeRecord({ ...input, envs: [{ ...writerEnv, kind: "" }], seed }, { writer }), { message: "kind-empty" });
    }
  }
  for (const n of [0, 31, 33, 64]) {
    assert.throws(() => signId(new Uint8Array(n), id(1)), /exactly 32 bytes/);
    assert.throws(() => seal(writerEnv, writerSalt, {}, new Uint8Array(n)), /exactly 32 bytes/);
  }
});

// D5: the first envelope and first payload decide; a later variant never answers.
test("a record input's first variant decides, and later variants must agree once it is written", () => {
  const noKind = { ...writerEnv, kind: "" }, untyped = { ...writerEnv, observer: "nobody" };
  const input = (envs, payloads) => ({ envs, payloads, salt: base32(writerSalt), seed: base32(writerSeed) });
  const fault = (e) => /^record input: /.test(e.message);
  for (const writer of ["seal", "writeText"]) {
    assert.throws(() => writeRecord(input([noKind, writerEnv], [{}]), { writer }), { message: "kind-empty" });
    assert.throws(() => writeRecord(input([noKind, untyped], [{}]), { writer }), { message: "kind-empty" });
    assert.throws(() => writeRecord(input([untyped, noKind], [{}]), { writer }), { message: "observer-not-typed" });
    assert.throws(() => writeRecord(input([writerEnv, noKind], [2 ** 53, {}]), { writer }), { message: "number-out-of-range" });
    assert.throws(() => writeRecord(input([noKind, writerEnv], [2 ** 53, {}]), { writer }), { message: "kind-empty" });
    for (const [envs, payloads] of [[[writerEnv, noKind], [{}]], [[writerEnv, untyped], [{}]],
      [[writerEnv, { ...writerEnv, kind: "other" }], [{}]], [[writerEnv], [{}, 2 ** 53]], [[writerEnv], [null, Infinity]],
      [[writerEnv], [{ a: 1 }, { a: 2 }]], [[writerEnv], [{ a: 1 }, { a: "\ud800" }]],
      [[writerEnv, { ...writerEnv, about: ["t:\ud800"] }], [{}]]]) {
      assert.throws(() => writeRecord(input(envs, payloads), { writer }), fault, JSON.stringify([envs, payloads]));
    }
    const agreed = input([{ ...writerEnv, about: ["t:a", "t:b"] }, { ...writerEnv, about: ["t:b", "t:a", "t:b"] }],
      [{ a: 1, b: 2 }, { b: 2, a: 1 }]);
    assert.equal(writeRecord(agreed, { writer }).id, writeRecord(agreed).id);
  }
  // A repeated member survives only in the text; the caller that kept it passes it on.
  const texts = ['{"a":1}', '{"a":1,"a":2}'];
  const repeated = input([writerEnv], [{ a: 1 }, { a: 2 }]);
  assert.throws(() => writeRecord(repeated, { writer: "writeText", payloadTexts: texts }),
    /payloads\[1\] is refused \(member-duplicate\) where payloads\[0\] is written/);
  assert.throws(() => writeRecord(repeated, { writer: "writeText", payloadTexts: texts.toReversed() }), { message: "member-duplicate" });
});

// Times on the way in. Lean and Rust refuse a held record whose `emitted` is
// not a canonical UTC instant, or whose `occurred` is not a span, when they
// decode it (`Gen.Read.record`). JavaScript once read a time only when it
// ordered that record. It answered seven of the cases below; the other four
// threw with no path, a `TypeError` for an `emitted` that is not text and
// `occurred-not-span` for one past 23 hours. It now refuses each before
// answering, with the writer's code for the field and the field's path in the
// list it was given, the first record first and `emitted` first.
test("a reader refuses a held record's times before it answers", () => {
  const ok = record(1, "task:a");
  const bad = (n, times) => ({ ...record(n, "task:a"), ...times });
  const refused = (code, path) => ({ message: code, code, path });
  const noEmitted = record(2, "task:a");
  delete noEmitted.emitted;
  const cases = [
    // never ordered: an accumulating fold reads no time
    [() => sum([ok, bad(2, { occurred: "2024-99" })]), refused("occurred-not-span", "[1].occurred")],
    [() => count([ok, bad(2, { occurred: null })]), refused("occurred-not-span", "[1].occurred")],
    [() => min([noEmitted]), refused("emitted-not-rfc3339", "[0].emitted")],
    // a date is a span, so content order read it, but `emitted` is an instant
    [() => latestBy([bad(2, { emitted: "2020-01-01" })]), refused("emitted-not-rfc3339", "[0].emitted")],
    [() => latestBy([bad(2, { emitted: "2020-01-01T00:00:00.10Z" })]), refused("emitted-not-rfc3339", "[0].emitted")],
    [() => heads([ok, bad(2, { emitted: 1577836800 })]), refused("emitted-not-rfc3339", "[1].emitted")],
    // `emitted` before `occurred`, and the first record first
    [() => max([bad(2, { emitted: "", occurred: "2024-99" }), bad(3, { occurred: "x" })]), refused("emitted-not-rfc3339", "[0].emitted")],
    [() => runSpec(spec, [ok, bad(2, { occurred: "2024-13" }), bad(3, { emitted: "x" })]), refused("occurred-not-span", "[1].occurred")],
    // a superseded record is never ordered, and is still refused
    [() => runSpec(spec, [bad(2, { occurred: "2024-99" }), record(3, "task:a", [id(2)])]), refused("occurred-not-span", "[0].occurred")],
    // the paths are into the forms as given, not the settled records
    [() => realm([record(3, "task:a"), ok, bad(2, { occurred: "2024-02-30" })]), refused("occurred-not-span", "[2].occurred")],
    [() => step(spec, "task:a", [record(3, "task:a"), bad(2, { emitted: "2020-01-01T24:00:00Z" })]), refused("emitted-not-rfc3339", "[1].emitted")],
  ];
  for (const [run, want] of cases) assert.throws(run, want);
  // What reads is answered as before.
  assert.deepEqual(sum([ok, bad(2, { occurred: "2024-02" })]), { value: 3, unread: 0 });
  assert.deepEqual(count([]), { value: 0, unread: 0 });
});

// The model answers a spec's slots in the order the spec writes them. An
// object's own enumeration puts integer-like names first, so `slotsText` once
// wrote "1" and "10" before "b"; it now writes the spec's order, which the
// parsed spec and the answer carry beside their members.
test("slots are written in the spec's order, integer-like names included", () => {
  const text = '{"vertex":"v","from":[{"store":"."}],"fold":{"b":{"count":true},"10":{"count":true},"a":{"count":true},"1":{"sum":"n"}}}';
  const { parsed } = parseSpecText(text);
  const written = ["b", "10", "a", "1"];
  assert.deepEqual(slotNames(parsed.fold), written);
  const out = runSpec(parsed, [record(1, "task:a"), record(2, "task:b")]);
  assert.deepEqual(slotNames(out), written);
  assert.deepEqual(readJson(slotsText(out)).members.map(([name]) => name), written);
  assert.equal(slotsText(out), '{"b":{"value":2,"unread":0},"10":{"value":2,"unread":0},"a":{"value":2,"unread":0},"1":{"value":3,"unread":0}}');
  // The order is beside the members: the answer compares as before.
  assert.deepEqual(Object.keys(out), ["1", "10", "b", "a"]);
  assert.deepEqual({ ...out }, { 1: { value: 3, unread: 0 }, 10: { value: 2, unread: 0 }, a: { value: 2, unread: 0 }, b: { value: 2, unread: 0 } });
});

// A keyed thing's `keys` is a map from key names, written in UTF-16 order as
// the model writes it. An object's own enumeration would put "10" and "2"
// before "a"; and "10" comes before "2" in UTF-16 order.
test("a keyed thing's keys are written in UTF-16 order, integer-like names included", () => {
  const root = { ...record(1, "task:a"), fields: { a: 1, 10: 2, 2: 3 } };
  const set = (n, k, prev) => ({ ...record(n, "task:a", prev), sets: [k] });
  const records = [root, set(2, "a", [root.id]), set(3, "2", [id(2)]), set(4, "10", [id(3)])];
  const { parsed } = parseSpecText('{"vertex":"v","from":[{"store":"."}],"fold":{"s":{"latest-by":"of","follow":"keys"}}}');
  const text = slotsText(runSpec(parsed, records));
  const keys = readJson(text).members[0][1].members[0][1].members[0][1].members[0][1];
  assert.deepEqual(keys.members.map(([k]) => k), ["10", "2", "a"]);
});

// A slot that follows keys groups things: each live setter's key, written in
// UTF-16 order as the model writes groups. A thing whose two live setters of
// the field hold different keys is in both groups, and counted once in each.
test("a keyed count groups each thing by its live setters' keys, in UTF-16 order", () => {
  const root = { ...record(1, "task:a"), fields: { k: "a" } };
  const set = (n, k) => ({ ...record(n, "task:a", [root.id]), sets: ["k"], fields: { k } });
  const other = { ...record(4, "task:b"), fields: { k: "10" } };
  const records = [root, set(2, "2"), set(3, "10"), other];
  const { parsed } = parseSpecText('{"vertex":"v","from":[{"store":"."}],"fold":{"s":{"count":true,"follow":"keys","by":"k"},"t":{"count":true,"follow":"keys"},"h":{"count":true,"follow":"prev"}}}');
  const out = runSpec(parsed, records);
  const groups = readJson(slotsText(out)).members[0][1].members[0][1];
  assert.deepEqual(groups.members.map(([k]) => k), ["10", "2"]);
  assert.deepEqual({ ...out.s.groups }, { 10: { value: 2, unread: 0 }, 2: { value: 1, unread: 0 } });
  assert.deepEqual([out.t.value, out.h.value], [2, 3]);
});

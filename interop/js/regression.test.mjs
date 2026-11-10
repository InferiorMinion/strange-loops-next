import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { base32, base32Decode, fixedBytes, canonical, checkEnvelopeKeys, seal, writeText, writeRecord, problem, validEmitted, occurredSpan, signId, verifyId, commitOf, idOf, parse as parseText, wellFormed, unicodeProblem } from "./loops.mjs";
import { readVector } from "../answers.mjs";
import { parseSpecText } from "./spec.mjs";
import { resolveBatch } from "./batch.mjs";
import { heads, latest, sum, min, max } from "./fold.mjs";
import { parse, edit, diff, apply, baseProblem, render } from "./lens.mjs";
import { lensEdit } from "./scenario.mjs";
import { check as checkLens } from "./check-lens.mjs";
import { runSpec } from "./spec.mjs";
import { step } from "./scenario.mjs";

const id = (n) => `r:${base32(Uint8Array.from({ length: 32 }, () => n))}`;
const record = (n, about, prev = []) => ({
  id: id(n), kind: "note", about, prev, emitted: `2020-01-01T00:00:0${n}Z`, n,
});
const spec = { from: [{ store: "." }], fold: { count: { count: true } } };

test("latest uses the union of per-thing heads", () => {
  const a = record(1, ["task:a", "task:b"]);
  const b = record(2, ["task:a"], [a.id]);
  assert.deepEqual(heads([a, b]).map((r) => r.id), [b.id, a.id]);
  assert.deepEqual(latest([a, b]), { winner: b.id, others: [a.id] });
  assert.deepEqual(latest([]), { winner: null, others: [] });
});

test("spec and scenario admit once against all settled records", () => {
  const a = record(1, ["task:a"]);
  const b = record(2, ["task:b"], [a.id]);
  const c = record(3, ["task:a"], [b.id]);
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
  const records = [1.0, 1.5, 2 ** 53, "2", true].map((n, i) => ({ ...record(i + 1, ["task:a"]), n }));
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
      { ...writerEnv, occurred: s }, { ...writerEnv, under: s }, { ...writerEnv, prev: [s] }, { ...writerEnv, erases: [s] }]) {
      assert.throws(() => seal(env, writerSalt, {}, writerSeed), refused);
      assert.throws(() => writeText(env, writerSalt, "{}", writerSeed), refused);
      assert.throws(() => idOf(env, "h:x"), refused);
      const draft = { env, salt: base32(writerSalt), payload: {} };
      assert.deepEqual(resolveBatch([draft], []), { refused: "string-not-unicode" });
    }
    assert.deepEqual(resolveBatch([{ env: writerEnv, salt: base32(writerSalt), payload: { a: [s] } }], []), { refused: "string-not-unicode" });
    assert.deepEqual(resolveBatch([{ label: "@" + s, env: writerEnv, salt: base32(writerSalt), payload: {} }], []), { refused: "string-not-unicode" });
    assert.deepEqual(resolveBatch([{ env: writerEnv, salt: base32(writerSalt), payload: {} }], [{ id: id(1), about: ["t:" + s] }]), { refused: "string-not-unicode" });
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
  assert.deepEqual(resolveBatch([draft(Infinity, { ...writerEnv, prev: [id(9)] })], []), { refused: "number-out-of-range" });
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

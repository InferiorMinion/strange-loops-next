// A small, independent writer for layer 0: canonical bytes, commitment, id,
// signature. Node's own SHA-256 and Ed25519; JCS from JSON.stringify, whose
// string escapes and UTF-16 key order are exactly RFC 8785's. No dependencies.

import { createHash, createPrivateKey, createPublicKey, sign, verify } from "node:crypto";

const B32 = "0123456789abcdefghijklmnopqrstuv";

export function base32(bytes) {
  let out = "", bits = 0, value = 0;
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

export function base32Decode(text) {
  const out = [];
  let bits = 0, value = 0;
  for (const c of text) {
    const v = B32.indexOf(c);
    if (v < 0) throw new Error(`not base32hex: ${c}`);
    value = (value << 5) | v;
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  const bytes = Uint8Array.from(out);
  if (base32(bytes) !== text) throw new Error("not canonical base32hex");
  return bytes;
}

// Decode the same canonical text at every fixed-width byte boundary.
export function fixedBytes(text, prefix, length) {
  if (typeof text !== "string" || !text.startsWith(prefix)) throw new Error("bad byte prefix");
  const bytes = base32Decode(text.slice(prefix.length));
  if (bytes.length !== length) throw new Error("bad byte length");
  return bytes;
}

// Text is Unicode scalar values. A JavaScript string is UTF-16 code units and
// can hold a surrogate with no partner, which names no character: RFC 8785
// has no bytes for it. It is refused by name, and nothing is put in its place.
export const NOT_UNICODE = "string-not-unicode";

export function wellFormed(s) {
  for (let i = 0; i < s.length; i++) {
    const u = s.charCodeAt(i);
    if (u >= 0xdc00 && u <= 0xdfff) return false;
    if (u >= 0xd800 && u <= 0xdbff) {
      const l = s.charCodeAt(++i);
      if (!(l >= 0xdc00 && l <= 0xdfff)) return false;
    }
  }
  return true;
}

// Every string in a value, member names included, or the refusal.
export function unicodeProblem(v) {
  if (typeof v === "string") return wellFormed(v) ? null : NOT_UNICODE;
  if (Array.isArray(v)) return v.some(unicodeProblem) ? NOT_UNICODE : null;
  if (v !== null && typeof v === "object") {
    return Object.keys(v).some((k) => !wellFormed(k) || unicodeProblem(v[k])) ? NOT_UNICODE : null;
  }
  return null;
}

// RFC 8785. Numbers use ECMAScript's own formatting, which is the RFC's.
// Nothing reaches a hash except through here, so a lone surrogate never does.
export function canonical(v) {
  if (typeof v === "string" && !wellFormed(v)) throw new Error(NOT_UNICODE);
  if (v === null || typeof v === "boolean" || typeof v === "string") return JSON.stringify(v);
  if (typeof v === "number") {
    if (!Number.isFinite(v)) throw new Error("JCS has no NaN or Infinity");
    return JSON.stringify(v);
  }
  if (Array.isArray(v)) return "[" + v.map(canonical).join(",") + "]";
  return "{" + Object.keys(v).sort().map((k) => canonical(k) + ":" + canonical(v[k])).join(",") + "}";
}

const utf8 = (s) => new TextEncoder().encode(s);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest();

// Sets: sorted by UTF-16 code units (JavaScript's default sort), repeats removed.
const canonSet = (xs) => [...new Set(xs)].sort();

export function envJson(env) {
  const out = { kind: env.kind, observer: env.observer, emitted: env.emitted };
  if (env.occurred !== undefined) out.occurred = env.occurred;
  for (const k of ["about", "prev", "erases"]) {
    if (env[k] && env[k].length) out[k] = canonSet(env[k]);
  }
  if (env.under !== undefined) out.under = env.under;
  // `of`, like `occurred` and `under`, is left out when absent, so a record
  // without it keeps its bytes and id.
  if (env.of !== undefined) out.of = env.of;
  return out;
}

const DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const leap = (y) => (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;

export function validEmitted(s) {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{0,8}[1-9]))?Z$/.exec(s);
  if (!m || m[0] !== s) return false;
  const [y, mo, d, h, mi, se] = m.slice(1, 7).map(Number);
  if (mo < 1 || mo > 12) return false;
  const days = mo === 2 && leap(y) ? 29 : DAYS[mo - 1];
  if (d < 1 || d > days || h > 23 || mi > 59 || se > 59) return false;
  return true;
}

// A moment: whole seconds since 1970-01-01T00:00:00Z, then the fraction's
// canonical fraction digits.
// setUTCFullYear, because Date.UTC reads years 0 to 99 as 1900 to 1999.
const daySecs = (y, m, d) => {
  const t = new Date(0);
  t.setUTCFullYear(y, m - 1, d);
  return t.getTime() / 1000;
};

function instantMoment(s) {
  if (!validEmitted(s)) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?Z$/.exec(s);
  const [y, mo, d, h, mi, se] = m.slice(1, 7).map(Number);
  return [daySecs(y, mo, d) + h * 3600 + mi * 60 + se, m[7] ?? ""];
}

// A calendar date's first moment and the moment after its last.
function dateMoments(s) {
  const span = (a, b) => [[a, ""], [b, ""]];
  let m;
  if ((m = /^(\d{2})XX$/.exec(s))) return span(daySecs(100 * m[1], 1, 1), daySecs(100 * m[1] + 100, 1, 1));
  if ((m = /^(\d{3})X$/.exec(s))) return span(daySecs(10 * m[1], 1, 1), daySecs(10 * m[1] + 10, 1, 1));
  if ((m = /^(\d{4})(?:-XX(?:-XX)?)?$/.exec(s))) return span(daySecs(+m[1], 1, 1), daySecs(+m[1] + 1, 1, 1));
  if ((m = /^(\d{4})-(\d{2})(?:-XX)?$/.exec(s))) {
    const [y, mo] = [+m[1], +m[2]];
    return mo >= 1 && mo <= 12 ? span(daySecs(y, mo, 1), daySecs(y, mo + 1, 1)) : null;
  }
  if ((m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s))) {
    const [y, mo, d] = [+m[1], +m[2], +m[3]];
    if (mo < 1 || mo > 12 || d < 1 || d > (mo === 2 && leap(y) ? 29 : DAYS[mo - 1])) return null;
    return span(daySecs(y, mo, d), daySecs(y, mo, d + 1));
  }
  return null;
}

function pointMoments(s) {
  if (/[?~%]$/.test(s)) return dateMoments(s.slice(0, -1));
  if (s.includes("T")) {
    const t = instantMoment(s);
    return t && [t, t];
  }
  return dateMoments(s);
}

export const momentBefore = (a, b) => a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]);

// occurred: its span, { start, end }, end null when open; null if it isn't one.
export function occurredSpan(s) {
  const parts = s.split("/");
  if (parts.length === 1) {
    const p = pointMoments(parts[0]);
    return p && { start: p[0], end: p[1] };
  }
  if (parts.length !== 2) return null;
  const p = pointMoments(parts[0]);
  if (!p) return null;
  if (parts[1] === "..") return { start: p[0], end: null };
  const q = pointMoments(parts[1]);
  if (!q || momentBefore(q[0], p[0])) return null;
  return { start: p[0], end: q[1] };
}

// Spans oldest first: by start, then end, an open end last.
export function spanBefore(a, b) {
  if (momentBefore(a.start, b.start)) return true;
  if (momentBefore(b.start, a.start)) return false;
  if (a.end === null) return false;
  return b.end === null || momentBefore(a.end, b.end);
}

const typedRef = (s) => /^[a-z][a-z0-9-]*:./su.test(s);
const recordId = (s) => {
  try { fixedBytes(s, "r:", 32); return true; } catch { return false; }
};
const ENV_KEYS = new Set(["kind", "observer", "emitted", "occurred", "about", "of", "prev", "under", "erases"]);

// The envelope's own fields, read by name: after the shape check there are no others.
const envUnicodeProblem = (env) =>
  env !== null && typeof env === "object" && [...ENV_KEYS].some((k) => unicodeProblem(env[k])) ? NOT_UNICODE : null;

export const unknownEnvelopeKeys = (env) => env !== null && typeof env === "object" && !Array.isArray(env) && Reflect.ownKeys(env).some((key) => !ENV_KEYS.has(key));

// Check all raw envelope keys before reading any of their values.
export function checkEnvelopeKeys(envs) {
  if (envs.some(unknownEnvelopeKeys)) throw new Error("envelope-field-unknown");
}

export const notObject = (env) => env === null || typeof env !== "object" || Array.isArray(env);

// Raw shape across every envelope (`envelopesShapeProblem`,
// model/Loops/Envelope.lean): an unknown key in any of them, then any that is
// not an object. Both come before any value, label or salt.
export function checkEnvelopeShapes(envs) {
  checkEnvelopeKeys(envs);
  if (envs.some(notObject)) throw new Error("envelope-not-object");
}

// What the reader reads for a field it can't read as its JSON type: a required
// field left out, a field of the wrong type, or a set member that is not text.
// It is empty text, which the field's own check refuses, so the refusal is the
// field's code in the field's place (`unreadable`, model/Loops/Envelope.lean).
const UNREADABLE = "";
const textOr = (v) => (typeof v === "string" ? v : UNREADABLE);
const textsOr = (v) => (Array.isArray(v) ? v.map(textOr) : [UNREADABLE]);

// An object envelope's values as the writer checks them (`decodeValues`,
// model/Loops/Read.lean). An envelope the writer accepts reads as itself.
export function readEnv(raw) {
  const has = (k) => Object.hasOwn(raw, k);
  const env = {
    kind: has("kind") ? textOr(raw.kind) : UNREADABLE,
    observer: has("observer") ? textOr(raw.observer) : UNREADABLE,
    emitted: has("emitted") ? textOr(raw.emitted) : UNREADABLE,
  };
  for (const k of ["occurred", "of", "under"]) if (has(k)) env[k] = textOr(raw[k]);
  for (const k of ["about", "prev", "erases"]) if (has(k)) env[k] = textsOr(raw[k]);
  return env;
}

// A salt written in base32hex. One left out, not text, or not canonical
// base32hex reads as no bytes, which the salt check refuses as
// `salt-not-16-bytes` in its place (`Read.salt`, model/Loops/Read.lean).
export function readSalt(text) {
  if (typeof text !== "string") return new Uint8Array(0);
  try { return base32Decode(text); } catch { return new Uint8Array(0); }
}

// The writer's envelope checks, in the model's order (`Env.problem`,
// model/Loops/Envelope.lean). `of` is one typed reference, checked right after
// `about`; a record with `prev` and no `of` is refused last, once every field
// is well formed.
export function problem(env) {
  if (!env.kind) return "kind-empty";
  if (!typedRef(env.observer)) return "observer-not-typed";
  if (!validEmitted(env.emitted)) return "emitted-not-rfc3339";
  if (env.occurred !== undefined && !occurredSpan(env.occurred)) return "occurred-not-span";
  if (!(env.about ?? []).every(typedRef)) return "about-not-typed";
  if (env.of !== undefined && !(typeof env.of === "string" && typedRef(env.of))) return "of-not-typed";
  if (!(env.prev ?? []).every(recordId)) return "prev-not-record-id";
  if (env.under !== undefined && !recordId(env.under)) return "under-not-record-id";
  if (!(env.erases ?? []).every(recordId)) return "erases-not-record-id";
  if ((env.prev ?? []).length && env.of === undefined) return "of-missing";
  return null;
}

export function commitOf(salt, payload) {
  return "h:" + base32(sha256(Buffer.concat([Buffer.from(salt), utf8(canonical(payload))])));
}

export const idPreimage = (env, commit) => canonical({ commit, env: envJson(env) });
export const idOf = (env, commit) => "r:" + base32(sha256(utf8(idPreimage(env, commit))));
export const signingInput = (id) => "loops/record/v1:" + id;

// An Ed25519 private key from its 32-byte seed (PKCS #8 wrapping). A seed of
// any other length is refused: none is padded or cut.
const PKCS8 = Buffer.from("302e020100300506032b657004220420", "hex");
const privateKey = (seed) => {
  if (seed.length !== 32) throw new Error(`a signing seed is exactly 32 bytes, not ${seed.length}`);
  return createPrivateKey({ key: Buffer.concat([PKCS8, Buffer.from(seed)]), format: "der", type: "pkcs8" });
};

export function publicKey(seed) {
  return createPublicKey(privateKey(seed)).export({ format: "der", type: "spki" }).subarray(-32);
}

export function signId(seed, id) {
  return { key: "k:" + base32(publicKey(seed)), ed25519: base32(sign(null, utf8(signingInput(id)), privateKey(seed))) };
}

export function verifyId(sig, id) {
  const spki = Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(fixedBytes(sig.key, "k:", 32))]);
  const key = createPublicKey({ key: spki, format: "der", type: "spki" });
  return verify(null, utf8(signingInput(id)), key, Buffer.from(fixedBytes(sig.ed25519, "", 64)));
}

// Writers bound magnitude, whole or not; numeric readers additionally require whole values.
const inRange = (v) =>
  typeof v === "number" ? Number.isFinite(v) && Math.abs(v) <= Number.MAX_SAFE_INTEGER
  : v !== null && typeof v === "object" ? Object.values(v).every(inRange)
  : true;

// JSON text to a value, refusing a repeated member name at any depth, which
// JSON.parse would silently resolve to the last one, and a lone surrogate,
// written out or as an escape. The text's syntax comes first: text that is not
// JSON is a fault with no refusal code, wherever a surrogate or a repeat
// stands in it. Of JSON text, a lone surrogate is refused before a repeated
// name: text is decoded, then validated.
export function parse(text) {
  let i = 0, repeated = false, lone = false;
  const string = (s) => { if (!wellFormed(s)) lone = true; return s; };
  const ws = () => { while (" \t\n\r".includes(text[i]) && i < text.length) i++; };
  const token = (re) => { re.lastIndex = i; const m = re.exec(text); if (!m) throw new Error(`bad JSON at ${i}`); i = re.lastIndex; return m[0]; };
  const STR = /"(?:[^"\\\u0000-\u001f]|\\.)*"/y, NUM = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
  function value() {
    ws();
    const c = text[i];
    if (c === "{") {
      i++; const o = {}, seen = new Set();
      ws(); if (text[i] === "}") { i++; return o; }
      for (;;) {
        ws(); const k = string(JSON.parse(token(STR)));
        if (seen.has(k)) repeated = true;
        // Defined, not assigned: assigning "__proto__" would set the prototype.
        seen.add(k); ws(); token(/:/y);
        Object.defineProperty(o, k, { value: value(), enumerable: true, writable: true, configurable: true });
        ws();
        if (text[i] === ",") { i++; continue; }
        token(/\}/y); return o;
      }
    }
    if (c === "[") {
      i++; const a = [];
      ws(); if (text[i] === "]") { i++; return a; }
      for (;;) { a.push(value()); ws(); if (text[i] === ",") { i++; continue; } token(/\]/y); return a; }
    }
    if (c === '"') return string(JSON.parse(token(STR)));
    if (c === "t" || c === "f" || c === "n") return JSON.parse(token(/true|false|null/y));
    return Number(token(NUM));
  }
  const v = value(); ws();
  if (i !== text.length) throw new Error(`bad JSON at ${i}`);
  if (lone) throw new Error(NOT_UNICODE);
  if (repeated) throw new Error("member-duplicate");
  return v;
}

const saltProblem = (salt) => (salt.length === 16 ? null : "salt-not-16-bytes");

// Write a record from its payload as JSON text, checking in the model's order:
// the envelope's shape, lone surrogates, the envelope's values, the salt,
// repeated member names, then numbers. A number past binary64's range reads as
// an infinity and meets the number check like any other out of range.
export function writeText(env, salt, text, seed) {
  checkEnvelopeShapes([env]);
  let payload, fault = null;
  try { payload = parse(text); } catch (e) { fault = e; }
  if (fault?.message === NOT_UNICODE || envUnicodeProblem(env)) throw new Error(NOT_UNICODE);
  env = readEnv(env);
  salt = prepare(env, salt);
  if (fault) throw fault;
  return finish(env, salt, payload, seed);
}

// The envelope's values, then the salt. Shape and Unicode come before this.
function prepare(env, salt) {
  const why = problem(env);
  if (why) throw new Error(why);
  salt = typeof salt === "function" ? salt() : salt;
  const saltWhy = saltProblem(salt);
  if (saltWhy) throw new Error(saltWhy);
  return salt;
}

// Seal a record: the stored line, and the line after erasure. Refuses unknown
// envelope keys and an envelope that is not an object, then a lone surrogate
// anywhere in the envelope or payload, then a bad envelope (a field left out or
// of the wrong type among them), a salt that isn't 16 bytes, and last a
// nonfinite or out-of-range payload number.
// A seed supplier defers fixture seed decoding until all write checks pass.
export function seal(env, salt, payload, seed) {
  checkEnvelopeShapes([env]);
  if (envUnicodeProblem(env) || unicodeProblem(payload)) throw new Error(NOT_UNICODE);
  env = readEnv(env);
  salt = prepare(env, salt);
  return finish(env, salt, payload, seed);
}

function finish(env, salt, payload, seed) {
  if (!inRange(payload)) throw new Error("number-out-of-range");
  const commit = commitOf(salt, payload);
  const id = idOf(env, commit);
  const sig = signId(typeof seed === "function" ? seed({ salt, commit, id }) : seed, id);
  return {
    commit, id, sig,
    line: canonical({ id, env: envJson(env), salt: base32(salt), payload, sig }),
    erasedLine: canonical({ id, env: envJson(env), commit, sig }),
  };
}

// JSON text for a value. A nonfinite number is written as a spelling that
// overflows back to it, since JSON.stringify would write null.
function textOf(v) {
  if (typeof v === "number") return Number.isFinite(v) ? JSON.stringify(v) : v < 0 ? "-1e999" : "1e999";
  if (Array.isArray(v)) return "[" + v.map(textOf).join(",") + "]";
  if (v !== null && typeof v === "object") return "{" + Object.keys(v).map((k) => JSON.stringify(k) + ":" + textOf(v[k])).join(",") + "}";
  return JSON.stringify(v);
}

// What refuses a variant that the writer isn't asked to write, or null.
function variantProblem(env, salt, payload) {
  try { seal(env, salt, payload, () => { throw new Error("written"); }); }
  catch (e) { return e.message === "written" ? null : e.message; }
  return null;
}

// A record input: envelope variants, payload variants (or one payload as
// text), a salt and a seed, as vectors/README.md describes. The fixture
// contract, in order:
//  * every envelope's raw shape is checked: an unknown key in any, then any
//    that is not an object. Siblings are checked here and the active envelope
//    at the writer's own shape boundary, so an unknown key in it still comes
//    before a sibling that is not an object;
//  * the first envelope and first payload are written, and their refusal is
//    the answer whatever the other variants are. A salt that is not base32hex
//    text reads as no bytes, which the salt check refuses;
//  * once they are written, every other variant must be writable and name the
//    same record, or the input is at fault (an error that is no refusal code);
//  * only then is the seed read, and it is exactly 32 bytes.
// `payloadTexts` gives each payload variant as it was written, for a caller
// that kept the raw text: a JavaScript value cannot hold a repeated member.
export function writeRecord(input, { envIndex = 0, payloadIndex = 0, writer = "seal", payloadTexts } = {}) {
  const siblings = input.envs.filter((_, i) => i !== envIndex);
  const env = input.envs[envIndex];
  checkEnvelopeKeys(siblings);
  if (siblings.some(notObject) && !unknownEnvelopeKeys(env)) throw new Error("envelope-not-object");
  const salt = () => readSalt(input.salt);
  const fault = (what) => { throw new Error(`record input: ${what}`); };
  const seed = ({ salt, commit, id }) => {
    input.envs.forEach((e, i) => {
      if (i === envIndex) return;
      const why = envUnicodeProblem(e) ?? problem(readEnv(e));
      if (why) fault(`envs[${i}] is refused (${why}) where envs[${envIndex}] is written`);
      if (idOf(e, commit) !== id) fault(`envs[${i}] names another record than envs[${envIndex}]`);
    });
    const variants = payloadTexts ?? input.payloads ?? [];
    variants.forEach((p, i) => {
      if (i === payloadIndex) return;
      let value = p;
      if (payloadTexts) {
        try { value = parse(p); }
        catch (e) { fault(`payloads[${i}] is refused (${e.message}) where payloads[${payloadIndex}] is written`); }
      }
      const why = variantProblem(env, salt, value);
      if (why) fault(`payloads[${i}] is refused (${why}) where payloads[${payloadIndex}] is written`);
      if (commitOf(salt, value) !== commit) fault(`payloads[${i}] commits to another payload than payloads[${payloadIndex}]`);
    });
    if (typeof input.seed !== "string") fault("seed is missing");
    return fixedBytes(input.seed, "", 32);
  };
  if (writer === "writeText") {
    const text = input.payload_text ?? payloadTexts?.[payloadIndex] ?? textOf(input.payloads[payloadIndex]);
    return writeText(env, salt, text, seed);
  }
  return seal(env, salt, input.payloads[payloadIndex], seed);
}

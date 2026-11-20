// The canonical result (D7), version 1: a vertex's whole result as bytes and
// a hash, separate from the reading `runSpec` returns. protocol/result.md
// defines it; this is an implementation of that text, not of the model.
//
// Every integer is a JSON string of decimal digits. It is never a JS Number on
// its way into the text: a Number is accepted only while it is a safe integer
// and is turned into digits through BigInt, and a BigInt is exact. An integer
// sits only at `value`, `unread` and `ungrouped`; `value` is null for a min or
// max that read nothing. Listed records are their ids alone, without `shown`.
// The bytes are JCS (RFC 8785) of that JSON, in UTF-8; the hash is SHA-256 of
// the label `loops/result/v1:` and the bytes, written `h:` and base32hex.

import { createHash } from "node:crypto";
import { base32, canonical } from "./loops.mjs";

export const RESULT_LABEL = "loops/result/v1:";

// An integer as the codec writes it: decimal digits, `-` before a negative,
// `0` for zero (never `-0`). Anything that is not an exact integer is refused.
export function intText(v) {
  if (typeof v === "bigint") return v.toString();
  if (typeof v === "number" && Number.isSafeInteger(v)) return BigInt(v).toString();
  throw new Error(`result: ${String(v)} is not an exact integer`);
}

// An integer as the codec reads it, as a BigInt: `0`, or digits with no
// leading zero, with `-` before them for a negative. Anything else is null.
export function intOf(text) {
  return typeof text === "string" && /^(0|-?[1-9][0-9]*)$/.test(text) ? BigInt(text) : null;
}

const idsOf = (hs) => {
  if (!Array.isArray(hs)) throw new Error("result: an id list is not a list");
  return hs.map((h) => {
    if (typeof h?.id !== "string") throw new Error("result: a listed record has no id");
    return h.id;
  });
};

// A map's members, each value written by f. Member order is JCS's to set.
const mapOf = (o, f) => {
  const out = Object.create(null);
  for (const k of Object.keys(o)) out[k] = f(o[k]);
  return out;
};

// One group's value, or a slot's without `by`: a number, or a list of heads.
function plain(v) {
  if ("heads" in v) return { heads: idsOf(v.heads), unread: intText(v.unread) };
  if ("value" in v) return { value: v.value === null ? null : intText(v.value), unread: intText(v.unread) };
  throw new Error(`result: a value of no known shape: ${Object.keys(v).join(", ")}`);
}

// A thing's keyed state: each declared key's live setters, the live
// whole-read emits and the heads, each by id.
const keyedThing = (k) => ({ keys: mapOf(k.keys, idsOf), whole: idsOf(k.whole), heads: idsOf(k.heads) });

// A slot's value in the canonical result. The reading's member names say which
// shape it is; a latest-by lists each thing's heads, one that follows `keys`
// gives each thing's keyed state.
export function canonicalSlot(v) {
  if (v === null || typeof v !== "object") throw new Error("result: a slot's value is not an object");
  if ("groups" in v) return { groups: mapOf(v.groups, plain), unread: intText(v.unread), ungrouped: intText(v.ungrouped) };
  if ("things" in v) {
    return { things: mapOf(v.things, (x) => (Array.isArray(x) ? idsOf(x) : keyedThing(x))), unread: intText(v.unread) };
  }
  return plain(v);
}

// A vertex's whole result, every slot, as canonical JSON text.
export function resultText(out) {
  const slots = Object.create(null);
  for (const n of Object.keys(out)) slots[n] = canonicalSlot(out[n]);
  return canonical(slots);
}

export const resultBytes = (out) => new TextEncoder().encode(resultText(out));

// What the hash covers: the label, then the canonical bytes.
export const resultPreimage = (out) => Buffer.concat([Buffer.from(RESULT_LABEL, "utf8"), resultBytes(out)]);

export const resultHash = (out) => "h:" + base32(createHash("sha256").update(resultPreimage(out)).digest());

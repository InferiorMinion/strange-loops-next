// A vertex's spec: parse it per protocol/spec.md (the parsed form, or a
// refusal with its code and path, checked in the stated order), and run it
// over records per protocol/laws.md, reusing the folds and where-tests.
// Written from protocol/ and vectors/README.md. No dependencies.

import { occurredSpan, problem, wellFormed, NOT_UNICODE } from "./loops.mjs";
import { answer, held, payloadOf, YES, UNKNOWN } from "./where.mjs";
import * as F from "./fold.mjs";
import { admit } from "./realm.mjs";

// JSON as written: an object keeps its members in order, repeats included,
// as { members: [[key, node], ...] }; an array is { items: [node, ...] };
// anything else is its value. JSON.parse would keep one of a repeat and put
// integer-like keys first, and the refusals depend on both.

export const isObject = (n) => n !== null && typeof n === "object" && Array.isArray(n.members);
export const isArray = (n) => n !== null && typeof n === "object" && Array.isArray(n.items);

export function readJson(text) {
  let i = 0, lone = false;
  // A lone surrogate is no text. This reader has no refusal of its own for
  // it: once the text is known to be JSON, it is a fault like any other.
  const string = (s) => { if (!wellFormed(s)) lone = true; return s; };
  const ws = () => { while (" \t\n\r".includes(text[i]) && i < text.length) i++; };
  const token = (re) => { re.lastIndex = i; const m = re.exec(text); if (!m) throw new Error(`bad JSON at ${i}`); i = re.lastIndex; return m[0]; };
  const STR = /"(?:[^"\\\u0000-\u001f]|\\.)*"/y, NUM = /-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/y;
  function value() {
    ws();
    const c = text[i];
    if (c === "{") {
      i++; const members = [];
      ws(); if (text[i] === "}") { i++; return { members }; }
      for (;;) {
        ws(); const k = string(JSON.parse(token(STR)));
        ws(); token(/:/y);
        members.push([k, value()]);
        ws();
        if (text[i] === ",") { i++; continue; }
        token(/\}/y); return { members };
      }
    }
    if (c === "[") {
      i++; const items = [];
      ws(); if (text[i] === "]") { i++; return { items }; }
      for (;;) { items.push(value()); ws(); if (text[i] === ",") { i++; continue; } token(/\]/y); return { items }; }
    }
    if (c === '"') return string(JSON.parse(token(STR)));
    if (c === "t" || c === "f" || c === "n") return JSON.parse(token(/true|false|null/y));
    return Number(token(NUM));
  }
  const v = value(); ws();
  if (i !== text.length) throw new Error(`bad JSON at ${i}`);
  if (lone) throw new Error(NOT_UNICODE);
  return v;
}

// A value JSON.parse gave, as a node. Its members come in Object.keys order,
// which puts integer-like keys first: read text with readJson when that matters.
export const nodeOf = (v) =>
  Array.isArray(v) ? { items: v.map(nodeOf) }
  : v !== null && typeof v === "object" ? { members: Object.entries(v).map(([k, x]) => [k, nodeOf(x)]) }
  : v;

// A node as a plain value. Only called once repeats are ruled out;
// fromEntries defines members, so a key "__proto__" stays a key.
const plain = (n) =>
  isArray(n) ? n.items.map(plain)
  : isObject(n) ? Object.fromEntries(n.members.map(([k, x]) => [k, plain(x)]))
  : n;

const get = (n, key) => n.members.find(([k]) => k === key)?.[1];

// The first repeated key, depth first in member order: a member's value is
// searched before the members after it, an array's elements in order. The
// path of keys from the top, or null when nothing repeats.
export function repeatAt(n, path = []) {
  if (isArray(n)) {
    for (const x of n.items) { const at = repeatAt(x, path); if (at) return at; }
  } else if (isObject(n)) {
    const seen = new Set();
    for (const [k, x] of n.members) {
      if (seen.has(k)) return [...path, k];
      seen.add(k);
      const at = repeatAt(x, [...path, k]);
      if (at) return at;
    }
  }
  return null;
}

// What layer 0 accepts in a field, asked of loops.mjs's own envelope check
// (an envelope valid but for that one field) so the two can't drift apart.
const VALID = { kind: "k", observer: "x:y", emitted: "1970-01-01T00:00:00Z" };
const text = (v) => typeof v === "string" && v !== "";
const typedRef = (v) => typeof v === "string" && problem({ ...VALID, about: [v] }) === null;
const recordId = (v) => typeof v === "string" && problem({ ...VALID, under: v }) === null;
const time = (v) => typeof v === "string" && occurredSpan(v) !== null;
// A payload number layer 0 writes: whole, within ±(2^53 − 1).
const whole = (v) => Number.isSafeInteger(v);
const payloadValue = (v) => typeof v === "string" || typeof v === "boolean" || whole(v);

// Each envelope field and what `is` and `not` may compare it with.
const ENV_FIELDS = {
  kind: text, observer: typedRef, about: typedRef, of: typedRef,
  prev: recordId, under: recordId, erases: recordId,
  emitted: time, occurred: time,
};

const TOP_KEYS = new Set(["vertex", "from", "fold", "note", "ticks", "order", "resolve", "window", "boundary", "param", "covers", "into", "bundle"]);
const SOURCE_KEYS = new Set(["store", "kind", "about", "of"]);
const FOLD_TYPES = ["count", "sum", "min", "max", "latest-by", "collect"];
const SLOT_KEYS = new Set([...FOLD_TYPES, "follow", "limit", "by", "where", "note"]);

// Each fold type's argument.
const ARGUMENT = {
  count: (v) => v === true, collect: (v) => v === true,
  sum: text, min: text, max: text,
  // Per thing, by `of`; `about` only mentions and makes no head.
  "latest-by": (v) => v === "of",
};

class Refusal extends Error {
  constructor(code, path) { super(code); this.code = code; this.path = path; }
}
const refuse = (code, path) => { throw new Refusal(code, path); };

// A where field's value test, or null when the name isn't a field.
function fieldValue(field) {
  if (field.startsWith("env.")) return Object.hasOwn(ENV_FIELDS, field.slice(4)) ? ENV_FIELDS[field.slice(4)] : null;
  return field === "" ? null : payloadValue;
}

function testFits(field, op, arg) {
  switch (op) {
    case "is": case "not": return fieldValue(field)(arg);
    case "present": case "absent": return arg === true;
    case "at-least": case "below":
      if (field === "env.emitted" || field === "env.occurred") return time(arg);
      return !field.startsWith("env.") && whole(arg);
    default: return false;
  }
}

// What `by` names, or null: `env.kind`, `env.observer`, `env.about:<prefix>`
// with a typed-reference prefix, or a payload field `where` could name. A
// bare name is a payload field, whatever it looks like.
const REF_PREFIX = /^[a-z][a-z0-9-]*$/;
function byOf(v) {
  if (typeof v !== "string") return null;
  if (v.startsWith("env.about:")) {
    const prefix = v.slice("env.about:".length);
    return REF_PREFIX.test(prefix) ? { about: prefix } : null;
  }
  if (v === "env.kind") return { env: "kind" };
  if (v === "env.observer") return { env: "observer" };
  return v !== "" && !v.startsWith("env.") ? { field: v } : null;
}

// A payload value as a group key: text as written, a whole number in decimal,
// a boolean as true or false; anything else has none.
const keyOf = (v) =>
  typeof v === "string" ? v : typeof v === "boolean" || whole(v) ? String(v) : undefined;

// The groups a record joins, each once, sorted; [] when its key is readably
// absent; null when it can't be read (a payload field the reader doesn't hold).
export function keysOf(by, r) {
  const b = byOf(by);
  if (b.env) return [r[b.env]];
  if (b.about) {
    const head = `${b.about}:`;
    return [...new Set((r.about ?? []).filter((a) => a.startsWith(head)).map((a) => a.slice(head.length)))].sort();
  }
  const p = payloadOf(r);
  if (p === null) return null;
  const k = Object.hasOwn(p, b.field) ? keyOf(p[b.field]) : undefined;
  return k === undefined ? [] : [k];
}

function whereOf(n, at) {
  if (!isObject(n) || n.members.length === 0) refuse("where-invalid", at);
  for (const [field, tests] of n.members) {
    const here = [...at, field];
    if (!fieldValue(field) || !isObject(tests) || tests.members.length === 0) refuse("where-invalid", here);
    for (const [op, arg] of tests.members) if (!testFits(field, op, arg)) refuse("where-invalid", here);
  }
  return plain(n);
}

function slotOf(name, n) {
  const at = ["fold", name];
  if (name === "" || !isObject(n)) refuse("fold-invalid", at);
  for (const [k] of n.members) if (!SLOT_KEYS.has(k)) refuse("unknown-key", [...at, k]);
  const types = n.members.map(([k]) => k).filter((k) => FOLD_TYPES.includes(k));
  if (types.length !== 1) refuse("fold-type", at);
  const [type] = types;
  const [follow, limit, by, where] = ["follow", "limit", "by", "where"].map((k) => get(n, k));
  if (type === "latest-by" && follow === undefined) refuse("follow-missing", [...at, "follow"]);
  // Every fold type may follow prev; latest-by must.
  if (follow !== undefined && follow !== "prev") refuse("follow-invalid", [...at, "follow"]);
  if (limit !== undefined && (!(Number.isSafeInteger(limit) && limit >= 1) || type !== "collect")) refuse("limit-invalid", [...at, "limit"]);
  // by: after limit, before the argument; count, sum, min and max, and collect without follow.
  if (by !== undefined && (byOf(by) === null || type === "latest-by" || (type === "collect" && follow !== undefined)))
    refuse("by-invalid", [...at, "by"]);
  if (!ARGUMENT[type](get(n, type))) refuse("fold-argument", [...at, type]);
  const slot = { [type]: get(n, type) };
  if (follow !== undefined) slot.follow = follow;
  if (limit !== undefined) slot.limit = limit;
  if (by !== undefined) slot.by = by;
  if (where !== undefined) slot.where = whereOf(where, [...at, "where"]);
  return slot;
}

// store, kind, about and of are checked in that order however they are written.
function sourceOf(n) {
  if (!isObject(n)) refuse("from-invalid", ["from"]);
  for (const [k] of n.members) if (!SOURCE_KEYS.has(k)) refuse("unknown-key", ["from", k]);
  const [store, kind, about, of] = ["store", "kind", "about", "of"].map((k) => get(n, k));
  if (!text(store)) refuse("from-invalid", ["from", "store"]);
  if (kind !== undefined && !(isArray(kind) && kind.items.length > 0 && kind.items.every(text))) refuse("from-invalid", ["from", "kind"]);
  if (about !== undefined && !typedRef(about)) refuse("from-invalid", ["from", "about"]);
  if (of !== undefined && !typedRef(of)) refuse("from-invalid", ["from", "of"]);
  const source = { store };
  if (kind !== undefined) source.kind = plain(kind);
  if (about !== undefined) source.about = about;
  if (of !== undefined) source.of = of;
  return source;
}

function specOf(n) {
  const repeat = repeatAt(n);
  if (repeat) refuse("member-duplicate", repeat);
  if (!isObject(n)) refuse("spec-not-object", []);
  for (const [k] of n.members) if (!TOP_KEYS.has(k)) refuse("unknown-key", [k]);

  const vertex = get(n, "vertex");
  if (vertex === undefined) refuse("vertex-missing", ["vertex"]);
  if (!text(vertex)) refuse("vertex-invalid", ["vertex"]);

  const from = get(n, "from");
  if (from === undefined || (isArray(from) && from.items.length === 0)) refuse("from-missing", ["from"]);
  if (!isArray(from)) refuse("from-invalid", ["from"]);
  const sources = from.items.map(sourceOf);

  const suppliedFold = get(n, "fold");
  const fold = suppliedFold === undefined ? { members: [] } : suppliedFold;
  if (!isObject(fold)) refuse("fold-invalid", ["fold"]);
  const slots = fold.members.map(([name, slot]) => [name, slotOf(name, slot)]);
  return { vertex, from: sources, fold: Object.fromEntries(slots) };
}

// A spec as a node: { parsed } in its parsed form, or { refused, path } with
// the path's keys joined by ".".
export function parseNode(n) {
  try {
    return { parsed: specOf(n) };
  } catch (e) {
    if (!(e instanceof Refusal)) throw e;
    return { refused: e.code, path: e.path.join(".") };
  }
}

export const parseSpecText = (t) => parseNode(readJson(t));
export const parseSpec = (v) => parseNode(nodeOf(v));

// Running a parsed spec over the records a reader holds, one per id, in the
// fold vectors' JSON form.

// A source takes a record of one of its kinds whose `about` names its `about`,
// as written, and whose thing is its `of`, both when both are given; the spec,
// a record any source takes. Only envelopes are read (vectors/README.md, Spec).
const sourceTakes = (s, r) =>
  (s.kind === undefined || s.kind.includes(r.kind)) &&
  (s.about === undefined || (r.about ?? []).includes(s.about)) &&
  (s.of === undefined || F.thingOf(r) === s.of);
export const takes = (spec, r) => spec.from.some((s) => sourceTakes(s, r));

const shownOf = (r) => ({ id: r.id, shown: held(r) ? "here" : r.shown });

// Of some records, those every test holds for, and those a test couldn't answer.
function choose(records, where) {
  const chosen = [], unknown = [];
  for (const r of records) {
    const a = answer(where, r);
    if (a === YES) chosen.push(r);
    else if (a === UNKNOWN) unknown.push(r);
  }
  return { chosen, unknown };
}

// Collect: the chosen records newest first, the newest `limit` when given.
function collect(records, slot) {
  const { chosen, unknown } = choose(records, slot.where);
  const heads = chosen.sort(F.newestFirst).map(shownOf);
  return { heads: slot.limit === undefined ? heads : heads.slice(0, slot.limit), unread: unknown.length };
}

// Latest-by: per thing, its heads `from` takes and every test holds for; a
// thing with none is left out. A head the tests couldn't answer is unread
// once; a record heads at most one thing.
function latestBy(states, spec, slot) {
  const things = Object.create(null), unread = new Set();
  for (const [t, hs] of states) {
    const { chosen, unknown } = choose(hs.filter((r) => takes(spec, r)), slot.where);
    for (const r of unknown) unread.add(r.id);
    if (chosen.length) things[t] = chosen.map(shownOf);
  }
  return { things, unread: unread.size };
}

// Accumulating slots select, then fold the records `from` takes. Slots over
// heads (latest-by, and collect, count, sum, min and max following prev)
// supersede over every record held, then choose among the heads, so selection
// never brings back what was superseded. Without follow they take every
// selected record.
function runSlot(slot, spec, ctx) {
  return slot.by === undefined ? plainSlot(slot, spec, ctx) : grouped(slot, spec, ctx);
}

// A slot with `by`. Settling, admission and the whole-reader heads are already
// done, over everything held; a group never redoes them. Each record the slot
// selects joins the group of each of its keys, and each group is the slot
// without `by` or `where` over its own records. Unknown `where` answers and
// unreadable keys are unread at the top; a record without a key is ungrouped.
function grouped(slot, spec, ctx) {
  const pool = slot.follow === "prev" ? ctx.current : ctx.taken;
  const members = new Map();
  let unread = 0, ungrouped = 0;
  for (const r of pool) {
    const a = answer(slot.where, r);
    if (a === UNKNOWN) { unread++; continue; }
    if (a !== YES) continue;
    const keys = keysOf(slot.by, r);
    if (keys === null) unread++;
    else if (keys.length === 0) ungrouped++;
    else for (const k of keys) members.set(k, [...(members.get(k) ?? []), r]);
  }
  const { by, where, ...each } = slot;
  // An object's own enumeration puts integer-like keys first, whatever the
  // insertion order, so `groups` is a value, not an order: `slotsText` writes
  // the order.
  const groups = Object.create(null);
  for (const k of [...members.keys()].sort(utf16Order)) {
    const rs = members.get(k);
    groups[k] = plainSlot(each, spec, { taken: rs, current: rs, states: null });
  }
  return { groups, unread, ungrouped };
}

function plainSlot(slot, spec, { taken, current, states }) {
  const where = slot.where;
  const pool = slot.follow === "prev" ? current : taken;
  if ("count" in slot) return F.count(pool, { where });
  for (const type of ["sum", "min", "max"]) if (type in slot) return F[type](pool, { where, field: slot[type] });
  if ("latest-by" in slot) return latestBy(states, spec, slot);
  return collect(pool, slot);
}

// Every slot's value, by name. `current` is the whole-reader heads `from`
// takes: each thing's heads, each id once (a record heads one thing).
// Strings in UTF-16 code-unit order, the order the model writes keyed members in.
export const utf16Order = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

// Slot values as JSON text, members in the order the model writes them: the
// keys of `groups` and of `things` in UTF-16 code-unit order (an object's own
// enumeration would put integer-like keys first), every other object in the
// order it was built. Integers past the safe range are written exactly.
export function slotsText(out) {
  const write = (v, keyed) => {
    if (typeof v === "bigint") return v.toString();
    if (v === null || typeof v !== "object") return JSON.stringify(v);
    if (Array.isArray(v)) return `[${v.map((x) => write(x, false)).join(",")}]`;
    const keys = keyed ? Object.keys(v).sort(utf16Order) : Object.keys(v);
    return `{${keys.map((k) => `${JSON.stringify(k)}:${write(v[k], false)}`).join(",")}}`;
  };
  const slot = (v) => `{${Object.keys(v).map((k) =>
    `${JSON.stringify(k)}:${write(v[k], k === "groups" || k === "things")}`).join(",")}}`;
  return `{${Object.keys(out).map((n) => `${JSON.stringify(n)}:${slot(out[n])}`).join(",")}}`;
}

export function runSpec(spec, records) {
  records = admit(records).admitted;
  const ctx = {
    taken: records.filter((r) => takes(spec, r)),
    current: F.heads(records).filter((r) => takes(spec, r)),
    states: F.states(records),
  };
  const out = Object.create(null);
  for (const [name, slot] of Object.entries(spec.fold)) out[name] = runSlot(slot, spec, ctx);
  return out;
}

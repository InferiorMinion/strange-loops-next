// spec.slice (protocol/laws.md, model/Laws/SpecSlice.lean), asked of this
// reader: a part of what is held that is a slice for a spec gives the spec's
// whole answer. The slice is every form of the records the spec takes, the
// records that erase one of them, the records one of them names in `prev`,
// the records that name one of them in `prev` (successors), and what those
// successors name in `prev`. Everything but the taken records travels here as
// an envelope alone, which the law allows.
//
// keyed.spec-slice (model/Laws/KeyedSpecSlice.lean): a slot that follows `keys`
// reads live setters the spec does not take, so the slice above is not enough
// for it. A part answers it when it covers every thing the spec touches: it
// holds the thing's live records whole, the members that lie between two
// records it holds of the thing, and what erases a record it holds of it.

import test from "node:test";
import assert from "node:assert/strict";
import { isDeepStrictEqual } from "node:util";
import { base32 } from "./loops.mjs";
import { parseSpec, runSpec, takes } from "./spec.mjs";
import { admit, settle } from "./realm.mjs";
import { above, live, thingOf } from "./fold.mjs";
import { formOf } from "./scenario.mjs";
import { generate } from "../fuzz/generate.mjs";

const PAYLOAD = ["value", "n", "fields"];
const envelope = (f) =>
  ({ ...Object.fromEntries(Object.entries(f).filter(([k]) => !PAYLOAD.includes(k))), shown: f.shown === "erased" ? "erased" : "withheld" });

// The five kinds, as sets of ids. `drop` leaves one of the last four out.
function slice(spec, forms, drop) {
  const taken = new Set(forms.filter((f) => takes(spec, f)).map((f) => f.id));
  const names = (f, field) => (f[field] ?? []).some((p) => taken.has(p));
  const kinds = {
    erasers: forms.filter((f) => names(f, "erases")).map((f) => f.id),
    targets: forms.filter((f) => taken.has(f.id)).flatMap((f) => f.prev ?? []),
    successors: forms.filter((f) => names(f, "prev")).map((f) => f.id),
    theirs: forms.filter((f) => names(f, "prev")).flatMap((f) => f.prev ?? []),
  };
  const rest = new Set(Object.entries(kinds).flatMap(([k, ids]) => (k === drop ? [] : ids)));
  const part = forms.flatMap((f) => (taken.has(f.id) ? [f] : rest.has(f.id) ? [envelope(f)] : []));
  return { part, left: forms.length - part.length };
}

const answer = (spec, forms) => runSpec(spec, settle(forms));

// The spec slice, and for each thing the spec touches (it takes a record of
// it): the thing's live records whole; the envelopes of its admitted records
// lying between two records the part holds of it, so that what is held of the
// thing is convex over `prev` and so witnessed; and the envelope of whatever
// erases a record the part holds of it. `drop` leaves one of the three out.
function cover(spec, forms, drop) {
  const base = slice(spec, forms).part;
  const held = new Set(base.map((f) => f.id));
  const { admitted } = admit(settle(forms));
  const touched = new Set(forms.filter((f) => takes(spec, f)).map(thingOf));
  const whole = new Set(), envelopes = new Set();
  for (const t of touched) {
    const rs = admitted.filter((r) => thingOf(r) === t);
    const up = new Map(rs.map((r) => [r.id, new Set(above(rs, r).map((q) => q.id))]));
    const lives = drop === "live" ? [] : rs.filter((r) => live(rs, r)).map((r) => r.id);
    const sent = new Set([...rs.filter((r) => held.has(r.id)).map((r) => r.id), ...lives]);
    const between = drop === "between" ? [] : rs.filter((m) =>
      [...up.get(m.id)].some((b) => sent.has(b)) && [...sent].some((a) => up.get(a).has(m.id))).map((m) => m.id);
    for (const i of lives) whole.add(i);
    for (const i of between) envelopes.add(i);
    const mine = new Set([...sent, ...between]);
    if (drop !== "erasers") for (const f of forms) if ((f.erases ?? []).some((e) => mine.has(e))) envelopes.add(f.id);
    // A record of the thing the part holds and the whole refuses: the part
    // holds what it names in `prev`, so that it refuses it too.
    const ok = new Set(rs.map((r) => r.id));
    for (const f of forms) if (thingOf(f) === t && !ok.has(f.id) && (held.has(f.id) || whole.has(f.id) || envelopes.has(f.id)))
      for (const p of f.prev ?? []) envelopes.add(p);
  }
  const taken = new Set(forms.filter((f) => takes(spec, f)).map((f) => f.id));
  return forms.flatMap((f) => (taken.has(f.id) || whole.has(f.id) ? [f] : held.has(f.id) || envelopes.has(f.id) ? [envelope(f)] : []));
}

// A task `a`. A note `d` of `a`'s thing replaces it. A note `c` of `a`'s thing
// names `a` and also `b`, a note of another thing, so the whole refuses `c`.
const id = (n) => `r:${base32(Uint8Array.from({ length: 32 }, () => n))}`;
const rec = (n, kind, prev = []) =>
  ({ id: id(n), kind, of: n === 2 ? "task:k2" : "task:k1", prev: prev.map(id), emitted: `2020-01-01T00:00:0${n}Z`, n });
const a = rec(1, "task"), b = rec(2, "note"), c = rec(3, "note", [1, 2]), d = rec(4, "note", [1]);
const tasks = parseSpec({ vertex: "v", from: [{ store: ".", kind: ["task"] }], fold: { current: { "latest-by": "of", follow: "prev" } } }).parsed;

// `m`, a note, sets `title`; `n`, a note, sets `status` from it; `h`, a task,
// sets `est` from `n`. The spec takes tasks, and its slice holds `h` and `n`.
test("a spec slice is not enough for a slot that follows keys: it reads a live setter the spec does not take", () => {
  const r = (n, kind, sets, prev = []) =>
    ({ id: id(n), kind, observer: "sl:x", about: [], of: "task:k1", prev: prev.map(id), emitted: `2020-01-01T00:00:0${n}Z`, sets, value: "", n, fields: {} });
  const m = r(1, "note", ["title"]), n = r(2, "note", ["status"], [1]), h = r(3, "task", ["est"], [2]);
  const spec = parseSpec({ vertex: "v", from: [{ store: ".", kind: ["task"] }], fold: { state: { "latest-by": "of", follow: "keys" } } }).parsed;
  const whole = [m, n, h], want = answer(spec, whole);
  assert.deepEqual(Object.keys(want.state.things["task:k1"].keys), ["est", "status", "title"]);
  assert.deepEqual(slice(spec, whole).part.map((f) => f.id), [n.id, h.id]);
  assert.deepEqual(Object.keys(answer(spec, slice(spec, whole).part).state.things["task:k1"].keys), ["est", "status"]);
  assert.deepEqual(answer(spec, cover(spec, whole)), want);
  assert.notDeepEqual(answer(spec, cover(spec, whole, "live")), want);
});

test("the records a spec takes are not enough: a successor it does not take replaces one", () => {
  const whole = [a, d];
  assert.deepEqual(Object.keys(answer(tasks, whole).current.things), []);
  assert.deepEqual(Object.keys(answer(tasks, [a]).current.things), ["task:k1"]);
  assert.deepEqual(answer(tasks, slice(tasks, whole).part), answer(tasks, whole));
  assert.notDeepEqual(answer(tasks, slice(tasks, whole, "successors").part), answer(tasks, whole));
});

test("a successor's other prev target decides whether it replaces", () => {
  const whole = [a, b, c];
  assert.deepEqual(Object.keys(answer(tasks, whole).current.things), ["task:k1"]);
  assert.deepEqual(answer(tasks, slice(tasks, whole).part), answer(tasks, whole));
  assert.deepEqual(Object.keys(answer(tasks, slice(tasks, whole, "theirs").part).current.things), []);
});

// The case the sweep met: A, a task of t:x whose `occurred` is no span, is
// superseded by C, a note of t:x; B is a task of its own. The whole never
// orders A, and this reader once refused A's time only when a slice without
// the successor C made A current and ordered it. It now refuses the time on
// the way in, as Lean and Rust do, so the whole is refused as every slice is.
test("a time that is no span refuses the whole, not only a slice that orders it", () => {
  const r = (n, kind, of, prev = [], occurred) =>
    ({ id: id(n), kind, observer: "sl:x", about: [], ...(of && { of }), prev: prev.map(id), emitted: `2020-01-01T00:00:0${n}Z`, ...(occurred && { occurred }) });
  const A = r(1, "task", "t:x", [], "2024-99"), B = r(2, "task"), C = r(3, "note", "t:x", [1]);
  const spec = parseSpec({ vertex: "v", from: [{ store: ".", kind: ["task"] }],
    fold: { current: { count: true, follow: "prev", by: "env.kind" } } }).parsed;
  const whole = [A, B, C], refused = { message: "occurred-not-span", path: "[0].occurred" };
  assert.throws(() => answer(spec, whole), refused);
  assert.throws(() => answer(spec, slice(spec, whole).part), refused);
  assert.throws(() => answer(spec, slice(spec, whole, "successors").part), refused);
});

// The fuzz generator's spec runs and scenario steps. The counts at the end
// keep the sweep honest: it must meet specs that leave records out, and each
// kind that the generator can exercise must matter at least once.
test("a spec slice answers as the whole, over generated specs and stores", () => {
  const seen = { pairs: 0, partial: 0, erasers: 0, targets: 0, successors: 0, followed: 0, followedMatters: 0, grouped: 0, groupedMatters: 0, groupedHeads: 0, threw: 0, keyed: 0, keyedDiffers: 0,
    covered: 0, coverPartial: 0, coverLive: 0, coverErasers: 0 };
  // A slot that follows keys, of any fold type, is outside spec.slice: it reads
  // live setters the spec does not take, so this slice need not hold them
  // (Slot.run_slice takes no such slot). The sweep checks the other slots over
  // it and counts how often the keyed ones differ. Over the cover every slot is
  // checked, the keyed ones among them (Spec.run_slice_keyed).
  const keyedOf = (spec) => Object.entries(spec.fold).filter(([, s]) => s.follow === "keys").map(([n]) => n);
  const unkeyed = (spec) => ({ ...spec, fold: Object.fromEntries(Object.entries(spec.fold).filter(([, s]) => s.follow !== "keys")) });
  // The slots that follow prev and accumulate: count, sum, min and max.
  const followedOf = (spec) => Object.entries(spec.fold)
    .filter(([, s]) => s.follow === "prev" && ["count", "sum", "min", "max"].some((t) => t in s)).map(([n]) => n);
  // The slots with `by`, followed or not.
  const groupedOf = (spec) => Object.entries(spec.fold).filter(([, s]) => s.by !== undefined).map(([n]) => n);
  const check = (full, forms) => {
    const keyed = keyedOf(full);
    if (keyed.length) {
      seen.keyed += 1;
      const whole = answer(full, forms), part = answer(full, slice(full, forms).part);
      seen.keyedDiffers += keyed.some((n) => !isDeepStrictEqual(part[n], whole[n]));
      const covering = cover(full, forms);
      assert.deepEqual(answer(full, covering), whole);
      seen.covered += 1;
      seen.coverPartial += covering.length < forms.length;
      for (const [drop, key] of [["live", "coverLive"], ["erasers", "coverErasers"]]) {
        let got;
        try { got = answer(full, cover(full, forms, drop)); } catch { seen[key] += 1; continue; }
        seen[key] += !isDeepStrictEqual(got, whole);
      }
    }
    const spec = unkeyed(full);
    const want = answer(spec, forms), { part, left } = slice(spec, forms);
    assert.deepEqual(answer(spec, part), want);
    seen.pairs += 1;
    seen.partial += left > 0;
    const followed = followedOf(spec), grouped = groupedOf(spec);
    seen.followed += followed.length > 0;
    seen.grouped += grouped.length > 0;
    seen.groupedHeads += grouped.some((n) => "collect" in spec.fold[n] && spec.fold[n].follow === "prev");
    let matters = false, groupMatters = false;
    for (const drop of ["erasers", "targets", "successors"]) {
      // A part holds only the whole's records, so its times read as the
      // whole's do; a part that throws all the same is an answer that changed.
      let got;
      try { got = answer(spec, slice(spec, forms, drop).part); } catch { seen[drop] += 1; seen.threw += 1; continue; }
      try { assert.deepEqual(got, want); } catch { seen[drop] += 1; }
      matters ||= followed.some((n) => !isDeepStrictEqual(got[n], want[n]));
      groupMatters ||= grouped.some((n) => !isDeepStrictEqual(got[n], want[n]));
    }
    seen.followedMatters += matters;
    seen.groupedMatters += groupMatters;
  };
  for (let i = 0; i < 1500; i++) {
    for (const family of ["spec", "scenario"]) {
      const { layer, input } = generate("spec-slice-1", i, family);
      let spec;
      try { spec = parseSpec(input.spec).parsed; } catch { continue; }
      if (!spec) continue;
      const stores = layer === "spec/run" ? [input.orders[0]]
        : layer === "scenario" ? (() => {
          const pool = new Map(input.records.map((r) => [r.id, r]));
          return input.steps.map((s) => s.forms.map((f) => formOf(pool.get(f.id), f.shown)));
        })() : [];
      for (const forms of stores) {
        try { answer(spec, forms); } catch { continue; } // an input this reader refuses is no store
        check(spec, forms);
      }
    }
  }
  assert.ok(seen.pairs > 500, JSON.stringify(seen));
  assert.ok(seen.partial > 50, JSON.stringify(seen));
  for (const kind of ["erasers", "targets", "successors"]) assert.ok(seen[kind] > 0, JSON.stringify(seen));
  // Followed count, sum, min and max are among the slots the slice answers for,
  // and dropping a kind changes one of them at least once.
  assert.ok(seen.followed > 500 && seen.followedMatters > 0, JSON.stringify(seen));
  // So are slots with `by`, plain and followed, and dropping a kind changes one.
  assert.ok(seen.grouped > 500 && seen.groupedMatters > 0, JSON.stringify(seen));
  // Among them a collect that follows prev, grouped over the whole view's heads.
  assert.ok(seen.groupedHeads > 500, JSON.stringify(seen));
  // Keyed slots met the sweep, and this slice does not answer for them.
  assert.ok(seen.keyed > 500 && seen.keyedDiffers > 0, JSON.stringify(seen));
  // The cover answers every slot, keyed ones among them, it is sometimes a
  // proper part, and its live records and its erasers each matter at least once.
  assert.ok(seen.covered > 500 && seen.coverPartial > 50 && seen.coverLive > 0 && seen.coverErasers > 0, JSON.stringify(seen));
  console.log(`spec-slice sweep: ${JSON.stringify(seen)}`);
});

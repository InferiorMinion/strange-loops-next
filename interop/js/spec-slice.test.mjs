// spec.slice (protocol/laws.md, model/Laws/SpecSlice.lean), asked of this
// reader: a part of what is held that is a slice for a spec gives the spec's
// whole answer. The slice is every form of the records the spec takes, the
// records that erase one of them, the records one of them names in `prev`,
// the records that name one of them in `prev` (successors), and what those
// successors name in `prev`. Everything but the taken records travels here as
// an envelope alone, which the law allows.

import test from "node:test";
import assert from "node:assert/strict";
import { isDeepStrictEqual } from "node:util";
import { base32 } from "./loops.mjs";
import { parseSpec, runSpec, takes } from "./spec.mjs";
import { settle } from "./realm.mjs";
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

// A task `a`. A note `d` of `a`'s thing replaces it. A note `c` of `a`'s thing
// names `a` and also `b`, a note of another thing, so the whole refuses `c`.
const id = (n) => `r:${base32(Uint8Array.from({ length: 32 }, () => n))}`;
const rec = (n, kind, prev = []) =>
  ({ id: id(n), kind, of: n === 2 ? "task:k2" : "task:k1", prev: prev.map(id), emitted: `2020-01-01T00:00:0${n}Z`, n });
const a = rec(1, "task"), b = rec(2, "note"), c = rec(3, "note", [1, 2]), d = rec(4, "note", [1]);
const tasks = parseSpec({ vertex: "v", from: [{ store: ".", kind: ["task"] }], fold: { current: { "latest-by": "of", follow: "prev" } } }).parsed;

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

// The fuzz generator's spec runs and scenario steps. The counts at the end
// keep the sweep honest: it must meet specs that leave records out, and each
// kind that the generator can exercise must matter at least once.
test("a spec slice answers as the whole, over generated specs and stores", () => {
  const seen = { pairs: 0, partial: 0, erasers: 0, targets: 0, successors: 0, followed: 0, followedMatters: 0 };
  // The slots that follow prev and accumulate: count, sum, min and max.
  const followedOf = (spec) => Object.entries(spec.fold)
    .filter(([, s]) => s.follow === "prev" && ["count", "sum", "min", "max"].some((t) => t in s)).map(([n]) => n);
  const check = (spec, forms) => {
    const want = answer(spec, forms), { part, left } = slice(spec, forms);
    assert.deepEqual(answer(spec, part), want);
    seen.pairs += 1;
    seen.partial += left > 0;
    const followed = followedOf(spec);
    seen.followed += followed.length > 0;
    let matters = false;
    for (const drop of ["erasers", "targets", "successors"]) {
      const got = answer(spec, slice(spec, forms, drop).part);
      try { assert.deepEqual(got, want); } catch { seen[drop] += 1; }
      matters ||= followed.some((n) => !isDeepStrictEqual(got[n], want[n]));
    }
    seen.followedMatters += matters;
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
  console.log(`spec-slice sweep: ${JSON.stringify(seen)}`);
});

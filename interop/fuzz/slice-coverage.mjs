// Count a class only when its omission loses an actual Rust-selected ID.
// IDs supplied by another class (or reached by another path) remain selected.
import assert from 'node:assert/strict';
import { takes } from '../js/spec.mjs';
import { thingOf } from '../js/fold.mjs';

export const classes = ['taken', 'eraser', 'target', 'successor', 'successorTarget',
  'keyedMember', 'keyedEraser', 'keyedTarget', 'downPrev', 'downUnder'];

function reasons(spec, records) {
  const taken = new Set(records.filter(r => takes(spec, r)).map(r => r.id));
  const keyed = Object.values(spec.fold).some(slot => slot.follow === 'keys');
  const touched = new Set(records.filter(r => keyed && taken.has(r.id)).map(thingOf));
  const members = new Set(records.filter(r => touched.has(thingOf(r))).map(r => r.id));
  const sets = Object.fromEntries(classes.slice(0, 8).map(k => [k, new Set()]));
  sets.taken = taken;
  sets.keyedMember = members;
  for (const r of records) {
    const prev = r.prev ?? [];
    const successor = prev.some(id => taken.has(id));
    if (successor) sets.successor.add(r.id);
    for (const id of prev) {
      if (taken.has(r.id)) sets.target.add(id);
      if (successor) sets.successorTarget.add(id);
      if (members.has(r.id)) sets.keyedTarget.add(id);
    }
    if (r.erases?.some(id => taken.has(id))) sets.eraser.add(r.id);
    if (r.erases?.some(id => members.has(id))) sets.keyedEraser.add(r.id);
  }
  return sets;
}

function reachable(records, roots, omitted) {
  const byId = new Map(records.map(r => [r.id, r]));
  const seen = new Set(roots);
  for (const id of seen) {
    const r = byId.get(id);
    if (omitted !== 'downPrev') for (const p of r.prev ?? []) if (byId.has(p)) seen.add(p);
    if (omitted !== 'downUnder' && byId.has(r.under)) seen.add(r.under);
  }
  return seen;
}

export function coverage(spec, forms, { exact, downward }) {
  const records = [...new Map(forms.map(r => [r.id, r])).values()];
  const sets = Object.entries(reasons(spec, records));
  const held = new Set(records.map(r => r.id));
  const union = omitted => new Set(sets.filter(([name]) => name !== omitted)
    .flatMap(([, ids]) => [...ids].filter(id => held.has(id))));
  assert.deepEqual(exact, [...union()].sort(), 'exact IDs from the law');
  assert.deepEqual(downward, [...reachable(records, exact)].sort(), 'downward IDs from the law');
  return classes.filter(name => {
    const down = name === 'downPrev' || name === 'downUnder';
    const retained = down ? reachable(records, exact, name) : union(name);
    const selected = down ? downward : exact;
    const mutant = selected.filter(id => retained.has(id));
    return mutant.length < selected.length;
  });
}

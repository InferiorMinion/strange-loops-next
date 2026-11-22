// Input construction only. Both executable readers see the same file; neither answer is synthesized here.
import { digest, render, specHash, resultHash } from './tick-wire.mjs';
import { base32, random, records, shuffled, spec } from './generate.mjs';

import { orderPairs, orderVector, thirdPoint } from './tick-order.mjs';

const operations = ['payload', 'members', 'payload-text', 'write-payload', 'make-and-check', 'check-tick'];
const versions = ['1', '1', '2', '10', '123456789012345678901'];
const id = rng => 'r:' + base32(rng.bytes(32));
const hide = (r, shown) => {
  const { fields, ...envelope } = r;
  return { ...envelope, shown };
};

// A constructive writer for count and sum: every revision names the root's thing,
// every admitted note has a small integer n, and the eraser and marker are outside the source.
export function countedHolding(rng) {
  const sample = () => ({ id: id(rng), kind: 'note', observer: 'o:' + rng.int(4), about: [], prev: [],
    emitted: '2026-11-21T20:00:00Z', fields: { n: rng.int(17) - 8 } });
  const root = sample(), notes = [root];
  for (let i = 0, n = 1 + rng.int(4); i < n; i++) {
    const r = sample();
    if (rng.int(3)) { r.of = root.id; r.prev = [root.id]; }
    notes.push(r);
  }
  const writer = notes.map(r => rng.int(4) ? r : hide(r, rng.pick(['withheld', 'erased'])));
  writer.push({ ...sample(), kind: 'marker' });
  const erased = rng.int(3) === 0 ? rng.pick(notes).id : null;
  if (erased) writer.push({ ...sample(), kind: 'erase', erases: [erased] });
  const withheld = writer.filter(r => r.shown || r.id === erased).map(r => r.id).sort();
  const name = 'runs-' + rng.int(50);
  const sp = { vertex: 'v-' + rng.int(1000), from: [{ store: 'lab', kind: ['note'] }], fold: { [name]: { count: true } } };
  const result = { [name]: { value: String(notes.length), unread: '0' } };
  if (rng.int(2)) {
    sp.fold.total = { sum: 'n' };
    const open = notes.filter(r => !withheld.includes(r.id));
    result.total = { value: String(open.reduce((n, r) => n + r.fields.n, 0)), unread: String(notes.length - open.length) };
  }
  const payload = { rules: '1', spec: sp, covers: writer.map(r => r.id).sort(), result };
  if (withheld.length) payload.withheld = withheld;
  payload['spec-hash'] = specHash(payload);
  payload['result-hash'] = resultHash(result);
  return { writer, payload };
}

function checker(rng, writer) {
  let forms = structuredClone(writer);
  const envelope = f => f.record ?? f;
  for (let i = 0, n = rng.int(5); i < n && forms.length; i++) {
    const j = rng.int(forms.length), r = envelope(forms[j]);
    switch (rng.int(7)) {
      case 0: forms = forms.filter(f => envelope(f).id !== r.id); break;
      case 1: forms[j] = hide(r, rng.pick(['withheld', 'erased'])); break;
      case 2: forms.push(structuredClone(forms[j])); break;
      case 3: forms.push({ ...r, id: id(rng), kind: 'erase', prev: [], erases: [r.id] }); break;
      case 4: forms.push({ ...r, id: id(rng), kind: 'note', prev: [] }); break;
      case 5: forms = shuffled(forms, rng); break;
      case 6: {
        const coveredErasers = writer.filter(f => envelope(f).erases?.length);
        if (coveredErasers.length) forms.push(structuredClone(rng.pick(coveredErasers)));
        break;
      }
    }
  }
  return forms;
}

// Pick from actual member paths, including objects reached through arrays.
function memberPaths(value, path = []) {
  if (!value || typeof value !== 'object') return [];
  if (Array.isArray(value)) return value.flatMap((v, i) => memberPaths(v, [...path, i]));
  return Object.entries(value).flatMap(([k, v]) => {
    const child = [...path, k];
    return [child, ...memberPaths(v, child)];
  });
}

const mutations = ['not-object', 'duplicate', 'unknown', 'missing', 'rules', 'spec-object',
  'ids-array', 'id', 'repeat-id', 'unsorted', 'empty', 'outside', 'spelling', 'spec-hash',
  'result-hash', 'noncanonical', 'result', 'unreadable', 'wrong-claim', 'omit-withheld'];

// Mutate in draw order. Later mutations can mask earlier ones. Spec/result edits
// independently choose whether to repair their hash, leaving deeper checks reachable.
function mutate(rng, payload, faults) {
  let p = structuredClone(payload), duplicate = false;
  for (const fault of faults) {
    if (!p || typeof p !== 'object') break;
    const list = rng.pick(['covers', 'withheld']);
    switch (fault) {
      case 'not-object': p = rng.pick([null, 1, 'tick']); break;
      case 'duplicate': duplicate = true; break;
      case 'unknown': p['extra.' + rng.int(3)] = []; break;
      case 'missing': delete p[rng.pick(['rules','spec','covers','result','spec-hash','result-hash'])]; break;
      case 'rules': p.rules = rng.pick(['0','01',2,'1 ']); break;
      case 'spec-object': p.spec = []; break;
      case 'ids-array': p[list] = {}; break;
      case 'id': p[list] = ['r:invalid']; break;
      case 'repeat-id': { const k = id(rng); p[list] = [k,k]; break; }
      case 'unsorted': p[list] = [id(rng),id(rng)].sort().reverse(); break;
      case 'empty': p.withheld = []; break;
      case 'outside': p.withheld = [id(rng)]; break;
      case 'spelling': p[rng.pick(['spec-hash','result-hash'])] = 'res:A'; break;
      case 'spec-hash': p['spec-hash'] = digest('spec:', 'wrong:', 0); break;
      case 'result-hash': p['result-hash'] = digest('res:', 'loops/result-hash/v2:', p.result ?? null); break;
      case 'omit-withheld': delete p.withheld; break;
      case 'noncanonical':
      case 'unreadable':
        if (p.spec && !Array.isArray(p.spec)) {
          if (fault === 'noncanonical') p.spec.note = 'carried';
          else p.spec.fold = { unknown: { future: true } };
          if (rng.int(4)) p['spec-hash'] = specHash(p);
        }
        break;
      case 'result':
      case 'wrong-claim':
        if (fault === 'result') p.result = rng.pick([[], null, { extra: true }]);
        else if (p.result) {
          const slot = Object.values(p.result)[0];
          if (slot && typeof slot === 'object') slot.value = '900';
        }
        if (p.result !== undefined && rng.int(4)) p['result-hash'] = resultHash(p.result);
        break;
    }
  }
  const paths = duplicate ? memberPaths(p) : [];
  return { payload: p, repeat: paths.length ? rng.pick(paths) : undefined };
}

export function generateTick(seed, index) {
  const rng = random(seed, index, 'tick');
  // Three dedicated pairs, one triple and four ordinary cases per eight.
  const block = Math.floor(index / 8), phase = index % 8;
  if (phase < 3) return orderVector(seed,index,orderPairs[(3 * block + phase) % orderPairs.length]);
  if (phase === 3) {
    const pair = rng.pick(orderPairs);
    return orderVector(seed,index,[...pair,thirdPoint(pair,rng)]);
  }
  const op = operations[(4 * block + phase - 4) % operations.length];
  const fuzz = { seed: String(seed), family: 'tick', index };
  const vector = input => ({ layer: 'tick', input, fuzz });
  const { writer, payload } = countedHolding(rng);
  if (op === 'write-payload' || op === 'make-and-check') {
    const rs = records(rng), sp = spec(rng, rs);
    sp.fold = Object.fromEntries(shuffled(Object.entries(sp.fold), rng).slice(0, 1 + rng.int(4)));
    if (rng.int(2)) sp.from = [{store: 'lab', kind: ['note']}];
    if (rng.int(2)) rs.push(structuredClone(rng.pick(rs)));
    if (rng.int(3) === 0) {
      const nested = { ...rs[0], kind: 'tick', id: id(rng), prev: [], shown: 'here' };
      delete nested.fields;
      rs.push(op === 'write-payload' ? { ...nested, fields: payload } : { record: nested, opening: payload });
    }
    if (op === 'write-payload') return vector({ op, spec: sp, records: rs });
    return vector({ op, spec: sp, writer: rs, checker: checker(rng, rs), fresh: !!rng.int(2) });
  }
  if (op === 'check-tick') {
    const rs = records(rng), sp = spec(rng,rs);
    sp.fold = Object.fromEntries(shuffled(Object.entries(sp.fold),rng).slice(0,1 + rng.int(5)));
    // Lean's writer materializes the full claim in one batch before the
    // checker sees it. Replays save that claim, independent of this recipe.
    fuzz.prepare = 'check-tick';
    return vector({op:'write-payload', spec:sp, records:rs});
  }
  payload.rules = rng.pick(versions);
  payload['spec-hash'] = specHash(payload);
  const n = rng.pick([0,0,1,1,1,2,3,4]);
  const faults = shuffled(mutations, rng).slice(0, n);
  fuzz.faults = faults;
  const changed = mutate(rng, payload, faults);
  if (op === 'payload-text') return vector({ op, text: render(changed.payload, changed.repeat) });
  if (changed.repeat) fuzz.repeat = ['input','payload',...changed.repeat];
  return vector({ op, payload:changed.payload });
}

// The model supplies only the writer's payload; both checkers independently
// replay it. Changing unread preserves the result shape for every fold.
export function finishCheck(vector, payload) {
  const {seed,index} = vector.fuzz, rng = random(seed,index,'checker');
  delete vector.fuzz.prepare;
  const writer = vector.input.records, mode = rng.int(5);
  const subject = {kind:'tick',payload};
  let current = '1', forms = writer;
  if (mode === 1) {
    Object.values(payload.result)[0].unread = '9999';
    payload['result-hash'] = resultHash(payload.result);
  } else if (mode >= 2) {
    forms = checker(rng,writer);
    if (mode === 3) current = rng.pick(versions);
    if (mode === 4) {
      payload.rules = rng.pick(versions);
      payload['spec-hash'] = specHash(payload);
      const faults = shuffled(mutations,rng).slice(0,rng.int(5));
      vector.fuzz.faults = faults;
      const changed = mutate(rng,payload,faults);
      subject.payload = changed.payload;
      if (changed.repeat) vector.fuzz.repeat = ['input','subject','payload',...changed.repeat];
      if (!rng.int(7)) subject.kind = 'note';
      if (!rng.int(7)) delete subject.payload;
    }
  }
  vector.input = {op:'check-tick',current,subject,checker:forms};
}

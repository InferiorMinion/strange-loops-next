// Each case has its own PRNG stream. Family scheduling does not change a case.
import { commitOf, idOf, readEnv, readSalt } from '../js/loops.mjs';
export const FAMILIES = Object.freeze(['record', 'fold', 'spec', 'realm', 'scenario', 'batch', 'lens']);
export const FOLD_LAYERS = Object.freeze(['count', 'sum', 'extrema', 'latest', 'latest-by', 'heads', 'view', 'select']);
const B32 = '0123456789abcdefghijklmnopqrstuv';
export function base32(bytes) {
  let value = 0, bits = 0, out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte; bits += 8;
    while (bits >= 5) { bits -= 5; out += B32[(value >>> bits) & 31]; }
  }
  return out + (bits ? B32[(value << (5 - bits)) & 31] : '');
}
function random(seed, index, family) {
  let state = 2166136261;
  for (const byte of new TextEncoder().encode(JSON.stringify([String(seed), index, family]))) {
    state = Math.imul(state ^ byte, 16777619) >>> 0;
  }
  state ||= 0x9e3779b9;
  const next = () => { state ^= state << 13; state ^= state >>> 17; state ^= state << 5; return state >>> 0; };
  const int = n => next() % n;
  return { int, pick: xs => xs[int(xs.length)], bytes: n => Array.from({ length: n }, () => int(256)) };
}
const clone = value => structuredClone(value);
function shuffled(xs, rng) {
  const out = [...xs];
  for (let i = out.length - 1; i > 0; i--) { const j = rng.int(i + 1); [out[i], out[j]] = [out[j], out[i]]; }
  return out;
}
// `from` sometimes takes one thing's records, or those mentioning a thing, or both.
function source(rng, rs) {
  const from = { store: 's:local' }, r = rng.pick(rs), choice = rng.int(6);
  if ((choice === 0 || choice === 2) && r.about.length) from.about = r.about[0];
  if (choice === 1 || choice === 2) from.of = r.of ?? r.id;
  return from;
}
function spec(rng, rs) {
  return {
    vertex: `vertex-${rng.int(1000)}`, from: [source(rng, rs)],
    fold: {
      count: { count: true }, total: { sum: 'n' }, least: { min: 'n' }, greatest: { max: 'n' },
      heads: { 'latest-by': 'of', follow: 'prev' },
      items: { collect: true, follow: 'prev', limit: 1 + rng.int(4) },
      chosen: { count: true, where: { n: { 'at-least': rng.int(5) - 2 } } },
      // Following prev, count, sum, min and max fold the whole view's heads.
      now: { count: true, follow: 'prev' },
      'now-flagged': { count: true, follow: 'prev', where: { flag: { is: true } } },
      'now-total': { sum: 'n', follow: 'prev' },
      'now-least': { min: 'n', follow: 'prev' },
      'now-greatest': { max: 'n', follow: 'prev' },
      'now-score': { sum: 'score', follow: 'prev', where: { 'env.kind': { not: 'erase' } } },
      'now-flagged-total': { sum: 'n', follow: 'prev', where: { flag: { is: true } } },
      'now-flagged-greatest': { max: 'n', follow: 'prev', where: { flag: { is: true }, 'env.kind': { is: 'note' } } },
      // `by` in every form: the envelope's kind and observer, the things a
      // record mentions (one or two), and payload fields of text, whole
      // numbers and booleans, and one no record has; plain and followed.
      'by-kind': { count: true, by: 'env.kind' },
      'by-observer-total': { sum: 'n', by: 'env.observer' },
      'by-mention': { count: true, by: 'env.about:t' },
      'by-mention-now': { count: true, follow: 'prev', by: 'env.about:t', where: { flag: { is: true } } },
      'by-mention-now-total': { sum: 'n', follow: 'prev', by: 'env.about:t' },
      'by-tag-least': { min: 'n', by: 'tag', where: { 'env.kind': { not: 'erase' } } },
      'by-flag-now-greatest': { max: 'n', follow: 'prev', by: 'flag' },
      'by-score-now': { count: true, follow: 'prev', by: 'score' },
      'by-missing': { count: true, by: 'missing' },
      'by-tag-items': { collect: true, limit: 1 + rng.int(3), by: 'tag', where: { flag: { is: true } } },
      // A collect that follows prev groups its current records.
      'by-mention-now-items': { collect: true, follow: 'prev', by: 'env.about:t' },
      'by-flag-now-items': { collect: true, follow: 'prev', limit: 1 + rng.int(3), by: 'flag', where: { 'env.kind': { not: 'erase' } } },
      // `rank` is a whole number as text, or as number text spelled several
      // ways (`1.0`, `10e-1`), which must share the text's group.
      'by-rank': { count: true, by: 'rank' },
      'by-rank-items': { collect: true, by: 'rank' },
    },
  };
}
const INSTANTS = ['2024-02-29T12:00:00Z', '2024-02-29T12:00:00.5Z',
  '2024-02-29T12:00:00.123456789Z'];
const OCCURRED = ['2024', '2024-02-29', '202X', '2024/..', ...INSTANTS,
  `${INSTANTS[2]}/${INSTANTS[1]}`, `${INSTANTS[1]}/..`];
// A record's thing: its `of` as written, or its own id.
const thingOf = r => r.of ?? r.id;
// A record names its thing in `of`, and only mentions things in `about`.
// Revisions usually state their target's thing; a few name another thing, or
// leave `of` out and so are of their own id, which admission refuses.
function ofFor(rng, prev, earlier, things) {
  if (prev.length) {
    const target = earlier.find(r => r.id === prev[0]), roll = rng.int(12);
    return roll < 9 ? thingOf(target) : roll === 9 ? things[1] : roll === 10 ? earlier[0].id : undefined;
  }
  // A root, a record of a natural key, or one of an earlier record's thing.
  const roll = rng.int(8);
  return roll < 3 ? undefined : roll < 6 ? things[0] : roll === 6 ? things[1] : earlier.length ? thingOf(rng.pick(earlier)) : undefined;
}
function records(rng) {
  const ids = Array.from({ length: 1 + rng.int(7) }, () => `r:${base32(rng.bytes(32))}`);
  const things = [`t:${rng.int(30)}`, `t:${rng.int(30) + 30}`];
  const out = [];
  for (const [i, id] of ids.entries()) {
    let about = [];
    if (rng.int(5) !== 0) {
      if (rng.int(3)) about = [things[0]];
      else about = [...things];
    }
    const prev = i && rng.int(3) ? [ids[rng.int(i)]] : [];
    const of = ofFor(rng, prev, out, things);
    const record = {
      id, kind: rng.pick(['note', 'vote', 'erase']), observer: `o:${rng.int(3)}`,
      about, ...(of !== undefined && { of }), prev, emitted: rng.pick(INSTANTS),
      value: rng.pick(['yes', 'no', '', 'α\nline', `value-${rng.int(10000)}`]), n: rng.int(31) - 15,
      fields: { flag: !!rng.int(2), score: rng.int(9), tag: `tag-${rng.int(4)}` },
    };
    if (rng.int(2)) record.occurred = rng.pick([...OCCURRED, '2024-02', '2024?']);
    if (i && rng.int(4) === 0) record.under = ids[rng.int(i)];
    if (i && rng.int(4) === 0) record.erases = [ids[rng.int(i)]];
    if (rng.int(5) === 0) {
      record.shown = rng.pick(['withheld', 'erased']);
      delete record.value; delete record.n; delete record.fields;
    }
    out.push(record);
  }
  return out;
}
function envelope(rng) {
  const env = {
    kind: rng.pick(['note', 'vote', `kind-${rng.int(100)}`]), observer: `o:${rng.int(6)}`,
    emitted: rng.pick(INSTANTS), occurred: rng.pick(OCCURRED),
    about: [`t:${rng.int(5)}`, 't:shared'], prev: [],
  };
  // Some records are of a natural key; the rest are roots.
  if (rng.int(3) === 0) env.of = 't:item';
  return env;
}
function payload(rng, depth = 0) {
  const scalar = () => rng.pick([null, !!rng.int(2), rng.int(1000) - 500, `text-${rng.int(10000)}`, 'α\n"\\']);
  if (depth > 1 || rng.int(3) === 0) return scalar();
  if (rng.int(2)) return Array.from({ length: rng.int(4) }, () => payload(rng, depth + 1));
  return Object.fromEntries(shuffled(['z', 'a', 'nested', '😀'], rng).slice(0, 1 + rng.int(4))
    .map(k => [k, payload(rng, depth + 1)]));
}
function reorder(value, rng) {
  if (Array.isArray(value)) return value.map(x => reorder(x, rng));
  if (value !== null && typeof value === 'object') return Object.fromEntries(shuffled(Object.entries(value), rng)
    .map(([k, v]) => [k, reorder(v, rng)]));
  return value;
}
function lens(rng) {
  const base = Array.from({ length: 1 + rng.int(4) }, (_, i) => ({ id: `t:${i}`, body: [`body-${rng.int(100)}`] }));
  const doc = base.filter(() => rng.int(4)).map(s => ({ ...s, body: rng.int(2) ? s.body : [`edit-${rng.int(100)}`, ''] }));
  if (rng.int(2)) doc.push({ id: 't:new', body: ['new text'] });
  return { base, text: doc.flatMap(s => [`::: ${s.id}`, ...s.body, ':::']) };
}

// Render body lines without using either implementation of the lens.
function renderSections(sections) {
  return sections.flatMap(({ id, body }) => [
    `::: ${id}`,
    ...body.map(line => line.startsWith(':::') || line.startsWith('\\') ? `\\${line}` : line),
    ':::',
  ]);
}

function numericRecords() {
  return [Number.MAX_SAFE_INTEGER, 2, -Number.MAX_SAFE_INTEGER].map((n, i) => ({
    id: `r:${base32(Array(32).fill(i + 1))}`, kind: 'note', observer: 'o:0',
    about: ['t:numeric'], of: 't:numeric', prev: [], emitted: '2024-02-29T00:00:00Z',
    shown: 'here', value: 'numeric', n,
  }));
}

export function generate(seed, index, family) {
  if (!FAMILIES.includes(family)) throw new Error(`unknown family: ${family}`);
  if (!Number.isSafeInteger(index) || index < 0) throw new Error('index must be a nonnegative safe integer');
  const rng = random(seed, index, family);
  let layer = family, input, nearMiss = null;
  let rs = records(rng);
  const numeric = index % 64 === 9 && ['fold', 'spec', 'realm', 'scenario'].includes(family);
  if (numeric) rs = numericRecords();
  const orders = () => [clone(rs), shuffled(clone(rs), rng), clone(rs).toReversed()];
  switch (family) {
    case 'fold':
      layer = `fold/${FOLD_LAYERS[index % FOLD_LAYERS.length]}`;
      // Gen.Run uses raw numeric primitives here. Views belong to spec slots.
      if (['fold/sum', 'fold/extrema'].includes(layer)) rs = rs.map(r =>
        (r.shown ?? 'here') === 'here' ? r : { ...r, shown: 'here', n: 0, value: '' });
      input = { orders: orders() };
      if (layer === 'fold/select') input.where = rng.pick([
        { n: { 'at-least': rng.int(7) - 3 } }, { 'env.kind': { is: 'note' } },
        { flag: { is: true } }, { 'env.occurred': { below: '2025' } },
        { 'env.of': { present: true } }, { 'env.of': { is: thingOf(rs[0]) } },
      ]);
      break;
    case 'spec':
      layer = index % 2 ? 'spec/run' : 'spec/parse';
      input = { spec: spec(rng, rs) };
      if (layer === 'spec/run') {
        // Spec.run accepts settled records, with one form per ID.
        const erased = new Set(rs.flatMap(r => r.erases ?? []));
        rs = rs.map(r => {
          if (!erased.has(r.id) && r.shown !== 'erased') return r;
          const { value: _value, n: _n, fields: _fields, ...envelope } = r;
          return { ...envelope, shown: erased.has(r.id) ? 'erased' : 'withheld' };
        });
        input.orders = orders();
      }
      break;
    case 'realm': {
      input = { forms: clone(rs) };
      // Different places hold forms of the same logical record.
      if (rng.int(2)) {
        const r = clone(rs[0]); delete r.value; delete r.n; delete r.fields;
        r.shown = 'withheld'; input.forms.push(r);
      }
      break;
    }
    case 'scenario': {
      const steps = Array.from({ length: numeric ? 3 : 1 + rng.int(4) }, (_, i) => {
        const forms = rs.slice(0, Math.min(rs.length, i + 1)).map(r => ({ id: r.id,
          shown: r.shown ?? rng.pick(['here', 'here', 'withheld', 'erased']) }));
        const ids = forms.map(f => f.id);
        return { name: `step-${i}`, forms, orders: [ids, shuffled(ids, rng)] };
      });
      input = { spec: spec(rng, rs), thing: 't:0', records: rs, steps, lens: lens(rng) };
      input.thing = thingOf(rs[0]);
      break;
    }
    case 'batch': {
      const salt = base32(rng.bytes(16));
      const env = envelope(rng);
      // One thing: the first draft's natural key, or the first draft itself, a
      // root, which the others name by label.
      const thing = env.of ?? '@d0';
      input = { held: [], drafts: Array.from({ length: 1 + rng.int(3) }, (_, i) => ({
        label: `@d${i}`, env: { ...clone(env), ...(i && { of: thing }), prev: i ? [`@d${rng.int(i)}`] : [] }, salt,
        payload: { n: rng.int(100), text: `draft-${rng.int(1000)}` },
      })) };
      if (rng.int(3) === 0) {
        input.drafts.at(-1).env.about.push('@d0');
        if (input.drafts.length === 1) nearMiss = 'label-cycle';
      }
      // The last draft's `of` breaks one rule: left out beside its prev, naming
      // a draft that is not a root, or naming another thing than its prev's.
      const ds = input.drafts, last = ds.at(-1);
      if (ds.length > 1 && rng.int(4) === 0) {
        const notRoot = ds[0].env.of !== undefined ? '@d0' : ds.length > 2 ? `@d${ds.length - 2}` : null;
        const choice = rng.int(3);
        if (choice === 0) { delete last.env.of; nearMiss = 'of-missing'; }
        else if (choice === 1 && notRoot) { last.env.of = notRoot; nearMiss = 'of-not-root'; }
        else { last.env.of = 't:elsewhere'; nearMiss = 'prev-unrelated'; }
      }
      break;
    }
    case 'record': {
      const env = envelope(rng), p = payload(rng);
      const variant = reorder(clone(env), rng);
      variant.about = variant.about.toReversed().concat(variant.about[0]);
      input = { envs: [env, variant], payloads: [p, reorder(p, rng)],
        salt: base32(rng.bytes(16)), seed: base32(rng.bytes(32)) };
      break;
    }
    case 'lens': layer = 'lens/section'; input = lens(rng); break;
  }
  // Scheduled boundaries precede the independent random mutation.
  if (family === 'lens') {
    const slot = index % 16;
    if (slot === 4) {
      const base = [{ id: 't:fence', body: ['::: old', '\\old', '', 'λ'] }];
      const edited = [{ id: 't:fence', body: [':::', '::: next', '\\next', '', '雪'] }];
      input = { base, text: renderSections(edited) };
    } else if (slot === 5) {
      input = { base: [], text: renderSections([{ id: 't:repeat', body: ['first'] },
        { id: 't:repeat', body: ['second'] }]) };
      nearMiss = 'section-repeat';
    } else if (slot === 6) {
      input = { base: [], text: ['::: ', 'body', ':::'] };
      nearMiss = 'section-empty';
    } else if (slot === 7) {
      const base = [{ id: 't:repeat', body: ['first'] }, { id: 't:repeat', body: ['second'] }];
      input = { base, text: renderSections([base[0]]) };
      nearMiss = 'base-repeat';
    }
  }
  // Held records: a root, and in slots 6 and 7 a revision of it, which is not
  // a root. The first draft continues the held thing, stands on the root, or
  // names the revision in `of`; the drafts of its thing follow it there.
  if (family === 'batch' && index % 16 >= 4 && index % 16 < 8) {
    const slot = index % 16, ds = input.drafts;
    const id = `r:${base32(Array(32).fill(7))}`, revision = `r:${base32(Array(32).fill(8))}`;
    input.held.push({ id, prev: [] });
    if (slot >= 6) input.held.push({ id: revision, of: id, prev: [id] });
    const retarget = of => {
      const was = ds[0].env.of ?? '@d0';
      for (const d of ds) if (d.env.of === was) d.env.of = of;
      ds[0].env.of = of;
    };
    if (slot === 4) { ds[0].env.prev = [id]; retarget(id); }
    if (slot === 5) ds[0].env.under = id;
    if (slot === 6) { ds[0].env.prev = [revision]; retarget(id); }
    if (slot === 7) { retarget(revision); nearMiss ??= 'of-not-root-held'; }
  }
  // Each named record boundary has a fixed slot and cannot be randomly overwritten.
  let scheduled = false;
  if (family === 'batch' && index % 64 >= 10 && index % 64 < 20) {
    const slot = index % 64 - 10, bad = slot % 5;
    const a = { label: '@a', env: { ...clone(input.drafts[0].env), prev: [] }, salt: input.drafts[0].salt, payload: null };
    const b = clone(a); b.label = '@b'; b.env.extra = true;
    if (bad === 0) b.label = '@a';
    if (bad === 1) a.env.prev = ['@missing'];
    if (bad === 2) a.env.prev = ['@a'];
    if (bad === 3) a.env.kind = 17;
    if (bad === 4) a.salt = '!';
    input.drafts = slot < 5 ? [a, b] : [b, a];
    nearMiss = `shape-${['duplicate-label', 'unknown-label', 'cycle-label', 'malformed-value', 'malformed-salt'][bad]}-${slot < 5 ? 'last' : 'first'}`;
    scheduled = true;
  }
  // Issue #1 and labels in `under`, each in a fixed slot: a draft whose
  // envelope is not an object or is left out, a field left out or of the
  // wrong type, a salt that is not text, and labels standing in `under`.
  if (family === 'batch' && index % 64 >= 24 && index % 64 < 35) {
    const slot = index % 64 - 24, ds = input.drafts, last = ds.at(-1);
    nearMiss = ['env-not-object', 'env-missing', 'kind-missing', 'about-text', 'salt-number',
      'label-in-under', 'under-label-unknown', 'under-label-cycle', 'label-in-erases',
      'draft-member-unknown', 'draft-member-unknown-after-shape'][slot];
    scheduled = true;
    // Undo the random label cycle and `of` faults above, so the slot alone decides.
    ds[0].env.about = ds[0].env.about.filter(a => a !== '@d0');
    for (const d of ds.slice(1)) d.env.of = ds[0].env.of ?? '@d0';
    if (slot === 0) last.env = rng.pick([7, null, 'note', [clone(last.env)]]);
    if (slot === 1) delete last.env;
    if (slot === 2) delete last.env.kind;
    if (slot === 3) last.env.about = 'topic:auth';
    if (slot === 4) last.salt = rng.pick([7, null]);
    // A grant written beside the record acting on it: the first draft is a
    // root, so a later one may stand under it by label.
    if (slot === 5) {
      ds.push({ label: '@acting', env: { ...clone(ds[0].env), of: undefined, prev: [], under: '@d0' }, salt: ds[0].salt,
        payload: { acting: true } });
      delete ds.at(-1).env.of;
    }
    if (slot === 6) last.env.under = '@nowhere';
    // Every later draft reaches the first through `prev`, so the first standing
    // under the last closes a cycle; a single draft stands under itself.
    if (slot === 7) ds[0].env.under = `@d${ds.length - 1}`;
    if (slot === 8) last.env.erases = ['@d0'];
    // An unknown draft member is malformed input, after the shape stage.
    if (slot === 9 || slot === 10) last.extra = 0;
    if (slot === 10) ds[0].env.extra = true;
  }
  // An `under` target must be held, in the batch or the store (Kyle,
  // 2026-11-13), and nothing else about it is checked. Each case has a fixed
  // slot: a target nobody holds, a held root, a held revision (no grant), the
  // raw id of a batch member, and an unheld target beside an unheld `prev`, an
  // `of` naming a held revision and a `prev` of another thing, in both orders.
  if (family === 'batch' && index % 64 >= 42 && index % 64 < 52) {
    const d = index % 64 - 42, ds = input.drafts, last = ds.at(-1);
    nearMiss = ['under-unheld', 'under-held-root', 'under-held-revision', 'under-batch-id',
      'under-beside-prev-unknown-first', 'under-beside-prev-unknown-last',
      'under-beside-of-not-root-first', 'under-beside-of-not-root-last',
      'under-beside-prev-unrelated-first', 'under-beside-prev-unrelated-last'][d];
    scheduled = true;
    // Undo the random label cycle and `of` faults above, so the slot alone decides.
    ds[0].env.about = ds[0].env.about.filter(a => a !== '@d0');
    for (const x of ds.slice(1)) x.env.of = ds[0].env.of ?? '@d0';
    const root = `r:${base32(Array(32).fill(7))}`, revision = `r:${base32(Array(32).fill(8))}`;
    const unheld = `r:${base32(Array(32).fill(9))}`;
    input.held.push({ id: root, prev: [] }, { id: revision, of: root, prev: [root] });
    if (d === 1) last.env.under = root;
    else if (d === 2) last.env.under = revision;
    else if (d === 3) {
      // The first draft names no label, so its id is known before the batch is written.
      const first = ds[0], id = idOf(readEnv(first.env), commitOf(readSalt(first.salt), first.payload));
      const acting = { label: '@acting', env: { ...clone(first.env), prev: [], under: id }, salt: first.salt,
        payload: { acting: true } };
      delete acting.env.of;
      ds.push(acting);
    } else {
      last.env.under = unheld;
      const beside = { label: '@beside', env: { ...clone(ds[0].env), prev: [] }, salt: ds[0].salt, payload: { beside: d } };
      if (d === 4 || d === 5) { beside.env.of = 't:item'; beside.env.prev = [`r:${base32(Array(32).fill(10))}`]; }
      if (d === 6 || d === 7) beside.env.of = revision;
      if (d === 8 || d === 9) { beside.env.of = 't:elsewhere'; beside.env.prev = [root]; }
      input.drafts = d % 2 ? [...ds, beside] : [beside, ...ds];
    }
  }
  // `sets` in a changeset, each case in a fixed slot: not an array, an empty
  // name, its place after `erases-not-record-id` and before `of-missing`
  // across drafts in both orders, a key that looks like a label, which is a
  // key, and an empty `sets`, which is written.
  if (family === 'batch' && index % 64 >= 56) {
    const d = index % 64 - 56, ds = input.drafts, last = ds.at(-1);
    nearMiss = ['sets-not-array', 'sets-empty-name', 'erases-before-sets-first', 'erases-before-sets-last',
      'sets-before-of-missing-first', 'sets-before-of-missing-last', 'sets-label-key', 'sets-empty'][d];
    scheduled = true;
    // Undo the random label cycle and `of` faults above, so the slot alone decides.
    ds[0].env.about = ds[0].env.about.filter(a => a !== '@d0');
    for (const x of ds.slice(1)) x.env.of = ds[0].env.of ?? '@d0';
    if (d === 0) last.env.sets = rng.pick(['title', null, 7, { title: true }]);
    if (d === 1) last.env.sets = ['title', ''];
    if (d >= 2 && d < 6) {
      last.env.sets = d < 4 ? [7] : [''];
      const beside = { label: '@beside', env: { ...clone(ds[0].env), prev: [] }, salt: ds[0].salt, payload: { beside: d } };
      if (d < 4) beside.env.erases = ['t:item'];
      else { delete beside.env.of; beside.env.prev = ['@d0']; }
      input.drafts = d % 2 ? [...ds, beside] : [beside, ...ds];
    }
    if (d === 6) last.env.sets = ['@d0', '@nowhere', 'n', '@d0'];
    if (d === 7) last.env.sets = [];
  }
  // One draft's envelope is not an object, or left out, or the draft is not an
  // object, beside another draft whose envelope has an unknown key, in both
  // orders: the unknown key is refused, whichever comes first.
  if (family === 'batch' && (index % 64 === 40 || index % 64 === 41)) {
    const first = index % 64 === 40, d = input.drafts[0];
    const unknown = { label: '@b', env: { ...clone(d.env), prev: [], extra: true }, salt: d.salt, payload: null };
    const other = { label: '@a', env: rng.pick([7, null, 'note', [clone(d.env)]]), salt: d.salt, payload: null };
    const choice = rng.int(3);
    if (choice === 1) delete other.env;
    input.drafts = first ? [choice === 2 ? 7 : other, unknown] : [unknown, choice === 2 ? 7 : other];
    nearMiss = first ? 'not-object-before-unknown-draft' : 'unknown-before-not-object-draft';
    scheduled = true;
  }
  if (family === 'record') {
    const slot = index % 64;
    const badTimes = [
      ['offset', '2024-02-29T12:00:00+01:00'],
      ['second-60', '2024-02-29T12:00:60Z'],
      ['trailing-zero', '2024-02-29T12:00:00.50Z'],
      ['ten-digits', '2024-02-29T12:00:00.1234567891Z'],
    ];
    const clean = { ...input.envs[0], emitted: INSTANTS[0], occurred: '2024' };
    if (slot < 7) {
      const names = ['id-uppercase', 'id-w', 'id-x', 'id-y', 'id-z', 'id-pad-bits', 'salt-pad-bits'];
      nearMiss = names[slot]; scheduled = true;
      const text = '0'.repeat(51);
      if (slot === 0) clean.prev = [`r:${'A'.repeat(51)}0`];
      if (slot >= 1 && slot <= 4) clean.prev = [`r:${text}${'wxyz'[slot - 1]}`];
      if (slot === 5) clean.prev = [`r:${text}1`];
      if (slot === 6) input.salt = `${'0'.repeat(25)}1`;
    } else if (slot < 27) {
      const n = slot - 7, [name, stamp] = badTimes[n % 4];
      const position = Math.floor(n / 4);
      const labels = ['emitted', 'occurred-single', 'occurred-start', 'occurred-stop', 'occurred-open'];
      nearMiss = `${labels[position]}-${name}`; scheduled = true;
      if (position === 0) clean.emitted = stamp;
      if (position === 1) clean.occurred = stamp;
      if (position === 2) clean.occurred = `${stamp}/${INSTANTS[0]}`;
      if (position === 3) clean.occurred = `${INSTANTS[0]}/${stamp}`;
      if (position === 4) clean.occurred = `${stamp}/..`;
    } else if (slot === 27) {
      nearMiss = 'unknown-envelope-key-empty-kind'; scheduled = true;
      clean.kind = ''; clean.extra = true;
    } else if (slot >= 28 && slot < 32) {
      const bad = { ...clone(clean), extra: true };
      const other = { ...clone(clean), kind: slot % 2 ? 17 : '' };
      if (slot % 2) input.salt = '!';
      input.envs = slot < 30 ? [other, bad] : [bad, other];
      nearMiss = `shape-multi-record-${slot}`; scheduled = true;
    }
    if (scheduled && slot < 28) input.envs = [clean];
    // D8: `of` is checked right after `about`, and a `prev` without `of` is
    // refused only once every field is well formed.
    if (slot >= 32 && slot < 37) {
      const d = slot - 32, target = `r:${'0'.repeat(52)}`;
      const { of: _of, ...root } = clean;
      nearMiss = ['of-untyped', 'of-missing', 'about-before-of', 'of-before-prev', 'erases-before-of-missing'][d];
      scheduled = true;
      input.envs = [[{ ...root, of: 'item' }, { ...root, prev: [target] }, { ...root, about: ['item'], of: 'item' },
        { ...root, of: 'item', prev: ['t:item'] }, { ...root, prev: [target], erases: ['t:item'] }][d]];
    }
    // Issue #1: an envelope that is not an object, a field left out or of the
    // wrong JSON type, and a salt that is not base32hex text. Each has a fixed
    // slot in one cycle of 64 in every four, so the other cycles keep their
    // random records; slot 40 stays unscheduled for the valid-record checks.
    const shapeSlots = [37, 38, 39, 41, 42, 43, 56, 57, 58, 59, 60, 61, 62, 63];
    // No envelope to write is malformed input; a text payload's lone surrogate comes first.
    if (index % 256 >= 64 && index % 256 < 128 && (slot === 37 || slot === 38)) {
      nearMiss = slot === 37 ? 'no-envelope' : 'no-envelope-lone';
      scheduled = true;
      input.envs = [];
      if (slot === 38) { delete input.payloads; input.payload_text = '["\\ud800"]'; }
    }
    // A variant that is not an object beside one with an unknown key, in both
    // orders: the unknown key is refused first, whichever comes first.
    if (index % 256 >= 64 && (slot === 39 || slot === 41)) {
      const other = rng.pick([7, null, 'note', true, [clone(clean)]]), unknown = { ...clone(clean), extra: true };
      nearMiss = slot === 39 ? 'not-object-before-unknown-variant' : 'unknown-before-not-object-variant';
      scheduled = true;
      input.envs = slot === 39 ? [other, unknown] : [unknown, other];
    }
    if (index % 256 < 64 && shapeSlots.includes(slot)) {
      const d = shapeSlots.indexOf(slot), target = `r:${'0'.repeat(52)}`;
      const without = (k) => { const env = clone(clean); delete env[k]; return env; };
      nearMiss = ['not-object', 'not-object-variant', 'not-object-array', 'missing-kind', 'missing-observer',
        'missing-emitted', 'observer-number', 'about-text', 'about-member-number', 'of-number', 'prev-text',
        'under-number', 'erases-null', 'salt-not-text'][d];
      scheduled = true;
      input.envs = [[rng.pick([7, null, 'note', true])], [clean, null], [[clean]], [without('kind')],
        [without('observer')], [without('emitted')], [{ ...clean, observer: 17 }], [{ ...clean, about: 't:x' }],
        [{ ...clean, about: ['t:x', 7] }], [{ ...clean, of: 7 }], [{ ...clean, of: 't:x', prev: target }],
        [{ ...clean, under: 7 }], [{ ...clean, erases: null }], [clean]][d];
      if (d === 13) {
        const salt = rng.pick([7, null, ['00'], undefined]);
        if (salt === undefined) delete input.salt; else input.salt = salt;
      }
    }
    // D2, D3 and D5: text payloads and variants that disagree. Each has a
    // fixed slot. The lone surrogate and the overflow travel as text, since a
    // JSON writer would drop the one and write null for the other.
    if (slot >= 44 && slot < 56) {
      const d = slot - 44, noKind = { ...clone(clean), kind: '' }, untyped = { ...clone(clean), observer: 'nobody' };
      const lone = d % 2 ? '{"a":1,"a":2,"k\\udc00":0}' : '["\\ud83d"]';
      const text = (envs, payload) => { input.envs = envs; delete input.payloads; input.payload_text = payload; };
      nearMiss = ['text-lone-high', 'text-lone-low-key', 'text-lone-after-shape', 'text-lone-before-kind',
        'text-lone-before-salt', 'text-overflow', 'text-overflow-after-kind', 'text-overflow-after-repeat',
        'variants-refused-first', 'variants-differently-refused', 'variants-refused-later', 'variants-payload-refused-later'][d];
      scheduled = true;
      if (d === 0 || d === 1) text([clean], lone);
      if (d === 2) text([{ ...clone(clean), extra: true }], lone);
      if (d === 3) text([noKind], lone);
      if (d === 4) { text([clean], lone); input.salt = base32(rng.bytes(15)); }
      if (d === 5) text([clean], '{"n":[1e999,-1E400]}');
      if (d === 6) text([noKind], '1e999');
      if (d === 7) text([clean], '{"n":1e999,"n":0}');
      if (d === 8) { input.envs = [noKind, clean]; input.payloads = [null]; }
      if (d === 9) { input.envs = [untyped, noKind]; input.payloads = [null]; }
      if (d === 10) { input.envs = [clean, noKind]; input.payloads = [null]; }
      if (d === 11) { input.envs = [clean]; input.payloads = [1, Number.MAX_SAFE_INTEGER + 1]; }
    }
    // `sets`, in slots of the cycles of 256 that issue #1's shapes leave free:
    // each way to be `sets-not-keys`, its place after `erases-not-record-id`
    // and before `of-missing`, and sets that are written: empty, a key the
    // payload lacks, beside a payload that is not an object, and variants in
    // other orders with repeats.
    const setsSlots = [37, 38, 42, 43, 56, 57, 58, 59, 60, 61, 62, 63];
    if (index % 256 >= 128 && setsSlots.includes(slot)) {
      const d = setsSlots.indexOf(slot), target = `r:${'0'.repeat(52)}`;
      const { of: _of, ...root } = clean;
      nearMiss = ['sets-text', 'sets-null', 'sets-object', 'sets-member-number', 'sets-member-null',
        'sets-empty-name', 'erases-before-sets', 'sets-before-of-missing', 'sets-empty', 'sets-key-not-in-payload',
        'sets-payload-not-object', 'sets-variants'][d];
      scheduled = true;
      input.envs = [[{ ...clean, sets: 'title' }], [{ ...clean, sets: null }], [{ ...clean, sets: { title: true } }],
        [{ ...clean, sets: ['title', 7] }], [{ ...clean, sets: [null] }], [{ ...clean, sets: ['title', ''] }],
        [{ ...clean, erases: ['t:item'], sets: [''] }], [{ ...root, prev: [target], sets: [7] }],
        [{ ...clean, sets: [] }], [{ ...clean, sets: ['missing'] }], [{ ...clean, sets: ['status', 'title'] }],
        [{ ...clean, sets: ['title', 'status', '@d0'] }, { ...clean, sets: ['@d0', 'status', 'title', 'status'] }]][d];
      if (d === 9) input.payloads = [{ title: 'kept' }];
      if (d === 10) input.payloads = ['done'];
    }
  }
  // A separate one-in-eight draw probes an independent input boundary.
  if (!scheduled && rng.int(8) === 0) {
    switch (family) {
      case 'record': {
        const choice = rng.int(8);
        nearMiss = ['payload-repeat', 'integer-bound', 'bad-edtf', 'bad-id', 'wrong-type', 'salt-length',
          'of-untyped', 'of-missing'][choice];
        if (choice === 0) { delete input.payloads; input.payload_text = '{"outer":{"x":1,"x":2}}'; }
        if (choice === 1) input.payloads = [Number.MAX_SAFE_INTEGER + 1];
        if (choice === 2) input.envs = [{ ...input.envs[0], occurred: '2024-02-30' }];
        if (choice === 3) input.envs = [{ ...input.envs[0], prev: ['r:bad'] }];
        if (choice === 4) input.envs[0].kind = 17;
        if (choice === 5) input.salt = base32(rng.bytes(15));
        // The `of` checks come late, so these envelopes are otherwise valid.
        if (choice === 6) input.envs = [{ ...input.envs[0], occurred: '2024', of: 'item' }];
        if (choice === 7) {
          const { of: _of, ...env } = input.envs[0];
          input.envs = [{ ...env, occurred: '2024', prev: [`r:${base32(rng.bytes(32))}`] }];
        }
        break;
      }
      case 'spec':
        if (layer === 'spec/parse') {
          const choice = rng.int(8);
          nearMiss = ['spec-repeat', 'latest-by-about', 'from-of-untyped', 'follow-not-prev',
            'by-on-latest-by', 'by-not-text', 'by-bad-prefix', 'by-env-other'][choice];
          if (choice === 0) input = { spec_text: '{"vertex":"a","vertex":"b","from":[{"store":"s:x"}]}' };
          if (choice === 1) input.spec.fold.heads['latest-by'] = 'about';
          if (choice === 2) input.spec.from[0].of = 'item';
          if (choice === 3) input.spec.fold[rng.pick(['now', 'now-total', 'now-least', 'now-greatest'])].follow = rng.pick(['next', true]);
          if (choice === 4) input.spec.fold.heads.by = 'env.kind';
          if (choice === 5) input.spec.fold['by-tag-items'].by = rng.pick([true, 3, null]);
          if (choice === 6) input.spec.fold['by-mention'].by = rng.pick(['env.about:', 'env.about:T', 'env.about:t:0']);
          if (choice === 7) input.spec.fold['by-kind'].by = rng.pick(['env.of', 'env.about', 'env.', '']);
        } else { nearMiss = 'bad-edtf'; input.spec.fold.chosen.where = { 'env.occurred': { is: '2024-13' } }; }
        break;
      case 'fold':
        nearMiss = 'bad-id';
        input.orders.forEach(order => {
          const record = order.find(r => r.id === rs[0].id);
          if (record) record.id = 'r:bad';
        });
        break;
      case 'realm':
        nearMiss = 'wrong-type';
        if (rng.int(2)) input.forms[0].about = 3; else input.forms[0].of = 3;
        break;
      case 'scenario': nearMiss = 'bad-edtf'; input.records[0].occurred = '2024-99'; break;
      case 'batch': nearMiss = 'label-unknown'; input.drafts[0].env.prev = ['@missing']; break;
      case 'lens': nearMiss = 'unclosed-section'; input.text = ['::: t:open', 'body']; break;
    }
  }
  // A spec run's held payloads get `rank` last, so every earlier draw stays as
  // it was. It is left out, text, a number, or number text that the file
  // writes as spelled: a whole number shares its text's group, a fraction has
  // none. One record has one rank in every order.
  if (layer === 'spec/run' && input.orders) {
    const ranks = new Map();
    for (const r of input.orders[0]) {
      if (!r.fields) continue;
      const k = rng.int(3), form = rng.int(4);
      if (form) ranks.set(r.id, form === 1 ? String(k) : form === 2 ? k
        : { $number: rng.pick([`${k}.0`, `${k * 10}e-1`, `${k}E0`, k ? `${k}.000` : '-0', `${k}.5`]) });
    }
    for (const order of input.orders) for (const r of order) if (ranks.has(r.id)) r.fields.rank = clone(ranks.get(r.id));
  }
  // `sets` on records and drafts no slot schedules, drawn last so every earlier
  // draw stays as it was: absent, empty, some key names with repeats, or now
  // and then not keys. A record's variants hold the same set in another order.
  const isObject = v => v !== null && typeof v === 'object' && !Array.isArray(v);
  const drawSets = () => {
    const roll = rng.int(10);
    if (roll < 5) return undefined;
    if (roll === 5) return [];
    if (roll < 9) return Array.from({ length: 1 + rng.int(3) }, () => rng.pick(['title', 'status', 'n', 'text', '@d0', '😀', 'z']));
    return rng.pick(['title', null, 7, [''], ['n', 7], [null]]);
  };
  if (!scheduled && family === 'record' && Array.isArray(input.envs)) {
    const sets = drawSets();
    if (sets !== undefined) input.envs.forEach((env, i) => {
      if (isObject(env)) env.sets = i && Array.isArray(sets) && sets.length ? [...shuffled(sets, rng), sets[0]] : clone(sets);
    });
  }
  if (!scheduled && family === 'batch') {
    for (const d of input.drafts) {
      if (!isObject(d) || !isObject(d.env)) continue;
      const sets = drawSets();
      if (sets !== undefined) d.env.sets = sets;
    }
  }
  return { layer, input, fuzz: { seed: String(seed), index, family, nearMiss } };
}

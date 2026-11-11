// Each case has its own PRNG stream. Family scheduling does not change a case.
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
          const choice = rng.int(4);
          nearMiss = ['spec-repeat', 'latest-by-about', 'from-of-untyped', 'follow-not-prev'][choice];
          if (choice === 0) input = { spec_text: '{"vertex":"a","vertex":"b","from":[{"store":"s:x"}]}' };
          if (choice === 1) input.spec.fold.heads['latest-by'] = 'about';
          if (choice === 2) input.spec.from[0].of = 'item';
          if (choice === 3) input.spec.fold[rng.pick(['now', 'now-total', 'now-least', 'now-greatest'])].follow = rng.pick(['next', true]);
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
  return { layer, input, fuzz: { seed: String(seed), index, family, nearMiss } };
}

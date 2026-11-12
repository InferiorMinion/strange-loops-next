// Unit checks only. No engine processes or campaigns run here.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { generate } from './generate.mjs';
import { evaluateJS, assess } from './evaluate.mjs';
import { occurredSpan } from '../js/loops.mjs';

const dir = new URL('../../vectors/byte-text/', import.meta.url);
test('byte-text adapter uses the public strict decoder', () => {
  for (const file of readdirSync(dir)) {
    const vector = JSON.parse(readFileSync(new URL(file, dir), 'utf8'));
    assert.deepEqual(evaluateJS(vector), [vector.expect], file);
  }
});

test('named record boundaries remain scheduled across seeds', () => {
  const names = ['id-uppercase', 'id-w', 'id-x', 'id-y', 'id-z', 'id-pad-bits', 'salt-pad-bits'];
  const parts = ['emitted', 'occurred-single', 'occurred-start', 'occurred-stop', 'occurred-open'];
  const kinds = ['offset', 'second-60', 'trailing-zero', 'ten-digits'];
  names.push(...parts.flatMap(part => kinds.map(kind => `${part}-${kind}`)), 'unknown-envelope-key-empty-kind');
  for (const seed of ['a', 'b', 'c', 'd']) {
    for (const [i, name] of names.entries()) {
      const v = generate(seed, i, 'record');
      assert.equal(v.fuzz.nearMiss, name);
      assert.equal(v.input.envs.length, 1);
      const env = v.input.envs[0];
      // A salt that is not canonical base32hex reads as no bytes (issue #1).
      const expected = i < 6 ? 'prev-not-record-id' : i === 6 ? 'salt-not-16-bytes' : i < 11 ? 'emitted-not-rfc3339'
        : i < 27 ? 'occurred-not-span' : 'envelope-field-unknown';
      assert.deepEqual(evaluateJS(v), [{ refused: expected }], name);
      if (i === 27) { assert.equal(env.kind, ''); assert.equal(env.extra, true); }
    }
  }
});

test('named of boundaries remain scheduled across seeds', () => {
  const cases = [['of-untyped', 'of-not-typed'], ['of-missing', 'of-missing'], ['about-before-of', 'about-not-typed'],
    ['of-before-prev', 'of-not-typed'], ['erases-before-of-missing', 'erases-not-record-id']];
  for (const seed of ['a', 'b', 'c', 'd']) {
    for (const [i, [name, expected]] of cases.entries()) {
      const v = generate(seed, 32 + i, 'record');
      assert.equal(v.fuzz.nearMiss, name);
      assert.deepEqual(evaluateJS(v), [{ refused: expected }], name);
    }
  }
});

test('envelope shape boundaries remain scheduled across seeds and refuse by name', () => {
  const record = [[37, 'not-object', 'envelope-not-object'], [38, 'not-object-variant', 'envelope-not-object'],
    [39, 'not-object-array', 'envelope-not-object'], [41, 'missing-kind', 'kind-empty'],
    [42, 'missing-observer', 'observer-not-typed'], [43, 'missing-emitted', 'emitted-not-rfc3339'],
    [56, 'observer-number', 'observer-not-typed'], [57, 'about-text', 'about-not-typed'],
    [58, 'about-member-number', 'about-not-typed'], [59, 'of-number', 'of-not-typed'],
    [60, 'prev-text', 'prev-not-record-id'], [61, 'under-number', 'under-not-record-id'],
    [62, 'erases-null', 'erases-not-record-id'], [63, 'salt-not-text', 'salt-not-16-bytes']];
  const batch = [[24, 'env-not-object', 'envelope-not-object'], [25, 'env-missing', 'envelope-not-object'],
    [26, 'kind-missing', 'kind-empty'], [27, 'about-text', 'about-not-typed'], [28, 'salt-number', 'salt-not-16-bytes'],
    [29, 'label-in-under', null], [30, 'under-label-unknown', 'label-unknown'],
    [31, 'under-label-cycle', 'label-cycle'], [32, 'label-in-erases', 'erases-not-record-id']];
  for (const seed of ['a', 'b', 'c', 'd']) {
    for (const [family, cases] of [['record', record], ['batch', batch]]) {
      for (const [slot, name, refused] of cases) {
        const v = generate(seed, 256 + slot, family);
        assert.equal(v.fuzz.nearMiss, name);
        const text = JSON.stringify(v);
        const [answer] = evaluateJS(JSON.parse(text), text);
        if (refused) { assert.deepEqual(answer, { refused }, `${family} ${name}`); continue; }
        // A label in `under` resolves to the id of the draft it names.
        const acting = v.input.drafts.findIndex(d => d.env.under === '@d0');
        assert.equal(JSON.parse(answer.records[acting].env_canonical).under, answer.records[0].id);
      }
    }
  }
});

test('record preflight and writer refusal precede seed decoding', () => {
  const env = { kind: '', observer: 'o:x', emitted: '2024-01-01T00:00:00Z' };
  const input = { envs: [env], payloads: [null], salt: '00000000000000000000000000', seed: '!' };
  assert.deepEqual(evaluateJS({ layer: 'record', input }), [{ refused: 'kind-empty' }]);
  input.envs.push({ ...env, extra: true });
  input.salt = '!';
  assert.deepEqual(evaluateJS({ layer: 'record', input }), [{ refused: 'envelope-field-unknown' }]);
});

test('record shape refusal escapes Cartesian expansion in both malformed orders', () => {
  for (const raw of [7, { kind: 7 }]) {
    const unknown = { kind: '', extra: true };
    for (const envs of [[raw, unknown], [unknown, raw], [{ extra: 1 }], [{ kind: 7, extra: 1 }]]) {
      const input = { envs, payloads: [null, {}], salt: '!', seed: '!' };
      assert.deepEqual(evaluateJS({ layer: 'record', input }), [{ refused: 'envelope-field-unknown' }]);
    }
  }
});

test('raw batch preflight retains every repeated envelope member', () => {
  for (const envs of ['"env":{"extra":1},"env":{"kind":""}', '"env":{"kind":""},"env":{"extra":1}']) {
    const raw = `{"layer":"batch","input":{"held":[],"drafts":[{${envs},"salt":"","payload":null}]}}`;
    assert.deepEqual(evaluateJS(JSON.parse(raw), raw), [{ refused: 'envelope-field-unknown' }]);
  }
});

test('a repeated envelope or draft member is malformed input from the raw source, after shape', () => {
  const env = '{"kind":"note","observer":"o:t","emitted":"2026-10-21T00:00:00Z"}';
  const repeated = '{"kind":0,"kind":"note","observer":"o:t","emitted":"2026-10-21T00:00:00Z"}';
  const salt = '"' + '0'.repeat(26) + '"';
  const batch = (drafts) => `{"layer":"batch","input":{"held":[],"drafts":[${drafts.join(',')}]}}`;
  const record = (envs, payload = '"payloads":[null]') =>
    `{"layer":"record","input":{"envs":[${envs.join(',')}],${payload},"salt":${salt}}}`;
  const answer = (raw) => evaluateJS(JSON.parse(raw), raw);
  const draft = (members) => `{${members},"salt":${salt},"payload":null}`;
  // Without the raw text, the value API sees only the last `kind` and accepts.
  const value = JSON.parse(batch([draft(`"env":${repeated}`)]));
  assert.ok(evaluateJS(value)[0].records);
  for (const raw of [batch([draft(`"env":${repeated}`)]), batch([draft(`"env":${env},"env":${env}`)]),
    batch([draft(`"env":${env}`), draft(`"label":"@a","label":"@b","env":${env}`)]),
    record([repeated]), record([env, repeated])]) {
    assert.throws(() => answer(raw), /repeats a member/, raw);
  }
  // Shape keeps priority over the repeat; a record's text refusal does too.
  assert.deepEqual(answer(batch([draft(`"env":${repeated}`), draft('"env":{"extra":1}')])), [{ refused: 'envelope-field-unknown' }]);
  assert.deepEqual(answer(batch([draft(`"env":${repeated}`), draft('"env":7')])), [{ refused: 'envelope-not-object' }]);
  assert.deepEqual(answer(record([repeated, '7'])), [{ refused: 'envelope-not-object' }]);
  assert.deepEqual(answer(record([repeated], '"payload_text":"\\"\\\\ud800\\""')), [{ refused: 'string-not-unicode' }]);
});

test('JS must also report malformed record and batch input where Lean does', () => {
  const v = { layer: 'batch', input: { held: [], drafts: [] } };
  const lean = { error: 'input.drafts[0]: has a member "extra" this input doesn\'t take' };
  assert.equal(assess(v, lean, { js: () => { throw new Error('batch input: a draft has an unknown member'); } }).status, 'decoder-refusal');
  assert.equal(assess(v, lean, { js: () => [{ records: [] }] }).status, 'mismatch');
  assert.equal(assess(v, lean, { js: () => [{ refused: 'kind-empty' }] }).status, 'mismatch');
  // Families without an input contract in JS still skip it.
  let called = false;
  assert.equal(assess({ layer: 'fold/count', input: {} }, lean, { js: () => { called = true; return [0]; } }).status, 'decoder-refusal');
  assert.equal(called, false);
});

test('unknown draft members and empty variants are scheduled and malformed in JS', () => {
  for (const seed of ['a', 'b']) {
    const unknown = generate(seed, 256 + 33, 'batch');
    assert.equal(unknown.fuzz.nearMiss, 'draft-member-unknown');
    assert.throws(() => evaluateJS(unknown), /unknown member/);
    const afterShape = generate(seed, 256 + 34, 'batch');
    assert.equal(afterShape.fuzz.nearMiss, 'draft-member-unknown-after-shape');
    assert.deepEqual(evaluateJS(afterShape), [{ refused: 'envelope-field-unknown' }]);
    const empty = generate(seed, 64 + 37, 'record');
    assert.equal(empty.fuzz.nearMiss, 'no-envelope');
    assert.throws(() => evaluateJS(empty), /no envelope given/);
    const lone = generate(seed, 64 + 38, 'record');
    assert.equal(lone.fuzz.nearMiss, 'no-envelope-lone');
    assert.deepEqual(evaluateJS(lone), [{ refused: 'string-not-unicode' }]);
  }
});

test('shape cases are scheduled in both orders', () => {
  for (const seed of ['a', 'b']) {
    for (let i = 10; i < 20; i++) {
      const v = generate(seed, i, 'batch');
      assert.match(v.fuzz.nearMiss, /^shape-/);
      assert.deepEqual(evaluateJS(v), [{ refused: 'envelope-field-unknown' }]);
    }
    for (let i = 28; i < 32; i++) {
      const v = generate(seed, i, 'record');
      assert.match(v.fuzz.nearMiss, /^shape-multi-record-/);
      assert.deepEqual(evaluateJS(v), [{ refused: 'envelope-field-unknown' }]);
    }
  }
});

test('valid generated records include fractional instants and occurred forms', () => {
  const envs = Array.from({ length: 128 }, (_, i) => generate('valid-times', i * 64 + 40, 'record'))
    .filter(v => v.fuzz.nearMiss === null).flatMap(v => v.input.envs);
  assert.ok(envs.some(env => /\.\d+Z$/.test(env.emitted)));
  assert.ok(envs.some(env => /T.*\//.test(env.occurred)));
  assert.ok(envs.some(env => /T.*Z$/.test(env.occurred)));
  const intervals = envs.filter(env => env.occurred.includes('/'));
  assert.ok(intervals.some(env => env.occurred.includes('T') && !env.occurred.endsWith('/..')));
  assert.ok(intervals.some(env => env.occurred.includes('T') && env.occurred.endsWith('/..')));
  for (const env of intervals) assert.ok(occurredSpan(env.occurred), env.occurred);
});

test('D2, D3 and D5 record boundaries are scheduled and answered as ruled', () => {
  const want = [
    ['text-lone-high', 'string-not-unicode'], ['text-lone-low-key', 'string-not-unicode'],
    ['text-lone-after-shape', 'envelope-field-unknown'], ['text-lone-before-kind', 'string-not-unicode'],
    ['text-lone-before-salt', 'string-not-unicode'], ['text-overflow', 'number-out-of-range'],
    ['text-overflow-after-kind', 'kind-empty'], ['text-overflow-after-repeat', 'member-duplicate'],
    ['variants-refused-first', 'kind-empty'], ['variants-differently-refused', 'observer-not-typed'],
    ['variants-refused-later', null], ['variants-payload-refused-later', null],
  ];
  for (const seed of ['a', 'b', 'c']) {
    for (const [d, [name, refused]] of want.entries()) {
      const v = generate(seed, 44 + d, 'record');
      assert.equal(v.fuzz.nearMiss, name);
      // The case survives the transport: its text is what the engines read.
      assert.deepEqual(JSON.parse(JSON.stringify(v)), v, name);
      if (refused) assert.deepEqual(evaluateJS(v, JSON.stringify(v)), [{ refused }], name);
      else assert.throws(() => evaluateJS(v, JSON.stringify(v)), /record input: (envs|payloads)\[1\]/, name);
    }
  }
});

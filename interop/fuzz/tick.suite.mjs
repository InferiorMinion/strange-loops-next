import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { campaign, parseArgs } from './run.mjs';
import { random } from './generate.mjs';
import { countedHolding, generateTick } from './tick.mjs';
import { ROOT, batchVectors, inputText, warmRust } from './transport.mjs';
import { assess } from './evaluate.mjs';

test('tick inputs reproduce independently of other cases and preserve deep repeated names', () => {
  const ops = new Set(), depths = new Set(), faultCounts = new Set();
  for (let i = 0; i < 384; i++) {
    const v = generateTick('tick-wire', i);
    ops.add(v.input.op);
    if (v.fuzz.faults) faultCounts.add(v.fuzz.faults.length);
    const wire = inputText(v);
    assert.deepEqual(JSON.parse(wire), v);
    assert.equal(wire, inputText(generateTick('tick-wire', i)));
    assert.notEqual(wire, inputText(generateTick('tick-other', i)));
    if (v.fuzz.repeat && wire !== JSON.stringify(v)) depths.add(v.fuzz.repeat.length);
  }
  assert.equal(ops.size, 6);
  assert.deepEqual(faultCounts, new Set([0,1,2,3,4]));
  assert.ok(depths.size >= 3);
});

test('unmutated checker claims equal the model writer on their original holdings', async () => {
  const cases = Array.from({ length: 36 }, (_, i) => countedHolding(random('tick-base', i, 'tick')));
  const inputs = cases.map(({writer, payload}) => ({layer:'tick',
    input:{op:'write-payload', spec:payload.spec, records:writer}}));
  const rows = await batchVectors(inputs);
  for (const [i,row] of rows.entries()) assert.deepEqual(row.expect?.payload, cases[i].payload);
});

test('tick campaign skips the JS witness through mismatch, reduction and replay', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'fuzz-test-tick-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const printed = t.mock.method(console, 'error', () => {});
  let jsCalls = 0;
  const builds = [];
  const wrong = { expect: { refused: 'tick-member-unknown', path: 'extra' } };
  const right = { expect: { refused: 'tick-member-duplicate', path: 'spec.from[0].store' } };
  const engines = {
    buildLean() { builds.push('lean'); }, buildRust() { builds.push('rust'); },
    js() { jsCalls++; throw new Error('JS witness invoked'); },
    batch: async (vectors, opts) => {
      assert.equal(opts.rust, true);
      return vectors.map(() => ({ lean: right, rust: wrong }));
    },
  };
  const run = await campaign(parseArgs(['--family','tick','--n','1','--out',join(dir,'run'),
    '--seed','planted','--shrink-trials','6']), engines);
  assert.deepEqual(run.summary.implementations, ['lean','rust']);
  assert.deepEqual(builds, ['lean','rust']);
  assert.equal(run.summary.mismatches, 1);
  assert.ok(run.summary.shrinkTrials > 0);
  const file = join(run.out,'mismatches','000000.json');
  const artifact = JSON.parse(await readFile(file,'utf8'));
  assert.deepEqual(artifact.answer.comparison.partition, [['lean'], ['rust']]);
  assert.deepEqual(artifact.shrink.answer.responses, artifact.answer.responses);
  const message = JSON.parse(printed.mock.calls[0].arguments[0]);
  assert.equal(message.seed, 'planted');
  assert.deepEqual(message.vector, artifact.vector);
  assert.deepEqual(message.answers, { lean:right, rust:wrong });
  const replay = await campaign(parseArgs(['--family','tick','--replay',file,'--no-shrink',
    '--out',join(dir,'replay')]), engines);
  assert.equal(replay.summary.mismatches, 1);
  assert.equal(replay.summary.requested, 1);
  assert.equal(jsCalls, 0);
  assert.throws(() => assess(artifact.vector,right,{tick:true}), /both Lean and Rust/);
});

test('tick replay accepts a named protocol vector and passes its original file text', async t => {
  const out = await mkdtemp(join(tmpdir(), 'fuzz-test-tick-replay-'));
  t.after(() => rm(out, { recursive: true, force: true }));
  const replay = join(ROOT, 'vectors/tick/outside-erased-taken.json');
  const source = await readFile(replay, 'utf8');
  const options = parseArgs(['--family', 'tick', '--replay', replay, '--out', out]);
  const { summary } = await campaign(options, {warm: false, batch: async ([v], opts) => {
    assert.equal(v.input.op, 'make-and-check');
    assert.equal(opts.rawText, source);
    const answer = { expect: {check: {answer: 'same'}} };
    return [{lean: answer, rust: answer}];
  }});
  assert.equal(summary.operations['make-and-check'].same, 1);
});

// Pin the widened stream and, separately, the unchanged families as generated at c654812.
test('two-way and three-way scheduling preserve the widened and unchanged family streams', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'fuzz-test-streams-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  for (const [seed, digest, unchangedDigest] of [
    ['ci-1','e685bddaf7ff51e8bdc41936db197532582c62ce0ef1591d6ee066c3ddb91e38',
      '0eefc4f6be65b3e39dfdfebda13b7162cce459ce597242651d106dfa85803443'],
    ['collect-limit-1','ba2afc22d4a2e9dd473706230f7e9bf2a67d9aa104171dbb6d92e739847862bc',
      '3227bb3425ca883d9008f086af420f6563d1face36884a06318c2b120a9e5aa2'],
  ]) {
    for (const rust of [false,true]) {
      const hash = createHash('sha256'), unchanged = createHash('sha256');
      const options = parseArgs([...(rust ? ['--rust'] : []),'--seed',seed,'--n','256','--out',join(dir, seed + rust)]);
      const { summary } = await campaign(options, { warm:false, js:() => [0], batch: async (vectors, opts) => {
        assert.equal(opts.rust, rust);
        return vectors.map(v => {
          const line = inputText(v) + '\n';
          hash.update(line);
          if (!['fold','realm','scenario'].includes(v.fuzz.family)) unchanged.update(line);
          return rust ? {lean:{expect:0}, rust:{expect:0}} : {expect:0};
        });
      } });
      assert.equal(summary.mismatches, 0);
      assert.deepEqual(summary.implementations, rust ? ['js','lean','rust'] : ['js','lean']);
      assert.equal(hash.digest('hex'), digest);
      assert.equal(unchanged.digest('hex'), unchangedDigest);
    }
  }
});

test('each decision site on its own reaches its declared Lean answer', async () => {
  const {decisionSites, orderVector} = await import('./tick-order.mjs');
  const vectors = decisionSites.map((id,i) => orderVector('single-site',i,[id]));
  const baseline = orderVector('single-site',0,['kind']);
  baseline.input.subject.kind = 'tick';
  const [control,...rows] = await batchVectors([baseline,...vectors]);
  assert.deepEqual(control.expect, {answer:'same',closure:'yes','taken-not-covered':0});
  for (const [i,row] of rows.entries()) {
    for (const [k,v] of Object.entries(vectors[i].fuzz.expected)) {
      assert.deepEqual(row.expect?.[k],v,decisionSites[i]);
    }
  }
});

test('both misspellings reach the wire and a third fault can hide their order', async () => {
  const {orderPairs,orderVector,excludedPairs,decisionSites} = await import('./tick-order.mjs');
  const spelling = ['spec-hash:spelling','result-hash:spelling'];
  const pair = orderVector('spelling',12,spelling);
  const p = pair.input.payload ?? pair.input.subject.payload;
  assert.equal(p['spec-hash'],'bad');
  assert.equal(p['result-hash'],'bad');
  const triple = orderVector('spelling',13,['kind',...spelling]);
  const [both,masked] = await batchVectors([pair,triple]);
  assert.equal(both.expect.path,'spec-hash');
  assert.equal(masked.expect.path,'kind');
  assert.equal(orderPairs.length + Object.keys(excludedPairs).length, decisionSites.length * (decisionSites.length-1)/2);
  assert.ok(excludedPairs['missing:covers / covers:unsorted']);
  const retained = JSON.parse(await readFile(join(ROOT,'interop/fuzz/evidence/tick-order-1/summary.json'),'utf8'));
  assert.deepEqual(Object.keys(retained.pairs).sort(),orderPairs.map(p => p.join(' / ')).sort());
  assert.deepEqual(retained.excludedPairs,excludedPairs);
});

test('the same pair can take different branches under current, older and newer rules', async () => {
  const {orderVector} = await import('./tick-order.mjs');
  const cases = [
    ['older','ignored-eraser','missing-record'],
    ['older','missing-record','missing-opening'],
    ['older','object:result','ignored-eraser'],
    ['older','object:result','differs'],
    ['newer','result-hash:fit','unreadable'],
    ['kind','unheld'],
    ['missing-record','missing-opening'],
  ].map((points,i) => orderVector('branches',i,points));
  // A link outside the cover determines closure even with a covered form
  // missing. A held source record outside it still contributes to taken.
  const joint = cases.at(-1), forms = joint.input.checker;
  const late = structuredClone(forms[0]);
  late.id = 'r:' + 'f'.repeat(51) + 'g';
  forms.push(late);
  forms[2].under = late.id;
  warmRust();
  const rows = await batchVectors(cases,{rust:true});
  for (const [i,{lean,rust}] of rows.entries()) {
    assert.ok(lean.expect, lean.error);
    assert.deepEqual(lean.expect,rust.expect);
    for (const [k,v] of Object.entries(cases[i].fuzz.expected)) assert.deepEqual(lean.expect[k],v);
  }
  assert.equal(rows.at(-1).lean.expect.closure,'no');
  assert.equal(rows.at(-1).lean.expect['taken-not-covered'],1);
});

test('slot errors follow the spec while named entries follow their input order', async () => {
  const {orderVector} = await import('./tick-order.mjs');
  function payloadFor(points) {
    const {input} = orderVector('member-order',0,points);
    return input.payload ?? input.subject.payload;
  }
  const slots = payloadFor(['value:result.a.unread','value:result.e.value']);
  slots.result = Object.fromEntries(Object.entries(slots.result).reverse());
  const keys = payloadFor(['list:result.c.things.t:one.keys.m','list:result.c.things.t:one.keys.n']);
  const thing = keys.result.c.things['t:one'];
  thing.keys = Object.fromEntries(Object.entries(thing.keys).reverse());
  // Reordering object members preserves both hashes.
  const vectors = [slots,keys].map(payload => ({layer:'tick',input:{op:'payload',payload}}));
  warmRust();
  const rows = await batchVectors(vectors,{rust:true});
  for (const [i,path] of ['result.a.unread','result.c.things.t:one.keys.n'].entries()) {
    const {lean,rust} = rows[i];
    assert.deepEqual(lean.expect,{refused:'tick-result-invalid',path});
    assert.deepEqual(lean.expect,rust.expect);
  }
});

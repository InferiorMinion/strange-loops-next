// Rust-only evaluator layer. The JS witness supplies a second set of IDs, not fold answers.
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readdir, readFile, writeFile } from 'node:fs/promises';
import { registerHooks } from 'node:module';
import { join, resolve } from 'node:path';
import { parseSpec } from '../js/spec.mjs';
import { ROOT, executeBatch, inputText, warmRust } from './transport.mjs';
import { sliceCases } from './slice-cases.mjs';
import { classes, coverage } from './slice-coverage.mjs';

// The frozen test defines slice and cover privately. Import a separate module
// instance with two extra exports and suppress only that instance's test
// registrations. Its helpers and all their dependencies run verbatim.
async function witness() {
  const url = new URL('../js/spec-slice.test.mjs?slice-witness', import.meta.url).href;
  const hook = registerHooks({
    resolve(name, context, next) {
      if (context.parentURL === url && name === 'node:test') {
        return { url: 'data:text/javascript,export default () => {}', shortCircuit: true };
      }
      return next(name, context);
    },
    load(name, context, next) {
      const loaded = next(name, context);
      return name === url ? { ...loaded, source: Buffer.from(loaded.source).toString() + '\nexport { slice, cover };' } : loaded;
    },
  });
  try { return await import(url); } finally { hook.deregister(); }
}

function assess(vector, result, reference, summary) {
  assert.ok(result.expect, result.error);
  const { exact, downward } = result.expect;
  const { spec, orders } = vector.input;
  const { parsed } = parseSpec(spec);
  const forms = orders[0];
  const keyed = Object.values(parsed.fold).some(s => s.follow === 'keys');
  const part = keyed ? reference.cover(parsed, forms) : reference.slice(parsed, forms).part;
  const js = [...new Set(part.map(r => r.id))].sort();
  assert.ok(js.every(id => exact.includes(id)), 'JS witness outside Rust Exact: ' + JSON.stringify({ js, exact }));
  if (js.length === exact.length) summary.jsAgreements++;
  else summary.jsDifferences++;
  const size = new Set(forms.map(r => r.id)).size;
  if (exact.length < size) summary.properExact++;
  if (downward.length < size) summary.properDownward++;
  for (const name of coverage(parsed, forms, result.expect)) summary.drops[name]++;
  summary.cases++;
  summary.orders += orders.length;
  if (forms.length > size) summary.duplicateCases++;
  summary.modes[vector.fuzz.mode]++;
}

export async function sliceCampaign(options) {
  warmRust();
  const out = options.out ?? await mkdtemp(join(ROOT, 'rust/target/fuzz-slice-'));
  await mkdir(out, { recursive: true });
  if ((await readdir(out)).length) throw Error('artifact directory is not empty: ' + out);
  const start = performance.now();
  const reference = await witness();
  const summary = { seed: options.seed, cases: 0, orders: 0, duplicateCases: 0, properExact: 0,
    properDownward: 0, jsAgreements: 0, jsDifferences: 0, mismatches: 0, skippedDraws: 0,
    modes: { plain: 0, mixed: 0, keyed: 0 }, drops: Object.fromEntries(classes.map(k => [k, 0])) };
  let replay;
  if (options.replay) replay = await readFile(options.replay, 'utf8');
  if (options.sliceCase) {
    const [draw, step] = options.sliceCase.split(':').map(Number);
    const one = sliceCases(options.seed, draw)[step];
    assert.ok(one, `no slice case ${options.sliceCase} for seed ${options.seed}`);
    replay = inputText(one);
  }
  const length = replay ? 1 : options.n;
  campaign: for (let offset = 0; offset < length; offset += options.chunkSize) {
    const directory = join(out, 'inputs', String(offset));
    await mkdir(directory, { recursive: true });
    const files = [], vectors = [];
    for (let i = offset; i < Math.min(length, offset + options.chunkSize); i++) {
      const cases = replay ? [JSON.parse(replay)] : sliceCases(options.seed, i);
      if (!cases.length) summary.skippedDraws++;
      for (const c of cases) {
        const wire = replay || inputText(c);
        const vector = JSON.parse(wire);
        assert.equal(vector.layer, 'slice');
        if (replay) summary.seed = vector.fuzz.seed;
        const file = resolve(directory, i + '-' + vector.fuzz.step + '.json');
        await writeFile(file, wire);
        vectors.push(vector);
        files.push(file);
      }
    }
    const results = executeBatch(directory, files, { engine: 'rust', timeoutMs: options.timeoutMs });
    for (const [i, vector] of vectors.entries()) {
      try {
        assess(vector, results[i], reference, summary);
      } catch (error) {
        summary.mismatches++;
        const failure = { ...vector.fuzz, case: vector.fuzz.draw + ':' + vector.fuzz.step,
          error: error.message, spec: vector.input.spec,
          orders: vector.input.orders, rust: results[i], replay: files[i] };
        await writeFile(join(out, 'mismatch.json'), JSON.stringify(failure, null, 2) + '\n');
        console.error(JSON.stringify(failure));
        break campaign;
      }
    }
  }
  if (!replay && !summary.mismatches) {
    for (const [name, count] of Object.entries({ ...summary.drops, ...summary.modes,
      properExact: summary.properExact, properDownward: summary.properDownward, duplicateCases: summary.duplicateCases })) {
      assert.ok(count > 0, `slice seed ${options.seed} n ${options.n} has zero cases for ${name}`);
    }
  }
  summary.seconds = (performance.now() - start) / 1000;
  await writeFile(join(out, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
  return { out, summary };
}

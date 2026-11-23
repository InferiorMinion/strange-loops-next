#!/usr/bin/env node
import { mkdir, mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
import { FAMILIES, generate } from './generate.mjs';
import { isDeepStrictEqual } from 'node:util';
import { excludedPairs, orderPairs } from './tick-order.mjs';
import { finishCheck, generateTick } from './tick.mjs';
import { assess } from './evaluate.mjs';
import { batchVectors, warmLean, warmRust } from './transport.mjs';
import { shrink } from './shrink.mjs';

function number(text, flag, min = 1) {
  const n = Number(text);
  if (!Number.isSafeInteger(n) || n < min) throw new Error(`${flag} needs a safe integer >= ${min}`);
  return n;
}
export function parseArgs(args) {
  const options = { seed: '1', n: 70, chunkSize: 64, timeoutMs: 120000, maxTrials: 80, noShrink: false, rust: false };
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--no-shrink') { options.noShrink = true; continue; }
    if (arg === '--rust') { options.rust = true; continue; }
    if (!['--seed', '--n', '--family', '--out', '--replay', '--chunk-size', '--timeout-ms', '--shrink-trials', '--case'].includes(arg)) {
      throw new Error(`unknown option: ${arg}`);
    }
    if (i + 1 === args.length || args[i + 1].startsWith('--')) throw new Error(`missing value: ${arg}`);
    const value = args[++i];
    if (arg === '--seed') options.seed = value;
    if (arg === '--family') options.family = value;
    if (arg === '--out') options.out = resolve(value);
    if (arg === '--replay') options.replay = resolve(value);
    if (arg === '--case') options.sliceCase = value;
    if (arg === '--n') options.n = number(value, arg);
    if (arg === '--chunk-size') options.chunkSize = number(value, arg);
    if (arg === '--timeout-ms') options.timeoutMs = number(value, arg);
    if (arg === '--shrink-trials') options.maxTrials = number(value, arg, 0);
  }
  if (options.family && !['tick', 'slice'].includes(options.family) && !FAMILIES.includes(options.family)) throw new Error(`unknown family: ${options.family}`);
  if (options.sliceCase && (options.family !== 'slice' || options.replay || !/^\d+:\d+$/.test(options.sliceCase))) {
    throw new Error('--case takes draw:step with --family slice, separately from --replay');
  }
  // The slice campaign also requires positive coverage of rare graph relations.
  if (options.family === 'slice') {
    if (!args.includes('--n')) options.n = 384;
    if (!args.includes('--seed')) options.seed = 'slice-ci-1';
  }
  if (options.chunkSize > 256) throw new Error('--chunk-size must be <= 256');
  if (options.rust && options.noShrink) throw new Error('--rust requires shrinking; --no-shrink is two-way only');
  options.implementations = engines(options);
  return options;
}

function engines(options) {
  if (options.family === 'slice') return ['rust', 'js-witness'];
  if (options.family === 'tick') {
    if (options.rust) throw new Error('--family tick already selects Lean and Rust; omit --rust');
    return ['lean', 'rust'];
  }
  return options.rust ? ['js', 'lean', 'rust'] : ['js', 'lean'];
}

function increment(counts, key) {
  counts[key] = (counts[key] ?? 0) + 1;
}

function countTick(summary, vector, answer) {
  const verdict = answer.check ?? answer;
  let outcome = 'payload';
  if (verdict.refused) {
    outcome = 'refused';
    increment(summary.refusals, verdict.refused);
  } else if (verdict.answer) {
    outcome = verdict.reason ?? verdict.answer;
    if (verdict.today) increment(summary.today, verdict.today['not-available'] ?? verdict.today);
  } else if (verdict['other-rules']) outcome = 'other-rules';
  else if (verdict['cannot-read']) outcome = 'cannot-read';
  else if (vector.input.op === 'members') outcome = 'members';
  increment(summary.operations[vector.input.op] ??= {}, outcome);
  if (vector.input.op === 'check-tick' && ['same','differs'].includes(outcome)) {
    for (const type of new Set(Object.values(vector.input.subject.payload.spec.fold).map(slot =>
      ['count','sum','min','max','collect','latest-by'].find(k => k in slot)))) {
      increment(summary.replayFolds[outcome] ??= {},type);
    }
  }
  if (vector.fuzz?.faults) increment(summary.faultCounts, vector.fuzz.faults.length);
}

// Planted pairs include the two later sites of a triple. Only a visible
// winner earns confirmation. A newer version disables the result fit check.
function countPrecedence(summary, vector, verdict, disagreement) {
  const {points, expected, winner} = vector.fuzz;
  if (!points) return;
  const confirmed = Object.entries(expected).every(([k,v]) => isDeepStrictEqual(verdict[k],v));
  for (const [i,a] of points.entries()) {
    for (const b of points.slice(i+1)) {
      const counts = summary.pairs[a + ' / ' + b];
      counts[0]++;
      const fitSkipped = a === 'result-hash:fit' && b === 'newer' && winner === b;
      if (confirmed && (winner === a || fitSkipped)) counts[1]++;
    }
  }
  if (!confirmed && !disagreement) {
    throw Error('invalid precedence witness ' + JSON.stringify({points,expected,verdict,index:vector.fuzz.index}));
  }
}

export async function persistMismatch(out, index, artifact) {
  await mkdir(join(out, 'mismatches'), { recursive: true });
  const file = join(out, 'mismatches', `${String(index).padStart(6, '0')}.json`);
  await writeFile(file, JSON.stringify(artifact, (_key, value) =>
    typeof value === 'bigint' ? { $integer: value.toString() } : value, 2) + '\n');
  return file;
}

export async function campaign(options, {
  js, warm = true, batch = batchVectors, buildLean = warmLean, buildRust = warmRust,
} = {}) {
  if (options.rust && options.noShrink) throw new Error('--rust requires shrinking; --no-shrink is two-way only');
  if (options.family === 'slice') {
    const { sliceCampaign } = await import('./slice-run.mjs');
    return sliceCampaign(options);
  }
  const implementations = engines(options), tick = options.family === 'tick';
  const rust = implementations.includes('rust');
  const out = options.out ?? await mkdtemp(join(tmpdir(), 'loops-fuzz-run-'));
  await mkdir(out, { recursive: true });
  const entries = await readdir(out);
  if (entries.length) throw new Error(`artifact directory is not empty: ${out}`);
  if (warm) {
    buildLean();
    if (rust) buildRust();
  }
  const start = performance.now();
  const families = options.family ? [options.family] : FAMILIES;
  let replay, rawText;
  if (options.replay) {
    let content;
    try { content = JSON.parse(await readFile(options.replay, 'utf8')); }
    catch (error) { throw new Error(`replay ${options.replay}: ${error.message}`, { cause: error }); }
    // A checked-in fixture also has a "vector" member: its name is a string.
    const artifact = content.vector && typeof content.vector === 'object';
    replay = artifact ? content.vector : content;
    if (tick && replay.layer !== 'tick') throw new Error('--family tick replay needs a tick vector');
    rawText = content.rawText ?? (artifact ? undefined : await readFile(options.replay, 'utf8'));
  }
  const summary = { implementations, seed: String(options.seed), requested: replay ? 1 : options.n, counts: {},
    mismatches: 0, decoderRefusals: 0, protocolRefusals: 0, chunks: 0, shrinkTrials: 0, seconds: 0 };
  if (tick) Object.assign(summary, { operations: {}, refusals: {}, today: {}, faultCounts: {}, replayFolds:{},
    pairs:Object.fromEntries(orderPairs.map(p => [p.join(' / '),[0,0]])), excludedPairs });
  await writeFile(join(out, 'options.json'), JSON.stringify({ ...options, implementations }, null, 2) + '\n');
  const diagnostics = [];
  let failureIndex = 0;
  for (let offset = 0; offset < summary.requested; offset += options.chunkSize) {
    const vectors = Array.from({ length: Math.min(options.chunkSize, summary.requested - offset) }, (_, j) => {
      const i = offset + j;
      if (tick) return replay ?? generateTick(options.seed, i);
      return replay ?? generate(options.seed, Math.floor(i / families.length), families[i % families.length]);
    });
    if (tick && !replay) {
      const recipes = vectors.filter(v => v.fuzz.prepare === 'check-tick');
      if (recipes.length) {
        const written = await batch(recipes, {timeoutMs:options.timeoutMs});
        for (const [i,row] of written.entries()) {
          if (!row.expect?.payload) throw Error('model writer: ' + JSON.stringify(row));
          finishCheck(recipes[i],row.expect.payload);
        }
      }
    }
    // A replay preserves raw spec member order and repeated members.
    const directory = join(out, 'inputs', String(summary.chunks).padStart(5, '0'));
    const answers = await batch(vectors, { directory, timeoutMs: options.timeoutMs, rawText, rust });
    if (answers.length !== vectors.length) throw new Error('campaign transport loses vectors');
    summary.chunks++;
    for (let j = 0; j < vectors.length; j++) {
      const vector = vectors[j], result = assess(vector, answers[j], { js, rawText, tick });
      const family = vector.fuzz?.family ?? vector.layer.split('/')[0];
      const counts = summary.counts[family] ??= { cases: 0, match: 0, 'protocol-refusal': 0, 'decoder-refusal': 0, mismatch: 0 };
      counts.cases++; counts[result.status]++;
      if (tick && !replay && result.status === 'decoder-refusal') {
        throw new Error(`malformed generated tick at seed ${options.seed} index ${offset + j}: ${result.diagnostic}; input in ${directory}`);
      }
      if (tick && !replay && result.lean) countPrecedence(summary,vector,result.lean,result.status === 'mismatch');
      if (tick && ['match', 'protocol-refusal'].includes(result.status)) countTick(summary, vector, result.lean);
      if (result.status === 'decoder-refusal') {
        summary.decoderRefusals++;
        diagnostics.push({ case: offset + j, vector: vector.fuzz, layer: vector.layer,
          diagnostic: result.diagnostic, diagnostics: result.diagnostics });
      }
      if (result.status === 'protocol-refusal') summary.protocolRefusals++;
      if (result.status !== 'mismatch') continue;
      summary.mismatches++;
      const artifact = { implementations, vector, rawText, answer: result,
        note: 'Unclassified disagreement. Parent review decides its cause.' };
      // Persist the raw case before any reduction or transport failure.
      const index = failureIndex++;
      await persistMismatch(out, index, artifact);
      if (tick) console.error(JSON.stringify({ seed: vector.fuzz?.seed ?? options.seed, case: vector.fuzz?.index ?? offset + j, vector, answers: result.responses },
        (_key, v) => typeof v === 'bigint' ? v.toString() : v));
      if (!options.noShrink && !rawText) {
        artifact.shrink = await shrink(vector, result, async candidates => {
          const responses = await batch(candidates, { timeoutMs: options.timeoutMs, rust });
          if (responses.length !== candidates.length) throw new Error('shrink transport loses candidates');
          return candidates.map((v, i) => assess(v, responses[i], { js, tick }));
        }, { maxTrials: options.maxTrials });
        summary.shrinkTrials += artifact.shrink.trials;
        await persistMismatch(out, index, artifact);
      }
    }
    summary.seconds = (performance.now() - start) / 1000;
    await writeFile(join(out, 'decoder-refusals.json'), JSON.stringify(diagnostics, null, 2) + '\n');
    await writeFile(join(out, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
  }
  return { out, summary };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const { out, summary } = await campaign(parseArgs(process.argv.slice(2)));
    process.stdout.write(JSON.stringify({ out, ...summary }, null, 2) + '\n');
    process.exitCode = summary.mismatches ? 1 : 0;
  } catch (error) { console.error(error.stack); process.exitCode = 2; }
}

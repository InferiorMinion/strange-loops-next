#!/usr/bin/env node
// Joins the result files of a sharded run (`run.mjs --shard i/n`) into one and
// checks that together they cover the catalogue exactly once, or with `--only`
// the entries it names.
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { realpathSync } from 'node:fs';
import path from 'node:path';

const cataloguePath = path.join(path.dirname(fileURLToPath(import.meta.url)), 'mutants.json');

// Returns the merged result and the problems that make the run incomplete.
// A survivor is an outcome, not a problem, as in the runner.
export function merge(ids, shards) {
  const problems = [], seen = new Map(), mutants = [];
  shards.forEach(({ name, result }) => {
    if (result.error) problems.push(`${name}: ${result.error}`);
    if (result.restoreError) problems.push(`${name}: restore failed: ${result.restoreError}`);
    if (result.finalBaseline?.status !== 'ok') problems.push(`${name}: the final baseline build is ${result.finalBaseline?.status ?? 'missing'}`);
    for (const m of result.mutants ?? []) {
      if (seen.has(m.id)) problems.push(`${m.id}: in both ${seen.get(m.id)} and ${name}`);
      else { seen.set(m.id, name); mutants.push({ ...m, shard: name }); }
      if (m.status === 'error') problems.push(`${m.id}: error (${m.failureDetail ?? 'no detail'})`);
    }
  });
  for (const id of ids) if (!seen.has(id)) problems.push(`${id}: in no result`);
  for (const id of seen.keys()) if (!ids.includes(id)) problems.push(`${id}: not in the catalogue`);
  const order = new Map(ids.map((id, at) => [id, at]));
  mutants.sort((a, b) => (order.get(a.id) ?? ids.length) - (order.get(b.id) ?? ids.length));
  const counts = {};
  for (const m of mutants) counts[m.status] = (counts[m.status] ?? 0) + 1;
  return { merged: { catalogue: ids.length, counts, mutants }, problems };
}

// The entries a run was asked for, in catalogue order: the whole catalogue, or
// the ones `--only` named, as `run.mjs --only` selects them.
export function selection(ids, only) {
  if (!only) return ids;
  if (new Set(only).size !== only.length || only.some(id => !ids.includes(id))) throw new Error('Unknown or duplicate mutant id');
  return ids.filter(id => only.includes(id));
}

export async function main(args) {
  const files = [], named = {};
  for (let i = 0; i < args.length; i++) {
    if (!['--output', '--only'].includes(args[i])) { files.push(args[i]); continue; }
    if (!args[i + 1] || args[i + 1].startsWith('--')) throw new Error(`Missing value for ${args[i]}`);
    named[args[i]] = args[++i];
  }
  const output = named['--output'] ?? null;
  if (!files.length) throw new Error('Usage: merge.mjs [--output merged.json] [--only id,id] results-1.json results-2.json ...');
  const ids = selection(JSON.parse(await readFile(cataloguePath, 'utf8')).map(m => m.id), named['--only']?.split(','));
  const shards = [];
  for (const file of files) shards.push({ name: path.basename(file), result: JSON.parse(await readFile(file, 'utf8')) });
  const { merged, problems } = merge(ids, shards);
  if (output) await writeFile(output, `${JSON.stringify(merged, null, 2)}\n`);
  console.log(`${merged.mutants.length} of ${merged.catalogue} entries: ${Object.entries(merged.counts).map(([k, n]) => `${n} ${k}`).join(', ')}`);
  for (const m of merged.mutants.filter(m => m.status === 'survived')) console.log(`survived: ${m.id} (${m.file}:${m.line})`);
  for (const problem of problems) console.error(`problem: ${problem}`);
  return problems.length ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) {
  main(process.argv.slice(2)).then(code => { process.exitCode = code; }).catch(e => { console.error(e.message); process.exitCode = 1; });
}

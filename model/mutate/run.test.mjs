import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, copyFile, chmod, rm } from 'node:fs/promises';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { changedPaths, replaceUnique, command, sourcePath } from './run.mjs';
import { merge, selection } from './merge.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));

test('catalogue keeps 328 unique mutations at current source sites; listing does not mutate', async () => {
  const catalogue = JSON.parse(await readFile(path.join(here, 'mutants.json'), 'utf8'));
  assert.equal(catalogue.length, 328);
  assert.ok(!catalogue.some(m => ['finish-occurred-stamp-of', 'wide-time-094'].includes(m.id)));
  for (const id of ["wide-families-013","wide-spec-064","extra-families-max-acc","extra-families-min-acc","finish-json-hexdigit","last-spec-tests-conds"]) {
    assert.ok(catalogue.some(m => m.id === id), id);
  }
  assert.equal(new Set(catalogue.map(m => m.id)).size, catalogue.length);
  const sources = new Map();
  for (const m of catalogue) {
    const file = path.resolve(here, '..', sourcePath(m.file).slice('model/'.length));
    if (!sources.has(file)) sources.set(file, await readFile(file, 'utf8'));
    assert.notEqual(m.before, m.after, m.id);
    assert.doesNotThrow(() => replaceUnique(sources.get(file), m.before, m.after), m.id);
  }
  const result = spawnSync(process.execPath, [path.join(here, 'run.mjs'), '--list'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim().split('\n').length, catalogue.length);
  for (const [file, before] of sources) assert.equal(await readFile(file, 'utf8'), before, file);
});

test('shards are disjoint, cover the selection in catalogue order, and refuse a bad index', async () => {
  const list = (...args) => spawnSync(process.execPath, [path.join(here, 'run.mjs'), '--list', ...args], { encoding: 'utf8' });
  const ids = r => r.stdout.trim().split('\n').map(line => line.split('\t')[0]);
  const all = ids(list());
  const shards = [1, 2, 3].map(i => ids(list('--shard', `${i}/3`)));
  // Every third entry: the sizes differ by at most one, whatever the catalogue holds.
  assert.deepEqual(shards.map(s => s.length), [0, 1, 2].map(i => Math.ceil((all.length - i) / 3)));
  assert.deepEqual(all.map((_, at) => shards[at % 3][Math.floor(at / 3)]), all);
  assert.deepEqual(ids(list('--shard', '1/1')), all);
  const only = all.slice(0, 5);
  assert.deepEqual(ids(list('--only', only.join(','), '--shard', '2/2')), [only[1], only[3]]);
  for (const bad of ['0/3', '4/3', '1', '1/0', 'a/b', '1/3/5']) {
    const r = list('--shard', bad);
    assert.equal(r.status, 1, bad);
    assert.match(r.stderr, /Shard must be/, bad);
  }
});

test('merging shard results needs every entry once, no error and a restored baseline', () => {
  const ok = { status: 'ok' };
  const shard = (name, entries, more = {}) => ({ name, result: { mutants: entries.map(([id, status]) => ({ id, status })), finalBaseline: ok, ...more } });
  const whole = merge(['a', 'b', 'c'], [shard('1', [['c', 'survived'], ['a', 'killed-by-vector']]), shard('2', [['b', 'rejected-by-build']])]);
  assert.deepEqual(whole.problems, []);
  assert.deepEqual(whole.merged.mutants.map(m => m.id), ['a', 'b', 'c']);
  assert.deepEqual(whole.merged.counts, { 'killed-by-vector': 1, 'rejected-by-build': 1, survived: 1 });
  assert.equal(Object.hasOwn(whole.merged, 'runtime'), false);
  const judged = (id, verdict) => ({ id, status: 'rejected-by-build', runtime: { verdict } });
  const second = merge(['a', 'b', 'c'], [{ name: '1', result: { mutants: [judged('a', 'vector'), judged('b', 'none'), { id: 'c', status: 'rejected-by-build' }], finalBaseline: ok } }]);
  assert.deepEqual(second.problems, []);
  assert.deepEqual(second.merged.runtime, { vector: 1, none: 1 });
  assert.deepEqual(second.merged.proofsAlone, ['b']);
  const problems = (...shards) => merge(['a', 'b'], shards).problems;
  assert.match(problems(shard('1', [['a', 'survived']]))[0], /^b: in no result/);
  assert.match(problems(shard('1', [['a', 'survived'], ['b', 'survived']]), shard('2', [['b', 'survived']]))[0], /^b: in both 1 and 2/);
  assert.match(problems(shard('1', [['a', 'survived'], ['b', 'survived'], ['z', 'survived']]))[0], /^z: not in the catalogue/);
  assert.match(problems(shard('1', [['a', 'error'], ['b', 'survived']]))[0], /^a: error/);
  assert.match(problems(shard('1', [['a', 'survived'], ['b', 'survived']], { finalBaseline: null }))[0], /final baseline build is missing/);
  assert.match(problems(shard('1', [['a', 'survived'], ['b', 'survived']], { error: 'Baseline build failed' }))[0], /Baseline build failed/);
});

test('merging a run of named entries needs those entries and no others', () => {
  assert.deepEqual(selection(['a', 'b', 'c'], undefined), ['a', 'b', 'c']);
  assert.deepEqual(selection(['a', 'b', 'c'], ['c', 'a']), ['a', 'c']);
  assert.throws(() => selection(['a', 'b'], ['a', 'z']), /Unknown or duplicate/);
  assert.throws(() => selection(['a', 'b'], ['a', 'a']), /Unknown or duplicate/);
  const shard = (entries) => ({ name: '1', result: { mutants: entries.map(id => ({ id, status: 'survived' })), finalBaseline: { status: 'ok' } } });
  assert.deepEqual(merge(selection(['a', 'b', 'c'], ['c', 'a']), [shard(['a', 'c'])]).problems, []);
  assert.match(merge(selection(['a', 'b', 'c'], ['c', 'a']), [shard(['a', 'b', 'c'])]).problems[0], /^b: not in the catalogue/);
});

test('catalogue source roots exclude traversal and unrelated files', () => {
  assert.equal(sourcePath('Time.lean'), 'model/Loops/Time.lean');
  assert.equal(sourcePath('Loops/Read.lean'), 'model/Loops/Read.lean');
  for (const file of ['Gen/Read.lean', '../Gen/Read.lean', '/Time.lean', 'Laws/Time.lean', 'Gen/../Read.lean']) {
    assert.throws(() => sourcePath(file), /Invalid mutant source/);
  }
});

test('replacement requires one exact site and reports its line', () => {
  assert.deepEqual(replaceUnique('a\nb\nc', 'b', 'x'), { text: 'a\nx\nc', line: 2 });
  assert.throws(() => replaceUnique('abc', 'z', 'x'), /not unique/);
  assert.throws(() => replaceUnique('abcabc', 'abc', 'x'), /not unique/);
});

test('recursive vector comparison detects paths and bytes', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'mutation-test-'));
  try {
    const a = path.join(dir, 'a'), b = path.join(dir, 'b');
    await mkdir(path.join(a, 'nested'), { recursive: true });
    await mkdir(path.join(b, 'nested'), { recursive: true });
    await writeFile(path.join(a, 'nested', 'same'), 'same');
    await writeFile(path.join(b, 'nested', 'same'), 'same');
    await writeFile(path.join(a, 'old'), 'old');
    await writeFile(path.join(b, 'new'), 'new');
    assert.deepEqual(await changedPaths(a, b), ['new', 'old']);
    await writeFile(path.join(b, 'nested', 'same'), 'changed');
    assert.deepEqual(await changedPaths(a, b), ['nested/same', 'new', 'old']);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('command reports success, failure, spawn failure, and timeout', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'mutation-command-'));
  try {
    const log = path.join(dir, 'out.log');
    assert.equal((await command(process.execPath, ['-e', 'console.log("ok")'], 5, log)).status, 'ok');
    assert.equal((await command(process.execPath, ['-e', 'process.exit(2)'], 5, log)).status, 'nonzero');
    assert.equal((await command('/no-such-executable', [], 5, log)).status, 'spawn-error');
    assert.equal((await command(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], 0.1, log)).status, 'timeout');
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('command kills a descendant ignoring SIGTERM with closed stdio', async () => {
  const dir = await mkdtemp(path.join(tmpdir(), 'mutation-tree-'));
  try {
    const marker = path.join(dir, 'escaped');
    const script = `const {spawn}=require('child_process'); spawn(process.execPath,['-e',${JSON.stringify(`process.on('SIGTERM',()=>{}); setTimeout(()=>require('fs').writeFileSync(${JSON.stringify(marker)},'alive'),800)`)}],{stdio:'ignore'}); setTimeout(()=>{},10000)`;
    assert.equal((await command(process.execPath, ['-e', script], 0.2, path.join(dir, 'log'))).status, 'timeout');
    await new Promise(resolve => setTimeout(resolve, 900));
    await assert.rejects(readFile(marker), { code: 'ENOENT' });
  } finally { await rm(dir, { recursive: true, force: true }); }
});

async function fixture() {
  const dir = await mkdtemp(path.join(tmpdir(), 'mutation-repo-'));
  for (const sub of ['model/mutate', 'model/Loops', 'vectors', 'bin']) await mkdir(path.join(dir, sub), { recursive: true });
  await copyFile(path.join(here, 'run.mjs'), path.join(dir, 'model/mutate/run.mjs'));
  await copyFile(path.join(here, 'mutants.json'), path.join(dir, 'model/mutate/mutants.json'));
  const mutants = ['syntax', 'type', 'vector', 'survive', 'genfail', 'timeout', 'signal', 'pvector', 'pgen', 'prun', 'pchecks'].map(id => ({ id, file: 'Test.lean', before: 'ORIGINAL', after: id, reason: id }));
  await writeFile(path.join(dir, 'model/mutate/mutants.json'), JSON.stringify(mutants));
  await writeFile(path.join(dir, 'model/Loops/Test.lean'), 'ORIGINAL\n');
  await writeFile(path.join(dir, 'vectors/data'), 'baseline');
  const lake = path.join(dir, 'bin/lake');
  // A proof fails for `type` and the `p…` mutants: `lake build --wfail` exits 1 and the generator still builds.
  // `syntax` breaks every build. Then `pvector` changes a vector, `pgen` fails the generator, `prun` fails `run --check`,
  // `pchecks` fails `checks`, and `type` passes them all.
  await writeFile(lake, `#!${process.execPath}\nconst fs=require('fs'); const s=fs.readFileSync('Loops/Test.lean','utf8').trim(); const args=process.argv.slice(2); const proof=['type','pvector','pgen','prun','pchecks'].includes(s); if(args[0]==='build') { if(s==='syntax'||(proof&&args.includes('--wfail'))) { console.error(s+' error'); process.exit(1); } if(s==='timeout'||s==='signal') { fs.writeFileSync(${JSON.stringify(path.join(dir, 'ready'))},s); setInterval(()=>{},1000); } } else if(args[1]==='gen') { if(s==='genfail'||s==='pgen') process.exit(2); fs.writeFileSync(args[2]+'/data',s==='vector'||s==='pvector'?'changed':'baseline'); } else if(args[1]==='run') { if(s==='prun') process.exit(3); } else if(args[1]==='checks') { if(s==='pchecks') process.exit(4); } else process.exit(9);\n`);
  await chmod(lake, 0o755);
  for (const args of [['init', '-q'], ['config', 'user.email', 'test@example.com'], ['config', 'user.name', 'Test'], ['add', '.'], ['commit', '-qm', 'fixture']]) {
    const r = spawnSync('git', args, { cwd: dir, encoding: 'utf8' }); assert.equal(r.status, 0, r.stderr);
  }
  return dir;
}

function runFixture(dir, args) {
  const child = spawn(process.execPath, [path.join(dir, 'model/mutate/run.mjs'), '--output', path.join(dir, 'result.json'), ...args], { cwd: dir, env: { ...process.env, PATH: `${path.join(dir, 'bin')}:${process.env.PATH}` }, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = ''; child.stdout.on('data', c => output += c); child.stderr.on('data', c => output += c);
  const done = new Promise((resolve, reject) => { child.on('error', reject); child.on('close', code => resolve({ code, output })); });
  return { child, done };
}

test('synthetic repository exercises mutation outcomes and restoration', async () => {
  const dir = await fixture();
  try {
    const result = await runFixture(dir, ['--only', 'syntax,type,vector,survive,genfail,timeout', '--timeout', '0.4']).done;
    assert.equal(result.code, 1, result.output);
    const report = JSON.parse(await readFile(path.join(dir, 'result.json')));
    assert.deepEqual(report.mutants.map(m => m.status), ['rejected-by-build', 'rejected-by-build', 'killed-by-vector', 'survived', 'error', 'error']);
    for (const entry of report.mutants.slice(0, 2)) {
      assert.equal(entry.buildRejectionNeedsAudit, true);
      assert.equal(Object.hasOwn(entry, 'proofKillNeedsAudit'), false);
      assert.equal(entry.commands[0].status, 'nonzero');
      assert.match(await readFile(entry.commands[0].log, 'utf8'), /^(syntax|type) error/);
    }
    // The second verdict sits beside the status and its commands stay out of `commands`.
    assert.deepEqual(report.mutants.map(m => m.runtime?.verdict), ['unbuilt', 'none', undefined, undefined, undefined, undefined]);
    assert.deepEqual(report.mutants.slice(0, 2).map(m => m.commands.length), [1, 1]);
    assert.deepEqual(report.mutants[1].runtime.commands.map(c => c.command.slice(1, 3).join(' ')), ['build gen', 'exe gen', 'exe run', 'exe checks']);
    assert.equal(report.mutants[4].commands[1].status, 'nonzero');
    assert.equal(report.mutants[5].commands[0].status, 'timeout');
    assert.equal(report.finalBaseline.status, 'ok');
    assert.equal(await readFile(path.join(dir, 'model/Loops/Test.lean'), 'utf8'), 'ORIGINAL\n');
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('a build rejection gets a second verdict from what runs, and its status stays', async () => {
  const dir = await fixture();
  try {
    const result = await runFixture(dir, ['--only', 'pvector,pgen,prun,pchecks', '--timeout', '5']).done;
    assert.equal(result.code, 0, result.output);
    const report = JSON.parse(await readFile(path.join(dir, 'result.json')));
    assert.deepEqual(report.mutants.map(m => m.status), Array(4).fill('rejected-by-build'));
    assert.deepEqual(report.mutants.map(m => m.runtime.verdict), ['vector', 'generator', 'run-check', 'checks']);
    assert.deepEqual(report.mutants[0].runtime.changedPaths, ['data']);
    assert.deepEqual(report.mutants.map(m => m.changedPaths), [[], [], [], []]);
    assert.match(result.output, /pvector: rejected-by-build \(at run time: vector\)/);
    const first = await runFixture(dir, ['--only', 'pvector', '--no-second-verdict']).done;
    assert.equal(first.code, 0, first.output);
    assert.equal(Object.hasOwn(JSON.parse(await readFile(path.join(dir, 'result.json'))).mutants[0], 'runtime'), false);
    assert.equal(await readFile(path.join(dir, 'model/Loops/Test.lean'), 'utf8'), 'ORIGINAL\n');
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('a second verdict whose step times out is unknown, and the entry is still a build rejection', async () => {
  const dir = await fixture();
  try {
    // `prun` passes the generator; a `run` that hangs is made by turning the fake `run` step into the build's hang.
    const lake = path.join(dir, 'bin/lake');
    await writeFile(lake, (await readFile(lake, 'utf8')).replace("if(s==='prun') process.exit(3);", "if(s==='prun') setInterval(()=>{},1000);"));
    const result = await runFixture(dir, ['--only', 'prun', '--timeout', '0.6']).done;
    const report = JSON.parse(await readFile(path.join(dir, 'result.json')));
    assert.equal(report.mutants[0].status, 'rejected-by-build');
    assert.equal(report.mutants[0].runtime.verdict, 'unknown');
    assert.equal(report.mutants[0].runtime.commands.at(-1).status, 'timeout');
    assert.equal(result.code, 0, result.output);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('SIGTERM restores source and removes lock', async () => {
  const dir = await fixture();
  try {
    const run = runFixture(dir, ['--only', 'signal', '--timeout', '10']);
    let ready = false;
    for (let i = 0; i < 200; i++) { try { if (await readFile(path.join(dir, 'ready'), 'utf8') === 'signal') { ready = true; break; } } catch {} await new Promise(resolve => setTimeout(resolve, 25)); }
    assert.ok(ready, 'mutant did not start');
    run.child.kill('SIGTERM');
    const result = await run.done; assert.equal(result.code, 143, result.output);
    assert.equal(await readFile(path.join(dir, 'model/Loops/Test.lean'), 'utf8'), 'ORIGINAL\n');
    await assert.rejects(readFile(path.join(dir, 'model/.mutation-run.lock')), { code: 'ENOENT' });
    const report = JSON.parse(await readFile(path.join(dir, 'result.json')));
    assert.equal(report.mutants[0].commands[0].status, 'interrupted');
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('dirty and staged candidate files are refused without changing bytes', async () => {
  for (const staged of [false, true]) {
    const dir = await fixture();
    try {
      const source = path.join(dir, 'model/Loops/Test.lean');
      await writeFile(source, 'ORIGINAL\nlocal changes\n');
      if (staged) assert.equal(spawnSync('git', ['add', 'model/Loops/Test.lean'], { cwd: dir }).status, 0);
      const before = await readFile(source);
      const result = await runFixture(dir, ['--only', 'syntax']).done;
      assert.equal(result.code, 1, result.output); assert.match(result.output, /dirty/);
      assert.deepEqual(await readFile(source), before);
    } finally { await rm(dir, { recursive: true, force: true }); }
  }
});

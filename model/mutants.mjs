import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const model = dirname(fileURLToPath(import.meta.url));
const root = dirname(model);
// Both runners use the same replacement sites. This runner builds only runtime checks.
const catalogue = JSON.parse(readFileSync(join(model, "mutate/mutants.json"), "utf8"));
const guardMutants = ["step1-base32-reencode","step1-fraction-final-zero","step1-seconds-bound","step1-zone-only-z","step1-envelope-keys-remove","step1-envelope-keys-after-value","step1-draft-keys-after-values","step1-record-keys-after-values","d1-base-problem-remove","d1-read-text-first","d2-lone-low-accepted","d2-lone-high-accepted","d2-low-surrogate-range","d2-high-surrogate-range","wide-families-013","wide-spec-064","extra-families-max-acc","extra-families-min-acc",
  "record-num-drop-int",
  "wide-record-049",
  "by-unreadable-ungrouped",
  "payload-nonobject-field",
  "payload-drop-other-field",
  "payload-whole-as-number",
  "payload-fraction-member",
  "payload-flatten-nested",
  "payload-list-first-match",
  "payload-empty-absent",
  "payload-other-absent",
  "payload-list-atleast-any",
  "payload-list-below-any",
  "payload-list-first-key",
  "payload-list-repeat-key",
  "payload-list-first-number",
  "payload-keys-union-is"].map(id => {
  const m = catalogue.find(m => m.id === id);
  if (!m) throw new Error(`missing guard mutant: ${id}`);
  return [m.id, m.file.includes("/") ? m.file : `Loops/${m.file}`, m.before, m.after];
});
const mutants = [
  ["typed about", "Gen/Read.lean", 'if typedRef s then pure s else bad p "expected a typed reference"', 'pure s'],
  ["typed of", "Gen/Read.lean", 'if typedRef s then pure s else bad (dot path "of") "expected a typed reference"', 'pure s'],
  ["lens validity", "Loops/Lens.lean", 'if Valid d then some d else none', 'some d'],
  ["integer bound", "Loops/Record.lean", 'if -(2^53 - 1) ≤ i ∧ i ≤ 2^53 - 1 then some i else none', 'some i'],
  ["fraction as zero", "Loops/Record.lean", '    | _ => none\n  else none\n\n/-- Withhold', '    | some (.one (.number _)) => some 0\n    | _ => none\n  else none\n\n/-- Withhold'],
  ["global heads", "Loops/Fold.lean", 'orderHeads ((perThing xs).flatMap fun ts => ts.2.heads)', '(LatestBy.fold .empty xs).heads'],
  // D2, D4 and D5 at the record adapter gen and run share.
  ["text refusal before values", "Gen/Records.lean", 'if let some why := payloadTextRefusal kvs then', 'if let some why := (none : Option String) then'],
  ["text refusal after shape", "Gen/Records.lean", '  if let .error why := shaped then if shapeCodes.contains why then throw why\n', ''],
  ["text refusal needs JSON", "Gen/Read.lean", '  | .ok _, .error _ => some notUnicode\n  | _, _ => none', '  | _, .error (what, _) => if what == notUnicode then some what else none\n  | _, _ => none'],
  ["text refusal reads the text", "Gen/Read.lean", '  | .ok _, .error _ => some notUnicode\n  | _, _ => none', '  | _, _ => none'],
  ["seed of 32 bytes", "Gen/Records.lean", '    if seed.length != 32 then bad "input.seed" s!"holds {seed.length} bytes; a signing seed is exactly 32"\n', ''],
  ["seed after writing", "Gen/Records.lean", '  let salt := Loops.Read.salt (kvs.lookup "salt")\n', '  let salt := Loops.Read.salt (kvs.lookup "salt")\n  let _ ← bytes "input.seed" (← req "input" kvs "seed")\n'],
  ["later envelope variant agrees", "Gen/Records.lean", 'if idOf H e\' (r.commit H) != id then', 'if false then'],
  ["later payload variant writable", "Gen/Records.lean", 'writeProblem e salt p then', 'writeProblem e salt payload then'],
  ["first variant decides", "Gen/Records.lean", '  | .error why => pure (.obj [("refused", .str why)])\n  | .ok w =>\n    let r := w.stored\n    let id := r.id H', '  | .error why => pure (.obj [("refused", .str ((firstRefusal ((envs.filterMap (·.problem)) ++ [why])).getD why))])\n  | .ok w =>\n    let r := w.stored\n    let id := r.id H'],
  ...guardMutants,
];
const work = mkdtempSync(join(tmpdir(), "loops-model-mutants-"));
const copy = join(work, "model");
function run(args) {
  const r = spawnSync("lake", args, { cwd: copy, encoding: "utf8", timeout: 240000, maxBuffer: 8 * 1024 * 1024 });
  if (r.error || r.signal) throw new Error(`lake ${args.join(" ")}: ${r.error?.message ?? r.signal}`);
  return r;
}
function success(r, label) {
  if (r.status !== 0) throw new Error(`${label} failed (${r.status}):\n${r.stdout}\n${r.stderr}`);
}
try {
  cpSync(model, copy, { recursive: true });
  success(run(["build", "checks"]), "baseline build");
  success(run(["exe", "checks", root]), "baseline checks");
  let killed = 0;
  for (const [name, file, from, to] of mutants) {
    const path = join(copy, file);
    const original = readFileSync(path, "utf8");
    if (original.split(from).length !== 2) throw new Error(`mutant is not unique: ${name}`);
    try {
      writeFileSync(path, original.replace(from, to));
      success(run(["build", "checks"]), `${name} build`);
      const result = run(["exe", "checks", root]);
      if (result.status === 0) throw new Error(`mutant survives: ${name}`);
      console.log(`killed ${name}: ${(result.stderr + result.stdout).trim()}`);
      killed++;
    } finally {
      writeFileSync(path, original);
    }
  }
  console.log(`${killed} of ${mutants.length} mutants killed`);
} finally {
  rmSync(work, { recursive: true, force: true });
}

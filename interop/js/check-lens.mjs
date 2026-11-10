// Run every lens vector against lens.mjs.
// Usage: node check-lens.mjs <vectors dir>

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readVector } from "../answers.mjs";
import * as Lens from "./lens.mjs";
import { tally, jsonFiles, runAlone } from "./tally.mjs";

export function check(dir) {
  const t = tally();
  for (const file of jsonFiles(join(dir, "lens", "section"))) {
    const v = readVector(readFileSync(file, "utf8"));
    const n = `lens/section/${v.vector}`;
    const { base, text } = v.input;
    t.files++;
    // A named refusal is the base's, given before the text is read.
    if (typeof v.expect.refused === "string") {
      t.expect(`${n} base refused`, Lens.edit(base, text), v.expect);
      continue;
    }
    const parsed = Lens.parse(text);
    if (v.expect.refused) {
      t.expect(`${n} refused`, Lens.edit(base, text), { parsed: null, refused: true });
      continue;
    }
    if (parsed === null) {
      t.failures.push(`${n} parsed: unexpected null`);
      continue;
    }
    t.expect(`${n} parsed`, parsed, v.expect.parsed);
    t.expect(`${n} normalized`, Lens.render(parsed), v.expect.normalized);
    const outcomes = Lens.diff(base, parsed);
    t.expect(`${n} outcomes`, outcomes, v.expect.outcomes);
    t.expect(`${n} applied`, Lens.apply(base, outcomes), v.expect.applied);
    // The public boundary gives the same answer whole.
    t.expect(`${n} edit`, Lens.edit(base, text), v.expect);
    // The laws the vectors illustrate, held over the vector's own documents.
    t.expect(`${n} round trip`, Lens.parse(Lens.render(parsed)), parsed);
    t.expect(`${n} base round trip`, Lens.parse(Lens.render(base)), base);
    t.expect(`${n} no-op`, Lens.diff(base, base), []);
  }
  return t;
}

runAlone(import.meta.url, check);

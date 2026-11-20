// Run every result vector against result.mjs: each arrival order run by
// spec.mjs, then written as the canonical result and hashed. Each canonical
// text is also read back: JSON with no number in it, an integer text exactly
// at `value`, `unread` and `ungrouped` (or null at `value`), and ids in lists.
// Usage: node check-result.mjs <vectors dir>

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readVector } from "../answers.mjs";
import * as S from "./spec.mjs";
import * as R from "./result.mjs";
import { canonical } from "./loops.mjs";
import { tally, jsonFiles, runAlone } from "./tally.mjs";

// A vector's input.spec as written, member order and all.
const specNode = (file) => S.readJson(readFileSync(file, "utf8")).members
  .find(([k]) => k === "input")[1].members.find(([k]) => k === "spec")[1];

const INTEGER_AT = new Set(["value", "unread", "ungrouped"]);
const ID = /^r:[0-9a-v]{52}$/;

// Every problem with where the canonical text puts what: no JSON number
// anywhere, an integer text at each integer position and nowhere else a value
// could be one, and every list a list of ids.
function positions(v, key = null, out = []) {
  if (typeof v === "number") out.push(`a JSON number at ${key}`);
  else if (Array.isArray(v)) {
    for (const x of v) if (typeof x !== "string" || !ID.test(x)) out.push(`a list at ${key} holds ${JSON.stringify(x)}`);
  } else if (v !== null && typeof v === "object") {
    for (const [k, x] of Object.entries(v)) positions(x, k, out);
  } else if (INTEGER_AT.has(key)) {
    if (!(key === "value" && v === null) && R.intOf(v) === null) out.push(`${key} holds ${JSON.stringify(v)}`);
  }
  return out;
}

export function check(dir) {
  const t = tally();
  for (const file of jsonFiles(join(dir, "result"))) {
    const v = readVector(readFileSync(file, "utf8"));
    const n = `${v.layer}/${v.vector}`;
    t.files++;
    const { parsed, refused } = S.parseNode(specNode(file));
    if (!parsed) { t.failures.push(`${n}: spec refused, ${refused}`); continue; }
    if (!v.input.orders.length) t.failures.push(`${n}: no orders`);
    for (const [i, rs] of v.input.orders.entries()) {
      let got;
      try {
        const out = S.runSpec(parsed, rs);
        got = { canonical: R.resultText(out), hash: R.resultHash(out) };
      } catch (e) { got = `threw ${e.message}`; }
      t.expect(`${n} order ${i}`, got, v.expect);
    }
    // The text as a third implementer reads it: JCS of what it parses to.
    const text = v.expect.canonical;
    t.expect(`${n} canonical text is JCS of itself`, canonical(JSON.parse(text)), text);
    t.expect(`${n} canonical positions`, positions(JSON.parse(text)), []);
  }
  return t;
}

runAlone(import.meta.url, check);

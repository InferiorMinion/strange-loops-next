// Run every spec vector against spec.mjs: spec/parse (the parsed form, or the
// refusal's code and path) and spec/run (every slot, in every arrival order).
// Usage: node check-spec.mjs <vectors dir>

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readVector } from "../answers.mjs";
import * as S from "./spec.mjs";
import { tally, jsonFiles, runAlone } from "./tally.mjs";

// A vector's input.spec as written: readJson keeps member order and repeats,
// which JSON.parse would not.
const specNode = (file) => S.readJson(readFileSync(file, "utf8")).members
  .find(([k]) => k === "input")[1].members.find(([k]) => k === "spec")[1];
// A vector's expect as written, members in the order the vector writes them.
const expectNode = (file) => S.readJson(readFileSync(file, "utf8")).members.find(([k]) => k === "expect")[1];

// The member names of every object, in order, at every depth: what a
// structural comparison cannot see.
const order = (n) => S.isObject(n) ? n.members.map(([k, x]) => [k, order(x)])
  : S.isArray(n) ? n.items.map(order) : null;

export function check(dir) {
  const t = tally();

  for (const file of jsonFiles(join(dir, "spec", "parse"))) {
    const v = readVector(readFileSync(file, "utf8"));
    const n = `${v.layer}/${v.vector}`;
    t.files++;
    // A spec that repeats a key comes as text too: read that one.
    const got = v.input.spec_text !== undefined ? S.parseSpecText(v.input.spec_text) : S.parseNode(specNode(file));
    t.expect(n, got, v.expect);
    // spec.parse: an accepted spec's parsed form parses back to itself.
    if (got.parsed) t.expect(`${n} reparses`, S.parseSpec(got.parsed), { parsed: got.parsed });
  }

  for (const file of jsonFiles(join(dir, "spec", "run"))) {
    const v = readVector(readFileSync(file, "utf8"));
    const n = `${v.layer}/${v.vector}`;
    t.files++;
    const { parsed, refused } = S.parseNode(specNode(file));
    if (!parsed) { t.failures.push(`${n}: spec refused, ${refused}`); continue; }
    if (!v.input.orders.length) t.failures.push(`${n}: no orders`);
    const want = order(expectNode(file));
    for (const [i, rs] of v.input.orders.entries()) {
      let got;
      try { got = S.runSpec(parsed, rs); } catch (e) { got = `threw ${e.message}`; }
      t.expect(`${n} order ${i}`, got, v.expect);
      // and written in the model's member order, keyed groups and things included
      if (typeof got !== "string") t.expect(`${n} order ${i} member order`, order(S.readJson(S.slotsText(got))), want);
    }
  }
  return t;
}

runAlone(import.meta.url, check);

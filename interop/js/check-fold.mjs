// Run every fold vector against fold.mjs, folding each arrival order in
// input.orders. Usage: node check-fold.mjs <vectors dir>

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { readVector } from "../answers.mjs";
import * as F from "./fold.mjs";
import * as S from "./spec.mjs";
import { tally, jsonFiles, runAlone } from "./tally.mjs";

// latest-by answers a map from things: written as `slotsText` writes a slot's
// `things`, its member order is checked against the vector's, as check-spec
// checks spec/run.
const KEYED = new Set(["fold/latest-by"]);
const order = (n) => S.isObject(n) ? n.members.map(([k, x]) => [k, order(x)])
  : S.isArray(n) ? n.items.map(order) : null;
const expectNode = (file) => S.readJson(readFileSync(file, "utf8")).members.find(([k]) => k === "expect")[1];

// What each family's vector expects, from one arrival order.
const RUN = {
  "fold/count": (rs) => F.count(rs).value,
  "fold/sum": (rs) => F.sum(rs).value,
  "fold/extrema": (rs) => ({ min: F.min(rs).value, max: F.max(rs).value }),
  "fold/latest-by": (rs) => F.latestBy(rs),
  "fold/latest": (rs) => F.latest(rs),
  "fold/heads": (rs) => ({ max: F.maxOfHeads(rs), "limit-2": F.heads(rs, { limit: 2 }).map((r) => r.id) }),
  "fold/view": (rs) => ({ heads: F.view(rs), max: F.maxOfHeads(rs) }),
  "fold/select": (rs, v) => ({ count: F.count(rs, v.input), sum: F.sum(rs, v.input) }),
};

export function check(dir) {
  const t = tally();
  for (const family of readdirSync(join(dir, "fold")).sort()) {
    for (const file of jsonFiles(join(dir, "fold", family))) {
      const v = readVector(readFileSync(file, "utf8"));
      t.files++;
      const n = `${v.layer}/${v.vector}`;
      if (!RUN[v.layer]) {
        t.failures.push(`${n}: no runner for ${v.layer}`);
        continue;
      }
      if (!v.input.orders.length) t.failures.push(`${n}: no orders`);
      const want = KEYED.has(v.layer) ? order(expectNode(file)) : null;
      for (const [i, rs] of v.input.orders.entries()) {
        let got;
        try { got = RUN[v.layer](rs, v); } catch (e) { got = `threw ${e.message}`; }
        t.expect(`${n} order ${i}`, got, v.expect);
        if (want && typeof got !== "string") t.expect(`${n} order ${i} member order`, order(S.readJson(S.jsonText(got, true))), want);
      }
    }
  }
  return t;
}

runAlone(import.meta.url, check);

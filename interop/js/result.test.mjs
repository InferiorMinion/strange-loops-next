// The canonical result (D7): regressions for the codec in result.mjs, each a
// way a port could go wrong that a vector alone might not name.

import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { base32 } from "./loops.mjs";
import { runSpec, parseSpec } from "./spec.mjs";
import { intText, intOf, resultText, resultHash, canonicalSlot, RESULT_LABEL } from "./result.mjs";

const id = (n) => `r:${base32(Uint8Array.from({ length: 32 }, () => n))}`;
const task = (n, of, fields = {}, prev = []) => ({
  id: id(n), kind: "task", observer: "sl:kyle", of, prev, emitted: `2020-01-01T00:00:0${n}Z`, value: "", fields,
});
const specOf = (fold) => parseSpec({ vertex: "x", from: [{ store: "." }], fold }).parsed;

test("a sum past 2^53 keeps every digit", () => {
  const spec = specOf({ total: { sum: "est" } });
  const out = runSpec(spec, [task(1, "task:1", { est: 9007199254740991 }), task(2, "task:2", { est: 2 })]);
  assert.equal(resultText(out), '{"total":{"unread":"0","value":"9007199254740993"}}');
  // JSON.stringify of the Number would have rounded it.
  assert.notEqual(JSON.stringify(Number(out.total.value)), "9007199254740993");
});

test("an integer is written only when it is exact", () => {
  assert.equal(intText(0), "0");
  assert.equal(intText(-0), "0");
  assert.equal(intText(-12), "-12");
  assert.equal(intText(-18014398509481984n), "-18014398509481984");
  assert.throws(() => intText(2 ** 53), /not an exact integer/);
  assert.throws(() => intText(1.5), /not an exact integer/);
  assert.throws(() => intText("3"), /not an exact integer/);
  assert.throws(() => canonicalSlot({ value: 2 ** 60, unread: 0 }), /not an exact integer/);
});

test("the reader takes exactly the texts the writer writes", () => {
  for (const n of [0n, 7n, -7n, 10n, 9007199254740993n, -9007199254740993n]) assert.equal(intOf(intText(n)), n);
  for (const t of ["-0", "007", "+1", "1e3", " 1", "1 ", "", "-", "0x10", "١"]) assert.equal(intOf(t), null, t);
});

test("min and max that read nothing are null, not zero", () => {
  const out = runSpec(specOf({ lo: { min: "est" }, hi: { max: "est" } }), []);
  assert.equal(resultText(out), '{"hi":{"unread":"0","value":null},"lo":{"unread":"0","value":null}}');
});

test("shown is not in the bytes", () => {
  const spec = specOf({ current: { "latest-by": "of", follow: "prev" }, every: { collect: true } });
  const a = task(1, "task:1", { status: "open" });
  const b = task(2, "task:1", { status: "closed" }, [a.id]);
  const { value, fields, ...envelope } = b;
  const withheld = { ...envelope, shown: "withheld" };
  const held = runSpec(spec, [a, b]), hidden = runSpec(spec, [a, withheld]);
  assert.notDeepEqual(held, hidden);
  assert.equal(resultText(held), resultText(hidden));
  assert.equal(resultHash(held), resultHash(hidden));
  assert.ok(!resultText(held).includes("shown"));
});

test("a list keeps its order and a map is in UTF-16 order", () => {
  const slot = { heads: [{ id: id(2), shown: "here" }, { id: id(1), shown: "erased" }], unread: 0 };
  assert.equal(resultText({ s: slot }), `{"s":{"heads":["${id(2)}","${id(1)}"],"unread":"0"}}`);
  const groups = { groups: { 2: { value: 1, unread: 0 }, 10: { value: 1, unread: 0 }, "": { value: 1, unread: 0 } }, unread: 0, ungrouped: 0 };
  assert.equal(resultText({ g: groups }),
    '{"g":{"groups":{"":{"unread":"0","value":"1"},"10":{"unread":"0","value":"1"},"2":{"unread":"0","value":"1"}},"ungrouped":"0","unread":"0"}}');
});

test("the hash frames the canonical bytes with the codec's version", () => {
  const out = { n: { value: 3, unread: 0 } };
  const text = '{"n":{"unread":"0","value":"3"}}';
  assert.equal(resultText(out), text);
  assert.equal(RESULT_LABEL, "loops/result/v1:");
  const want = "h:" + base32(createHash("sha256").update(Buffer.from(RESULT_LABEL + text, "utf8")).digest());
  assert.equal(resultHash(out), want);
  // Unframed bytes are another hash.
  assert.notEqual(resultHash(out), "h:" + base32(createHash("sha256").update(Buffer.from(text, "utf8")).digest()));
});

// A changeset of drafts whose about, of, prev and under may name each other by
// @label, resolved to real ids, written with loops.mjs's writer so the ids
// match layer 0's. Written from vectors/batch and protocol/; no dependencies.

import {
  canonical, envJson, idOf, commitOf, checkEnvelopeShapes, notObject, readEnv, readSalt, seal,
  unicodeProblem, NOT_UNICODE,
} from "./loops.mjs";

const isLabel = (s) => typeof s === "string" && s.startsWith("@");
const labelsOf = (d) => [...(d.env.about ?? []), d.env.of, ...(d.env.prev ?? []), d.env.under].filter(isLabel);

// What writing refuses, in the order protocol/record.md checks it.
const WRITE_CODES = [
  "kind-empty", "observer-not-typed", "emitted-not-rfc3339", "occurred-not-span",
  "about-not-typed", "of-not-typed", "prev-not-record-id", "under-not-record-id",
  "erases-not-record-id", "of-missing", "salt-not-16-bytes", "member-duplicate", "number-out-of-range",
];

// A record id that names nothing, standing in for a label while a draft is
// checked on its own: only its shape matters there.
const STAND_IN = "r:" + "0".repeat(52);
const NO_KEY = new Uint8Array(32);

const withRefs = (env, ref) => {
  const out = { ...env };
  for (const k of ["about", "prev"]) if (env[k]) out[k] = env[k].map((s) => (isLabel(s) ? ref(s) : s));
  for (const k of ["of", "under"]) if (isLabel(env[k])) out[k] = ref(env[k]);
  return out;
};

// The code writing the draft gives, or null. Erases takes no label: one there
// is simply not a record id.
function writeProblem(d) {
  try {
    seal(withRefs(d.env, () => STAND_IN), d.salt, d.payload, NO_KEY);
    return null;
  } catch (e) {
    if (!WRITE_CODES.includes(e.message)) throw e;
    return e.message;
  }
}

// A label cycle: a draft reaching itself through labels in about, of, prev or under.
function hasCycle(drafts, byLabel) {
  const state = new Map(); // draft -> "open" | "done"
  const visit = (d) => {
    if (state.get(d) === "done") return false;
    if (state.get(d) === "open") return true;
    state.set(d, "open");
    const cyclic = labelsOf(d).some((l) => visit(byLabel.get(l)));
    state.set(d, "done");
    return cyclic;
  };
  return drafts.some(visit);
}

// A record's thing: its `of`, as written, or without `of` its own id.
const thingOf = (id, of) => of ?? id;

// A root has no `of` and no `prev`; it starts the thing its id names.
const isRoot = (h) => h.of === undefined && !h.prev.length;

// The batch's records, one per draft in the order given, each { id,
// env_canonical }; or { refused } with the first stage that refuses: shape,
// lone surrogates, labels (duplicate, unknown, cycle), writing from kind-empty, then
// prev-unknown, of-not-root, then prev-unrelated (vectors/README.md, Batch).
// Each stage looks at the whole batch, so no draft order changes the answer.
// A draft that is not an object, has a label that is not text or has no
// payload is malformed input, which throws rather than refuses.
export function resolveBatch(raw, held) {
  // The library checks every envelope's shape before labels or values: an
  // unknown key in any, then any that is not an object or is left out.
  const objects = raw.filter((d) => !notObject(d));
  try { checkEnvelopeShapes(objects.map((d) => (Object.hasOwn(d, "env") ? d.env : null))); }
  catch (e) { if (e.message.startsWith("envelope-")) return { refused: e.message }; throw e; }
  if (objects.length !== raw.length) throw new Error("batch input: a draft is not an object");
  for (const d of raw) {
    if (d.label !== undefined && typeof d.label !== "string") throw new Error("batch input: a label is not text");
    if (!Object.hasOwn(d, "payload")) throw new Error("batch input: a draft has no payload");
  }
  // Then every string: a lone surrogate anywhere in the changeset or in what
  // is held refuses before a label is compared or a draft written.
  if (unicodeProblem(raw) || unicodeProblem(held)) return { refused: NOT_UNICODE };
  // A field left out or of the wrong type, and a salt that is not base32hex
  // text, read so that writing refuses them with their own codes.
  const drafts = raw.map((d) => ({ ...d, env: readEnv(d.env), salt: readSalt(d.salt) }));
  const byLabel = new Map();
  for (const d of drafts) {
    if (d.label === undefined) continue;
    if (byLabel.has(d.label)) return { refused: "label-duplicate" };
    byLabel.set(d.label, d);
  }
  if (drafts.some((d) => labelsOf(d).some((l) => !byLabel.has(l)))) return { refused: "label-unknown" };
  if (hasCycle(drafts, byLabel)) return { refused: "label-cycle" };

  const problems = drafts.map(writeProblem).filter(Boolean);
  if (problems.length) return { refused: problems.sort((a, b) => WRITE_CODES.indexOf(a) - WRITE_CODES.indexOf(b))[0] };

  // Acyclic, so every label's record can be written once what it names is.
  const written = new Map();
  const write = (d) => {
    if (!written.has(d)) {
      const env = withRefs(d.env, (l) => write(byLabel.get(l)).id);
      written.set(d, { id: idOf(env, commitOf(d.salt, d.payload)), env });
    }
    return written.get(d);
  };
  const records = drafts.map(write);

  // What the writer holds of each record, in the batch or the store: its `of`
  // and its `prev`. A `held` entry is { id, of?, prev }.
  const holds = new Map(held.map((h) => [h.id, { of: h.of, prev: h.prev ?? [] }]));
  for (const r of records) holds.set(r.id, { of: r.env.of, prev: r.env.prev ?? [] });
  const prevs = records.flatMap((r) => (r.env.prev ?? []).map((p) => [r, p]));
  if (prevs.some(([, p]) => !holds.has(p))) return { refused: "prev-unknown" };
  // An `of` naming a held record names a root; one the writer doesn't hold is not judged.
  if (records.some((r) => holds.has(r.env.of) && !isRoot(holds.get(r.env.of)))) return { refused: "of-not-root" };
  // Succession is within a thing: each `prev` target is of the record's thing.
  if (prevs.some(([r, p]) => thingOf(p, holds.get(p).of) !== thingOf(r.id, r.env.of))) return { refused: "prev-unrelated" };

  return { records: records.map((r) => ({ id: r.id, env_canonical: canonical(envJson(r.env)) })) };
}

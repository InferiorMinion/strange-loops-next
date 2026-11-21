import { inRange } from "./loops.mjs";

// The reader projection of a stored opening (Loops.payloadFields).
const nameable = v => typeof v === "string" || typeof v === "boolean" || Number.isSafeInteger(v);

export function readHeld(v) {
  if (Array.isArray(v)) return { many: v.filter(nameable) };
  if (v === null || typeof v === "object") return { other: true };
  return { one: v };
}

export function payloadFields(payload) {
  if (payload === null || typeof payload !== "object" || Array.isArray(payload)) return {};
  return Object.fromEntries(Object.entries(payload).map(([name, v]) => [name, readHeld(v)]));
}

// The tagged scalar format of payload-family answers.
function scalar(v) {
  if (typeof v === "string") return { text: v };
  if (typeof v === "boolean") return { bool: v };
  return Number.isSafeInteger(v) ? { whole: String(v) } : { number: String(v) };
}

export function payloadAnswer(payload) {
  if (!inRange(payload)) throw new Error("input.payload: number-out-of-range");
  const fields = Object.entries(payloadFields(payload)).map(([k, h]) => {
    if (Object.hasOwn(h, "one")) return [k, { one: scalar(h.one) }];
    if (h.many) return [k, { many: h.many.map(scalar) }];
    return [k, h];
  });
  return { fields: Object.fromEntries(fields) };
}

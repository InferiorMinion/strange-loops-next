import { createHash } from 'node:crypto';
import { base32 } from './generate.mjs';

// All generated scalars are well-formed strings, finite small numbers, booleans or null.
// Sorted keys produce the JCS hash preimage. A selected key can occur twice on the wire.
export function render(value, repeat, sorted = false) {
  return renderPaths(value, repeat ? [repeat] : [], sorted);
}

function renderPaths(value, paths, sorted) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  const children = k => paths.filter(p => p[0] === k).map(p => p.slice(1));
  if (Array.isArray(value)) {
    return '[' + value.map((v,i) => renderPaths(v,children(i),sorted)).join(',') + ']';
  }
  const keys = Object.keys(value);
  if (sorted) keys.sort();
  const pairs = [];
  for (const key of keys) {
    const tails = children(key);
    const pair = JSON.stringify(key) + ':' + renderPaths(value[key], tails, sorted);
    pairs.push(pair);
    if (tails.some(t => t.length === 0)) pairs.push(pair);
  }
  return '{' + pairs.join(',') + '}';
}

export function repeatedTickText(vector) {
  return renderPaths(vector, vector.fuzz.repeats ?? [vector.fuzz.repeat], false);
}

export function digest(prefix, label, value) {
  return prefix + base32(createHash('sha256').update(label).update(render(value, undefined, true)).digest());
}
export const specHash = p => digest('spec:', 'loops/spec-hash/v' + p.rules + ':', p.spec);
export const resultHash = result => digest('res:', 'loops/result-hash/v1:', result);

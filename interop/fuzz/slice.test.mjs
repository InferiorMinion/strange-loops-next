import test from 'node:test';
import assert from 'node:assert/strict';
import { coverage } from './slice-coverage.mjs';

const spec = { from: [{ store: '.', kind: ['firing'] }], fold: { runs: { count: true } } };

test('a class earns a drop only when the Rust selection loses an ID with every other reason retained', () => {
  // The second target of the retirement is also a taken firing. Removing
  // successorTarget cannot remove either firing, but removing successor loses c.
  const forms = [
    { id: 'a', kind: 'firing' }, { id: 'b', kind: 'firing' },
    { id: 'c', kind: 'retired', prev: ['a', 'b'], under: 'd' },
    { id: 'd', kind: 'grant', prev: ['e'] }, { id: 'e', kind: 'grant' },
    { id: 'f', kind: 'yard' },
  ];
  const exact = ['a', 'b', 'c'], downward = [...exact, 'd', 'e'];
  assert.deepEqual(coverage(spec, forms, { exact, downward }), ['successor', 'downPrev', 'downUnder']);
  assert.throws(() => coverage(spec, forms, { exact: [...downward, 'f'], downward }),
    /exact IDs from the law/);
});

test('an eraser owed by both ordinary and keyed coverage cannot earn either omission', () => {
  const forms = [
    { id: 'a', kind: 'firing', of: 'kiln:east' }, { id: 'b', kind: 'note', erases: ['a'] },
  ];
  const exact = ['a', 'b'];
  const keyed = { ...spec, fold: { runs: { count: true, follow: 'keys' } } };
  assert.deepEqual(coverage(keyed, forms, { exact, downward: exact }), []);
});

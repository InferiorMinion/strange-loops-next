// Each vector describes one spec and one holding in all its arrival orders.
import { generate, random, spec as generateSpec } from './generate.mjs';
import { formOf } from '../js/scenario.mjs';

export function sliceCases(seed, draw) {
  const family = ['spec', 'scenario', 'realm'][draw % 3];
  const round = Math.floor(draw / 3);
  const index = family === 'spec' ? 2 * round + 1 : round; // odd indices are spec/run
  const { input, fuzz } = generate(seed, index, family);
  if (fuzz.nearMiss) return []; // decoder and grammar refusal inputs are tested in their own families
  const rng = random(seed, index, 'slice-' + family);
  let spec = input.spec;
  let holdings;
  if (family === 'spec') {
    holdings = [input.orders];
  } else if (family === 'scenario') {
    const records = new Map(input.records.map(r => [r.id, r]));
    holdings = input.steps.map(step => {
      const forms = new Map(step.forms.map(f => [f.id, formOf(records.get(f.id), f.shown)]));
      return [[...forms.values()], ...step.orders.map(order => order.map(id => forms.get(id)))];
    });
  } else {
    const forms = input.forms;
    spec = generateSpec(rng, forms);
    // The realm family supplies the forms (including bare copies) and tests every rotation.
    // A kind-restricted source can distinguish two records of the same thing.
    spec.from = [{ store: '.', kind: [rng.pick(forms).kind] }];
    holdings = [forms.map((_, i) => [...forms.slice(i), ...forms.slice(0, i)])];
  }
  const mode = ['mixed', 'keyed', 'plain'][round % 3];
  if (mode !== 'mixed') spec = { ...spec, fold: Object.fromEntries(Object.entries(spec.fold)
    .filter(([, slot]) => (slot.follow === 'keys') === (mode === 'keyed'))) };
  return holdings.map((orders, step) => ({
    layer: 'slice', input: { spec, orders },
    fuzz: { family, seed, index, draw, step, mode },
  }));
}

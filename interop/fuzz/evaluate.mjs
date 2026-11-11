// Adapters ask the public JS APIs the same questions as Gen.Run.
import { jsonSourceAt } from '../json-source.mjs';
import { equal } from '../answers.mjs';
import * as F from '../js/fold.mjs';
import * as L from '../js/loops.mjs';
import * as S from '../js/spec.mjs';
import * as Lens from '../js/lens.mjs';
import { realm } from '../js/realm.mjs';
import { resolveBatch } from '../js/batch.mjs';
import { formOf, step, lensEdit } from '../js/scenario.mjs';

const WRITE_CODES = new Set([
  'kind-empty', 'observer-not-typed', 'emitted-not-rfc3339', 'occurred-not-span',
  'about-not-typed', 'of-not-typed', 'prev-not-record-id', 'under-not-record-id', 'erases-not-record-id',
  'of-missing', 'salt-not-16-bytes', 'member-duplicate', 'number-out-of-range',
  'string-not-unicode', 'envelope-field-unknown', 'envelope-not-object',
]);
const folds = {
  'fold/count': rs => F.count(rs).value,
  'fold/sum': rs => F.sum(rs).value,
  'fold/extrema': rs => ({ min: F.min(rs).value, max: F.max(rs).value }),
  'fold/latest-by': rs => F.latestBy(rs),
  'fold/latest': rs => F.latest(rs),
  'fold/heads': rs => ({ max: F.max(F.heads(rs)), 'limit-2': F.heads(rs).slice(0, 2).map(r => r.id) }),
  'fold/view': rs => ({ heads: F.heads(rs).map(r => ({ id: r.id, shown: r.shown ?? 'here' })), max: F.max(F.heads(rs)) }),
  'fold/select': (rs, input) => ({ count: F.count(rs, input), sum: F.sum(rs, input) }),
};

// Whether an object node, as `S.readJson` reads one, repeats a member name.
const repeats = (node) => {
  const names = (node?.members ?? []).map(([key]) => key);
  return new Set(names).size !== names.length;
};

// The raw vector keeps spec member order, including integer-like member names.
function specNode(vector, rawText) {
  if (vector.input.spec_text !== undefined) return S.readJson(vector.input.spec_text);
  if (!rawText) return S.nodeOf(vector.input.spec);
  const input = S.readJson(rawText).members.find(([k]) => k === 'input')[1];
  return input.members.find(([k]) => k === 'spec')[1];
}
// The public base boundary answers in the vectors' own form.
const lensAnswer = ({ base, text }) => Lens.edit(base, text);
// The record input contract is the library's: the first envelope and first
// payload decide, later variants must agree once those are written, and the
// seed is read last. Text refusals and raw shape are answers; any other error
// is a fault in the input, which Lean reports as a decoder refusal.
function recordAnswer(input, payloadTexts) {
  const env = input.envs[0];
  const fromText = input.payload_text !== undefined || payloadTexts !== undefined;
  let r, payload;
  try {
    r = L.writeRecord(input, { writer: fromText ? "writeText" : "seal", payloadTexts });
    payload = input.payload_text !== undefined ? L.parse(input.payload_text)
      : payloadTexts ? L.parse(payloadTexts[0]) : input.payloads[0];
  } catch (error) {
    if (!WRITE_CODES.has(error.message)) throw error;
    return { refused: error.message };
  }
  return {
    env_canonical: L.canonical(L.envJson(env)), payload_canonical: L.canonical(payload),
    commit: r.commit, id_preimage: L.idPreimage(env, r.commit), id: r.id,
    signing_input: L.signingInput(r.id), key: r.sig.key, ed25519: r.sig.ed25519,
    line: r.line, erased_line: r.erasedLine,
  };
}

// One answer per arrival order exposes disagreement in any listed order.
export function evaluateJS(vector, rawText) {
  const { layer, input } = vector;
  if (folds[layer]) return input.orders.map(rs => folds[layer](rs, input));
  switch (layer) {
    case 'byte-text': return [{ accepted: input.texts.map(text => {
      try { L.fixedBytes(text, input.prefix, input.bytes); return true; }
      catch { return false; }
    }) }];
    case 'record': {
      // One answer, as Lean gives: the variants are checked, not answered.
      // The raw text keeps each payload's repeated members for the library.
      const payloadTexts = rawText !== undefined && input.payload_text === undefined && Array.isArray(input.payloads)
        ? input.payloads.map((_, i) => jsonSourceAt(rawText, ['input', 'payloads', i])) : undefined;
      // A repeated envelope member is malformed input, after raw shape and a
      // text payload's lone surrogate, as in Lean. A value cannot hold it.
      if (rawText !== undefined) {
        const envs = S.readJson(jsonSourceAt(rawText, ['input', 'envs']));
        if ((envs.items ?? []).some(repeats)) {
          try { L.checkEnvelopeShapes(input.envs); }
          catch (error) { if (WRITE_CODES.has(error.message)) return [{ refused: error.message }]; throw error; }
          if (typeof input.payload_text === 'string') {
            try { L.parse(input.payload_text); }
            catch (error) { if (error.message === L.NOT_UNICODE) return [{ refused: error.message }]; }
          }
          throw new Error('record input: an envelope repeats a member');
        }
      }
      return [recordAnswer(input, payloadTexts)];
    }
    case 'lens/section': return [lensAnswer(input)];
    case 'spec/parse': return [S.parseNode(specNode(vector, rawText))];
    case 'spec/run': {
      const parsed = S.parseNode(specNode(vector, rawText));
      if (!parsed.parsed) return [parsed];
      // Both spec readers admit their input. Settlement belongs to realm/scenario.
      return input.orders.map(rs => S.runSpec(parsed.parsed, rs));
    }
    case 'batch': {
      // JSON.parse keeps only the last repeated env; shape checks need every
      // occurrence. A draft object with none has an envelope that is not an object.
      if (rawText !== undefined) {
        const drafts = S.readJson(jsonSourceAt(rawText, ['input', 'drafts']));
        const envs = drafts.items.flatMap(draft => {
          if (!draft?.members) return [];
          const found = draft.members.filter(([key]) => key === 'env')
            .map(([, env]) => env?.members ? Object.fromEntries(env.members.map(([key]) => [key, null])) : null);
          return found.length ? found : [null];
        });
        try { L.checkEnvelopeShapes(envs); }
        catch (error) {
          if (error.message.startsWith('envelope-')) return [{ refused: error.message }];
          throw error;
        }
        // After shape: a draft or a draft's envelope that repeats a member,
        // two `env` members included, is malformed input, as in Lean.
        for (const draft of drafts.items) {
          if (!draft?.members) continue;
          if (repeats(draft) || draft.members.some(([key, env]) => key === 'env' && repeats(env))) {
            throw new Error('batch input: a draft repeats a member');
          }
        }
      }
      return [resolveBatch(input.drafts, input.held)];
    }
    case 'realm': {
      const forms = input.forms;
      return Array.from({ length: Math.max(1, forms.length) }, (_, i) => {
        const { settled, refused, things, sum } = realm([...forms.slice(i), ...forms.slice(0, i)]);
        return { settled, refused, things, sum };
      });
    }
    case 'scenario': {
      const { parsed } = S.parseSpec(input.spec);
      if (!parsed) throw new Error('scenario spec does not parse');
      const pool = new Map(input.records.map(r => [r.id, r]));
      const answers = input.steps.map(s => {
        const forms = new Map(s.forms.map(f => [f.id, formOf(pool.get(f.id), f.shown)]));
        return [step(parsed, input.thing, [...forms.values()]),
          ...s.orders.map(order => step(parsed, input.thing, order.map(id => forms.get(id))))];
      });
      const base = { steps: answers.map(as => as[0]), lens: lensEdit(input.lens) };
      return [base, ...answers.flatMap((as, i) => as.slice(1).map(a => ({
        ...base, steps: base.steps.map((b, j) => i === j ? a : b),
      })))];
    }
    default: throw new Error(`unsupported layer: ${layer}`);
  }
}

// Equality groups describe observations. They do not assign a guilty implementation.
function comparison(outcomes) {
  const acceptance = Object.fromEntries(Object.entries(outcomes).map(([name, o]) => [name, o.status]));
  const nodes = Object.entries(outcomes).flatMap(([name, o]) => {
    if (o.status === 'not-called') return [];
    if (name === 'js' && o.status === 'answered') {
      return o.answers.map((answer, i) => [`js:${i}`, { status: 'decoded', answer }]);
    }
    return [[name, o]];
  });
  const equivalent = (a, b) => a.status === b.status && (
    a.status === 'decoder-refusal' || equal(a.answer, b.answer));
  const pairwise = {}, groups = [];
  for (let i = 0; i < nodes.length; i++) {
    const [name, outcome] = nodes[i];
    for (const [other, value] of nodes.slice(i + 1)) pairwise[`${name}:${other}`] = equivalent(outcome, value);
    const group = groups.find(g => equivalent(outcome, g.outcome));
    if (group) group.names.push(name); else groups.push({ outcome, names: [name] });
  }
  return { acceptance, pairwise, partition: groups.map(g => g.names) };
}

// A Lean wrapper keeps the two-way API. Rust mode accepts { lean, rust } wrappers.
// Decoder refusals are not protocol answers. Typed JS APIs have no fixture decoder,
// except the record and batch inputs, whose contract says what is malformed: there
// JS must also report malformed input (throw) where Lean does, or it is a mismatch.
const MALFORMED_HELD = new Set(['record', 'batch']);

export function assess(vector, response, { rawText, js = evaluateJS } = {}) {
  const bundled = response && Object.hasOwn(response, 'lean');
  const responses = { lean: bundled ? response.lean : response };
  if (bundled && Object.hasOwn(response, 'rust')) responses.rust = response.rust;
  const outcomes = {}, result = { implementations: ['js', ...Object.keys(responses)], responses };
  for (const [name, r] of Object.entries(responses)) {
    const hasError = r && Object.hasOwn(r, 'error'), hasExpect = r && Object.hasOwn(r, 'expect');
    if (!r || hasError === hasExpect || (hasError && typeof r.error !== 'string')) {
      throw new Error(`${name === 'lean' ? 'Lean' : 'Rust'} response has no answer or is invalid`);
    }
    outcomes[name] = hasError ? { status: 'decoder-refusal' } : { status: 'decoded', answer: r.expect };
    if (hasError) result[`${name}Error`] = r.error;
    else result[name] = r.expect;
  }
  if (outcomes.lean.status === 'decoder-refusal' && !MALFORMED_HELD.has(vector.layer)) {
    outcomes.js = { status: 'not-called' };
  } else if (outcomes.lean.status === 'decoder-refusal') {
    try {
      const answers = js(vector, rawText);
      result.jsAnswers = answers;
      result.js = answers?.[0];
      outcomes.js = { status: 'answered', answers };
    } catch (error) {
      result.jsError = error.message;
      outcomes.js = { status: 'decoder-refusal' };
    }
  } else {
    let answers;
    try { answers = js(vector, rawText); }
    catch (error) {
      result.jsException = { name: error.name, message: error.message };
      outcomes.js = { status: 'exception', answer: result.jsException };
    }
    if (!outcomes.js) {
      if (!Array.isArray(answers) || !answers.length) throw new Error('adapter produces no answers');
      result.jsAnswers = answers;
      outcomes.js = { status: 'answered', answers };
      const order = answers.findIndex(answer => !equal(answer, responses.lean.expect));
      result.js = answers[order === -1 ? 0 : order];
      if (order !== -1) result.order = order;
    }
  }
  result.comparison = comparison(outcomes);
  if (Object.values(responses).every(r => Object.hasOwn(r, 'error'))
    && ['not-called', 'decoder-refusal'].includes(outcomes.js.status)) {
    result.status = 'decoder-refusal';
    result.diagnostic = responses.lean.error;
    result.diagnostics = Object.fromEntries(Object.entries(responses).map(([name, r]) => [name, r.error]));
  } else if (Object.values(outcomes).some(o => o.status !== 'decoded' && o.status !== 'answered')
    || result.comparison.partition.length !== 1) {
    result.status = 'mismatch';
  } else {
    const refused = responses.lean.expect?.refused;
    result.status = typeof refused === 'string' || refused === true ? 'protocol-refusal' : 'match';
  }
  return result;
}

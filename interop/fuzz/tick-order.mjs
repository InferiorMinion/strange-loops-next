// Decision sites in Loops.Tick, Loops.Result.ResultReader and Loops.TickCheck.
// A site describes a wire edit and its observable answer. Path conflicts are
// about the same occurrence; sibling slots, array elements and named entries
// have separate witnesses. The model supplies every confirmation.
import { base32, random, shuffled } from './generate.mjs';
import { specHash, resultHash } from './tick-wire.mjs';

const ids = Array.from({length: 9}, (_, i) => 'r:' + base32([...Array(31).fill(0), i]));
function baseline(rng) {
  const n = 1 + rng.int(200);
  const checker = ids.slice(0, 8).map((id, i) => ({
    id, kind: i ? 'marker' : 'note', of: i ? 't:marker' : 't:one', observer: 'o:1',
    about: [], prev: [], emitted: '2026-11-21T20:00:00Z',
    ...(i >= 4 ? {shown: 'withheld'} : {fields: {n}}),
  }));
  // One admitted root sets both keys. Each occurrence of an id list must
  // be independent so an edit to one slot cannot corrupt a sibling slot.
  checker[0].fields.m = n;
  checker[0].sets = ['m','n'];
  const spec = {vertex: 'order-' + n, from: [{store:'lab', kind:['note']}], fold: {
    a: {count:true}, b: {collect:true, by:'env.kind'}, c: {'latest-by':'of', follow:'keys'},
    d: {'latest-by':'of', follow:'prev'}, e: {sum:'n'}, f: {min:'n'},
  }};
  const heads = () => [ids[0]];
  const result = {
    a: {unread:'0', value:'1'},
    b: {groups:{note:{heads:heads(), unread:'0'}}, ungrouped:'0', unread:'0'},
    c: {things:{'t:one':{heads:heads(), keys:{m:heads(), n:heads()}, whole:[]}}, unread:'0'},
    d: {things:{'t:one':heads()}, unread:'0'},
    e: {unread:'0', value:String(n)},
    f: {unread:'0', value:String(n)},
  };
  const p = {rules:'1', spec, covers:ids.slice(0,8), withheld:ids.slice(4,8), result};
  return {p, checker, current:'1', kind:'tick', repeats:[]};
}

const sites = [];
const fault = (code, path) => ({refused:'tick-' + code, path});
const reason = reason => ({reason});
function site(id, answer, plant, {needs = [], replaces, domain = 'payload'} = {}) {
  sites.push({id, answer, plant, needs, replaces, domain});
}
const required = ['rules','spec','spec-hash','covers','result','result-hash'];
const get = (obj, path) => path.reduce((v,k) => v[k], obj);
function edit(path, fn) {
  return s => fn(get(s.p, path.slice(0,-1)), path.at(-1));
}
function replace(id, path, value, answer) {
  site(id, answer, edit(path, (o,k) => { o[k] = value; }), {replaces:path});
}
function missing(path, answer) {
  site('missing:' + path.join('.'), answer, edit(path,(o,k) => { delete o[k]; }), {replaces:path});
}

site('kind', fault('kind','kind'), s => {s.kind = 'note';}, {domain:'subject'});
site('unheld', reason("the tick's payload is not held"), s => {s.unheld = true;}, {domain:'subject'});
site('not-object', fault('not-object',''), s => {s.p = [{x:0,y:1}];}, {replaces:[]});
// The duplicate witness is chosen after edits, so deleting one required name
// doesn't accidentally remove it. Two repeats demonstrate depth-first order.
for (const id of ['repeat-first','repeat-next']) site(id, null, () => {});
for (const k of ['extra','another']) site('unknown:' + k, fault('member-unknown',k), s => {s.p[k] = 1;});
for (const k of required) missing([k], fault('member-missing',k));
replace('rules-invalid',['rules'],'01',fault('rules-invalid','rules'));
replace('spec-object',['spec'],[],fault('spec-not-object','spec'));

for (const k of ['covers','withheld']) {
  replace(k + ':array',[k],false,fault('ids-not-array',k));
  for (const i of [0,1]) site(k + ':id' + i, fault('id-invalid',k + '[' + i + ']'),
    s => {s.p[k][i] = '!';}, {needs:[[k]]});
  site(k + ':duplicate', fault('id-duplicate',k), s => {s.p[k].push(s.p[k].at(-1));}, {needs:[[k]]});
  site(k + ':unsorted', fault('ids-unsorted',k), s => {[s.p[k][2], s.p[k][3]] = [s.p[k][3], s.p[k][2]];}, {needs:[[k]]});
}
replace('withheld:empty',['withheld'],[],fault('withheld-empty','withheld'));
site('withheld:outside',fault('withheld-outside','withheld'), s => {s.p.withheld.push(ids[8]);}, {needs:[['withheld']]});
for (const k of ['spec-hash','result-hash']) replace(k + ':spelling',[k],'bad',fault('hash-invalid',k));
for (const [k,prefix] of [['spec-hash','spec:'],['result-hash','res:']])
  replace(k + ':fit',[k],prefix + '0'.repeat(52),fault(k + '-mismatch',k));

site('newer',reason('made under newer rules'),s => {s.p.rules = '2';}, {replaces:['rules']});
site('older',reason('made under older rules'),s => {s.current = '2';}, {needs:[['rules']]});
site('unreadable',reason('this binary cannot read the spec'),s => {s.p.spec.vertex = false;}, {needs:[['spec']]});
site('noncanonical',fault('spec-noncanonical','spec'),s => {s.p.spec.note = 0;}, {needs:[['spec']]});

// fixed: object, first extra in input order, then missing in declared order.
// slot values subsequently follow spec slot order. Dynamic maps and arrays
// traverse in input order. The tick-wide repeat check owns all duplicate keys.
const pathOf = path => path.map((s,i) => typeof s === 'number' ? '[' + s + ']' : (i ? '.' : '') + s).join('');
const invalid = path => fault('result-invalid',pathOf(path));
function bad(path, suffix = 'value') {
  replace(suffix + ':' + pathOf(path),path,false,invalid(path));
}
function fixed(path, names) {
  bad(path,'object');
  const extra = [...path,'extra'];
  site('extra:' + pathOf(path),invalid(extra),edit(extra,(o,k) => {o[k] = 0;}), {needs:[path]});
  for (const name of names) missing([...path,name],invalid([...path,name]));
}
function list(path) {
  bad(path,'list');
  for (const i of [0,1]) {
    const at = [...path,i];
    site('id:' + pathOf(at),invalid(at), s => {
      const xs = get(s.p,path);
      while (xs.length <= i) xs.push(ids[0]);
      xs[i] = '!';
    }, {needs:[path]});
  }
}
function scalar(path, name) {
  fixed(path,['unread',name]);
  bad([...path,'unread']);
  if (name === 'heads') list([...path,name]); else bad([...path,name]);
}
const r = ['result'];
fixed(r,['a','b','c','d','e','f']);
scalar([...r,'a'],'value');
const group = [...r,'b'];
fixed(group,['groups','ungrouped','unread']);
bad([...group,'groups'],'object');
scalar([...group,'groups','note'],'heads');
bad([...group,'ungrouped']);
bad([...group,'unread']);
for (const slot of ['c','d']) {
  const p = [...r,slot], things = [...p,'things'], thing = [...things,'t:one'];
  fixed(p,['things','unread']);
  bad([...p,'unread']); // read before things, despite fixed's presence order
  bad(things,'object');
  if (slot === 'd') list(thing);
  else {
    fixed(thing,['heads','keys','whole']);
    list([...thing,'heads']);
    bad([...thing,'keys'],'object');
    for (const k of ['m','n']) {
      const path = [...thing,'keys',k];
      list(path);
    }
    list([...thing,'whole']);
  }
}
scalar([...r,'e'],'value'); // integer
scalar([...r,'f'],'value'); // optional integer (min and max share this reader)

site('ignored-eraser',fault('withheld-erased','withheld'),s => {s.checker[3].erases = [ids[0]];}, {domain:'holding'});
site('missing-record',reason('missing inputs'),s => {s.checker = s.checker.filter(x => x.id !== ids[1]);}, {domain:'holding'});
site('missing-opening',reason('missing inputs'),s => {const f = s.checker.find(x => x.id === ids[2]); delete f.fields; f.shown = 'withheld';}, {domain:'holding'});
site('differs',{answer:'differs'},s => {s.checker[0].fields.n++;}, {domain:'holding'});
// With no replay issue, equal canonical bytes give same, otherwise differs.
// Members-only success echoes the raw claim. Payload success re-encodes the
// parsed slots; a different version returns other-rules before reading them.
// checkMembers computes closure first (a link outside: no; all held: yes;
// else unknown). Newer stops there. For older, readSpec failures become
// today's diagnostic; a parsed spec goes straight to replay, skipping slots.
// Current reads slots before replay. Taken outside is reported once the spec
// can be read. Missing records and openings are one joint answer.

const byId = new Map(sites.map((s,i) => [s.id,{...s,rank:i}]));
const prefix = (path, other) => path.length <= other.length && path.every((k,i) => k === other[i]);
function conflict(a,b) {
  if (a.id === 'unheld' && b.domain === 'payload') return 'an absent subject payload has no wire members';
  if (a.id === 'newer' && b.id === 'older') return 'the two rules comparisons are exclusive';
  if (a.id === 'not-object' && b.id.startsWith('repeat-')) return;
  if (a.id === 'not-object' && b.domain === 'payload') return 'the payload has no top-level object members';
  for (const [x,y] of [[a,b],[b,a]]) {
    if (!x.replaces) continue;
    const other = [...y.needs, ...(y.replaces ? [y.replaces] : [])];
    if (other.some(p => prefix(x.replaces,p))) return x.id + ' removes or replaces ' + (pathOf(x.replaces) || 'the payload');
  }
}

export const orderPairs = [], excludedPairs = {};
for (const [i,a] of sites.entries()) for (const b of sites.slice(i+1)) {
  const why = conflict(a,b);
  if (why) excludedPairs[a.id + ' / ' + b.id] = why;
  else orderPairs.push([a.id,b.id]);
}
// Adjacent cases lead each sweep, so a short CI run exercises local reversals.
const rank = id => byId.get(id).rank;
orderPairs.sort(([a,b],[c,d]) => (rank(b)-rank(a)) - (rank(d)-rank(c)));
export const decisionSites = sites.map(s => s.id);

function paths(v,path=[]) {
  if (!v || typeof v !== 'object') return [];
  return Object.entries(v).flatMap(([k,child]) => {
    const p = [...path, Array.isArray(v) ? Number(k) : k];
    return [...paths(child,p), ...(Array.isArray(v) ? [] : [p])];
  });
}

export function orderVector(seed, index, points) {
  const rng = random(seed,index,'tick-order'), s = baseline(rng);
  const selected = sites.filter(x => points.includes(x.id));
  s.p['spec-hash'] = specHash(s.p);
  s.p['result-hash'] = resultHash(s.p.result);
  for (const x of selected) x.plant(s);
  if (!Array.isArray(s.p)) {
    if (s.p.spec !== undefined && !points.some(id => ['missing:spec-hash','spec-hash:fit','spec-hash:spelling'].includes(id)))
      s.p['spec-hash'] = specHash(s.p);
    if (s.p.result !== undefined && !points.some(id => ['missing:result-hash','result-hash:fit','result-hash:spelling'].includes(id)))
      s.p['result-hash'] = resultHash(s.p.result);
  }
  const duplicates = points.filter(id => id.startsWith('repeat-'));
  if (duplicates.length) {
    const choices = paths(s.p);
    if (choices.length < duplicates.length) throw Error('too few duplicate sites');
    // Insertion repeats a member immediately after its first value: completion
    // order (postorder) is the wire order of the repeated names.
    const chosen = shuffled(choices,rng).slice(0,duplicates.length);
    s.repeats = choices.filter(p => chosen.includes(p));
  }

  const first = selected.find(x => !(x.id === 'result-hash:fit' && points.includes('newer')));
  let expected = first.answer;
  if (first.id.startsWith('repeat-')) expected = fault('member-duplicate',pathOf(s.repeats[0]));
  if (['missing-record','missing-opening'].includes(first.id))
    expected = {...expected,'missing-records':Number(points.includes('missing-record')),'missing-openings':Number(points.includes('missing-opening'))};
  if (first.id === 'older') {
    let today = 'the same result';
    if (points.includes('unreadable')) today = {'not-available':'this binary cannot read the spec'};
    else if (points.includes('noncanonical')) today = {'not-available':"spec is not in today's parsed form"};
    else if (points.includes('ignored-eraser')) today = {'not-available':'tick-withheld-erased'};
    else if (points.includes('missing-record') || points.includes('missing-opening'))
      today = {'not-available':'missing inputs','missing-records':Number(points.includes('missing-record')),'missing-openings':Number(points.includes('missing-opening'))};
    else if (points.includes('differs') || selected.some(x => x.answer?.refused === 'tick-result-invalid')) today = 'another result';
    expected = {...expected,today};
  }
  let input = {op:'check-tick',current:s.current,subject:{kind:s.kind,...(s.unheld ? {} : {payload:s.p})},checker:s.checker};
  const fuzz = {seed:String(seed),index,family:'tick', points:selected.map(x => x.id), winner:first.id, expected};
  let payloadPath = ['input','subject','payload'];
  if (selected.every(x => x.domain === 'payload' && x.id !== 'older') && rng.int(2)) {
    const members = selected.every(x => rank(x.id) < rank('newer')) && rng.int(2);
    input = {op:members ? 'members' : 'payload', payload:s.p};
    payloadPath = ['input','payload'];
    if (first.id === 'newer') fuzz.expected = {'other-rules':'2'};
    if (first.id === 'unreadable') fuzz.expected = {'cannot-read':'spec'};
  }
  if (s.repeats.length) fuzz.repeats = s.repeats.map(p => [...payloadPath,...p]);
  return {layer:'tick',input,fuzz};
}

export function thirdPoint(pair,rng) {
  const candidates = sites.filter(s => !pair.includes(s.id) && pair.every(id => {
    const other = byId.get(id);
    return !(rank(id) < rank(s.id) ? conflict(other,s) : conflict(s,other));
  }));
  return rng.pick(candidates).id;
}

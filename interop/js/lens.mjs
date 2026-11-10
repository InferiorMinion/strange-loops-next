// This independent keyed-section lens renders, parses, diffs and applies edits.
// See protocol/lens.md for the protocol and law premises.
// No-op and put-get laws require distinct base ids; `edit`, the public
// boundary, refuses a base without them. Put-get also requires the same
// ordered ids. This module has no dependencies.

const FENCE = ":::";

// A body line that would read as a fence, or that starts with a backslash,
// gets one backslash in front, so any body survives a render and a parse.
const escape = (line) => (line.startsWith("\\") || line.startsWith(FENCE) ? "\\" + line : line);

// The reverse, and only where rendering would have written one: a lone
// backslash before anything else is the body's own.
const unescape = (line) =>
  line.startsWith("\\") && (line[1] === "\\" || line.slice(1).startsWith(FENCE)) ? line.slice(1) : line;

export function render(sections) {
  return sections.flatMap((s) => [`${FENCE} ${s.id}`, ...s.body.map(escape), FENCE]);
}

// Text (a list of lines) to sections, or null when it is malformed: text
// outside a section, a fence-like line in a body that isn't the closing one,
// a section left open, an empty id, an id given twice.
export function parse(lines) {
  const sections = [];
  let open = null;
  for (const line of lines) {
    if (open === null) {
      if (!line.startsWith(FENCE + " ") || line.length === FENCE.length + 1) return null;
      open = { id: line.slice(FENCE.length + 1), body: [] };
    } else if (line === FENCE) {
      sections.push(open);
      open = null;
    } else if (line.startsWith(FENCE)) {
      return null;
    } else {
      open.body.push(unescape(line));
    }
  }
  if (open !== null) return null;
  return new Set(sections.map((s) => s.id)).size === sections.length ? sections : null;
}

const same = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);

// What the edited sections say against the base, matched by id. An edited
// section is accepted when its body differs, unknown when the base has no
// such id; a base section the text leaves out is absent. A section that
// matches the base says nothing.
// Raw: it takes the first base section with an id, as the model's lookup does.
// A base that repeats an id is refused at the public boundary, `edit`.
export function diff(base, edited) {
  const held = new Map();
  for (const s of base) if (!held.has(s.id)) held.set(s.id, s);
  const outcomes = [];
  for (const s of edited) {
    const was = held.get(s.id);
    if (!was) outcomes.push({ unknown: s.id });
    else if (!same(was.body, s.body)) outcomes.push({ accepted: s.id, body: s.body });
  }
  const seen = new Set(edited.map((s) => s.id));
  for (const s of base) if (!seen.has(s.id)) outcomes.push({ absent: s.id });
  return outcomes;
}

// The base with the accepted bodies in place. Unknown and absent change nothing.
export function apply(base, outcomes) {
  const accepted = new Map(outcomes.filter((o) => "accepted" in o).map((o) => [o.accepted, o.body]));
  return base.map((s) => (accepted.has(s.id) ? { id: s.id, body: accepted.get(s.id) } : s));
}

// The public base boundary: section ids are distinct. The raw functions above
// don't check, and keep their first-match behavior.
export const baseProblem = (base) =>
  new Set(base.map((s) => s.id)).size === base.length ? null : "base-id-duplicate";

// Read an edited text against a base, as vectors/lens/section writes the
// answer. The base is checked first, so a repeated base id is refused whatever
// the text says; then the text's own refusal; then the edit.
export function edit(base, lines) {
  const why = baseProblem(base);
  if (why) return { refused: why };
  const parsed = parse(lines);
  if (parsed === null) return { parsed: null, refused: true };
  const outcomes = diff(base, parsed);
  return { parsed, normalized: render(parsed), outcomes, applied: apply(base, outcomes) };
}

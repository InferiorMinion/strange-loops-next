//! Preflight the entire group using compact recipes pointing into frozen exports.
//! Payloads and signed lines are materialized one bounded changeset at a time.
use super::{
    Input, Point, Runtime, digest, event,
    journal::{Journal, Seed},
    mapping::{History, Row, Unnamed},
    mismatch,
    report::{Counts, Report},
    source::{self, Fact},
};
use crate::{Fail, prepare_changeset, refused, text, unclean};
use loops::{base32, batch, json::Json};
use loops_place::{
    dir::Held,
    key::Key,
    line::{Line, hash},
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    ops::Range,
};

const MAX_ROWS: usize = 1000;
const MAX_BYTES: usize = 4 * 1024 * 1024;

fn fingerprint(f: &Fact) -> String {
    let payload = match f.parsed_payload() {
        Ok(j) if j.keys_distinct() && j.in_range() => Json::Arr(vec![text("json"), j]),
        _ => Json::Arr(vec![text("raw"), text(&f.payload)]),
    };
    digest(
        Json::Arr(vec![
            text(&f.kind),
            text(f.ts_bits.to_string()),
            text(&f.observer),
            text(&f.origin),
            f.signature.as_ref().map_or(Json::Null, text),
            payload,
        ])
        .canon()
        .as_bytes(),
    )
}

pub(super) struct Identities {
    pub seeds: HashMap<String, Seed>,
    omitted: HashMap<String, String>,
}
impl From<&Journal> for Identities {
    fn from(j: &Journal) -> Self {
        Self {
            seeds: j.seeds.clone(),
            omitted: j.omitted.clone(),
        }
    }
}
impl Identities {
    fn omit(&mut self, old: &str, fp: String, omitted_rows: &mut Vec<Json>) {
        if self.fingerprint(old).is_none() {
            omitted_rows.push(Json::Arr(vec![text(old), text(&fp)]));
            self.omitted.insert(old.to_owned(), fp);
        }
    }
    fn parameters(
        &mut self,
        old: &str,
        fp: String,
        runtime: &Runtime,
    ) -> Result<(&[u8], &str), Fail> {
        let seed = match self.seeds.entry(old.to_owned()) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => e.insert((runtime.seed)(old, fp)?),
        };
        Ok((&seed.salt, &seed.emitted))
    }
    fn fingerprint(&self, old: &str) -> Option<&String> {
        self.seeds
            .get(old)
            .map(|s| &s.fingerprint)
            .or_else(|| self.omitted.get(old))
    }
}
struct Item {
    old: String,
    id: String,
    draft: batch::Draft,
    span: Range<u64>,
    sum: Vec<u8>,
    prior: Option<usize>,
}
struct Chunk {
    items: Vec<Item>,
    bytes: usize,
}
pub(super) struct Plan {
    held: Held,
    file: File,
    chunks: Vec<Chunk>,
    cut: (usize, String),
    omitted: Vec<Json>,
}
impl Plan {
    fn verify(&mut self, key: &Key) -> Result<(), Fail> {
        let mut staged = HashMap::new();
        for chunk in &self.chunks {
            prepare(&mut self.file, chunk, key, &self.held, &mut staged)?;
        }
        Ok(())
    }
    fn enqueue(&mut self, p: super::mapping::Prepared, f: Fact, prior: Option<usize>) {
        let mut draft = p.draft;
        let size = draft.payload.canon().len() + draft.env.to_json().canon().len() + 512;
        draft.payload = Json::Null;
        self.push(
            Item {
                old: f.id,
                id: p.id,
                draft,
                span: f.span,
                sum: f.raw_digest,
                prior,
            },
            size,
        );
    }
    fn push(&mut self, item: Item, size: usize) {
        if self.chunks.last().is_none_or(|c| {
            c.items.len() >= MAX_ROWS
                || c.bytes.saturating_add(size) > MAX_BYTES
                || c.items[0].prior != item.prior
        }) {
            self.chunks.push(Chunk {
                items: Vec::new(),
                bytes: 0,
            });
        }
        let c = self.chunks.last_mut().expect("chunk exists");
        c.bytes += size;
        c.items.push(item);
    }
}

// Use the held envelopes and the batches prepared earlier in this pass.
// Only raw payload text is reread; the verified span must still match exactly.
fn prepare(
    file: &mut File,
    chunk: &Chunk,
    key: &Key,
    held: &Held,
    staged: &mut HashMap<String, batch::Held>,
) -> Result<Vec<Line>, Fail> {
    let mut drafts = Vec::with_capacity(chunk.items.len());
    for item in &chunk.items {
        file.seek(SeekFrom::Start(item.span.start))?;
        let mut bytes = vec![
            0;
            usize::try_from(item.span.end - item.span.start)
                .expect("span came from a String")
        ];
        file.read_exact(&mut bytes)?;
        if hash(&bytes) != item.sum {
            return Err(mismatch("export changed after preflight"));
        }
        let json = loops_eval::parse_json(std::str::from_utf8(&bytes).expect("verified UTF-8"))
            .map_err(super::invalid)?;
        let Json::Obj(fields) = json else {
            return Err(mismatch("export row changed"));
        };
        let raw = fields
            .iter()
            .find(|(k, _)| k == "payload")
            .ok_or_else(|| mismatch("export lost payload"))?;
        let mut d = item.draft.clone();
        d.payload =
            loops_eval::parse_writer_payload(source::string(&raw.1).map_err(super::invalid)?)
                .map_err(super::invalid)?;
        drafts.push(d);
    }
    let view = |id: &str| staged.get(id).cloned().or_else(|| held.writer_view(id));
    let lines = prepare_changeset(&view, key, &drafts)?;
    for (item, line) in chunk.items.iter().zip(&lines) {
        if line.id() != item.id {
            return Err(mismatch("checked batch disagrees with prospective id"));
        }
        staged.insert(
            item.id.clone(),
            batch::Held {
                of: line.env().of.clone(),
                prev: line.env().prev.clone(),
            },
        );
    }
    Ok(lines)
}

fn open(input: &Input, key: &Key, j: &Journal, counts: &mut Counts) -> Result<Plan, Fail> {
    let held = input.dest.read()?;
    counts.add("destination_reads", 1);
    if let Some(problem) = unclean(&held) {
        return Err(refused(problem));
    }
    let prior = j.records.get(&input.name);
    let allowed: HashSet<_> = prior
        .into_iter()
        .flat_map(|rows| rows.values().map(|(id, _)| id.as_str()))
        .collect();
    if held
        .lines()
        .iter()
        .any(|line| !allowed.contains(line.id()) || line.key() != key.public())
    {
        return Err(mismatch(&format!(
            "{}: destination holds an unreserved record or signer",
            input.name
        )));
    }
    let file = File::open(&input.facts)?;
    Ok(Plan {
        held,
        file,
        chunks: Vec::new(),
        omitted: Vec::new(),
        cut: (0, base32::base32(&[])),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "The preflight shares group reservations and reporting, and owns a distinct history for this source."
)]
pub(super) fn preflight(
    input: &Input,
    i: usize,
    unnamed: Unnamed,
    key: &Key,
    j: &Journal,
    identities: &mut Identities,
    runtime: &Runtime,
    report: &mut Report,
) -> Result<Plan, Fail> {
    let map = input.map.as_ref().map_err(refused)?;
    let mut plan = open(input, key, j, &mut report.sources[i].1)?;
    let reader = BufReader::new(plan.file.try_clone()?);
    let prior = j.records.get(&input.name);
    let mut h = History::default();
    let mut prefix = Vec::new();
    let mut matched = 0;
    let checkpoint = j.cuts.get(&input.name);
    let mut drift = false;
    let mut system = None;
    let summary = source::scan(reader, map, |f| {
        report.sources[i].1.add("facts_read", 1);
        if f.observer.is_empty() {
            report.sources[i].1.add("empty_observer", 1);
        }
        plan.cut.0 += 1;
        prefix.extend_from_slice(&f.raw_digest);
        prefix = hash(&prefix);
        if checkpoint.is_some_and(|(n, sum)| *n == plan.cut.0 && *sum != base32::base32(&prefix)) {
            drift = true;
        }
        let omitted = !map.kinds.contains_key(&f.kind) && unnamed == Unnamed::Omit;
        let fp = fingerprint(&f);
        let local = prior.and_then(|p| p.get(&f.id));
        if local.is_some() {
            matched += 1;
        }
        let context = |why: &str| format!("{} {:?} {}: {why}", input.name, f.coordinate, f.id);
        if identities
            .fingerprint(&f.id)
            .is_some_and(|known| known != &fp)
        {
            let code = if local.is_some() {
                "shadow-state-mismatch"
            } else {
                "shadow-id-conflict"
            };
            report.problem(i, code, context(code));
            return;
        }
        let (salt, time) = if omitted {
            identities.omit(&f.id, fp, &mut plan.omitted);
            (&[][..], "")
        } else {
            match identities.parameters(&f.id, fp, runtime) {
                Ok(pair) => pair,
                Err(error) => {
                    system = Some(error);
                    return;
                }
            }
        };
        match h.prepare(map, unnamed, &f, key, salt, time) {
            Err(e) => report.problem(i, e.code(), context(&e.to_string())),
            Ok(Row::Omitted { defect }) => {
                report.omitted(i, &f.id, defect.as_ref());
            }
            Ok(Row::Included(p)) => {
                if local.is_some_and(|(expected, _)| *expected != p.id) {
                    drift = true;
                    return;
                }
                report.included(i, &f.id, &p, map.kinds.get(&f.kind));
                let counts = &mut report.sources[i].1;
                if local.is_some() && plan.held.line(&p.id).is_some() {
                    counts.add("already_imported", 1);
                    return;
                }
                counts.add("would_write", 1);
                plan.enqueue(*p, f, local.map(|(_, b)| *b));
            }
        }
    })
    .map_err(|e| match e {
        source::ScanError::Invalid(why) => refused(format!("{}: {why}", input.name)),
        source::ScanError::Io(e) => Fail::System(format!("{}: {e}", input.facts.display())),
    })?;
    if let Some(fail) = system {
        return Err(fail);
    }
    plan.cut.1 = base32::base32(&prefix);
    report.sources[i].1.add(
        "old_ticks_left_out",
        usize::try_from(summary.ticks).map_err(super::invalid)?,
    );
    debug_assert_eq!(summary.facts, u64::try_from(plan.cut.0).expect("count"));
    if drift
        || checkpoint.is_some_and(|(n, _)| *n > plan.cut.0)
        || prior.is_some_and(|p| matched != p.len())
    {
        return Err(mismatch(&format!(
            "{}: recorded source prefix or mapping changed",
            input.name
        )));
    }
    if report.errors.is_empty() {
        plan.verify(key)?;
    }
    Ok(plan)
}

impl Plan {
    #[expect(
        clippy::too_many_arguments,
        reason = "Publication owns the only journal and segment writes for a preflighted source."
    )]
    pub(super) fn publish(
        &mut self,
        input: &Input,
        key: &Key,
        j: &mut Journal,
        log: &mut File,
        seeds: &HashMap<String, Seed>,
        runtime: &Runtime,
        counts: &mut Counts,
    ) -> Result<(), Fail> {
        if j.cuts.get(&input.name) != Some(&self.cut) {
            j.append(
                log,
                &event(
                    "cut",
                    [
                        text(&input.name),
                        text(self.cut.0.to_string()),
                        text(&self.cut.1),
                        Json::Arr(std::mem::take(&mut self.omitted)),
                    ],
                ),
            )?;
        }
        let mut staged = HashMap::new();
        for chunk in &self.chunks {
            let lines = prepare(&mut self.file, chunk, key, &self.held, &mut staged)?;
            let batch = if let Some(n) = chunk.items[0].prior {
                n
            } else {
                let n = j.batches.len();
                let entries = chunk
                    .items
                    .iter()
                    .map(|r| seeds[&r.old].entry(&r.old, &r.id))
                    .collect();
                (runtime.at)(Point::BeforeReserve)?;
                j.append(
                    log,
                    &event("reserve", [text(&input.name), Json::Arr(entries)]),
                )?;
                (runtime.at)(Point::Reserved)?;
                n
            };
            input.dest.append(&lines)?;
            counts.add("written", lines.len());
            (runtime.at)(Point::Appended)?;
            if chunk.items[0].prior.is_none() {
                j.append(log, &event("done", [text(batch.to_string())]))?;
            }
        }
        // This includes a stop after the segment rename with all IDs already held.
        for n in 0..j.batches.len() {
            if j.batches[n] == input.name && !j.done.contains(&n) {
                j.append(log, &event("done", [text(n.to_string())]))?;
            }
        }
        Ok(())
    }
}

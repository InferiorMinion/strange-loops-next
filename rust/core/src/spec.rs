//! Spec slots, parsing, and execution from model/Loops/Spec.lean.
use crate::{
    envelope::typed_ref,
    families::{self, Result as FoldResult, Tri},
    fold::{ordered_forms, whole_heads},
    json::{Json, canon_set, str_cmp},
    live::{Keyed, LiveState},
    number::{Int, MAX_EXACT, Nat},
    record::{Digest, Record, Shown, Val},
    things::{admit, by_thing, ids_of, per_thing},
    where_::{Cond, EnvField, Field, Refusal, RefusalCode, Tests, parse_where},
};

use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
};

/// Lean: `Loops.Follow`, model/Loops/Spec.lean.
/// How a slot reads supersession: as a thing's heads, or as its live setters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    /// A thing is its heads.
    Prev,
    /// A thing is its live setters.
    Keys,
}
impl Follow {
    /// Lean: `Loops.Follow.name`, model/Loops/Spec.lean.
    /// Returns `follow` as written.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Prev => "prev",
            Self::Keys => "keys",
        }
    }
}

/// Lean: `Loops.FoldType`, model/Loops/Spec.lean.
/// A slot uses one fold from this closed vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FoldType {
    /// Counts every selected record with a known test result.
    Count {
        /// Counts the whole-reader heads (`prev`) or the selected things (`keys`) instead of all records.
        follow: Option<Follow>,
    },
    /// Adds readable integers from the named payload field.
    Sum {
        /// The payload field read.
        field: String,
        /// Adds over the whole-reader heads (`prev`) or the selected things (`keys`).
        follow: Option<Follow>,
    },
    /// Finds the smallest readable integer in the named payload field.
    Min {
        /// The payload field read.
        field: String,
        /// Reads the whole-reader heads (`prev`) or the selected things (`keys`).
        follow: Option<Follow>,
    },
    /// Finds the largest readable integer in the named payload field.
    Max {
        /// The payload field read.
        field: String,
        /// Reads the whole-reader heads (`prev`) or the selected things (`keys`).
        follow: Option<Follow>,
    },
    /// Reports selected current heads, or following `keys` the keyed state, for each thing.
    LatestBy {
        /// How each thing's supersession is read.
        follow: Follow,
    },
    /// Reports selected records newest first.
    Collect {
        /// Uses current per-thing heads (`prev`), or the selected things' live records (`keys`).
        follow: Option<Follow>,
        /// Caps known matches without changing the pre-limit unknown-form count.
        /// That count represents distinct ids only for addressed input.
        limit: Option<Nat>,
    },
}

impl FoldType {
    /// Lean: `Loops.FoldType.follow`, model/Loops/Spec.lean.
    /// How the slot reads supersession, if it follows.
    pub const fn follow(&self) -> Option<Follow> {
        match self {
            Self::Count { follow }
            | Self::Sum { follow, .. }
            | Self::Min { follow, .. }
            | Self::Max { follow, .. }
            | Self::Collect { follow, .. } => *follow,
            Self::LatestBy { follow } => Some(*follow),
        }
    }
}

/// Lean: `Loops.By`, model/Loops/Spec.lean.
/// What `by` groups a slot's records by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum By {
    /// The envelope's kind: `env.kind`.
    Kind,
    /// The envelope's observer: `env.observer`.
    Observer,
    /// The members of `about` under a typed-reference prefix: `env.about:<prefix>`.
    About(String),
    /// A payload field's value, named as `where` names payload fields.
    Payload(String),
}

/// Lean: `Loops.refPrefix`, model/Loops/Spec.lean.
/// A typed reference's prefix: a lower-case letter, then lower-case letters, digits and hyphens.
pub fn ref_prefix(p: &str) -> bool {
    let mut bytes = p.bytes();
    bytes.next().is_some_and(|c| c.is_ascii_lowercase())
        && bytes.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// Lean: `Loops.Val.key`, model/Loops/Spec.lean.
/// A payload value as a group key: text as written, a safe whole number in
/// decimal, a boolean as `true` or `false`; nothing for any other value.
pub fn val_key(v: &Val) -> Option<String> {
    match v {
        Val::Str(s) => Some(s.clone()),
        Val::Int(i) if *i >= Int::from(-MAX_EXACT) && *i <= Int::from(MAX_EXACT) => {
            Some(i.0.to_string())
        }
        Val::Bool(b) => Some(b.to_string()),
        Val::Int(_) | Val::Number(_) => None,
    }
}

impl By {
    /// Lean: `Loops.By.ofName`, model/Loops/Spec.lean.
    /// Reads `env.about:<prefix>`, `env.kind`, `env.observer`, or a payload field name.
    pub fn of_name(s: &str) -> Option<Self> {
        if let Some(p) = s.strip_prefix("env.about:") {
            return ref_prefix(p).then(|| Self::About(p.into()));
        }
        match Field::of_name(s)? {
            Field::Env(EnvField::Kind) => Some(Self::Kind),
            Field::Env(EnvField::Observer) => Some(Self::Observer),
            Field::Payload(n) => Some(Self::Payload(n)),
            Field::Env(_) => None,
        }
    }
    /// Lean: `Loops.By.name`, model/Loops/Spec.lean.
    /// Returns `by` as written.
    pub fn name(&self) -> String {
        match self {
            Self::Kind => "env.kind".into(),
            Self::Observer => "env.observer".into(),
            Self::About(p) => format!("env.about:{p}"),
            Self::Payload(n) => n.clone(),
        }
    }
    /// Lean: `Loops.By.keys`, model/Loops/Spec.lean.
    /// The groups a record joins, each once in canonical order; `None` when a
    /// payload key is on a payload the reader does not hold.
    pub fn keys(&self, r: &Record) -> Option<Vec<String>> {
        match self {
            Self::Kind => Some(vec![r.kind.clone()]),
            Self::Observer => Some(vec![r.observer.clone()]),
            Self::About(p) => {
                let head = format!("{p}:");
                Some(canon_set(
                    &r.about
                        .iter()
                        .filter_map(|a| a.strip_prefix(head.as_str()).map(String::from))
                        .collect::<Vec<_>>(),
                ))
            }
            Self::Payload(f) => (r.shown == Shown::Here)
                .then(|| r.get(f).as_ref().and_then(val_key).into_iter().collect()),
        }
    }
    /// Gives what Lean `Loops.By.thingKeys` gives. Queries indexed setters or selected heads.
    /// The groups thing `t` joins, each once: a payload field's key read from
    /// each of its live setters, `None` when one of them is not held; an
    /// envelope key read from each head `from` and the envelope tests choose.
    pub fn thing_keys(
        &self,
        sel: impl Fn(&Record) -> bool,
        cs: &[Cond],
        xs: &[Record],
        t: &str,
    ) -> Option<Vec<String>> {
        keys_of(
            self,
            &LiveState::new(xs.iter().filter(|r| r.is_of(t)).collect()),
            &sel,
            cs,
        )
    }
}

/// Lean: `Loops.Slot`, model/Loops/Spec.lean.
/// A named fold keeps its where tests in written order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The result member name.
    pub name: String,
    /// The fold and its arguments.
    pub fold_type: FoldType,
    /// The ordered field tests, or an empty conjunction.
    pub tests: Tests,
    /// `by`, when the slot groups.
    pub group: Option<By>,
}

/// Lean: `Loops.GroupValue`, model/Loops/Spec.lean.
/// What one group holds: what its slot holds without `by`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupValue {
    /// A count and its unread count.
    Nat(FoldResult<Nat>),
    /// An integer sum and its unread count.
    Int(FoldResult<Int>),
    /// An optional integer extremum and its unread count.
    Opt(FoldResult<Option<Int>>),
    /// Records newest first.
    Heads {
        /// The ids and reader forms of the group's records.
        heads: Vec<(Digest, Shown)>,
        /// The group's unread count.
        unread: Nat,
    },
}

/// Lean: `Loops.KeyedView`, model/Loops/Spec.lean.
/// A thing's keyed state as a slot shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyedView {
    /// Each declared key's live setters, newest first.
    pub keys: Vec<(String, Vec<(Digest, Shown)>)>,
    /// The live whole-read emits, newest first.
    pub whole: Vec<(Digest, Shown)>,
    /// The thing's heads, newest first.
    pub heads: Vec<(Digest, Shown)>,
}
impl Keyed {
    /// Lean: `Loops.Keyed.view`, model/Loops/Spec.lean.
    /// Each record by its id and what the reader holds of it.
    pub fn view(&self) -> KeyedView {
        KeyedView {
            keys: self
                .keys
                .iter()
                .map(|(k, hs)| (k.clone(), shown_ids(hs)))
                .collect(),
            whole: shown_ids(&self.whole),
            heads: shown_ids(&self.heads),
        }
    }
}

/// Lean: `Loops.SlotValue`, model/Loops/Spec.lean.
/// A slot result keeps its value and unread count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotValue {
    /// A count and its unknown test count.
    Nat(FoldResult<Nat>),
    /// An integer sum and its unread count.
    Int(FoldResult<Int>),
    /// An optional integer extremum and its unread count.
    Opt(FoldResult<Option<Int>>),
    /// Current heads by thing, with distinct unknown ids counted across things.
    Things {
        /// Each selected thing and the selected current heads it holds.
        things: Vec<(String, Vec<(Digest, Shown)>)>,
        /// The number of distinct chosen ids with unknown payload tests.
        unread: Nat,
    },
    /// Records newest first, with unknown forms counted before the limit.
    Heads {
        /// The ids and reader forms of the selected records.
        heads: Vec<(Digest, Shown)>,
        /// The number of chosen unknown forms after raw ordering, before the limit.
        /// Counts distinct ids only for addressed input (equal ids imply equal records).
        unread: Nat,
    },
    /// Latest-by following `keys`: each selected thing's keyed state.
    Keyed {
        /// Each selected thing and its whole keyed state.
        things: Vec<(String, KeyedView)>,
        /// The chosen things whose payload tests were unknown.
        unread: Nat,
    },
    /// A slot with `by`: each group by key in canonical order, then the top unread and ungrouped counts.
    Groups {
        /// Each group's key and what the slot holds over it.
        groups: Vec<(String, GroupValue)>,
        /// Selected records whose `where` was unknown or whose key could not be read.
        unread: Nat,
        /// Selected records with no key.
        ungrouped: Nat,
    },
}

impl SlotValue {
    /// Lean: `Loops.SlotValue.group`, model/Loops/Spec.lean.
    /// A slot's value as one group holds it; the last two shapes never arise.
    #[must_use]
    pub fn group(self) -> GroupValue {
        match self {
            Self::Nat(r) => GroupValue::Nat(r),
            Self::Int(r) => GroupValue::Int(r),
            Self::Opt(r) => GroupValue::Opt(r),
            Self::Heads { heads, unread } => GroupValue::Heads { heads, unread },
            Self::Things { unread, .. }
            | Self::Keyed { unread, .. }
            | Self::Groups { unread, .. } => GroupValue::Heads {
                heads: Vec::new(),
                unread,
            },
        }
    }
}

impl Record {
    /// Lean: `Loops.Record.unlinked`, model/Loops/Spec.lean.
    /// Clones the record without succession links.
    #[must_use]
    pub fn unlinked(&self) -> Self {
        Self {
            prev: Vec::new(),
            ..self.clone()
        }
    }
}

/// Gives what Lean `Loops.newestFirst` gives. Stably sorts unlinked forms and removes fully equal records within ties.
/// Clears succession links and orders records by the raw latest-by fold.
/// Duplicate removal uses full record equality after the links are cleared.
pub fn newest_first(xs: &[Record]) -> Vec<Record> {
    ordered_forms(xs.iter().map(Record::unlinked).collect())
}

/// Lean: `Loops.limitTo`, model/Loops/Spec.lean.
/// Returns at most the limit, or all items when the limit exceeds machine width.
pub fn limit_to<'a, T>(limit: Option<&Nat>, xs: &'a [T]) -> &'a [T] {
    let n = limit.map_or(xs.len(), |k| usize::try_from(&k.0).unwrap_or(xs.len()));
    &xs[..n.min(xs.len())]
}

/// Lean: `Loops.shownIds`, model/Loops/Spec.lean.
/// Returns ids with their reader forms in record order.
pub fn shown_ids(hs: &[Record]) -> Vec<(Digest, Shown)> {
    hs.iter().map(|h| (h.id, h.shown)).collect()
}

/// Lean: `Loops.chosen`, model/Loops/Spec.lean.
/// Chooses a head by its source and envelope tests only.
pub fn chosen(sel: impl Fn(&Record) -> bool, cs: &[Cond], h: &Record) -> bool {
    sel(h)
        && cs
            .iter()
            .filter(|c| c.field.on_envelope())
            .rfold(Tri::Yes, |t, c| c.eval(h).and(t))
            == Tri::Yes
}

/// Lean: `Loops.payTest`, model/Loops/Spec.lean.
/// Combines only payload tests, with an empty conjunction equal to Yes.
pub fn pay_test(cs: &[Cond], h: &Record) -> Tri {
    cs.iter()
        .filter(|c| !c.field.on_envelope())
        .rfold(Tri::Yes, |t, c| c.eval(h).and(t))
}

/// Lean: `Loops.latestOf`, model/Loops/Spec.lean.
/// Resolves each thing over all records before applying source and where tests.
pub fn latest_of(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record]) -> SlotValue {
    let mut things = Vec::new();
    let mut unknown = Vec::new();
    for (thing, state) in per_thing(xs) {
        let mut kept = Vec::new();
        for h in state.heads.iter().filter(|h| chosen(&sel, cs, h)) {
            match pay_test(cs, h) {
                Tri::Yes => kept.push((h.id, h.shown)),
                Tri::Unknown => unknown.push(h.clone()),
                Tri::No => {}
            }
        }
        if !kept.is_empty() {
            things.push((thing, kept));
        }
    }
    SlotValue::Things {
        things,
        unread: Nat(ids_of(&unknown).len().into()),
    }
}

/// Lean: `Loops.Tri.agree`, model/Loops/Spec.lean.
/// The answer every setter gives, `Unknown` when they differ, `none` with no setter.
pub fn agree(ts: &[Tri], none: Tri) -> Tri {
    if ts.is_empty() {
        none
    } else if ts.iter().all(|t| *t == Tri::Yes) {
        Tri::Yes
    } else if ts.iter().all(|t| *t == Tri::No) {
        Tri::No
    } else {
        Tri::Unknown
    }
}

impl Cond {
    /// Gives what Lean `Loops.Cond.onThing` gives. Queries indexed setters of the tested field.
    /// A payload test asked of a thing's records `rs`: of every live setter of
    /// its field. With no setter the field is absent. An envelope test is not
    /// asked here.
    pub fn on_thing(&self, rs: &[Record]) -> Tri {
        test_setters(self, &LiveState::new(rs.iter().collect()))
    }
}

/// Gives what Lean `Loops.thingTest` gives. Shares the successor index across the tests.
/// A thing's payload tests: every one holds of the thing.
pub fn thing_test(cs: &[Cond], rs: &[Record]) -> Tri {
    indexed_test(cs, &LiveState::new(rs.iter().collect()))
}

/// Gives what Lean `Loops.keyedOf` gives. Indexes the records by thing before evaluating selection and liveness.
/// Each thing one of whose heads `from` and the envelope tests choose, and
/// whose payload tests hold of it; a chosen thing that answers unknown is unread once.
pub fn keyed_of(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record]) -> SlotValue {
    let ty = FoldType::LatestBy {
        follow: Follow::Keys,
    };
    selected_value(&ty, &index_things(xs), |t| {
        evaluate_thing(&t.state, &sel, cs)
    })
}

/// Lean: `Loops.collectOf`, model/Loops/Spec.lean.
/// Selects all records or current per-thing heads, then orders and limits known matches.
/// Unknown payload tests count forms after raw ordering, before the limit.
/// This equals a distinct-id count only for addressed input (equal ids imply equal records).
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "Lean collectOf takes the explicit follow flag"
)]
pub fn collect_of(
    sel: impl Fn(&Record) -> bool,
    cs: &[Cond],
    follow: bool,
    limit: Option<&Nat>,
    xs: &[Record],
) -> SlotValue {
    let pool = if follow {
        per_thing(xs)
            .into_iter()
            .flat_map(|(_, s)| s.heads)
            .collect()
    } else {
        xs.to_vec()
    };
    let hs = newest_first(
        &pool
            .into_iter()
            .filter(|h| chosen(&sel, cs, h))
            .collect::<Vec<_>>(),
    );
    let unknown = hs
        .iter()
        .filter(|h| pay_test(cs, h) == Tri::Unknown)
        .count();
    let kept: Vec<_> = hs
        .into_iter()
        .filter(|h| pay_test(cs, h) == Tri::Yes)
        .collect();
    SlotValue::Heads {
        heads: shown_ids(limit_to(limit, &kept)),
        unread: Nat(unknown.into()),
    }
}

/// Lean: `Loops.accPool`, model/Loops/Spec.lean.
/// What an accumulating slot folds: every record, or following `prev` the
/// whole-reader heads, each id once.
pub fn acc_pool(follow: Option<Follow>, xs: &[Record]) -> Vec<Record> {
    if follow == Some(Follow::Prev) {
        whole_heads(xs)
    } else {
        xs.to_vec()
    }
}

/// Gives what Lean `Loops.thingEv` gives. Queries the heads and payload tests of the indexed thing.
/// Whether a slot that follows `keys` selects thing `t`: no, unless `from`
/// and the envelope tests choose one of its heads; then its payload tests,
/// asked of the thing.
pub fn thing_ev(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record], t: &str) -> Tri {
    evaluate_thing(
        &LiveState::new(xs.iter().filter(|r| r.is_of(t)).collect()),
        &sel,
        cs,
    )
}

/// Gives what Lean `Loops.soleSetter` gives. Reads the field from the indexed thing.
/// The record thing `t`'s field is read from: its one live setter of it.
/// With none, or two or more, there is none: the core never compares values.
pub fn sole_setter(xs: &[Record], f: &str, t: &str) -> Option<Record> {
    let state = LiveState::new(xs.iter().filter(|r| r.is_of(t)).collect());
    match state.setters(f).as_slice() {
        [r] => Some((*r).clone()),
        _ => None,
    }
}

/// Gives what Lean `Loops.thingsInOrder` gives. Tracks already visited things in a hash set.
/// The things of records `rs`, each once, in the order of each one's first record.
pub fn things_in_order(rs: &[Record]) -> Vec<String> {
    let mut seen = HashSet::new();
    rs.iter()
        .map(Record::thing)
        .filter(|t| seen.insert(t.clone()))
        .collect()
}

/// Gives what Lean `Loops.wholeThings` gives. Tracks the first limited set of things while walking the ordered records.
/// A limit over things: of records `rs`, newest first, those of the first
/// `limit` things, each listed whole, in the order of `rs`.
pub fn whole_things(limit: Option<&Nat>, rs: Vec<Record>) -> Vec<Record> {
    let Some(limit) = limit else { return rs };
    let n = usize::try_from(&limit.0).unwrap_or(usize::MAX);
    let mut kept = HashSet::new();
    rs.into_iter()
        .filter(|r| {
            let t = r.thing();
            if kept.len() < n {
                kept.insert(t);
                true
            } else {
                kept.contains(&t)
            }
        })
        .collect()
}

/// Gives what Lean `Loops.keyedValue` gives. Partitions records by thing and evaluates each selection once.
/// A slot that follows `keys`, over the things `ev` selects: count counts
/// them, sum, min and max read each one's field from its one live setter,
/// collect lists their live records newest first, of at most `limit` things,
/// those whose newest live records are newest, each listed whole. A thing
/// `ev` cannot answer for is unread once.
pub fn keyed_value(ty: &FoldType, xs: &[Record], ev: impl Fn(&str) -> Tri) -> SlotValue {
    selected_value(ty, &index_things(xs), |t| ev(&t.name))
}

struct Thing<'a> {
    name: String,
    state: LiveState<'a>,
}

fn index_things(xs: &[Record]) -> Vec<Thing<'_>> {
    by_thing(xs)
        .into_iter()
        .map(|(name, rs)| Thing {
            name,
            state: LiveState::new(rs),
        })
        .collect()
}

fn test_setters(c: &Cond, state: &LiveState<'_>) -> Tri {
    match &c.field {
        Field::Env(_) => Tri::Yes,
        Field::Payload(f) => agree(
            &state
                .setting(f)
                .into_iter()
                .map(|r| c.eval(r))
                .collect::<Vec<_>>(),
            Tri::of_bool(c.op.test(&[])),
        ),
    }
}

fn indexed_test(cs: &[Cond], state: &LiveState<'_>) -> Tri {
    cs.iter()
        .rfold(Tri::Yes, |t, c| test_setters(c, state).and(t))
}

fn evaluate_thing(state: &LiveState<'_>, sel: &impl Fn(&Record) -> bool, cs: &[Cond]) -> Tri {
    if state.head_records().any(|h| chosen(sel, cs, h)) {
        indexed_test(cs, state)
    } else {
        Tri::No
    }
}

fn keys_of(
    b: &By,
    state: &LiveState<'_>,
    sel: &impl Fn(&Record) -> bool,
    cs: &[Cond],
) -> Option<Vec<String>> {
    let mut keys = Vec::new();
    if let By::Payload(f) = b {
        for r in state.setters(f) {
            keys.extend(b.keys(r)?);
        }
    } else {
        for r in state.head_records().filter(|r| chosen(sel, cs, r)) {
            keys.extend(b.keys(r)?);
        }
    }
    Some(canon_set(&keys))
}

fn selected_value(
    ty: &FoldType,
    things: &[Thing<'_>],
    ev: impl Fn(&Thing<'_>) -> Tri,
) -> SlotValue {
    let mut yes = Vec::new();
    let mut unread = 0;
    for t in things {
        match ev(t) {
            Tri::Yes => yes.push(t),
            Tri::Unknown => unread += 1,
            Tri::No => {}
        }
    }
    value_for(ty, &yes, unread)
}

fn numeric<T>(
    acc: &families::Acc<T, impl Fn(T, &Record) -> T, impl Fn(&Record) -> bool>,
    field: &str,
    yes: &[&Thing<'_>],
    mut unread: usize,
) -> FoldResult<T>
where
    T: Clone,
{
    let mut value = acc.init.clone();
    for t in yes {
        let setters = t.state.setters(field);
        if let [r] = setters.as_slice() {
            if (acc.readable)(r) {
                value = (acc.step)(value, r);
                continue;
            }
        }
        unread += 1;
    }
    FoldResult {
        value,
        unread: Nat(unread.into()),
    }
}

fn keyed_view(state: &LiveState<'_>) -> KeyedView {
    let ids = |rs: Vec<&Record>| rs.into_iter().map(|r| (r.id, r.shown)).collect();
    KeyedView {
        keys: state
            .keys()
            .into_iter()
            .map(|key| {
                let setters = ids(state.setters(&key));
                (key, setters)
            })
            .collect(),
        whole: ids(state.whole()),
        heads: shown_ids(&state.heads()),
    }
}

fn value_for(ty: &FoldType, yes: &[&Thing<'_>], unread: usize) -> SlotValue {
    match ty {
        FoldType::Count { .. } => SlotValue::Nat(FoldResult {
            value: Nat(yes.len().into()),
            unread: Nat(unread.into()),
        }),
        FoldType::Sum { field, .. } => {
            SlotValue::Int(numeric(&families::sum_on(field), field, yes, unread))
        }
        FoldType::Min { field, .. } => {
            SlotValue::Opt(numeric(&families::min_on(field), field, yes, unread))
        }
        FoldType::Max { field, .. } => {
            SlotValue::Opt(numeric(&families::max_on(field), field, yes, unread))
        }
        FoldType::LatestBy { .. } => SlotValue::Keyed {
            things: yes
                .iter()
                .map(|t| (t.name.clone(), keyed_view(&t.state)))
                .collect(),
            unread: Nat(unread.into()),
        },
        FoldType::Collect { limit, .. } => {
            let rs = ordered_forms(
                yes.iter()
                    .flat_map(|t| t.state.live().map(Record::unlinked))
                    .collect(),
            );
            SlotValue::Heads {
                heads: shown_ids(&whole_things(limit.as_ref(), rs)),
                unread: Nat(unread.into()),
            }
        }
    }
}

struct Groups<T> {
    buckets: HashMap<String, Vec<T>>,
    unread: usize,
    ungrouped: usize,
}

impl<T: Copy> Groups<T> {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            unread: 0,
            ungrouped: 0,
        }
    }
    fn join(&mut self, item: T, ev: Tri, keys: impl FnOnce() -> Option<Vec<String>>) {
        match ev {
            Tri::No => {}
            Tri::Unknown => self.unread += 1,
            Tri::Yes => match keys() {
                None => self.unread += 1,
                Some(ks) if ks.is_empty() => self.ungrouped += 1,
                Some(ks) => {
                    for k in ks {
                        self.buckets.entry(k).or_default().push(item);
                    }
                }
            },
        }
    }
    fn finish(self, mut value: impl FnMut(&str, Vec<T>) -> GroupValue) -> SlotValue {
        let mut groups: Vec<_> = self
            .buckets
            .into_iter()
            .map(|(k, members)| {
                let v = value(&k, members);
                (k, v)
            })
            .collect();
        groups.sort_by(|a, b| str_cmp(&a.0, &b.0));
        SlotValue::Groups {
            groups,
            unread: Nat(self.unread.into()),
            ungrouped: Nat(self.ungrouped.into()),
        }
    }
}

/// Lean: `Loops.groupKeys`, model/Loops/Spec.lean.
/// Every key of a selected record, once, in canonical order.
pub fn group_keys(b: &By, ev: impl Fn(&Record) -> Tri, pool: &[Record]) -> Vec<String> {
    canon_set(
        &pool
            .iter()
            .filter(|r| ev(r) == Tri::Yes)
            .flat_map(|r| b.keys(r).unwrap_or_default())
            .collect::<Vec<_>>(),
    )
}

/// Lean: `Loops.inGroup`, model/Loops/Spec.lean.
/// A record is in group `g` when the slot selects it and `g` is one of its keys.
pub fn in_group(b: &By, ev: impl Fn(&Record) -> Tri, g: &str, r: &Record) -> bool {
    ev(r) == Tri::Yes && b.keys(r).unwrap_or_default().iter().any(|k| k == g)
}

impl Slot {
    /// Lean: `Loops.Slot.run`, model/Loops/Spec.lean.
    /// Runs one slot: grouped when it has `by`, plain otherwise.
    pub fn run(&self, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
        match &self.group {
            None => self.plain(sel, xs),
            Some(b) if self.fold_type.follow() == Some(Follow::Keys) => {
                self.keyed_grouped(b, sel, xs)
            }
            Some(b) => self.grouped(b, sel, xs),
        }
    }

    /// Lean: `Loops.Slot.ev`, model/Loops/Spec.lean.
    /// Whether the slot selects a record: `from` takes it and its `where` answers.
    pub fn ev(&self, sel: impl Fn(&Record) -> bool, r: &Record) -> Tri {
        if sel(r) {
            Cond::all(&self.tests.conds(), r)
        } else {
            Tri::No
        }
    }

    /// Lean: `Loops.Slot.pool`, model/Loops/Spec.lean.
    /// Every record held, or following `prev` the whole-reader heads.
    pub fn pool(&self, xs: &[Record]) -> Vec<Record> {
        match &self.fold_type {
            FoldType::Count { follow }
            | FoldType::Sum { follow, .. }
            | FoldType::Min { follow, .. }
            | FoldType::Max { follow, .. }
            | FoldType::Collect { follow, .. } => acc_pool(*follow, xs),
            FoldType::LatestBy { .. } => xs.to_vec(),
        }
    }

    /// Gives what Lean `Loops.Slot.grouped` gives. Buckets records after whole-reader head resolution, then folds each bucket.
    /// Settling, admission and the whole-reader heads are already done over
    /// everything held; each group runs the slot without `by` or `where` over
    /// all of it, choosing only its own records.
    pub fn grouped(&self, b: &By, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
        if self.fold_type.follow() == Some(Follow::Keys) {
            // Direct callers can request record grouping for a keyed fold.
            return self.group_records_by_selection(b, sel, xs);
        }
        let cs = self.tests.conds();
        let ev = |r: &Record| if sel(r) { Cond::all(&cs, r) } else { Tri::No };
        let pool = self.pool(xs);
        let mut groups = Groups::new();
        for r in &pool {
            groups.join(r, ev(r), || b.keys(r));
        }
        // collectOf(prev) retains conflicting forms of a head, while the
        // group discovery pool keeps just the last form per id.
        let mut collect_groups = HashMap::<String, Vec<Record>>::new();
        if matches!(
            &self.fold_type,
            FoldType::Collect {
                follow: Some(Follow::Prev),
                ..
            }
        ) {
            for (_, state) in per_thing(xs) {
                for r in state.heads {
                    if ev(&r) == Tri::Yes {
                        for k in b.keys(&r).unwrap_or_default() {
                            if groups.buckets.contains_key(&k) {
                                collect_groups.entry(k).or_default().push(r.clone());
                            }
                        }
                    }
                }
            }
        }
        groups.finish(|k, members| {
            let rows: Vec<_> = members.into_iter().cloned().collect();
            match &self.fold_type {
                FoldType::Count { .. } => GroupValue::Nat(FoldResult {
                    value: Nat(rows.len().into()),
                    unread: Nat::default(),
                }),
                FoldType::Sum { field, .. } => {
                    GroupValue::Int(families::sum_on(field).select(|_| Tri::Yes, &rows))
                }
                FoldType::Min { field, .. } => {
                    GroupValue::Opt(families::min_on(field).select(|_| Tri::Yes, &rows))
                }
                FoldType::Max { field, .. } => {
                    GroupValue::Opt(families::max_on(field).select(|_| Tri::Yes, &rows))
                }
                FoldType::LatestBy { .. } => GroupValue::Heads {
                    heads: Vec::new(),
                    unread: Nat::default(),
                },
                FoldType::Collect { limit, follow } => {
                    let rows = if *follow == Some(Follow::Prev) {
                        collect_groups.remove(k).unwrap_or_default()
                    } else {
                        rows
                    };
                    GroupValue::Heads {
                        heads: shown_ids(limit_to(limit.as_ref(), &newest_first(&rows))),
                        unread: Nat::default(),
                    }
                }
            }
        })
    }

    // A direct call to grouped on a keyed fold discovers group names on
    // records, but includes a thing only when one of its heads joins.
    fn group_records_by_selection(
        &self,
        b: &By,
        sel: impl Fn(&Record) -> bool,
        xs: &[Record],
    ) -> SlotValue {
        let cs = self.tests.conds();
        let ev = |r: &Record| if sel(r) { Cond::all(&cs, r) } else { Tri::No };
        let mut groups = Groups::new();
        for r in xs {
            groups.join((), ev(r), || b.keys(r));
        }
        let things = index_things(xs);
        let mut members = HashMap::<String, Vec<&Thing<'_>>>::new();
        for t in &things {
            let keys: HashSet<_> = t
                .state
                .head_records()
                .filter(|r| ev(r) == Tri::Yes)
                .flat_map(|r| b.keys(r).unwrap_or_default())
                .collect();
            for k in keys {
                members.entry(k).or_default().push(t);
            }
        }
        groups.finish(|k, _| {
            value_for(
                &self.fold_type,
                members.get(k).map_or(&[], Vec::as_slice),
                0,
            )
            .group()
        })
    }

    /// Gives what Lean `Loops.Slot.keyedGrouped` gives. Buckets selected things once by their indexed group keys.
    /// A slot with `by` that follows `keys`: each selected thing joins the
    /// group of each of its keys, once in each; each group is the slot without
    /// `by` over its things. Unknown tests and unreadable keys are unread at
    /// the top, once a thing; a thing with no key is ungrouped.
    pub fn keyed_grouped(&self, b: &By, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
        self.keyed_groups(b, &sel, &index_things(xs))
    }

    fn keyed_groups(&self, b: &By, sel: &impl Fn(&Record) -> bool, ts: &[Thing<'_>]) -> SlotValue {
        let cs = self.tests.conds();
        let mut groups = Groups::new();
        for t in ts {
            groups.join(t, evaluate_thing(&t.state, sel, &cs), || {
                keys_of(b, &t.state, sel, &cs)
            });
        }
        groups.finish(|_, members| value_for(&self.fold_type, &members, 0).group())
    }

    fn keyed_slot(&self, sel: &impl Fn(&Record) -> bool, ts: &[Thing<'_>]) -> SlotValue {
        if let Some(b) = &self.group {
            return self.keyed_groups(b, sel, ts);
        }
        let cs = self.tests.conds();
        selected_value(&self.fold_type, ts, |t| evaluate_thing(&t.state, sel, &cs))
    }
    /// Gives what Lean `Loops.Slot.plain` gives. Uses the indexed thing pool for keyed folds.
    /// Runs one slot without `by`, admission, settlement, or duplicate-id removal.
    pub fn plain(&self, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
        let cs = self.tests.conds();
        let ev = |r: &Record| if sel(r) { Cond::all(&cs, r) } else { Tri::No };
        if self.fold_type.follow() == Some(Follow::Keys)
            && !matches!(self.fold_type, FoldType::LatestBy { .. })
        {
            return selected_value(&self.fold_type, &index_things(xs), |t| {
                evaluate_thing(&t.state, &sel, &cs)
            });
        }
        let pool = |follow| {
            if follow == Some(Follow::Prev) {
                std::borrow::Cow::Owned(whole_heads(xs))
            } else {
                std::borrow::Cow::Borrowed(xs)
            }
        };
        match &self.fold_type {
            FoldType::Count { follow } => {
                SlotValue::Nat(families::count_acc().select(ev, &pool(*follow)))
            }
            FoldType::Sum { field, follow } => {
                SlotValue::Int(families::sum_on(field).select(ev, &pool(*follow)))
            }
            FoldType::Min { field, follow } => {
                SlotValue::Opt(families::min_on(field).select(ev, &pool(*follow)))
            }
            FoldType::Max { field, follow } => {
                SlotValue::Opt(families::max_on(field).select(ev, &pool(*follow)))
            }
            FoldType::LatestBy {
                follow: Follow::Prev,
            } => latest_of(sel, &cs, xs),
            FoldType::LatestBy {
                follow: Follow::Keys,
            } => keyed_of(sel, &cs, xs),
            FoldType::Collect { follow, limit } => {
                collect_of(sel, &cs, *follow == Some(Follow::Prev), limit.as_ref(), xs)
            }
        }
    }

    /// Lean: `Loops.Slot.toJson`, model/Loops/Spec.lean.
    /// Encodes the fold before follow, limit, and optional where members.
    pub fn to_json(&self) -> (String, Json) {
        let follow_json = |follow: Option<Follow>| {
            follow.map_or_else(Vec::new, |f| {
                vec![("follow".into(), Json::Str(f.name().into()))]
            })
        };
        let with_follow = |kv: (String, Json), follow: Option<Follow>| {
            let mut out = vec![kv];
            out.extend(follow_json(follow));
            out
        };
        let mut kvs = match &self.fold_type {
            FoldType::Count { follow } => with_follow(("count".into(), Json::Bool(true)), *follow),
            FoldType::Sum { field, follow } => {
                with_follow(("sum".into(), Json::Str(field.clone())), *follow)
            }
            FoldType::Min { field, follow } => {
                with_follow(("min".into(), Json::Str(field.clone())), *follow)
            }
            FoldType::Max { field, follow } => {
                with_follow(("max".into(), Json::Str(field.clone())), *follow)
            }
            FoldType::LatestBy { follow } => vec![
                ("latest-by".into(), Json::Str("of".into())),
                ("follow".into(), Json::Str(follow.name().into())),
            ],
            FoldType::Collect { follow, limit } => {
                let mut out = with_follow(("collect".into(), Json::Bool(true)), *follow);
                if let Some(k) = limit {
                    out.push(("limit".into(), Json::Int(Int(k.0.clone().into()))));
                }
                out
            }
        };
        if let Some(b) = &self.group {
            kvs.push(("by".into(), Json::Str(b.name())));
        }
        if !self.tests.0.is_empty() {
            kvs.push(("where".into(), self.tests.to_json()));
        }
        (self.name.clone(), Json::Obj(kvs))
    }
}

/// Lean: `Loops.Source`, model/Loops/Spec.lean.
/// A source selects by envelope data. The caller resolves its store name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    /// The store that supplies records before execution.
    pub store: String,
    /// The allowed kinds, or all kinds when empty.
    pub kinds: Vec<String>,
    /// The mention `about` must hold, as written, or any record when absent.
    pub about: Option<String>,
    /// The selected thing, or all things when absent.
    pub of: Option<String>,
}
impl Source {
    /// Lean: `Loops.Source.takes`, model/Loops/Spec.lean.
    /// Tests the kind, what `about` mentions as written, and the record's thing,
    /// which for a record without `of` is its own id.
    pub fn takes(&self, r: &Record) -> bool {
        (self.kinds.is_empty() || self.kinds.contains(&r.kind))
            && self.about.as_ref().is_none_or(|t| r.about.contains(t))
            && self.of.as_ref().is_none_or(|t| r.is_of(t))
    }
    /// Lean: `Loops.Source.toJson`, model/Loops/Spec.lean.
    /// Encodes store, optional kinds, optional mention, then optional thing.
    pub fn to_json(&self) -> Json {
        let mut kvs = vec![("store".into(), Json::Str(self.store.clone()))];
        if !self.kinds.is_empty() {
            kvs.push((
                "kind".into(),
                Json::Arr(self.kinds.iter().cloned().map(Json::Str).collect()),
            ));
        }
        if let Some(a) = &self.about {
            kvs.push(("about".into(), Json::Str(a.clone())));
        }
        if let Some(t) = &self.of {
            kvs.push(("of".into(), Json::Str(t.clone())));
        }
        Json::Obj(kvs)
    }
}

/// Lean: `Loops.Spec`, model/Loops/Spec.lean.
/// A vertex spec keeps source and slot order. It discards accepted unused JSON keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Spec {
    /// The vertex name.
    pub vertex: String,
    /// The sources, in written order.
    pub sources: Vec<Source>,
    /// The named folds, in written order.
    pub slots: Vec<Slot>,
}
impl Spec {
    /// Lean: `Loops.Spec.takes`, model/Loops/Spec.lean.
    /// Accepts a record when any source accepts its envelope.
    pub fn takes(&self, r: &Record) -> bool {
        self.sources.iter().any(|s| s.takes(r))
    }
    /// Lean: `Loops.Spec.select`, model/Loops/Spec.lean.
    /// Filters records without admission, settlement, or where tests.
    pub fn select(&self, xs: &[Record]) -> Vec<Record> {
        xs.iter().filter(|r| self.takes(r)).cloned().collect()
    }
    /// Lean: `Loops.Spec.run`, model/Loops/Spec.lean.
    /// Admits records with every replacement permitted, then runs each slot.
    /// The caller supplies one record per id. This method does not settle or remove duplicate ids.
    pub fn run(&self, xs: &[Record]) -> Vec<(String, SlotValue)> {
        self.run_with(xs, |_, _| true)
    }
    /// Lean: `Loops.Spec.run`, model/Loops/Spec.lean.
    /// Admits against the original records, then runs each slot.
    /// The caller supplies a pure `EnvOnly` rule that depends only on both envelopes.
    /// Rust does not enforce this proof requirement. This method does not settle or remove duplicate ids.
    pub fn run_with(
        &self,
        xs: &[Record],
        may: impl Fn(&Record, &Record) -> bool,
    ) -> Vec<(String, SlotValue)> {
        self.run_admitted(&admit(xs, may))
    }
    /// Runs the slots over an already admitted realm, sharing the thing
    /// index across slots that follow keys.
    pub fn run_admitted(&self, ys: &[Record]) -> Vec<(String, SlotValue)> {
        let index = OnceCell::new();
        self.slots
            .iter()
            .map(|s| {
                let sel = |r: &Record| self.takes(r);
                let v = if s.fold_type.follow() == Some(Follow::Keys) {
                    s.keyed_slot(&sel, index.get_or_init(|| index_things(ys)))
                } else {
                    s.run(sel, ys)
                };
                (s.name.clone(), v)
            })
            .collect()
    }
    /// Lean: `Loops.Spec.ofObject`, model/Loops/Spec.lean.
    /// Parses the object without checking duplicate members at any depth.
    ///
    /// # Errors
    /// Returns the first shape or grammar refusal in Lean's fixed check order.
    pub fn of_object(j: &Json) -> Result<Self, Refusal> {
        let Json::Obj(kvs) = j else {
            return Err(refuse(RefusalCode::SpecNotObject, ""));
        };
        check_keys(SPEC_KEYS, None, kvs)?;
        let vertex = spec_vertex(lookup(kvs, "vertex"))?;
        let sources = spec_sources(lookup(kvs, "from"))?;
        let slots = spec_slots(lookup(kvs, "fold"))?;
        Ok(Self {
            vertex,
            sources,
            slots,
        })
    }
    /// Lean: `Loops.Spec.ofJson`, model/Loops/Spec.lean.
    /// Checks raw duplicate members before shape, keys, vertex, sources, and slots.
    ///
    /// # Errors
    /// Returns the first refusal with its exact wire code and dot path.
    pub fn of_json(j: &Json) -> Result<Self, Refusal> {
        if let Some(path) = j.repeat_at(None) {
            return Err(refuse(RefusalCode::MemberDuplicate, &path));
        }
        Self::of_object(j)
    }
    /// Lean: `Loops.Spec.toJson`, model/Loops/Spec.lean.
    /// Encodes vertex, sources, and slots, without accepted unused metadata.
    pub fn to_json(&self) -> Json {
        Json::Obj(vec![
            ("vertex".into(), Json::Str(self.vertex.clone())),
            (
                "from".into(),
                Json::Arr(self.sources.iter().map(Source::to_json).collect()),
            ),
            (
                "fold".into(),
                Json::Obj(self.slots.iter().map(Slot::to_json).collect()),
            ),
        ])
    }
}

fn lookup<'a>(kvs: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    kvs.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}
fn refuse(code: RefusalCode, path: &str) -> Refusal {
    Refusal {
        code,
        path: path.into(),
    }
}

/// Lean: `Loops.memberPath`, model/Loops/Spec.lean.
/// Joins a member path. An empty parent differs from no parent.
pub fn member_path(parent: Option<&str>, key: &str) -> String {
    parent.map_or_else(|| key.into(), |p| format!("{p}.{key}"))
}
impl Json {
    /// Lean: `Loops.Json.repeatAt`, `repeatIn`, and `repeatMember`, model/Loops/Spec.lean.
    /// Finds the first repeat depth first in written member order. Arrays share their parent's path.
    pub fn repeat_at(&self, path: Option<&str>) -> Option<String> {
        match self {
            Self::Arr(xs) => xs.iter().find_map(|x| x.repeat_at(path)),
            Self::Obj(kvs) => {
                for (i, (k, v)) in kvs.iter().enumerate() {
                    let p = member_path(path, k);
                    if kvs[..i].iter().any(|(seen, _)| seen == k) {
                        return Some(p);
                    }
                    if let Some(repeat) = v.repeat_at(Some(&p)) {
                        return Some(repeat);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Lean: `Loops.foldTypes`, model/Loops/Spec.lean.
/// Lists the closed fold vocabulary in declaration order.
pub const FOLD_TYPES: &[&str] = &["count", "sum", "min", "max", "latest-by", "collect"];
/// Lean: `Loops.slotKeys`, model/Loops/Spec.lean.
/// Lists allowed slot keys, including unused note metadata.
pub const SLOT_KEYS: &[&str] = &[
    "count",
    "sum",
    "min",
    "max",
    "latest-by",
    "collect",
    "where",
    "follow",
    "limit",
    "by",
    "note",
];
/// Lean: `Loops.sourceKeys`, model/Loops/Spec.lean.
/// Lists allowed source keys.
pub const SOURCE_KEYS: &[&str] = &["store", "kind", "about", "of"];
/// Lean: `Loops.specKeys`, model/Loops/Spec.lean.
/// Lists allowed root keys, including keys without runtime behavior.
pub const SPEC_KEYS: &[&str] = &[
    "vertex", "note", "from", "fold", "ticks", "order", "resolve", "window", "boundary", "param",
    "covers", "into", "bundle",
];

/// Lean: `Loops.checkKeys`, model/Loops/Spec.lean.
/// Checks allowed member names without checking repeats.
///
/// # Errors
/// Returns unknown-key for the first unknown member in written order.
pub fn check_keys(
    allowed: &[&str],
    path: Option<&str>,
    kvs: &[(String, Json)],
) -> Result<(), Refusal> {
    for (k, _) in kvs {
        if !allowed.contains(&k.as_str()) {
            return Err(refuse(RefusalCode::UnknownKey, &member_path(path, k)));
        }
    }
    Ok(())
}

/// Lean: `Loops.fieldArg`, model/Loops/Spec.lean.
/// Parses a numeric fold's nonempty payload field name.
///
/// # Errors
/// Returns fold-argument at the supplied path for other values.
pub fn field_arg(path: &str, j: &Json) -> Result<String, Refusal> {
    match j {
        Json::Str(s) if !s.is_empty() => Ok(s.clone()),
        _ => Err(refuse(RefusalCode::FoldArgument, path)),
    }
}
/// Lean: `Loops.slotType`, model/Loops/Spec.lean.
/// Finds exactly one written fold member without checking its argument.
///
/// # Errors
/// Returns fold-type for zero or multiple fold members, including repeats.
pub fn slot_type<'a>(path: &str, kvs: &'a [(String, Json)]) -> Result<&'a (String, Json), Refusal> {
    let mut types = kvs.iter().filter(|(k, _)| FOLD_TYPES.contains(&k.as_str()));
    match (types.next(), types.next()) {
        (Some(p), None) => Ok(p),
        _ => Err(refuse(RefusalCode::FoldType, path)),
    }
}
/// Lean: `Loops.slotFollow`, model/Loops/Spec.lean.
/// Parses `follow`: `"prev"` or `"keys"`, which any fold type may carry;
/// latest-by must carry one.
///
/// # Errors
/// Returns follow-missing on latest-by, or follow-invalid for any other value.
pub fn slot_follow(path: &str, t: &str, j: Option<&Json>) -> Result<Option<Follow>, Refusal> {
    match j {
        None if t == "latest-by" => Err(refuse(
            RefusalCode::FollowMissing,
            &format!("{path}.follow"),
        )),
        None => Ok(None),
        Some(Json::Str(s)) if s == "prev" => Ok(Some(Follow::Prev)),
        Some(Json::Str(s)) if s == "keys" => Ok(Some(Follow::Keys)),
        Some(_) => Err(refuse(
            RefusalCode::FollowInvalid,
            &format!("{path}.follow"),
        )),
    }
}
/// Lean: `Loops.slotLimit`, model/Loops/Spec.lean.
/// Parses an unbounded positive collect limit.
///
/// # Errors
/// Returns limit-invalid when a present limit fails the fold or value check.
pub fn slot_limit(path: &str, t: &str, j: Option<&Json>) -> Result<Option<Nat>, Refusal> {
    match j {
        None => Ok(None),
        Some(Json::Int(i)) if t == "collect" && *i > Int::default() => {
            Ok(i.0.to_biguint().map(Nat))
        }
        Some(_) => Err(refuse(RefusalCode::LimitInvalid, &format!("{path}.limit"))),
    }
}
/// Lean: `Loops.slotBy`, model/Loops/Spec.lean.
/// Parses `by` on count, sum, min, max and collect, following or not.
///
/// # Errors
/// Returns by-invalid for anything but a `by` name, or on latest-by.
pub fn slot_by(path: &str, t: &str, j: Option<&Json>) -> Result<Option<By>, Refusal> {
    let invalid = || refuse(RefusalCode::ByInvalid, &format!("{path}.by"));
    match j {
        None => Ok(None),
        Some(Json::Str(s)) => match By::of_name(s) {
            Some(_) if t == "latest-by" => Err(invalid()),
            Some(b) => Ok(Some(b)),
            None => Err(invalid()),
        },
        Some(_) => Err(invalid()),
    }
}
/// Lean: `Loops.foldType`, model/Loops/Spec.lean.
/// Parses a fold argument without validating the separate follow and limit parameters.
///
/// # Errors
/// Returns fold-argument at path.type for an invalid argument or unknown fold type.
pub fn fold_type(
    path: &str,
    t: &str,
    follow: Option<Follow>,
    limit: Option<Nat>,
    arg: &Json,
) -> Result<FoldType, Refusal> {
    let p = format!("{path}.{t}");
    let latest = follow.unwrap_or(Follow::Prev);
    match (t, arg) {
        ("count", Json::Bool(true)) => Ok(FoldType::Count { follow }),
        ("sum", f) => field_arg(&p, f).map(|field| FoldType::Sum { field, follow }),
        ("min", f) => field_arg(&p, f).map(|field| FoldType::Min { field, follow }),
        ("max", f) => field_arg(&p, f).map(|field| FoldType::Max { field, follow }),
        ("latest-by", Json::Str(s)) if s == "of" => Ok(FoldType::LatestBy { follow: latest }),
        ("collect", Json::Bool(true)) => Ok(FoldType::Collect { follow, limit }),
        _ => Err(refuse(RefusalCode::FoldArgument, &p)),
    }
}
/// Lean: `Loops.slotTests`, model/Loops/Spec.lean.
/// Parses an optional where map without checking duplicate names.
///
/// # Errors
/// Returns the first where-invalid at the slot's where path.
pub fn slot_tests(path: &str, j: Option<&Json>) -> Result<Tests, Refusal> {
    j.map_or_else(
        || Ok(Tests::default()),
        |w| parse_where(&format!("{path}.where"), w),
    )
}
/// Lean: `Loops.parseSlot`, model/Loops/Spec.lean.
/// Checks name, keys, fold count, follow, limit, by, argument, then where.
///
/// # Errors
/// Returns the first slot refusal in that order, without duplicate validation.
pub fn parse_slot(name: &str, j: &Json) -> Result<Slot, Refusal> {
    let path = member_path(Some("fold"), name);
    if name.is_empty() {
        return Err(refuse(RefusalCode::FoldInvalid, &path));
    }
    let Json::Obj(kvs) = j else {
        return Err(refuse(RefusalCode::FoldInvalid, &path));
    };
    check_keys(SLOT_KEYS, Some(&path), kvs)?;
    let (t, arg) = slot_type(&path, kvs)?;
    let follow = slot_follow(&path, t, lookup(kvs, "follow"))?;
    let limit = slot_limit(&path, t, lookup(kvs, "limit"))?;
    let group = slot_by(&path, t, lookup(kvs, "by"))?;
    let fold_type = fold_type(&path, t, follow, limit, arg)?;
    let tests = slot_tests(&path, lookup(kvs, "where"))?;
    Ok(Slot {
        name: name.into(),
        fold_type,
        tests,
        group,
    })
}
/// Lean: `Loops.parseKinds`, model/Loops/Spec.lean.
/// Parses one or more nonempty kind names and keeps repeats.
///
/// # Errors
/// Returns from-invalid at the supplied path for invalid shapes or names.
pub fn parse_kinds(path: &str, j: &Json) -> Result<Vec<String>, Refusal> {
    let Json::Arr(ks) = j else {
        return Err(refuse(RefusalCode::FromInvalid, path));
    };
    if ks.is_empty() {
        return Err(refuse(RefusalCode::FromInvalid, path));
    }
    ks.iter()
        .map(|k| match k {
            Json::Str(s) if !s.is_empty() => Ok(s.clone()),
            _ => Err(refuse(RefusalCode::FromInvalid, path)),
        })
        .collect()
}
/// Lean: `Loops.sourceStore`, model/Loops/Spec.lean.
/// Parses the required nonempty store name.
///
/// # Errors
/// Returns from-invalid at from.store when the store is absent or invalid.
pub fn source_store(j: Option<&Json>) -> Result<String, Refusal> {
    match j {
        Some(Json::Str(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(refuse(RefusalCode::FromInvalid, "from.store")),
    }
}
/// Lean: `Loops.sourceKinds`, model/Loops/Spec.lean.
/// Parses optional kinds. Absence permits every kind.
///
/// # Errors
/// Returns from-invalid at from.kind for invalid present kinds.
pub fn source_kinds(j: Option<&Json>) -> Result<Vec<String>, Refusal> {
    j.map_or_else(|| Ok(Vec::new()), |k| parse_kinds("from.kind", k))
}
/// Lean: `Loops.sourceAbout`, model/Loops/Spec.lean.
/// Parses an optional typed reference that `about` must mention.
///
/// # Errors
/// Returns from-invalid at from.about for an invalid present reference.
pub fn source_about(j: Option<&Json>) -> Result<Option<String>, Refusal> {
    match j {
        None => Ok(None),
        Some(Json::Str(a)) if typed_ref(a) => Ok(Some(a.clone())),
        Some(_) => Err(refuse(RefusalCode::FromInvalid, "from.about")),
    }
}
/// Lean: `Loops.sourceOf`, model/Loops/Spec.lean.
/// Parses an optional typed thing reference.
///
/// # Errors
/// Returns from-invalid at from.of for an invalid present reference.
pub fn source_of(j: Option<&Json>) -> Result<Option<String>, Refusal> {
    match j {
        None => Ok(None),
        Some(Json::Str(t)) if typed_ref(t) => Ok(Some(t.clone())),
        Some(_) => Err(refuse(RefusalCode::FromInvalid, "from.of")),
    }
}
/// Lean: `Loops.parseSource`, model/Loops/Spec.lean.
/// Checks source keys, store, kinds, about, then of, without checking duplicates.
///
/// # Errors
/// Returns the first source refusal in that fixed order.
pub fn parse_source(j: &Json) -> Result<Source, Refusal> {
    let Json::Obj(kvs) = j else {
        return Err(refuse(RefusalCode::FromInvalid, "from"));
    };
    check_keys(SOURCE_KEYS, Some("from"), kvs)?;
    let store = source_store(lookup(kvs, "store"))?;
    let kinds = source_kinds(lookup(kvs, "kind"))?;
    let about = source_about(lookup(kvs, "about"))?;
    let of = source_of(lookup(kvs, "of"))?;
    Ok(Source {
        store,
        kinds,
        about,
        of,
    })
}
/// Lean: `Loops.specVertex`, model/Loops/Spec.lean.
/// Parses the required nonempty vertex name.
///
/// # Errors
/// Returns vertex-missing for absence, or vertex-invalid for other invalid input.
pub fn spec_vertex(j: Option<&Json>) -> Result<String, Refusal> {
    match j {
        None => Err(refuse(RefusalCode::VertexMissing, "vertex")),
        Some(Json::Str(s)) if !s.is_empty() => Ok(s.clone()),
        Some(_) => Err(refuse(RefusalCode::VertexInvalid, "vertex")),
    }
}
/// Lean: `Loops.specSources`, model/Loops/Spec.lean.
/// Parses a nonempty source array in written order.
///
/// # Errors
/// Returns from-missing for absence or an empty array, or the first source refusal.
pub fn spec_sources(j: Option<&Json>) -> Result<Vec<Source>, Refusal> {
    match j {
        None => Err(refuse(RefusalCode::FromMissing, "from")),
        Some(Json::Arr(xs)) if xs.is_empty() => Err(refuse(RefusalCode::FromMissing, "from")),
        Some(Json::Arr(xs)) => xs.iter().map(parse_source).collect(),
        Some(_) => Err(refuse(RefusalCode::FromInvalid, "from")),
    }
}
/// Lean: `Loops.specSlots`, model/Loops/Spec.lean.
/// Parses optional named slots in written order.
///
/// # Errors
/// Returns fold-invalid for a nonobject fold, or the first slot refusal.
pub fn spec_slots(j: Option<&Json>) -> Result<Vec<Slot>, Refusal> {
    match j {
        None => Ok(Vec::new()),
        Some(Json::Obj(fs)) => fs.iter().map(|(n, s)| parse_slot(n, s)).collect(),
        Some(_) => Err(refuse(RefusalCode::FoldInvalid, "fold")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        occurred::Stamp,
        where_::{Field, Op},
    };

    fn record(id: u8, of: &str, hour: u8) -> Record {
        let mut bytes = [0; 32];
        bytes[31] = id;
        Record {
            id: Digest(bytes),
            kind: "keep".into(),
            of: Some(of.into()),
            emitted: Stamp::of(&format!("2026-10-29T{hour:02}:00:00Z"))
                .expect("valid test timestamp"),
            ..Record::default()
        }
    }

    #[test]
    fn newest_first_clears_links_but_preserves_distinct_raw_forms() {
        let original = record(1, "thing:a", 2);
        let linked = Record {
            prev: vec![Digest([9; 32])],
            ..original.clone()
        };
        let hidden = linked.withhold();
        assert_eq!(
            newest_first(&[original.clone(), linked, hidden.clone()]),
            vec![original, hidden.unlinked()]
        );
        assert_eq!(newest_first(&[]), Vec::<Record>::new());
    }

    #[test]
    fn a_limit_over_things_lists_each_thing_whole() {
        // Newest first, two things interleaved in time.
        let rs = newest_first(&[
            record(1, "thing:a", 1),
            record(2, "thing:b", 2),
            record(3, "thing:a", 3),
            record(4, "thing:b", 4),
            record(5, "thing:a", 5),
        ]);
        let ids = |k: Option<usize>| -> Vec<u8> {
            whole_things(k.map(|k| Nat(k.into())).as_ref(), rs.clone())
                .iter()
                .map(|r| r.id.0[31])
                .collect()
        };
        assert_eq!(
            things_in_order(&rs),
            vec!["thing:a".to_string(), "thing:b".to_string()]
        );
        assert_eq!(ids(None), vec![5, 4, 3, 2, 1]);
        assert_eq!(ids(Some(0)), Vec::<u8>::new());
        assert_eq!(ids(Some(1)), vec![5, 3, 1]);
        assert_eq!(ids(Some(2)), vec![5, 4, 3, 2, 1]);
        assert_eq!(ids(Some(3)), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn head_slots_resolve_every_thing_before_source_selection() {
        let shared = record(1, "thing:b", 4);
        let old = record(2, "thing:a", 1);
        let successor = Record {
            kind: "drop".into(),
            prev: vec![shared.id, old.id],
            ..record(3, "thing:a", 2)
        };
        let fork = record(4, "thing:b", 3);
        let xs = [old.clone(), shared.clone(), successor, fork.clone()];
        let select = |r: &Record| r.kind == "keep";
        assert_eq!(
            collect_of(select, &[], true, None, &xs),
            SlotValue::Heads {
                heads: vec![(shared.id, Shown::Here), (fork.id, Shown::Here)],
                unread: Nat::default(),
            }
        );
        assert_eq!(
            latest_of(select, &[], &xs),
            SlotValue::Things {
                things: vec![(
                    "thing:b".into(),
                    vec![(shared.id, Shown::Here), (fork.id, Shown::Here)]
                )],
                unread: Nat::default(),
            }
        );
        assert_eq!(
            collect_of(select, &[], false, None, &xs),
            SlotValue::Heads {
                heads: vec![
                    (shared.id, Shown::Here),
                    (fork.id, Shown::Here),
                    (old.id, Shown::Here)
                ],
                unread: Nat::default(),
            }
        );
    }

    #[test]
    fn raw_collect_counts_conflicting_unknown_forms_but_latest_counts_ids() {
        let hidden = record(1, "thing:a", 4).withhold();
        let erased = hidden.erase();
        assert_eq!(hidden.id, erased.id);
        assert_ne!(hidden, erased);
        let xs = [hidden, erased];
        let tests = [Cond {
            field: Field::Payload("missing".into()),
            op: Op::Absent,
        }];
        for follow in [false, true] {
            for limit in [None, Some(Nat::default())] {
                assert_eq!(
                    collect_of(|_| true, &tests, follow, limit.as_ref(), &xs),
                    SlotValue::Heads {
                        heads: vec![],
                        unread: Nat::from(2_u64),
                    },
                    "follow {follow}, limit {limit:?}"
                );
            }
        }
        assert_eq!(
            latest_of(|_| true, &tests, &xs),
            SlotValue::Things {
                things: vec![],
                unread: Nat::from(1_u64),
            }
        );
    }

    #[test]
    fn addressed_head_slots_count_unread_ids_once_before_limiting_known_heads() {
        let hidden = record(1, "thing:a", 4).withhold();
        let first = record(2, "thing:a", 3);
        let second = record(3, "thing:b", 2);
        let xs = [
            hidden.clone(),
            hidden.clone(),
            second.clone(),
            first.clone(),
        ];
        // Addressed permits identical repeats: equal ids must imply equal records.
        assert!(xs.iter().all(|a| xs.iter().all(|b| a.id != b.id || a == b)));
        let tests = [Cond {
            field: Field::Payload("missing".into()),
            op: Op::Absent,
        }];
        for follow in [false, true] {
            for (limit, heads) in [
                (
                    None,
                    vec![(first.id, Shown::Here), (second.id, Shown::Here)],
                ),
                (Some(Nat::from(1_u64)), vec![(first.id, Shown::Here)]),
                (Some(Nat::default()), vec![]),
            ] {
                assert_eq!(
                    collect_of(|_| true, &tests, follow, limit.as_ref(), &xs),
                    SlotValue::Heads {
                        heads,
                        unread: Nat::from(1_u64)
                    },
                    "follow {follow}, limit {limit:?}"
                );
            }
        }
        assert_eq!(
            latest_of(|_| true, &tests, &xs),
            SlotValue::Things {
                things: vec![
                    ("thing:a".into(), vec![(first.id, Shown::Here)]),
                    ("thing:b".into(), vec![(second.id, Shown::Here)]),
                ],
                unread: Nat::from(1_u64),
            }
        );
        assert_eq!(
            collect_of(|r| r.id != hidden.id, &tests, true, None, &xs),
            SlotValue::Heads {
                heads: vec![(first.id, Shown::Here), (second.id, Shown::Here)],
                unread: Nat::default(),
            }
        );
    }

    #[test]
    fn keyed_slots_count_a_fork_once_and_leave_two_setters_unread() {
        // A root read whole and two concurrent emits that each set `n`: one
        // thing with two heads, and two live setters of `n` that hold the same
        // number.
        let root = Record {
            n: Int::from(3_i64),
            ..record(1, "thing:a", 1)
        };
        let set = |id, hour| Record {
            prev: vec![root.id],
            sets: Some(vec!["n".into()]),
            n: Int::from(5_i64),
            ..record(id, "thing:a", hour)
        };
        let xs = [root.clone(), set(2, 2), set(3, 3)];
        let slot = |fold_type| Slot {
            name: "s".into(),
            fold_type,
            tests: Tests::default(),
            group: None,
        };
        let count = |follow| slot(FoldType::Count { follow }).run(|_| true, &xs);
        let sum = |follow| {
            slot(FoldType::Sum {
                field: "n".into(),
                follow,
            })
            .run(|_| true, &xs)
        };
        let n = |value: u8, unread: u8| FoldResult {
            value: Nat(value.into()),
            unread: Nat(unread.into()),
        };
        assert_eq!(count(Some(Follow::Keys)), SlotValue::Nat(n(1, 0)));
        assert_eq!(count(Some(Follow::Prev)), SlotValue::Nat(n(2, 0)));
        assert_eq!(
            sum(Some(Follow::Keys)),
            SlotValue::Int(FoldResult {
                value: Int::default(),
                unread: Nat(1_u8.into()),
            })
        );
        assert_eq!(
            sum(Some(Follow::Prev)),
            SlotValue::Int(FoldResult {
                value: Int::from(10_i64),
                unread: Nat::default(),
            })
        );
        // Collect lists the thing's live records: both setters, and the root,
        // which they replace for `n` only, so it is still live for every other key.
        let SlotValue::Heads { heads, .. } = slot(FoldType::Collect {
            follow: Some(Follow::Keys),
            limit: None,
        })
        .run(|_| true, &xs) else {
            panic!("collect holds heads")
        };
        assert_eq!(heads.len(), 3);
    }
}

//! List-scan oracles for differential tests of the indexed read paths.
use super::{
    families::{self, Result as FoldResult, Tri},
    fold::{LatestBy, insert_id, insert_rec},
    json::canon_set,
    live::Keyed,
    number::Nat,
    record::{Digest, Record, Shown},
    spec::*,
    things::{entitled, pick, things_of},
    where_::{Cond, Field, Op, Tests},
};
use std::collections::HashSet;
fn ids_of(xs: &[Record]) -> Vec<Digest> {
    xs.iter().fold(Vec::new(), |ids, r| insert_id(r.id, &ids))
}

fn settle(xs: &[Record]) -> Vec<Record> {
    ids_of(xs).iter().filter_map(|id| pick(xs, id)).collect()
}

fn admit(xs: &[Record], may: impl Fn(&Record, &Record) -> bool) -> Vec<Record> {
    xs.iter()
        .filter(|r| entitled(xs, r, &may))
        .cloned()
        .collect()
}

fn state_in(thing: &str, xs: &[Record]) -> LatestBy {
    let mut state = LatestBy::default();
    state.fold(
        &xs.iter()
            .filter(|r| r.is_of(thing))
            .cloned()
            .collect::<Vec<_>>(),
    );
    state
}

fn per_thing(xs: &[Record]) -> Vec<(String, LatestBy)> {
    things_of(xs)
        .into_iter()
        .map(|t| {
            let state = state_in(&t, xs);
            (t, state)
        })
        .collect()
}

fn order_heads(xs: &[Record]) -> Vec<Record> {
    xs.iter()
        .enumerate()
        .filter(|(i, r)| !xs[i + 1..].iter().any(|later| later.id == r.id))
        .fold(Vec::new(), |heads, (_, r)| insert_rec(r.clone(), &heads))
}

fn latest_fold(xs: &[Record]) -> LatestBy {
    let named = per_thing(xs)
        .into_iter()
        .flat_map(|(_, state)| state.named)
        .fold(Vec::new(), |ids, id| insert_id(id, &ids));
    LatestBy {
        named,
        heads: whole_heads(xs),
    }
}

fn whole_heads(xs: &[Record]) -> Vec<Record> {
    order_heads(
        &per_thing(xs)
            .into_iter()
            .flat_map(|(_, state)| state.heads)
            .collect::<Vec<_>>(),
    )
}

fn upward(rs: &[Record], ids: &[Digest]) -> HashSet<Digest> {
    let mut reached: HashSet<Digest> = ids.iter().copied().collect();
    let mut grew = true;
    while grew {
        grew = false;
        for q in rs {
            if !reached.contains(&q.id) && q.prev.iter().any(|p| reached.contains(p)) {
                reached.insert(q.id);
                grew = true;
            }
        }
    }
    reached
}

fn above<'a>(rs: &'a [Record], r: &Record) -> Vec<&'a Record> {
    let reached = upward(rs, &[r.id]);
    rs.iter()
        .filter(|q| q.prev.iter().any(|p| reached.contains(p)))
        .collect()
}

fn live_for(rs: &[Record], k: &str, r: &Record) -> bool {
    r.sets_key(k) && !above(rs, r).iter().any(|q| q.sets_key(k))
}

fn live_env(rs: &[Record], r: &Record) -> bool {
    above(rs, r).is_empty()
}

fn live(rs: &[Record], r: &Record) -> bool {
    let up = above(rs, r);
    if up.is_empty() {
        return true;
    }
    match &r.sets {
        None => !up.iter().any(|q| q.whole()),
        Some(ks) => ks.iter().any(|k| live_for(rs, k, r)),
    }
}

fn live_keys(rs: &[Record]) -> Vec<String> {
    canon_set(
        &rs.iter()
            .filter(|r| live(rs, r))
            .flat_map(|r| r.declares().iter().cloned())
            .collect::<Vec<_>>(),
    )
}

fn keyed_in(t: &str, xs: &[Record]) -> Keyed {
    let rs: Vec<Record> = xs.iter().filter(|r| r.is_of(t)).cloned().collect();
    let pick = |p: &dyn Fn(&Record) -> bool| {
        order_heads(&rs.iter().filter(|r| p(r)).cloned().collect::<Vec<_>>())
    };
    Keyed {
        keys: live_keys(&rs)
            .into_iter()
            .map(|k| {
                let ss = pick(&|r| live_for(&rs, &k, r));
                (k, ss)
            })
            .collect(),
        whole: pick(&|r| r.whole() && live(&rs, r)),
        heads: state_in(t, xs).heads,
    }
}

fn records_of(t: &str, xs: &[Record]) -> Vec<Record> {
    xs.iter().filter(|r| r.is_of(t)).cloned().collect()
}

fn setters(rs: &[Record], k: &str) -> Vec<Record> {
    order_heads(
        &rs.iter()
            .filter(|r| live_for(rs, k, r))
            .cloned()
            .collect::<Vec<_>>(),
    )
}

fn live_in(xs: &[Record], t: &str) -> Vec<Record> {
    let rs = records_of(t, xs);
    rs.iter().filter(|r| live(&rs, r)).cloned().collect()
}

fn newest_first(xs: &[Record]) -> Vec<Record> {
    let mut state = LatestBy::default();
    state.fold(&xs.iter().map(Record::unlinked).collect::<Vec<_>>());
    state.heads
}

fn things_in_order(rs: &[Record]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in rs {
        let t = r.thing();
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

fn whole_things(limit: Option<&Nat>, rs: Vec<Record>) -> Vec<Record> {
    if limit.is_none() {
        return rs;
    }
    let ts = things_in_order(&rs);
    let kept = limit_to(limit, &ts);
    rs.into_iter()
        .filter(|r| kept.contains(&r.thing()))
        .collect()
}

fn keyed_value(ty: &FoldType, xs: &[Record], ev: impl Fn(&str) -> Tri) -> SlotValue {
    let ts = things_of(xs);
    let yes: Vec<&String> = ts.iter().filter(|t| ev(t) == Tri::Yes).collect();
    let unread = Nat(ts.iter().filter(|t| ev(t) == Tri::Unknown).count().into());
    match ty {
        FoldType::Count { .. } => SlotValue::Nat(FoldResult {
            value: Nat(yes.len().into()),
            unread,
        }),
        FoldType::Sum { field, .. } => SlotValue::Int(families::sum_on(field).over_things(
            |t| sole_setter(xs, field, t),
            &ev,
            &ts,
        )),
        FoldType::Min { field, .. } => SlotValue::Opt(families::min_on(field).over_things(
            |t| sole_setter(xs, field, t),
            &ev,
            &ts,
        )),
        FoldType::Max { field, .. } => SlotValue::Opt(families::max_on(field).over_things(
            |t| sole_setter(xs, field, t),
            &ev,
            &ts,
        )),
        FoldType::LatestBy { .. } => SlotValue::Keyed {
            things: yes
                .into_iter()
                .map(|t| (t.clone(), keyed_in(t, xs).view()))
                .collect(),
            unread,
        },
        FoldType::Collect { limit, .. } => {
            let live = newest_first(&yes.iter().flat_map(|t| live_in(xs, t)).collect::<Vec<_>>());
            SlotValue::Heads {
                heads: shown_ids(&whole_things(limit.as_ref(), live)),
                unread,
            }
        }
    }
}

fn keyed_of(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record]) -> SlotValue {
    let mut things = Vec::new();
    let mut unread = 0usize;
    for t in things_of(xs) {
        let k = keyed_in(&t, xs);
        if !k.heads.iter().any(|h| chosen(&sel, cs, h)) {
            continue;
        }
        let rs: Vec<Record> = xs.iter().filter(|r| r.is_of(&t)).cloned().collect();
        match thing_test(cs, &rs) {
            Tri::Yes => things.push((t, k.view())),
            Tri::Unknown => unread += 1,
            Tri::No => {}
        }
    }
    SlotValue::Keyed {
        things,
        unread: Nat(unread.into()),
    }
}

fn thing_ev(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record], t: &str) -> Tri {
    if state_in(t, xs).heads.iter().any(|h| chosen(&sel, cs, h)) {
        thing_test(cs, &records_of(t, xs))
    } else {
        Tri::No
    }
}

fn sole_setter(xs: &[Record], f: &str, t: &str) -> Option<Record> {
    let mut ss = setters(&records_of(t, xs), f);
    if ss.len() == 1 { ss.pop() } else { None }
}

fn thing_test(cs: &[Cond], rs: &[Record]) -> Tri {
    cs.iter()
        .filter(|c| !c.field.on_envelope())
        .rfold(Tri::Yes, |t, c| on_thing(c, rs).and(t))
}

fn latest_of(sel: impl Fn(&Record) -> bool, cs: &[Cond], xs: &[Record]) -> SlotValue {
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

fn acc_pool(follow: Option<Follow>, xs: &[Record]) -> Vec<Record> {
    if follow == Some(Follow::Prev) {
        whole_heads(xs)
    } else {
        xs.to_vec()
    }
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "Reference to Lean collectOf"
)]
fn collect_of(
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

fn on_thing(this: &Cond, rs: &[Record]) -> Tri {
    match &this.field {
        Field::Env(_) => Tri::Yes,
        Field::Payload(f) => agree(
            &rs.iter()
                .filter(|r| live_for(rs, f, r))
                .map(|r| this.eval(r))
                .collect::<Vec<_>>(),
            Tri::of_bool(this.op.test(&[])),
        ),
    }
}

fn thing_keys(
    this: &By,
    sel: impl Fn(&Record) -> bool,
    cs: &[Cond],
    xs: &[Record],
    t: &str,
) -> Option<Vec<String>> {
    let src = match this {
        By::Payload(f) => setters(&records_of(t, xs), f),
        _ => state_in(t, xs)
            .heads
            .into_iter()
            .filter(|h| chosen(&sel, cs, h))
            .collect(),
    };
    let mut keys = Vec::new();
    for r in &src {
        keys.extend(this.keys(r)?);
    }
    Some(canon_set(&keys))
}

fn grouped(this: &Slot, b: &By, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
    let ev = |r: &Record| this.ev(&sel, r);
    let pool = pool(this, xs);
    let each = Slot {
        tests: Tests::default(),
        group: None,
        ..this.clone()
    };
    let groups = group_keys(b, ev, &pool)
        .into_iter()
        .map(|g| {
            let v = plain(&each, |r| in_group(b, ev, &g, r), xs).group();
            (g, v)
        })
        .collect();
    let unread = pool.iter().filter(|r| ev(r) == Tri::Unknown).count()
        + pool
            .iter()
            .filter(|r| ev(r) == Tri::Yes && b.keys(r).is_none())
            .count();
    let ungrouped = pool
        .iter()
        .filter(|r| ev(r) == Tri::Yes && b.keys(r).is_some_and(|k| k.is_empty()))
        .count();
    SlotValue::Groups {
        groups,
        unread: Nat(unread.into()),
        ungrouped: Nat(ungrouped.into()),
    }
}

fn keyed_grouped(this: &Slot, b: &By, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
    let cs = this.tests.conds();
    let ts = things_of(xs);
    let evs: Vec<(String, Tri, Option<Vec<String>>)> = ts
        .into_iter()
        .map(|t| {
            let e = thing_ev(&sel, &cs, xs, &t);
            let k = if e == Tri::Yes {
                thing_keys(b, &sel, &cs, xs, &t)
            } else {
                None
            };
            (t, e, k)
        })
        .collect();
    let keys = canon_set(
        &evs.iter()
            .filter(|(_, e, _)| *e == Tri::Yes)
            .flat_map(|(_, _, k)| k.clone().unwrap_or_default())
            .collect::<Vec<_>>(),
    );
    let groups = keys
        .into_iter()
        .map(|g| {
            let v = keyed_value(&this.fold_type, xs, |t| {
                let joins = evs.iter().any(|(u, e, k)| {
                    u == t && *e == Tri::Yes && k.as_ref().is_some_and(|k| k.contains(&g))
                });
                if joins { Tri::Yes } else { Tri::No }
            })
            .group();
            (g, v)
        })
        .collect();
    let unread = evs.iter().filter(|(_, e, _)| *e == Tri::Unknown).count()
        + evs
            .iter()
            .filter(|(_, e, k)| *e == Tri::Yes && k.is_none())
            .count();
    let ungrouped = evs
        .iter()
        .filter(|(_, e, k)| *e == Tri::Yes && k.as_ref().is_some_and(Vec::is_empty))
        .count();
    SlotValue::Groups {
        groups,
        unread: Nat(unread.into()),
        ungrouped: Nat(ungrouped.into()),
    }
}

fn plain(this: &Slot, sel: impl Fn(&Record) -> bool, xs: &[Record]) -> SlotValue {
    let cs = this.tests.conds();
    let ev = |r: &Record| if sel(r) { Cond::all(&cs, r) } else { Tri::No };
    if this.fold_type.follow() == Some(Follow::Keys)
        && !matches!(this.fold_type, FoldType::LatestBy { .. })
    {
        return keyed_value(&this.fold_type, xs, |t| thing_ev(&sel, &cs, xs, t));
    }
    match &this.fold_type {
        FoldType::Count { follow } => {
            SlotValue::Nat(families::count_acc().select(ev, &acc_pool(*follow, xs)))
        }
        FoldType::Sum { field, follow } => {
            SlotValue::Int(families::sum_on(field).select(ev, &acc_pool(*follow, xs)))
        }
        FoldType::Min { field, follow } => {
            SlotValue::Opt(families::min_on(field).select(ev, &acc_pool(*follow, xs)))
        }
        FoldType::Max { field, follow } => {
            SlotValue::Opt(families::max_on(field).select(ev, &acc_pool(*follow, xs)))
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

fn pool(this: &Slot, xs: &[Record]) -> Vec<Record> {
    match &this.fold_type {
        FoldType::Count { follow }
        | FoldType::Sum { follow, .. }
        | FoldType::Min { follow, .. }
        | FoldType::Max { follow, .. }
        | FoldType::Collect { follow, .. } => acc_pool(*follow, xs),
        FoldType::LatestBy { .. } => xs.to_vec(),
    }
}

// Small ids deliberately collide. Some pools are DAGs and some have cycles,
// missing predecessors and conflicting envelopes for the same id.
fn records(mut seed: u32) -> Vec<Record> {
    let mut next = |bound: u8| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        u8::try_from(seed % u32::from(bound)).expect("bounded by a byte")
    };
    let mut rs = Vec::new();
    for i in 1..=16 {
        let id = Digest([i; 32]);
        let of = match next(5) {
            0 => None,
            1 => Some(crate::record::id_text(&Digest([1; 32]))),
            2 => Some("thing:\u{e000}".into()),
            _ => Some("thing:🦀".into()),
        };
        let prev = (0..next(4))
            .map(|_| Digest([next(if i % 4 == 0 { 20 } else { i }); 32]))
            .collect();
        let sets = match next(5) {
            0 => None,
            1 => Some(Vec::new()),
            2 => Some(vec!["n".into(), "n".into()]),
            3 => Some(vec!["status".into()]),
            _ => Some(vec!["status".into(), "n".into()]),
        };
        let mut r = Record {
            id,
            of,
            prev,
            sets,
            emitted: crate::occurred::Stamp::of(
                ["2026-01-01T00:00:00Z", "2026-01-01T00:00:00.1Z"][usize::from(next(2))],
            )
            .expect("fixture stamp"),
            occurred: match next(4) {
                0 => crate::occurred::Claim::of("2026-01-01/.."),
                1 => crate::occurred::Claim::of("2026-01-01/2026-01-02"),
                _ => None,
            },
            fields: vec![
                (
                    "n".into(),
                    crate::record::Val::Int(i64::from(next(10)).into()),
                ),
                (
                    "status".into(),
                    crate::record::Val::Str(format!("s{}", next(3))),
                ),
            ],
            kind: format!("k{}", next(2)),
            about: vec![format!("g:{}", next(4)), format!("g:{}", next(4))],
            erases: if next(4) == 0 {
                vec![Digest([next(17); 32])]
            } else {
                vec![]
            },
            ..Record::default()
        };
        match next(3) {
            0 => r = r.withhold(),
            1 => r = r.erase(),
            _ => {}
        }
        rs.push(r.clone());
        if i % 4 == 0 {
            rs.push(r.clone());
            r.sets = None;
            r.prev = vec![Digest([next(18); 32])];
            r.kind = "k0".into();
            r.shown = Shown::Here;
            rs.push(r);
        }
    }
    for i in (1..rs.len()).rev() {
        let j = usize::from(next(u8::try_from(i + 1).expect("small fixture")));
        rs.swap(i, j);
    }
    rs
}

#[test]
fn generated_forms_match_the_list_scans() {
    use crate::{fold, live, spec, things};
    for seed in 1..=100 {
        let rs = records(seed);
        let may = |a: &Record, b: &Record| a.kind == b.kind;
        assert_eq!(things::settle(&rs), settle(&rs));
        assert_eq!(things::admit(&rs, may), admit(&rs, may));
        assert_eq!(fold::latest_fold(&rs), latest_fold(&rs));
        assert_eq!(spec::newest_first(&rs), newest_first(&rs));
        assert_eq!(spec::things_in_order(&rs), things_in_order(&rs));
        for t in things_of(&rs) {
            assert_eq!(live::keyed_in(&t, &rs), keyed_in(&t, &rs), "seed {seed}");
            assert_eq!(live::live_in(&rs, &t), live_in(&rs, &t));
        }
        let missing = Record {
            id: Digest([0; 32]),
            ..Record::default()
        };
        for r in rs.iter().chain([&missing]) {
            assert_eq!(
                live::upward(&rs, &[r.id, missing.id]),
                upward(&rs, &[r.id, missing.id])
            );
            assert_eq!(live::above(&rs, r), above(&rs, r));
            assert_eq!(live::live_env(&rs, r), live_env(&rs, r));
            assert_eq!(live::live_for(&rs, "n", r), live_for(&rs, "n", r));
            assert_eq!(live::live(&rs, r), live(&rs, r));
            for q in &rs {
                assert_eq!(r.is_of(&q.thing()), r.thing() == q.thing());
                assert_eq!(crate::things::shares(r, q), r.thing() == q.thing());
                assert_eq!(
                    r.newer(q),
                    r.span()
                        .cmp(&q.span())
                        .then_with(|| r.emitted.moment().cmp(q.emitted.moment()))
                        .then_with(|| r.id.cmp(&q.id))
                        .is_gt()
                );
            }
        }
    }
}

#[test]
fn generated_slots_match_the_list_scans() {
    for seed in 1..=36 {
        let rs = records(seed);
        let tests = match seed % 3 {
            0 => Tests::default(),
            1 => Tests(vec![(Field::Payload("status".into()), vec![Op::Absent])]),
            _ => Tests(vec![(Field::Payload("n".into()), vec![Op::Present])]),
        };
        let sel = |r: &Record| r.kind == "k0";
        for follow in [None, Some(Follow::Keys), Some(Follow::Prev)] {
            for ty in [
                FoldType::Count { follow },
                FoldType::Sum {
                    follow,
                    field: "n".into(),
                },
                FoldType::Min {
                    follow,
                    field: "n".into(),
                },
                FoldType::Max {
                    follow,
                    field: "n".into(),
                },
                FoldType::Collect {
                    follow,
                    limit: Some(Nat::from(u64::from(seed % 5))),
                },
                FoldType::LatestBy {
                    follow: follow.unwrap_or(Follow::Keys),
                },
            ] {
                for group in [
                    None,
                    Some(By::About("g".into())),
                    Some(By::Payload("status".into())),
                ] {
                    let slot = Slot {
                        name: "s".into(),
                        fold_type: ty.clone(),
                        tests: tests.clone(),
                        group,
                    };
                    if let Some(b) = &slot.group {
                        assert_eq!(slot.grouped(b, sel, &rs), grouped(&slot, b, sel, &rs));
                    }
                    let old = match &slot.group {
                        None => plain(&slot, sel, &rs),
                        Some(b) if ty.follow() == Some(Follow::Keys) => {
                            keyed_grouped(&slot, b, sel, &rs)
                        }
                        Some(b) => grouped(&slot, b, sel, &rs),
                    };
                    assert_eq!(slot.run(sel, &rs), old, "seed {seed}: {slot:?}");
                }
            }
        }
    }
}

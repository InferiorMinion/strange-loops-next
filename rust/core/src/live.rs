//! Live setters, a thing read by its keys, from model/Loops/Live.lean.
use crate::{
    fold::order_heads,
    json::canon_set,
    record::{Digest, Record},
    things::state_in,
};
use std::collections::HashSet;

impl Record {
    /// Lean: `Loops.Record.whole`, model/Loops/Live.lean.
    /// A record without `sets` is read whole.
    pub const fn whole(&self) -> bool {
        self.sets.is_none()
    }
    /// Lean: `Loops.Record.declares`, model/Loops/Live.lean.
    /// The keys a record declares: its `sets`, none when it is read whole.
    pub fn declares(&self) -> &[String] {
        self.sets.as_deref().unwrap_or_default()
    }
    /// Lean: `Loops.Record.setsKey`, model/Loops/Live.lean.
    /// A record sets `k` when it declares `k`, or when it is read whole.
    pub fn sets_key(&self, k: &str) -> bool {
        self.sets
            .as_ref()
            .is_none_or(|ks| ks.iter().any(|x| x == k))
    }
}

/// Lean: `Loops.upward`, model/Loops/Live.lean.
/// `ids`, with the id of every record of `rs` that names an id it holds added
/// until no record of `rs` names one it lacks.
pub fn upward(rs: &[Record], ids: &[Digest]) -> HashSet<Digest> {
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

/// Lean: `Loops.above`, model/Loops/Live.lean.
/// The records of `rs` above `r` over `prev`, through `rs`.
pub fn above<'a>(rs: &'a [Record], r: &Record) -> Vec<&'a Record> {
    let reached = upward(rs, &[r.id]);
    rs.iter()
        .filter(|q| q.prev.iter().any(|p| reached.contains(p)))
        .collect()
}

/// Lean: `Loops.liveFor`, model/Loops/Live.lean.
/// A setter of `k` is live for `k` when no record above it sets `k`.
pub fn live_for(rs: &[Record], k: &str, r: &Record) -> bool {
    r.sets_key(k) && !above(rs, r).iter().any(|q| q.sets_key(k))
}

/// Lean: `Loops.liveEnv`, model/Loops/Live.lean.
/// Live for the envelope, which every record sets: nothing is above it.
pub fn live_env(rs: &[Record], r: &Record) -> bool {
    above(rs, r).is_empty()
}

/// Lean: `Loops.live`, model/Loops/Live.lean.
/// Live for some key, the envelope among them.
pub fn live(rs: &[Record], r: &Record) -> bool {
    let up = above(rs, r);
    if up.is_empty() {
        return true;
    }
    match &r.sets {
        None => !up.iter().any(|q| q.whole()),
        Some(ks) => ks.iter().any(|k| live_for(rs, k, r)),
    }
}

/// Lean: `Loops.Keyed`, model/Loops/Live.lean.
/// A thing read by its keys: each declared key's live setters, the live
/// whole-read emits and the heads, each newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keyed {
    /// Each key some live record declares, in canonical order, with its live setters.
    pub keys: Vec<(String, Vec<Record>)>,
    /// The live whole-read emits.
    pub whole: Vec<Record>,
    /// The thing's heads.
    pub heads: Vec<Record>,
}

/// Lean: `Loops.liveKeys`, model/Loops/Live.lean.
/// The keys some live record of `rs` declares, in canonical order.
pub fn live_keys(rs: &[Record]) -> Vec<String> {
    canon_set(
        &rs.iter()
            .filter(|r| live(rs, r))
            .flat_map(|r| r.declares().iter().cloned())
            .collect::<Vec<_>>(),
    )
}

/// Lean: `Loops.keyedIn`, model/Loops/Live.lean.
/// Thing `t`'s keyed state over the records a reader holds.
pub fn keyed_in(t: &str, xs: &[Record]) -> Keyed {
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

/// Lean: `Loops.recordsOf`, model/Loops/Live.lean.
/// The records of thing `t` a reader holds.
pub fn records_of(t: &str, xs: &[Record]) -> Vec<Record> {
    xs.iter().filter(|r| r.is_of(t)).cloned().collect()
}

/// Lean: `Loops.setters`, model/Loops/Live.lean.
/// The live setters of key `k` among a thing's records `rs`, newest first, each once.
pub fn setters(rs: &[Record], k: &str) -> Vec<Record> {
    order_heads(
        &rs.iter()
            .filter(|r| live_for(rs, k, r))
            .cloned()
            .collect::<Vec<_>>(),
    )
}

/// Lean: `Loops.liveIn`, model/Loops/Live.lean.
/// Thing `t`'s live records, its heads among them.
pub fn live_in(xs: &[Record], t: &str) -> Vec<Record> {
    let rs = records_of(t, xs);
    rs.iter().filter(|r| live(&rs, r)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::occurred::Stamp;

    fn record(id: u8, prev: &[u8], sets: Option<&[&str]>, emitted: &str) -> Record {
        let digest = |n: u8| {
            let mut bytes = [0; 32];
            bytes[31] = n;
            Digest(bytes)
        };
        Record {
            id: digest(id),
            kind: "task".into(),
            of: Some("task:1".into()),
            prev: prev.iter().map(|&p| digest(p)).collect(),
            sets: sets.map(|ks| ks.iter().map(|&k| k.into()).collect()),
            emitted: Stamp::of(emitted).expect("a fixture instant"),
            ..Record::default()
        }
    }

    #[test]
    fn a_head_that_sets_one_key_leaves_an_older_setter_live_for_another() {
        let root = record(1, &[], None, "2026-10-01T00:00:01Z");
        let status = record(2, &[1], Some(&["status"]), "2026-10-01T00:00:02Z");
        let title = record(3, &[2], Some(&["title"]), "2026-10-01T00:00:03Z");
        let k = keyed_in("task:1", &[root.clone(), status.clone(), title.clone()]);
        assert_eq!(
            k.keys,
            vec![
                ("status".into(), vec![status]),
                ("title".into(), vec![title.clone()])
            ]
        );
        assert_eq!(k.whole, vec![root]);
        assert_eq!(k.heads, vec![title]);
    }

    #[test]
    fn a_pure_join_is_a_head_and_nothing_else() {
        let root = record(1, &[], None, "2026-10-01T00:00:01Z");
        let a = record(2, &[1], Some(&["status"]), "2026-10-01T00:00:02Z");
        let b = record(3, &[1], Some(&["status"]), "2026-10-01T00:00:03Z");
        let join = record(4, &[2, 3], Some(&[]), "2026-10-01T00:00:04Z");
        let xs = [root.clone(), a.clone(), b.clone(), join.clone()];
        let k = keyed_in("task:1", &xs);
        assert_eq!(k.keys, vec![("status".into(), vec![b, a])]);
        assert_eq!(k.whole, vec![root]);
        assert_eq!(k.heads, vec![join.clone()]);
        assert!(live(&xs, &join) && live_env(&xs, &join));
    }

    #[test]
    fn a_prev_cycle_ends() {
        let a = record(1, &[2], None, "2026-10-01T00:00:01Z");
        let b = record(2, &[1], None, "2026-10-01T00:00:02Z");
        let xs = [a.clone(), b];
        assert_eq!(above(&xs, &a).len(), 2);
        assert!(!live(&xs, &a));
    }
}

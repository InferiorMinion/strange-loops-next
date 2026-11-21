//! Summarize strict successors once per component of the prev graph.
use super::{Keyed, Record};
use crate::{
    fold::{ordered_forms, ordered_ids},
    json::str_cmp,
};
use std::{
    collections::{HashMap, HashSet},
    mem,
    rc::Rc,
};

// None covers all keys. Shared sets let forks forward a summary without
// copying it, and a chain extend a uniquely owned set in place.
#[derive(Clone, Debug)]
struct Shadow<'a> {
    keys: Option<Rc<HashSet<&'a str>>>,
    named: bool,
}

impl Default for Shadow<'_> {
    fn default() -> Self {
        Self {
            keys: Some(Rc::default()),
            named: false,
        }
    }
}

impl<'a> Shadow<'a> {
    fn add(&mut self, r: &'a Record) {
        self.named = true;
        match (&mut self.keys, &r.sets) {
            (_, None) => self.keys = None,
            (Some(keys), Some(ks)) => {
                for k in ks {
                    if !keys.contains(k.as_str()) {
                        Rc::make_mut(keys).insert(k);
                    }
                }
            }
            _ => {}
        }
    }

    fn merge(&mut self, other: Self) {
        self.named |= other.named;
        match (self.keys.take(), other.keys) {
            (Some(mut a), Some(mut b)) => {
                if a.len() < b.len() {
                    mem::swap(&mut a, &mut b);
                }
                if !Rc::ptr_eq(&a, &b) {
                    for &k in b.iter() {
                        if !a.contains(k) {
                            Rc::make_mut(&mut a).insert(k);
                        }
                    }
                }
                self.keys = Some(a);
            }
            _ => self.keys = None,
        }
    }
}

// Edges go from a form to its held predecessors. Kosaraju's two iterative
// traversals give components in successor-before-predecessor order, even
// for unchecked cycles. The edge label identifies the particular form that
// names the predecessor when several forms have the same id.
type Edge = (usize, usize);

fn components(edges: &[Vec<Edge>]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let mut reverse = vec![Vec::new(); edges.len()];
    for (i, es) in edges.iter().enumerate() {
        for &(j, _) in es {
            reverse[j].push(i);
        }
    }
    let mut seen = vec![false; edges.len()];
    let mut order = Vec::new();
    let mut stack = Vec::new();
    for start in 0..edges.len() {
        if mem::replace(&mut seen[start], true) {
            continue;
        }
        stack.push((start, 0));
        while let Some((v, next)) = stack.last_mut() {
            if let Some(&(w, _)) = edges[*v].get(*next) {
                *next += 1;
                if !mem::replace(&mut seen[w], true) {
                    stack.push((w, 0));
                }
            } else {
                order.push(*v);
                stack.pop();
            }
        }
    }
    let mut component = vec![usize::MAX; edges.len()];
    let mut groups = Vec::new();
    let mut todo = Vec::new();
    for start in order.into_iter().rev() {
        if component[start] != usize::MAX {
            continue;
        }
        let id = groups.len();
        let mut members = Vec::new();
        component[start] = id;
        todo.push(start);
        while let Some(v) = todo.pop() {
            members.push(v);
            for &w in &reverse[v] {
                if component[w] == usize::MAX {
                    component[w] = id;
                    todo.push(w);
                }
            }
        }
        groups.push(members);
    }
    (component, groups)
}

/// Live records and setters over one pool of records in arrival order.
#[derive(Debug, Default)]
pub struct LiveState<'a> {
    records: Vec<&'a Record>,
    heads: Vec<usize>,
    live: Vec<bool>,
    declared: HashMap<&'a str, Vec<usize>>,
    whole: Vec<(usize, Rc<HashSet<&'a str>>)>,
}

impl<'a> LiveState<'a> {
    /// Indexes the prev graph over exactly these records. Missing targets
    /// add no edge between held records.
    pub fn new(records: Vec<&'a Record>) -> Self {
        let mut ids = HashMap::new();
        let mut forms = Vec::<Vec<usize>>::new();
        for (i, r) in records.iter().enumerate() {
            let v = *ids.entry(r.id).or_insert_with(|| {
                forms.push(Vec::new());
                forms.len() - 1
            });
            forms[v].push(i);
        }
        let mut edges = vec![Vec::new(); forms.len()];
        for (v, fs) in forms.iter().enumerate() {
            for &i in fs {
                edges[v].extend(
                    records[i]
                        .prev
                        .iter()
                        .filter_map(|p| Some((*ids.get(p)?, i))),
                );
            }
        }
        let (component, groups) = components(&edges);
        let mut above = vec![Shadow::default(); groups.len()];
        let mut state = Self {
            live: vec![false; records.len()],
            records,
            ..Self::default()
        };
        for (c, vs) in groups.iter().enumerate() {
            let mut shadow = mem::take(&mut above[c]);
            for &v in vs {
                for &(w, i) in &edges[v] {
                    if component[w] == c {
                        shadow.add(state.records[i]);
                    }
                }
            }
            let mut external = Vec::new();
            for &v in vs {
                for &i in &forms[v] {
                    state.classify(i, &shadow);
                }
                external.extend(
                    edges[v]
                        .iter()
                        .filter_map(|&(w, i)| (component[w] != c).then_some((component[w], i))),
                );
            }
            let mut edges = external.into_iter().peekable();
            while let Some((target, i)) = edges.next() {
                let inherited = if edges.peek().is_none() {
                    mem::take(&mut shadow)
                } else {
                    shadow.clone()
                };
                above[target].merge(inherited);
                above[target].add(state.records[i]);
            }
        }
        state.heads.sort_unstable();
        state
    }

    fn classify(&mut self, i: usize, shadow: &Shadow<'a>) {
        let r = self.records[i];
        self.live[i] = !shadow.named;
        if !shadow.named {
            self.heads.push(i);
        }
        let Some(keys) = &shadow.keys else { return };
        if r.whole() {
            self.live[i] = true;
            self.whole.push((i, Rc::clone(keys)));
        } else {
            for k in r.declares() {
                if !keys.contains(k.as_str()) {
                    self.live[i] = true;
                    self.declared.entry(k).or_default().push(i);
                }
            }
        }
    }

    /// All records live for at least one key or the envelope, in input order.
    pub fn live(&self) -> impl Iterator<Item = &'a Record> + '_ {
        self.records
            .iter()
            .zip(&self.live)
            .filter_map(|(&r, &live)| live.then_some(r))
    }

    /// Heads in raw latest-by order, retaining distinct conflicting forms.
    pub fn heads(&self) -> Vec<Record> {
        ordered_forms(self.head_records().cloned().collect())
    }

    pub(crate) fn head_records(&self) -> impl Iterator<Item = &'a Record> + '_ {
        self.heads.iter().map(|&i| self.records[i])
    }

    /// All keys declared by live records, in canonical UTF-16 order.
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self
            .live()
            .flat_map(|r| r.declares().iter().cloned())
            .collect();
        keys.sort_by(|a, b| str_cmp(a, b));
        keys.dedup();
        keys
    }

    pub(crate) fn setting(&self, k: &str) -> Vec<&'a Record> {
        let mut indices = self.declared.get(k).cloned().unwrap_or_default();
        indices.extend(
            self.whole
                .iter()
                .filter(|(_, blocked)| !blocked.contains(k))
                .map(|(i, _)| *i),
        );
        indices.sort_unstable();
        indices.dedup();
        indices.into_iter().map(|i| self.records[i]).collect()
    }

    /// Live setters of a key, newest first, retaining the last form per id.
    pub fn setters(&self, k: &str) -> Vec<&'a Record> {
        ordered_ids(self.setting(k).into_iter())
    }

    /// Live whole-read records, newest first, retaining the last form per id.
    pub fn whole(&self) -> Vec<&'a Record> {
        let mut indices: Vec<_> = self.whole.iter().map(|(i, _)| *i).collect();
        indices.sort_unstable();
        ordered_ids(indices.into_iter().map(|i| self.records[i]))
    }

    /// Gives what Lean `Loops.keyedIn` gives for this thing's records.
    pub fn keyed(&self) -> Keyed {
        Keyed {
            keys: self
                .keys()
                .into_iter()
                .map(|k| {
                    let rs = self.setters(&k).into_iter().cloned().collect();
                    (k, rs)
                })
                .collect(),
            whole: self.whole().into_iter().cloned().collect(),
            heads: self.heads(),
        }
    }
}

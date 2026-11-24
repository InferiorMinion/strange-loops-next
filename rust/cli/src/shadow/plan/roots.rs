//! Resolve record-valued targets before replaying destination histories.
use super::{Cut, Identities, Index, Occurrence};
use crate::{
    Fail, refused,
    shadow::{
        mapping::{Fate, History, Target},
        refs::{Changes, Lookup},
    },
};
use loops_place::key::Key;

pub(in crate::shadow) struct Roots {
    component: Vec<usize>,
    ids: Vec<Option<String>>,
}

impl Index {
    fn candidates(&self, id: &str, source: usize) -> &[Occurrence] {
        let Some(copies) = self.copies.get(id) else {
            return &[];
        };
        if let Ok(pos) = copies.binary_search_by_key(&source, |r| r.source)
            && matches!(copies[pos].fate, Fate::Included(Ok(_)))
        {
            return &copies[pos..=pos];
        }
        copies
    }

    fn dependencies(&self) -> Vec<Vec<usize>> {
        self.roots
            .iter()
            .enumerate()
            .map(|(node, (old, source))| {
                let mut edges: Vec<_> = self
                    .occurrence(old, *source)
                    .fields
                    .iter()
                    .flat_map(super::refs::Field::parts)
                    .flat_map(|id| self.candidates(id, *source))
                    .filter_map(|r| r.root)
                    .filter(|&target| target != node)
                    .collect();
                edges.sort_unstable();
                edges.dedup();
                edges
            })
            .collect()
    }
}

impl Roots {
    pub(in crate::shadow) fn build(
        index: &Index,
        cuts: &mut [Cut],
        seeds: &Identities,
        key: &Key,
    ) -> Result<Self, Fail> {
        let components = dependency_order(&index.dependencies());
        let mut roots = Self {
            component: vec![0; index.roots.len()],
            ids: vec![None; index.roots.len()],
        };
        for (c, nodes) in components.iter().enumerate() {
            for &node in nodes {
                roots.component[node] = c;
            }
        }
        for nodes in components {
            for node in nodes {
                let (old, source) = &index.roots[node];
                let row = index.occurrence(old, *source);
                let Fate::Included(Ok(included)) = &row.fate else {
                    unreachable!("indexed root")
                };
                let changes = roots.changes(index, row, None);
                let seed = &seeds.seeds[old];
                let payload = row.line.payload(&mut cuts[*source].file)?;
                let record = History::default()
                    .prepare_included(included, payload, changes, key, &seed.salt, &seed.emitted)
                    .map_err(|e| refused(e.to_string()))?;
                roots.ids[node] = Some(record.id);
            }
        }
        Ok(roots)
    }

    pub(super) fn changes(&self, index: &Index, row: &Occurrence, own: Option<&str>) -> Changes {
        Changes::resolve(&row.fields, own, |id| self.lookup(index, row, id))
    }

    fn lookup(&self, index: &Index, row: &Occurrence, id: &str) -> Lookup {
        let candidates = index.candidates(id, row.source);
        if candidates.is_empty() {
            return Lookup::Nonmember;
        }
        let mut result = None;
        let mut ambiguous = false;
        let mut omitted = "left_out_unnamed";
        for target in candidates {
            let name = match &target.fate {
                Fate::Omitted { defect } => {
                    if defect.is_some() {
                        omitted = "left_out_defect";
                    }
                    continue;
                }
                Fate::Included(Err(_)) => continue, // This group already has a row refusal.
                Fate::Included(Ok(included)) => {
                    if let Target::Keyed(thing) = &included.target {
                        thing
                    } else {
                        let node = target.root.expect("root indexed");
                        if row.root == Some(node) {
                            return Lookup::SelfReference;
                        }
                        if row
                            .root
                            .is_some_and(|r| self.component[r] == self.component[node])
                        {
                            return Lookup::Unresolved("cycle");
                        }
                        self.ids[node]
                            .as_ref()
                            .expect("external root dependency was prepared first")
                    }
                }
            };
            ambiguous |= result
                .replace(name)
                .is_some_and(|previous| previous != name);
        }
        if ambiguous {
            Lookup::Unresolved("ambiguous_target")
        } else {
            result.map_or(Lookup::Unresolved(omitted), |name| {
                Lookup::Target(name.clone())
            })
        }
    }
}

// Iterative Kosaraju traversal. Edges point from a referrer to its targets.
// Reverse the component order to prepare all external dependencies first.
fn dependency_order(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut incoming = vec![Vec::new(); edges.len()];
    for (node, targets) in edges.iter().enumerate() {
        for &target in targets {
            incoming[target].push(node);
        }
    }
    let mut visited = vec![false; edges.len()];
    let mut finish = Vec::new();
    for start in 0..edges.len() {
        let mut stack = vec![(start, false)];
        while let Some((node, done)) = stack.pop() {
            if done {
                finish.push(node);
            } else if !std::mem::replace(&mut visited[node], true) {
                stack.push((node, true));
                stack.extend(edges[node].iter().map(|&next| (next, false)));
            }
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    for start in finish.into_iter().rev() {
        if visited[start] {
            continue;
        }
        let mut nodes = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if !std::mem::replace(&mut visited[node], true) {
                nodes.push(node);
                stack.extend(&incoming[node]);
            }
        }
        components.push(nodes);
    }
    components.reverse();
    components
}

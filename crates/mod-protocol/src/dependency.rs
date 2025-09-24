use std::{
    collections::{BinaryHeap, HashSet},
    hash::{Hash, RandomState},
};

use indexmap::{map::Entry, IndexMap};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub trait DependencyId: Eq + PartialEq + Hash + Clone {}
impl<T: Eq + PartialEq + Hash + Clone + for<'de> Deserialize<'de> + Serialize> DependencyId for T {}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
pub struct Dependent<T: DependencyId> {
    pub id: T,
    pub optional: bool,
}

impl<T: DependencyId> Dependent<T> {
    pub fn id(&self) -> T {
        self.id.clone()
    }
}

pub enum DependencyOrder {
    Before,
    After,
}

pub struct DependencyLink<T: DependencyId> {
    optional: bool,
    order: DependencyOrder,
    id: T,
}

pub trait Dependency {
    type UniqueId: DependencyId;

    fn id(&self) -> Self::UniqueId;

    fn dependencies(&self) -> impl Iterator<Item = DependencyLink<Self::UniqueId>> {
        self.load_after()
            .iter()
            .map(|dep| DependencyLink {
                optional: dep.optional,
                order: DependencyOrder::After,
                id: dep.id(),
            })
            .chain(self.load_before().iter().map(|dep| DependencyLink {
                optional: dep.optional,
                order: DependencyOrder::Before,
                id: dep.id(),
            }))
    }

    fn load_before(&self) -> &[Dependent<Self::UniqueId>];

    fn load_after(&self) -> &[Dependent<Self::UniqueId>];
}

#[derive(Debug, thiserror::Error)]
pub enum DependencyError<T: Dependency> {
    #[error("Required dependency is unavailable: {0}")]
    MissingDependency(T::UniqueId),

    #[error("Dependencies resulted in cycles, remaining dependencies: {0:?}")]
    Cyclic(Vec<T::UniqueId>),
}

#[derive(Clone)]
struct DependencyNode<T> {
    num_prec: usize,
    succ: HashSet<T>,
}

impl<T> DependencyNode<T> {
    fn new() -> Self {
        Self {
            num_prec: 0,
            succ: HashSet::new(),
        }
    }
}

trait DependencySorter {
    type UniqueId: DependencyId;

    fn add_dependency(&mut self, prec: Self::UniqueId, succ: Self::UniqueId);
    fn pop_dependency(&mut self) -> Option<(Self::UniqueId, bool)>;
}

// <https://crates.io/crates/topological-sort>
impl<T> DependencySorter for IndexMap<T, DependencyNode<T>>
where
    T: DependencyId + Clone,
{
    type UniqueId = T;

    fn add_dependency(&mut self, prec: T, succ: T) {
        match self.entry(prec) {
            Entry::Vacant(e) => {
                let mut dep = DependencyNode::new();
                dep.succ.insert(succ.clone());
                e.insert(dep);
            }
            Entry::Occupied(e) => {
                if !e.into_mut().succ.insert(succ.clone()) {
                    // Already registered
                    return;
                }
            }
        }

        match self.entry(succ) {
            Entry::Vacant(e) => {
                let mut dep = DependencyNode::new();
                dep.num_prec += 1;
                e.insert(dep);
            }
            Entry::Occupied(e) => {
                e.into_mut().num_prec += 1;
            }
        }
    }

    fn pop_dependency(&mut self) -> Option<(T, bool)> {
        self.iter()
            .find_map(|(k, v)| (v.num_prec == 0).then_some(k.clone()))
            .map(|key| {
                if let Some(p) = self.shift_remove(&key) {
                    for s in &p.succ {
                        if let Some(y) = self.get_mut(s) {
                            y.num_prec -= 1;
                        }
                    }

                    (key, !p.succ.is_empty())
                } else {
                    (key, false)
                }
            })
    }
}

struct DependencyRun<T> {
    max_index: usize,
    dependencies: Vec<T>,
}

impl<T> Default for DependencyRun<T> {
    fn default() -> Self {
        Self {
            max_index: 0,
            dependencies: vec![],
        }
    }
}

impl<T> PartialEq for DependencyRun<T> {
    fn eq(&self, other: &Self) -> bool {
        other.max_index.eq(&self.max_index)
    }
}

impl<T> PartialOrd for DependencyRun<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Eq for DependencyRun<T> {}

impl<T> Ord for DependencyRun<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.max_index.cmp(&self.max_index)
    }
}

pub fn sort_dependencies<T: Dependency, I: IntoIterator<Item = T>>(
    items: I,
) -> Result<Vec<T>, DependencyError<T>> {
    let mut sorter = IndexMap::<T::UniqueId, DependencyNode<T::UniqueId>>::new();
    let mut all = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| (item.id(), (item, index)))
        .collect::<IndexMap<_, _, RandomState>>();

    for (id, (item, _)) in &all {
        for dep in item.dependencies() {
            if !all.contains_key(&dep.id) {
                if !dep.optional {
                    return Err(DependencyError::MissingDependency(dep.id.clone()));
                }
                continue;
            }

            let (prec, succ) = match dep.order {
                DependencyOrder::Before => (id.clone(), dep.id),
                DependencyOrder::After => (dep.id, id.clone()),
            };

            sorter.add_dependency(prec, succ)
        }
    }

    let mut all_runs = BinaryHeap::new();
    let mut current_run = DependencyRun::<T>::default();

    let mut max_index = None;

    while let Some((key, has_succ)) = sorter.pop_dependency() {
        let (item, index) = all.shift_remove(&key).expect("item already removed?");

        if max_index.is_none() && item.load_after().is_empty() && item.load_before().is_empty() {
            max_index = Some(index);
        }

        current_run.dependencies.push(item);

        if !has_succ {
            current_run.max_index = max_index.take().unwrap_or(index);
            all_runs.push(std::mem::take(&mut current_run));
        }
    }

    if !sorter.is_empty() {
        return Err(DependencyError::Cyclic(all.keys().cloned().collect()));
    }

    let mut sorted = vec![];

    for (item, index) in all.into_values() {
        if all_runs.peek().is_some_and(|run| run.max_index < index) {
            sorted.extend(all_runs.pop().unwrap().dependencies);
        }
        sorted.push(item);
    }

    sorted.extend(all_runs.into_iter().flat_map(|run| run.dependencies));

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::{sort_dependencies, Dependent};
    use crate::{dependency::Dependency as _, mod_file::ModFile, package::Package};

    fn mock_package(
        id: &str,
        load_after: Vec<Dependent<String>>,
        load_before: Vec<Dependent<String>>,
    ) -> Package {
        Package {
            inner: ModFile {
                name: id.to_owned(),
                ..ModFile::new("pkg")
            },
            load_after,
            load_before,
        }
    }

    #[test]
    fn detect_cycles() {
        let pkg1 = mock_package(
            "pkg1",
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
            vec![],
        );

        let pkg2 = mock_package(
            "pkg2",
            vec![Dependent {
                id: "pkg1".to_owned(),
                optional: false,
            }],
            vec![],
        );

        assert!(sort_dependencies(vec![pkg1, pkg2]).is_err())
    }

    #[test]
    fn loads_before_and_after() {
        let pkg1 = mock_package(
            "pkg1",
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
            vec![],
        );

        let pkg2 = mock_package("pkg2", vec![], vec![]);
        let pkg3 = mock_package(
            "pkg3",
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
            vec![Dependent {
                id: "pkg1".to_owned(),
                optional: false,
            }],
        );
        let sorted_pkgs = sort_dependencies(vec![pkg1, pkg2, pkg3]).expect("failed to sort");

        assert_eq!("pkg2", sorted_pkgs[0].id());
        assert_eq!("pkg3", sorted_pkgs[1].id());
        assert_eq!("pkg1", sorted_pkgs[2].id());
    }

    #[test]
    fn loads_after() {
        let pkg1 = mock_package(
            "pkg1",
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
            vec![],
        );

        let pkg2 = mock_package("pkg2", vec![], vec![]);
        let pkg3 = mock_package("pkg3", vec![], vec![]);
        let sorted_pkgs = sort_dependencies(vec![pkg1, pkg2, pkg3]).expect("failed to sort");

        assert_eq!("pkg2", sorted_pkgs[0].id());
        assert_eq!("pkg1", sorted_pkgs[1].id());
    }

    #[test]
    fn smoke_test() {
        let pkg1 = mock_package("pkg1", vec![], vec![]);
        let pkg2 = mock_package("pkg2", vec![], vec![]);
        let pkg3 = mock_package("pkg3", vec![], vec![]);

        sort_dependencies(vec![pkg1, pkg2, pkg3]).expect("failed to sort");
    }

    #[test]
    fn preserves_iteration_order() {
        let pkg1 = mock_package("pkg1", vec![], vec![]);
        let pkg2 = mock_package("pkg2", vec![], vec![]);
        let pkg3 = mock_package(
            "pkg3",
            vec![],
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
        );
        let pkg4 = mock_package(
            "pkg4",
            vec![],
            vec![Dependent {
                id: "pkg2".to_owned(),
                optional: false,
            }],
        );
        let pkg5 = mock_package("pkg5", vec![], vec![]);

        let sorted = sort_dependencies(vec![pkg1, pkg2, pkg3, pkg4, pkg5]).unwrap();

        assert_eq!("pkg1", sorted[0].id());
        assert_eq!("pkg3", sorted[1].id());
        assert_eq!("pkg4", sorted[2].id());
        assert_eq!("pkg2", sorted[3].id());
        assert_eq!("pkg5", sorted[4].id());
    }
}

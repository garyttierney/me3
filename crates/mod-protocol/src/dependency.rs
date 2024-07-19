use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, RandomState},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topological_sort::TopologicalSort;

pub trait DependencyId: Eq + PartialEq + Hash + Clone {}
impl<T: Eq + PartialEq + Hash + Clone + for<'de> Deserialize<'de> + Serialize> DependencyId for T {}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct Dependent<T: DependencyId> {
    id: T,
    optional: bool,
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
        self.loads_after()
            .iter()
            .map(|dep| DependencyLink {
                optional: dep.optional,
                order: DependencyOrder::After,
                id: dep.id(),
            })
            .chain(self.loads_before().iter().map(|dep| DependencyLink {
                optional: dep.optional,
                order: DependencyOrder::Before,
                id: dep.id(),
            }))
    }

    fn loads_after(&self) -> &[Dependent<Self::UniqueId>];

    fn loads_before(&self) -> &[Dependent<Self::UniqueId>];
}

pub fn sort_dependencies<T: Dependency>(items: Vec<T>) -> Option<Vec<T>> {
    let mut sorter = TopologicalSort::<T::UniqueId>::new();
    let mut all = HashMap::new();
    all.extend(items.into_iter().map(|item| (item.id(), item)));

    for item in all.values() {
        for dep in item.dependencies() {
            if !all.contains_key(&dep.id) {
                if !dep.optional {
                    panic!("Missing required dependency");
                }

                continue;
            }

            let (prec, succ) = match dep.order {
                DependencyOrder::Before => (item.id(), dep.id),
                DependencyOrder::After => (dep.id, item.id()),
            };

            sorter.add_dependency(prec, succ)
        }
    }

    let mut remaining: HashSet<_, RandomState> = HashSet::from_iter(all.keys().cloned());
    let mut sorted = vec![];

    while let Some(key) = sorter.pop() {
        let item = all.remove(&key).expect("item already removed?");

        sorted.push(item);
        remaining.remove(&key);
    }

    if !remaining.is_empty() {
        panic!("Dependency Graph has cycles");
    }

    Some(sorted)
}

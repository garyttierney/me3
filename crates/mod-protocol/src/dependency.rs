use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, RandomState},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use topological_sort::TopologicalSort;

pub trait DependencyId: Eq + PartialEq + Hash + Clone {}
impl<T: Eq + PartialEq + Hash + Clone + for<'de> Deserialize<'de> + Serialize> DependencyId for T {}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
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

#[derive(Debug, thiserror::Error)]
pub enum DependencyError<T: Dependency> {
    #[error("Required dependency is unavailable: {0}")]
    MissingDependency(T::UniqueId),

    #[error("Dependencies resulted in cycles, remaining dependencies: {0:?}")]
    Cyclic(Vec<T::UniqueId>),
}

pub fn sort_dependencies<T: Dependency>(items: Vec<T>) -> Result<Vec<T>, DependencyError<T>> {
    let mut sorter = TopologicalSort::<T::UniqueId>::new();
    let mut all = HashMap::new();
    all.extend(items.into_iter().map(|item| (item.id(), item)));

    for item in all.values() {
        for dep in item.dependencies() {
            if !all.contains_key(&dep.id) {
                if !dep.optional {
                    return Err(DependencyError::MissingDependency(dep.id.clone()));
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

    if !sorter.is_empty() {
        return Err(DependencyError::Cyclic(remaining.into_iter().collect()));
    }

    sorted.extend(
        remaining
            .into_iter()
            .map(|key| all.remove(&key).expect("item already removed?")),
    );

    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{sort_dependencies, Dependent};
    use crate::{
        dependency::Dependency as _,
        package::{ModFile, Package},
    };

    fn mock_package(
        id: &str,
        load_after: Vec<Dependent<String>>,
        load_before: Vec<Dependent<String>>,
    ) -> Package {
        Package {
            id: Some(id.to_owned()),
            enabled: true,
            path: ModFile(PathBuf::from(id)),
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
}

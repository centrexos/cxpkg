use std::collections::{HashMap, HashSet, VecDeque};
use crate::backend::BackendRegistry;
use crate::error::{CxpkgError, Result};
use crate::package::{Package, PackageQuery};

pub struct Resolver<'a> {
    registry: &'a BackendRegistry,
}

#[derive(Debug, Default)]
pub struct ResolutionPlan {
    pub to_install: Vec<Package>,
    pub to_remove: Vec<Package>,
    pub to_upgrade: Vec<Package>,
    pub already_satisfied: Vec<String>,
}

impl ResolutionPlan {
    pub fn is_empty(&self) -> bool {
        self.to_install.is_empty() && self.to_remove.is_empty() && self.to_upgrade.is_empty()
    }

    pub fn package_count(&self) -> usize {
        self.to_install.len() + self.to_upgrade.len()
    }

    pub fn total_download_size(&self) -> u64 {
        self.to_install.iter().chain(self.to_upgrade.iter()).map(|p| p.size).sum()
    }
}

impl<'a> Resolver<'a> {
    pub fn new(registry: &'a BackendRegistry) -> Self {
        Self { registry }
    }

    pub fn resolve_install(&self, names: &[&str]) -> Result<ResolutionPlan> {
        let mut plan = ResolutionPlan::default();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = names.iter().map(|s| s.to_string()).collect();

        while let Some(name) = queue.pop_front() {
            if visited.contains(&name) {
                continue;
            }
            visited.insert(name.clone());

            let query = PackageQuery::new(&name);
            let pkg = self.registry
                .find_package(&query)?
                .ok_or_else(|| CxpkgError::PackageNotFound(name.clone()))?;

            // Check if already installed
            if matches!(pkg.state, crate::package::PackageState::Installed) {
                plan.already_satisfied.push(name.clone());
                // Still need to check deps for completeness
            } else {
                // Queue all required dependencies
                for dep in pkg.dependencies.iter().filter(|d| !d.optional) {
                    if !visited.contains(&dep.name) {
                        queue.push_back(dep.name.clone());
                    }
                }
                plan.to_install.push(pkg);
            }
        }

        // Topological sort so dependencies install before dependents
        plan.to_install = self.topological_sort(plan.to_install)?;
        Ok(plan)
    }

    pub fn resolve_remove(&self, names: &[&str]) -> Result<ResolutionPlan> {
        let mut plan = ResolutionPlan::default();
        for &name in names {
            let query = PackageQuery::new(name);
            match self.registry.find_package(&query)? {
                Some(pkg) => plan.to_remove.push(pkg),
                None => return Err(CxpkgError::PackageNotFound(name.to_string())),
            }
        }
        Ok(plan)
    }

    pub fn resolve_upgrade(&self, names: &[&str]) -> Result<ResolutionPlan> {
        let mut plan = ResolutionPlan::default();
        for &name in names {
            let query = PackageQuery::new(name);
            if let Some(pkg) = self.registry.find_package(&query)? {
                plan.to_upgrade.push(pkg);
            }
        }
        Ok(plan)
    }

    fn topological_sort(&self, packages: Vec<Package>) -> Result<Vec<Package>> {
        let pkg_map: HashMap<String, &Package> = packages.iter().map(|p| (p.name.clone(), p)).collect();

        let mut in_degree: HashMap<String, usize> = pkg_map.keys().map(|k| (k.clone(), 0)).collect();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for pkg in packages.iter() {
            for dep in pkg.dependencies.iter().filter(|d| !d.optional) {
                if pkg_map.contains_key(&dep.name) {
                    adjacency.entry(dep.name.clone()).or_default().push(pkg.name.clone());
                    *in_degree.entry(pkg.name.clone()).or_insert(0) += 1;
                }
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(name) = queue.pop_front() {
            if let Some(pkg) = pkg_map.get(&name) {
                sorted.push((*pkg).clone());
            }
            if let Some(dependents) = adjacency.get(&name) {
                for dep in dependents {
                    let deg = in_degree.entry(dep.clone()).or_insert(0);
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        if sorted.len() != packages.len() {
            return Err(CxpkgError::DependencyCycle(
                packages.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }

        Ok(sorted)
    }

    pub fn check_conflicts(&self, plan: &ResolutionPlan) -> Result<()> {
        for pkg in plan.to_install.iter() {
            for conflict in pkg.conflicts.iter() {
                if plan.to_install.iter().any(|p| &p.name == conflict) {
                    return Err(CxpkgError::DependencyConflict(format!(
                        "{} conflicts with {}",
                        pkg.name, conflict
                    )));
                }
            }
        }
        Ok(())
    }
}

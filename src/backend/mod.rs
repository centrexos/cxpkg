pub mod apt;
pub mod dnf;
pub mod flatpak;

pub use apt::AptBackend;
pub use dnf::DnfBackend;
pub use flatpak::FlatpakBackend;

use crate::error::Result;
use crate::package::{Package, PackageQuery};

pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn search(&self, query: &str) -> Result<Vec<Package>>;
    fn info(&self, query: &PackageQuery) -> Result<Option<Package>>;
    fn list_installed(&self) -> Result<Vec<Package>>;
    fn list_upgradable(&self) -> Result<Vec<Package>>;

    fn install(&self, packages: &[&str]) -> Result<()>;
    fn remove(&self, packages: &[&str]) -> Result<()>;
    fn update_index(&self) -> Result<()>;
    fn upgrade_all(&self) -> Result<()>;
}

pub struct BackendRegistry {
    backends: Vec<Box<dyn Backend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: Vec::new() }
    }

    pub fn register(&mut self, backend: Box<dyn Backend>) {
        if backend.is_available() {
            log::info!("registered backend: {}", backend.name());
            self.backends.push(backend);
        } else {
            log::debug!("backend not available, skipping: {}", backend.name());
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn Backend> {
        self.backends.iter().find(|b| b.name() == name).map(|b| b.as_ref())
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn Backend> {
        self.backends.iter().map(|b| b.as_ref())
    }

    pub fn search_all(&self, query: &str) -> Result<Vec<Package>> {
        let mut results = Vec::new();
        for backend in self.backends.iter() {
            match backend.search(query) {
                Ok(pkgs) => results.extend(pkgs),
                Err(e) => log::warn!("search failed on {}: {}", backend.name(), e),
            }
        }
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results.dedup_by(|a, b| a.name == b.name && a.backend == b.backend);
        Ok(results)
    }

    pub fn find_package(&self, query: &PackageQuery) -> Result<Option<Package>> {
        if let Some(kind) = &query.backend {
            let name = kind.to_string();
            if let Some(backend) = self.get(&name) {
                return backend.info(query);
            }
        }
        for backend in self.backends.iter() {
            if let Ok(Some(pkg)) = backend.info(query) {
                return Ok(Some(pkg));
            }
        }
        Ok(None)
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn run_command(program: &str, args: &[&str]) -> Result<std::process::Output> {
    use std::process::Command;
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| crate::error::CxpkgError::BackendError {
            backend: program.into(),
            message: e.to_string(),
        })?;
    Ok(output)
}

pub(crate) fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

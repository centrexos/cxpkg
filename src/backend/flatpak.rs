use std::process::Command;
use crate::backend::{Backend, command_exists};
use crate::error::{CxpkgError, Result};
use crate::package::{Package, PackageQuery, PackageState, BackendKind};

pub struct FlatpakBackend;

impl FlatpakBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for FlatpakBackend {
    fn name(&self) -> &str {
        "flatpak"
    }

    fn is_available(&self) -> bool {
        command_exists("flatpak")
    }

    fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args(["search", "--columns=application,version,description", query])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() >= 1 {
                let app_id = parts[0].trim();
                let version = parts.get(1).copied().unwrap_or("").trim();
                let desc = parts.get(2).copied().unwrap_or("").trim();
                let mut pkg = Package::new(app_id, version, BackendKind::Flatpak);
                pkg.description = desc.to_string();
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn info(&self, query: &PackageQuery) -> Result<Option<Package>> {
        let output = Command::new("flatpak")
            .args(["info", &query.name])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut name = query.name.clone();
        let mut version = String::new();
        let mut description = String::new();

        for line in stdout.lines() {
            if let Some(v) = line.strip_prefix("          ID: ") {
                name = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("     Version: ") {
                version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix(" Description: ") {
                description = v.trim().to_string();
            }
        }

        let mut pkg = Package::new(&name, &version, BackendKind::Flatpak);
        pkg.description = description;
        pkg.state = PackageState::Installed;
        Ok(Some(pkg))
    }

    fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,description"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() >= 1 && !parts[0].is_empty() {
                let app_id = parts[0].trim();
                let version = parts.get(1).copied().unwrap_or("").trim();
                let desc = parts.get(2).copied().unwrap_or("").trim();
                let mut pkg = Package::new(app_id, version, BackendKind::Flatpak);
                pkg.description = desc.to_string();
                pkg.state = PackageState::Installed;
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn list_upgradable(&self) -> Result<Vec<Package>> {
        // flatpak update --no-deploy shows what would be updated
        let output = Command::new("flatpak")
            .args(["remote-ls", "--updates", "--columns=application,version"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() >= 1 && !parts[0].is_empty() {
                let name = parts[0].trim();
                let version = parts.get(1).copied().unwrap_or("").trim();
                packages.push(Package::new(name, version, BackendKind::Flatpak));
            }
        }
        Ok(packages)
    }

    fn install(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["install", "--noninteractive", "-y"])
            .args(packages)
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: "flatpak install failed".into(),
            });
        }
        Ok(())
    }

    fn remove(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["remove", "--noninteractive", "-y"])
            .args(packages)
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: "flatpak remove failed".into(),
            });
        }
        Ok(())
    }

    fn update_index(&self) -> Result<()> {
        // Flatpak doesn't have a separate index update step; remotes are auto-refreshed
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["update", "--noninteractive", "-y"])
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "flatpak".into(),
                message: "flatpak update failed".into(),
            });
        }
        Ok(())
    }
}

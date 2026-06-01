use std::process::Command;
use crate::backend::{Backend, command_exists};
use crate::error::{CxpkgError, Result};
use crate::package::{Package, PackageQuery, PackageState, BackendKind};

pub struct AptBackend;

impl AptBackend {
    pub fn new() -> Self {
        Self
    }

    fn parse_dpkg_output(&self, output: &str) -> Vec<Package> {
        let mut packages = Vec::new();
        let mut current: Option<Package> = None;

        for line in output.lines() {
            if let Some(name) = line.strip_prefix("Package: ") {
                if let Some(pkg) = current.take() {
                    packages.push(pkg);
                }
                current = Some(Package::new(name.trim(), "", BackendKind::Apt));
            } else if let Some(ver) = line.strip_prefix("Version: ") {
                if let Some(ref mut pkg) = current {
                    pkg.version = ver.trim().to_string();
                }
            } else if let Some(desc) = line.strip_prefix("Description: ") {
                if let Some(ref mut pkg) = current {
                    pkg.description = desc.trim().to_string();
                }
            } else if let Some(size) = line.strip_prefix("Installed-Size: ") {
                if let Some(ref mut pkg) = current {
                    pkg.installed_size = size.trim().parse().unwrap_or(0) * 1024;
                }
            } else if let Some(section) = line.strip_prefix("Section: ") {
                if let Some(ref mut pkg) = current {
                    pkg.section = Some(section.trim().to_string());
                }
            } else if let Some(homepage) = line.strip_prefix("Homepage: ") {
                if let Some(ref mut pkg) = current {
                    pkg.homepage = Some(homepage.trim().to_string());
                }
            } else if let Some(arch) = line.strip_prefix("Architecture: ") {
                if let Some(ref mut pkg) = current {
                    pkg.arch = arch.trim().to_string();
                }
            }
        }
        if let Some(pkg) = current {
            packages.push(pkg);
        }
        packages
    }
}

impl Backend for AptBackend {
    fn name(&self) -> &str {
        "apt"
    }

    fn is_available(&self) -> bool {
        command_exists("apt-get")
    }

    fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("apt-cache")
            .args(["search", "--names-only", query])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            if let Some((name, desc)) = line.split_once(" - ") {
                let mut pkg = Package::new(name.trim(), "", BackendKind::Apt);
                pkg.description = desc.trim().to_string();
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn info(&self, query: &PackageQuery) -> Result<Option<Package>> {
        let output = Command::new("apt-cache")
            .args(["show", &query.name])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut pkgs = self.parse_dpkg_output(&stdout);

        if pkgs.is_empty() {
            return Ok(None);
        }

        // Check if installed
        let installed = Command::new("dpkg-query")
            .args(["-W", "-f=${Status}", &query.name])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).to_string();
                if s.contains("install ok installed") { Some(()) } else { None }
            });

        if installed.is_some() {
            pkgs[0].state = PackageState::Installed;
        }

        Ok(Some(pkgs.remove(0)))
    }

    fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("dpkg-query")
            .args(["-W", "-f=${Package}\t${Version}\t${Architecture}\t${Description}\n"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                let mut pkg = Package::new(parts[0], parts[1], BackendKind::Apt);
                if parts.len() >= 3 { pkg.arch = parts[2].to_string(); }
                if parts.len() >= 4 { pkg.description = parts[3].to_string(); }
                pkg.state = PackageState::Installed;
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn list_upgradable(&self) -> Result<Vec<Package>> {
        let output = Command::new("apt")
            .args(["list", "--upgradable", "-q"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines().skip(1) {
            // Format: name/repo version arch [upgradable from: old_version]
            let parts: Vec<&str> = line.splitn(2, '/').collect();
            if parts.len() == 2 {
                let name = parts[0].trim();
                let rest = parts[1];
                let ver_part: Vec<&str> = rest.splitn(2, ' ').collect();
                let version = ver_part.get(1).copied().unwrap_or("").splitn(2, ' ').next().unwrap_or("");
                let pkg = Package::new(name, version, BackendKind::Apt);
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn install(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("apt-get")
            .args(["install", "-y"])
            .args(packages)
            .env("DEBIAN_FRONTEND", "noninteractive")
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "apt".into(),
                message: "apt-get install failed".into(),
            });
        }
        Ok(())
    }

    fn remove(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("apt-get")
            .args(["remove", "-y"])
            .args(packages)
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "apt".into(),
                message: "apt-get remove failed".into(),
            });
        }
        Ok(())
    }

    fn update_index(&self) -> Result<()> {
        let status = Command::new("apt-get")
            .arg("update")
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "apt".into(),
                message: "apt-get update failed".into(),
            });
        }
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        let status = Command::new("apt-get")
            .args(["upgrade", "-y"])
            .env("DEBIAN_FRONTEND", "noninteractive")
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "apt".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "apt".into(),
                message: "apt-get upgrade failed".into(),
            });
        }
        Ok(())
    }
}

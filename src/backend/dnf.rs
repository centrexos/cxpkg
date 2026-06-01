use std::process::Command;
use crate::backend::{Backend, command_exists};
use crate::error::{CxpkgError, Result};
use crate::package::{Package, PackageQuery, PackageState, BackendKind};

pub struct DnfBackend;

impl DnfBackend {
    pub fn new() -> Self {
        Self
    }

    fn parse_info_output(&self, output: &str) -> Option<Package> {
        let mut name = String::new();
        let mut version = String::new();
        let mut arch = String::new();
        let mut description = String::new();
        let mut size: u64 = 0;

        for line in output.lines() {
            if let Some(v) = line.strip_prefix("Name         : ") {
                name = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Version      : ") {
                version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Architecture : ") {
                arch = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Summary      : ") {
                description = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Size         : ") {
                size = parse_dnf_size(v.trim());
            }
        }

        if name.is_empty() {
            return None;
        }

        let mut pkg = Package::new(&name, &version, BackendKind::Dnf);
        pkg.arch = arch;
        pkg.description = description;
        pkg.size = size;
        Some(pkg)
    }
}

fn parse_dnf_size(s: &str) -> u64 {
    let (num, unit) = s.split_once(' ').unwrap_or((s, ""));
    let base: u64 = num.replace(',', "").parse().unwrap_or(0);
    match unit.trim() {
        "k" | "K" | "kB" | "KB" => base * 1024,
        "M" | "MB" => base * 1024 * 1024,
        "G" | "GB" => base * 1024 * 1024 * 1024,
        _ => base,
    }
}

impl Backend for DnfBackend {
    fn name(&self) -> &str {
        "dnf"
    }

    fn is_available(&self) -> bool {
        command_exists("dnf")
    }

    fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("dnf")
            .args(["search", "--quiet", query])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            // Format: name.arch : description
            if let Some((name_arch, desc)) = line.split_once(" : ") {
                let name = name_arch.split('.').next().unwrap_or(name_arch).trim();
                let mut pkg = Package::new(name, "", BackendKind::Dnf);
                pkg.description = desc.trim().to_string();
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn info(&self, query: &PackageQuery) -> Result<Option<Package>> {
        let output = Command::new("dnf")
            .args(["info", "--quiet", &query.name])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(self.parse_info_output(&stdout))
    }

    fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("rpm")
            .args(["-qa", "--queryformat", "%{NAME}\t%{VERSION}-%{RELEASE}\t%{ARCH}\t%{SUMMARY}\n"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                let mut pkg = Package::new(parts[0], parts[1], BackendKind::Dnf);
                if parts.len() >= 3 { pkg.arch = parts[2].to_string(); }
                if parts.len() >= 4 { pkg.description = parts[3].to_string(); }
                pkg.state = PackageState::Installed;
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn list_upgradable(&self) -> Result<Vec<Package>> {
        let output = Command::new("dnf")
            .args(["list", "--upgrades", "--quiet"])
            .output()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].split('.').next().unwrap_or(parts[0]);
                let pkg = Package::new(name, parts[1], BackendKind::Dnf);
                packages.push(pkg);
            }
        }
        Ok(packages)
    }

    fn install(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("dnf")
            .args(["install", "-y"])
            .args(packages)
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "dnf".into(),
                message: "dnf install failed".into(),
            });
        }
        Ok(())
    }

    fn remove(&self, packages: &[&str]) -> Result<()> {
        let status = Command::new("dnf")
            .args(["remove", "-y"])
            .args(packages)
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "dnf".into(),
                message: "dnf remove failed".into(),
            });
        }
        Ok(())
    }

    fn update_index(&self) -> Result<()> {
        let status = Command::new("dnf")
            .args(["makecache", "--quiet"])
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "dnf".into(),
                message: "dnf makecache failed".into(),
            });
        }
        Ok(())
    }

    fn upgrade_all(&self) -> Result<()> {
        let status = Command::new("dnf")
            .args(["upgrade", "-y"])
            .status()
            .map_err(|e| CxpkgError::BackendError {
                backend: "dnf".into(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(CxpkgError::BackendError {
                backend: "dnf".into(),
                message: "dnf upgrade failed".into(),
            });
        }
        Ok(())
    }
}

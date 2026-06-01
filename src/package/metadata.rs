use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub description: String,
    pub size: u64,
    pub installed_size: u64,
    pub dependencies: Vec<Dependency>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub backend: BackendKind,
    pub state: PackageState,
    pub section: Option<String>,
    pub homepage: Option<String>,
    pub maintainer: Option<String>,
}

impl Package {
    pub fn new(name: impl Into<String>, version: impl Into<String>, backend: BackendKind) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            arch: String::from("amd64"),
            description: String::new(),
            size: 0,
            installed_size: 0,
            dependencies: Vec::new(),
            provides: Vec::new(),
            conflicts: Vec::new(),
            backend,
            state: PackageState::Available,
            section: None,
            homepage: None,
            maintainer: None,
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{} [{}]", self.name, self.version, self.backend)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub version_req: Option<VersionReq>,
    pub optional: bool,
}

impl Dependency {
    pub fn required(name: impl Into<String>) -> Self {
        Self { name: name.into(), version_req: None, optional: false }
    }

    pub fn with_version(name: impl Into<String>, req: VersionReq) -> Self {
        Self { name: name.into(), version_req: Some(req), optional: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionReq {
    pub op: VersionOp,
    pub version: String,
}

impl VersionReq {
    pub fn new(op: VersionOp, version: impl Into<String>) -> Self {
        Self { op, version: version.into() }
    }

    pub fn satisfies(&self, candidate: &str) -> bool {
        match semver_compare(candidate, &self.version) {
            Some(ord) => match self.op {
                VersionOp::Eq => ord == std::cmp::Ordering::Equal,
                VersionOp::Ge => ord != std::cmp::Ordering::Less,
                VersionOp::Gt => ord == std::cmp::Ordering::Greater,
                VersionOp::Le => ord != std::cmp::Ordering::Greater,
                VersionOp::Lt => ord == std::cmp::Ordering::Less,
            },
            None => false,
        }
    }
}

fn semver_compare(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    let av = parse(a);
    let bv = parse(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let ai = av.get(i).copied().unwrap_or(0);
        let bi = bv.get(i).copied().unwrap_or(0);
        match ai.cmp(&bi) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }
    Some(std::cmp::Ordering::Equal)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VersionOp {
    Eq,
    Ge,
    Gt,
    Le,
    Lt,
}

impl fmt::Display for VersionOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eq => write!(f, "="),
            Self::Ge => write!(f, ">="),
            Self::Gt => write!(f, ">"),
            Self::Le => write!(f, "<="),
            Self::Lt => write!(f, "<"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum PackageState {
    #[default]
    Available,
    Installed,
    Upgradable { current: String, available: String },
    Broken,
}

impl fmt::Display for PackageState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::Installed => write!(f, "installed"),
            Self::Upgradable { current, available } => {
                write!(f, "upgradable ({} -> {})", current, available)
            }
            Self::Broken => write!(f, "broken"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BackendKind {
    #[default]
    Apt,
    Dnf,
    Flatpak,
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Apt => write!(f, "apt"),
            Self::Dnf => write!(f, "dnf"),
            Self::Flatpak => write!(f, "flatpak"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageQuery {
    pub name: String,
    pub version_req: Option<VersionReq>,
    pub backend: Option<BackendKind>,
}

impl PackageQuery {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), version_req: None, backend: None }
    }

    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn with_version(mut self, req: VersionReq) -> Self {
        self.version_req = Some(req);
        self
    }
}

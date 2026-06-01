use thiserror::Error;

#[derive(Error, Debug)]
pub enum CxpkgError {
    #[error("package not found: {0}")]
    PackageNotFound(String),

    #[error("backend error ({backend}): {message}")]
    BackendError { backend: String, message: String },

    #[error("dependency conflict: {0}")]
    DependencyConflict(String),

    #[error("dependency cycle detected involving: {0}")]
    DependencyCycle(String),

    #[error("version constraint not satisfied: {package} requires {constraint}, found {found}")]
    VersionConflict {
        package: String,
        constraint: String,
        found: String,
    },

    #[error("no backend available for package: {0}")]
    NoBackend(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("permission denied: operation requires root")]
    PermissionDenied,
}

pub type Result<T> = std::result::Result<T, CxpkgError>;

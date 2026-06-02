# cxpkg — CentrexOS Unified Package Manager

`cxpkg` is the single package manager for CentrexOS. It provides a unified interface to APT, DNF, and Flatpak backends, resolves cross-backend dependencies, and presents a consistent CLI regardless of the underlying package system.

---

## Usage

```sh
cxpkg install <package>          # Install one or more packages
cxpkg remove  <package>          # Remove packages
cxpkg search  <query>            # Search across all enabled backends
cxpkg info    <package>          # Show detailed package information
cxpkg update                     # Refresh all package indexes
cxpkg upgrade --all              # Upgrade all installed packages
cxpkg list                       # List installed packages
cxpkg list --upgradable          # List packages with available upgrades
cxpkg install <package> --dry-run  # Simulate without making changes
cxpkg config show                  # Print current configuration
```

### Flags

| Flag | Description |
|---|---|
| `-v`, `--verbose` | Enable debug logging |
| `--backend <name>` | Force a specific backend (`apt`, `dnf`, `flatpak`) |
| `-y`, `--yes` | Skip confirmation prompts |
| `-n`, `--dry-run` | Show what would happen without doing it |

---

## Architecture

```
User
 │
 ▼
CLI (clap)
 │
 ▼
Resolver Engine ──── BackendRegistry ──┬── AptBackend
 │                                     ├── DnfBackend
 │                                     └── FlatpakBackend
 ▼
ResolutionPlan
 │
 ▼
Per-backend dispatch
```

The **resolver** handles dependency lookup, topological sorting (Kahn's algorithm), and conflict detection. It is pure — no I/O, no side effects. The **backends** are thin wrappers over system CLI tools and are stateless.

---

## Module Structure

```
cxpkg/src/
├── main.rs          Entry point: CLI dispatch, registry construction
├── cli.rs           clap CLI definitions (Commands, flags)
├── config.rs        TOML config (/etc/cxpkg/config.toml)
├── error.rs         CxpkgError enum (thiserror)
├── package/
│   ├── mod.rs       Re-exports
│   └── metadata.rs  Package, Dependency, VersionReq, BackendKind, PackageState
├── backend/
│   ├── mod.rs       Backend trait + BackendRegistry
│   ├── apt.rs       APT backend (apt-get, apt-cache, dpkg-query)
│   ├── dnf.rs       DNF backend (dnf, rpm)
│   └── flatpak.rs   Flatpak backend (flatpak)
└── resolver/
    └── mod.rs       Resolver, ResolutionPlan, topological sort, conflict check
```

---

## Configuration

`/etc/cxpkg/config.toml`:

```toml
[backends]
apt_enabled     = true
dnf_enabled     = false
flatpak_enabled = true
priority        = ["apt", "flatpak", "dnf"]

[resolver]
allow_downgrades    = false
auto_remove_orphans = false

[cache]
dir          = "/var/cache/cxpkg"
max_age_days = 7
```

`priority` controls backend search order. The first backend that can satisfy a request wins.

---

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` | CLI parsing |
| `serde` / `serde_json` | Package metadata serialisation |
| `toml` | Config file parsing |
| `thiserror` | Typed error variants |
| `anyhow` | Error propagation in binaries |
| `indicatif` | Progress spinners and bars |
| `console` | Coloured terminal output |
| `reqwest` | Reserved for future remote metadata fetching |
| `semver` | Version constraint parsing |
| `log` / `env_logger` | Structured logging |

---

## Building

```sh
cargo build --manifest-path cxpkg/Cargo.toml --release
# Binary: cxpkg/target/release/cxpkg
```

```sh
cargo test --manifest-path cxpkg/Cargo.toml
```

---

## Adding a Backend

1. Create `src/backend/<name>.rs` and implement the `Backend` trait
2. Register it in `main.rs::build_registry()` under the appropriate config key
3. Add it to `BackendConfig::priority` defaults in `config.rs`

See `src/backend/apt.rs` for a complete example.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cxpkg",
    version = env!("CARGO_PKG_VERSION"),
    about = "CentrexOS unified package manager",
    long_about = "cxpkg provides a unified interface to APT, DNF, and Flatpak package backends."
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, global = true, value_name = "BACKEND", help = "Force a specific backend (apt|dnf|flatpak)")]
    pub backend: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(alias = "i", about = "Install one or more packages")]
    Install {
        packages: Vec<String>,
        #[arg(short = 'n', long, help = "Simulate only, do not install")]
        dry_run: bool,
        #[arg(short, long, help = "Skip confirmation prompt")]
        yes: bool,
    },

    #[command(aliases = ["rm", "uninstall"], about = "Remove one or more packages")]
    Remove {
        packages: Vec<String>,
        #[arg(short, long, help = "Skip confirmation prompt")]
        yes: bool,
    },

    #[command(alias = "s", about = "Search for packages by name or keyword")]
    Search {
        query: String,
        #[arg(long, help = "Show extended package info in results")]
        extended: bool,
    },

    #[command(about = "Show detailed info about a package")]
    Info {
        package: String,
    },

    #[command(alias = "up", about = "Update package index from all enabled backends")]
    Update,

    #[command(about = "Upgrade installed packages")]
    Upgrade {
        packages: Vec<String>,
        #[arg(short, long, help = "Upgrade all installed packages")]
        all: bool,
        #[arg(short, long, help = "Skip confirmation prompt")]
        yes: bool,
    },

    #[command(alias = "ls", about = "List installed packages")]
    List {
        #[arg(short, long, help = "Show only upgradable packages")]
        upgradable: bool,
        #[arg(long, value_name = "BACKEND", help = "Filter by backend")]
        backend: Option<String>,
    },

    #[command(about = "Show or edit cxpkg configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,
    #[command(about = "Set a configuration key")]
    Set {
        key: String,
        value: String,
    },
    #[command(about = "Reset configuration to defaults")]
    Reset,
}

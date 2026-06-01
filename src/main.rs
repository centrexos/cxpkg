mod backend;
mod cli;
mod config;
mod error;
mod package;
mod resolver;

use clap::Parser;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use backend::{AptBackend, BackendRegistry, DnfBackend, FlatpakBackend};
use cli::{Cli, Commands, ConfigAction};
use config::Config;
use error::{CxpkgError, Result};
use package::PackageQuery;
use resolver::Resolver;

fn main() {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "warn" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    if let Err(e) = run(cli) {
        eprintln!("{} {}", style("error:").red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let config = Config::load()?;
    let registry = build_registry(&config);

    match cli.command {
        Commands::Install { packages, dry_run, yes } => {
            cmd_install(&registry, &packages, dry_run, yes)
        }
        Commands::Remove { packages, yes } => {
            cmd_remove(&registry, &packages, yes)
        }
        Commands::Search { query, extended } => {
            cmd_search(&registry, &query, extended)
        }
        Commands::Info { package } => {
            cmd_info(&registry, &package)
        }
        Commands::Update => {
            cmd_update(&registry)
        }
        Commands::Upgrade { packages, all, yes } => {
            cmd_upgrade(&registry, &packages, all, yes)
        }
        Commands::List { upgradable, backend } => {
            cmd_list(&registry, upgradable, backend.as_deref())
        }
        Commands::Config { action } => {
            cmd_config(action, &config)
        }
    }
}

fn build_registry(config: &Config) -> BackendRegistry {
    let mut registry = BackendRegistry::new();
    for name in &config.backends.priority {
        match name.as_str() {
            "apt" if config.backends.apt_enabled => {
                registry.register(Box::new(AptBackend::new()));
            }
            "dnf" if config.backends.dnf_enabled => {
                registry.register(Box::new(DnfBackend::new()));
            }
            "flatpak" if config.backends.flatpak_enabled => {
                registry.register(Box::new(FlatpakBackend::new()));
            }
            _ => {}
        }
    }
    registry
}

fn cmd_install(registry: &BackendRegistry, packages: &[String], dry_run: bool, yes: bool) -> Result<()> {
    if packages.is_empty() {
        return Err(CxpkgError::Config("no packages specified".into()));
    }

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    let resolver = Resolver::new(registry);
    let plan = resolver.resolve_install(&pkg_refs)?;

    if plan.is_empty() {
        println!("{} All requested packages are already installed.", style("✓").green());
        return Ok(());
    }

    resolver.check_conflicts(&plan)?;

    println!("{}", style("Install plan:").bold());
    for pkg in &plan.to_install {
        println!("  {} {} ({})", style("+").green(), pkg.name, pkg.backend);
    }
    if !plan.already_satisfied.is_empty() {
        println!("  {} {} already installed", plan.already_satisfied.len(), style("packages").dim());
    }

    let total_size = plan.total_download_size();
    if total_size > 0 {
        println!("  Download: {}", format_size(total_size));
    }

    if dry_run {
        println!("{} Dry run: no changes made.", style("i").blue());
        return Ok(());
    }

    if !yes {
        confirm("Proceed with installation?")?;
    }

    // Group by backend and install
    let mut by_backend: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
    for pkg in &plan.to_install {
        by_backend.entry(pkg.backend.to_string()).or_default().push(&pkg.name);
    }

    for (backend_name, pkg_names) in &by_backend {
        if let Some(backend) = registry.get(backend_name) {
            let spinner = spinner(&format!("Installing via {}...", backend_name));
            backend.install(pkg_names)?;
            spinner.finish_with_message(format!("{} Installed {} package(s) via {}", style("✓").green(), pkg_names.len(), backend_name));
        }
    }

    Ok(())
}

fn cmd_remove(registry: &BackendRegistry, packages: &[String], yes: bool) -> Result<()> {
    if packages.is_empty() {
        return Err(CxpkgError::Config("no packages specified".into()));
    }

    let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    let resolver = Resolver::new(registry);
    let plan = resolver.resolve_remove(&pkg_refs)?;

    println!("{}", style("Remove plan:").bold());
    for pkg in &plan.to_remove {
        println!("  {} {} ({})", style("-").red(), pkg.name, pkg.backend);
    }

    if !yes {
        confirm("Proceed with removal?")?;
    }

    let mut by_backend: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
    for pkg in &plan.to_remove {
        by_backend.entry(pkg.backend.to_string()).or_default().push(&pkg.name);
    }

    for (backend_name, pkg_names) in &by_backend {
        if let Some(backend) = registry.get(backend_name) {
            let spinner = spinner(&format!("Removing via {}...", backend_name));
            backend.remove(pkg_names)?;
            spinner.finish_with_message(format!("{} Removed {} package(s) via {}", style("✓").green(), pkg_names.len(), backend_name));
        }
    }

    Ok(())
}

fn cmd_search(registry: &BackendRegistry, query: &str, extended: bool) -> Result<()> {
    let spinner = spinner("Searching...");
    let results = registry.search_all(query)?;
    spinner.finish_and_clear();

    if results.is_empty() {
        println!("No packages found matching: {}", style(query).yellow());
        return Ok(());
    }

    println!("{} result(s) for {}:\n", results.len(), style(query).bold());
    for pkg in &results {
        if extended {
            println!("{}/{}", style(&pkg.name).bold().cyan(), style(&pkg.version).green());
            if !pkg.description.is_empty() {
                println!("  {}", pkg.description);
            }
            println!("  Backend: {}", pkg.backend);
            if let Some(ref section) = pkg.section {
                println!("  Section: {}", section);
            }
            println!();
        } else {
            println!(
                "{:40} {:15} [{}]",
                style(&pkg.name).bold(),
                style(&pkg.version).green(),
                style(&pkg.backend).dim()
            );
            if !pkg.description.is_empty() {
                let desc = if pkg.description.len() > 70 {
                    format!("{}...", &pkg.description[..67])
                } else {
                    pkg.description.clone()
                };
                println!("  {}", style(desc).dim());
            }
        }
    }
    Ok(())
}

fn cmd_info(registry: &BackendRegistry, package: &str) -> Result<()> {
    let query = PackageQuery::new(package);
    let pkg = registry.find_package(&query)?.ok_or_else(|| CxpkgError::PackageNotFound(package.to_string()))?;

    println!("{}", style(&pkg.name).bold().cyan());
    println!("  Version:     {}", pkg.version);
    println!("  Arch:        {}", pkg.arch);
    println!("  Backend:     {}", pkg.backend);
    println!("  State:       {}", pkg.state);
    if !pkg.description.is_empty() {
        println!("  Description: {}", pkg.description);
    }
    if pkg.size > 0 {
        println!("  Size:        {}", format_size(pkg.size));
    }
    if pkg.installed_size > 0 {
        println!("  Installed:   {}", format_size(pkg.installed_size));
    }
    if let Some(ref hp) = pkg.homepage {
        println!("  Homepage:    {}", hp);
    }
    if let Some(ref section) = pkg.section {
        println!("  Section:     {}", section);
    }
    if !pkg.dependencies.is_empty() {
        let dep_names: Vec<&str> = pkg.dependencies.iter().filter(|d| !d.optional).map(|d| d.name.as_str()).collect();
        println!("  Depends:     {}", dep_names.join(", "));
    }
    Ok(())
}

fn cmd_update(registry: &BackendRegistry) -> Result<()> {
    for backend in registry.all() {
        let spinner = spinner(&format!("Updating {} index...", backend.name()));
        match backend.update_index() {
            Ok(()) => spinner.finish_with_message(format!("{} {} index updated", style("✓").green(), backend.name())),
            Err(e) => spinner.finish_with_message(format!("{} {} failed: {}", style("✗").red(), backend.name(), e)),
        }
    }
    Ok(())
}

fn cmd_upgrade(registry: &BackendRegistry, packages: &[String], all: bool, yes: bool) -> Result<()> {
    if !all && packages.is_empty() {
        return Err(CxpkgError::Config("specify packages or use --all".into()));
    }

    if all {
        if !yes {
            confirm("Upgrade all packages?")?;
        }
        for backend in registry.all() {
            let spinner = spinner(&format!("Upgrading via {}...", backend.name()));
            match backend.upgrade_all() {
                Ok(()) => spinner.finish_with_message(format!("{} {} upgraded", style("✓").green(), backend.name())),
                Err(e) => spinner.finish_with_message(format!("{} {} failed: {}", style("✗").red(), backend.name(), e)),
            }
        }
    } else {
        let pkg_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
        let resolver = Resolver::new(registry);
        let plan = resolver.resolve_upgrade(&pkg_refs)?;

        if plan.to_upgrade.is_empty() {
            println!("{} Nothing to upgrade.", style("✓").green());
            return Ok(());
        }

        println!("{}", style("Upgrade plan:").bold());
        for pkg in &plan.to_upgrade {
            println!("  {} {} ({})", style("↑").yellow(), pkg.name, pkg.backend);
        }

        if !yes {
            confirm("Proceed with upgrade?")?;
        }

        let mut by_backend: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
        for pkg in &plan.to_upgrade {
            by_backend.entry(pkg.backend.to_string()).or_default().push(&pkg.name);
        }

        for (backend_name, pkg_names) in &by_backend {
            if let Some(backend) = registry.get(backend_name) {
                let spinner = spinner(&format!("Upgrading via {}...", backend_name));
                backend.install(pkg_names)?;
                spinner.finish_with_message(format!("{} Upgraded {} package(s)", style("✓").green(), pkg_names.len()));
            }
        }
    }
    Ok(())
}

fn cmd_list(registry: &BackendRegistry, upgradable: bool, backend_filter: Option<&str>) -> Result<()> {
    if upgradable {
        let mut all_upgradable = Vec::new();
        for backend in registry.all() {
            if let Some(name) = backend_filter {
                if backend.name() != name { continue; }
            }
            match backend.list_upgradable() {
                Ok(pkgs) => all_upgradable.extend(pkgs),
                Err(e) => log::warn!("list_upgradable failed for {}: {}", backend.name(), e),
            }
        }
        if all_upgradable.is_empty() {
            println!("{} All packages are up to date.", style("✓").green());
        } else {
            println!("{} upgradable package(s):\n", all_upgradable.len());
            for pkg in &all_upgradable {
                println!("  {:40} {} [{}]", style(&pkg.name).bold(), style(&pkg.version).yellow(), pkg.backend);
            }
        }
    } else {
        let mut all_installed = Vec::new();
        for backend in registry.all() {
            if let Some(name) = backend_filter {
                if backend.name() != name { continue; }
            }
            match backend.list_installed() {
                Ok(pkgs) => all_installed.extend(pkgs),
                Err(e) => log::warn!("list_installed failed for {}: {}", backend.name(), e),
            }
        }
        println!("{} installed package(s):\n", all_installed.len());
        for pkg in &all_installed {
            println!("  {:40} {:20} [{}]", style(&pkg.name).bold(), style(&pkg.version).green(), pkg.backend);
        }
    }
    Ok(())
}

fn cmd_config(action: ConfigAction, config: &Config) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let toml_str = toml::to_string_pretty(config)
                .map_err(|e| CxpkgError::Config(e.to_string()))?;
            println!("{}", toml_str);
        }
        ConfigAction::Set { key, value } => {
            println!("Set {key} = {value} (edit /etc/cxpkg/config.toml to persist)");
        }
        ConfigAction::Reset => {
            let default_config = Config::default();
            default_config.save()?;
            println!("{} Configuration reset to defaults.", style("✓").green());
        }
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<()> {
    use std::io::{self, Write};
    print!("{} [y/N] ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("y") {
        Ok(())
    } else {
        Err(CxpkgError::Config("aborted by user".into()))
    }
}

fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

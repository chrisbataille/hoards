use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use std::collections::HashSet;
use std::thread;

use hoard::{
    all_sources, is_installed, scan_known_tools, scan_missing_tools, scan_path_tools, source_for,
    AiCommands, BundleCommands, Cli, Commands, ConfigCommands, Database, GhCommands,
    InstallSource, Tool, UsageCommands, KNOWN_TOOLS,
    cmd_install, cmd_uninstall, cmd_upgrade,
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install,
    cmd_bundle_list, cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
    cmd_ai_categorize, cmd_ai_describe, cmd_ai_set, cmd_ai_show,
    cmd_ai_suggest_bundle, cmd_ai_test,
    cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit,
    cmd_gh_search, cmd_gh_sync,
    cmd_labels, cmd_recommend, cmd_unused, cmd_usage_scan,
    cmd_usage_show, cmd_usage_tool,
    cmd_doctor, cmd_edit, cmd_export, cmd_import,
    cmd_config_link, cmd_config_unlink, cmd_config_list, cmd_config_show,
    cmd_config_sync, cmd_config_status, cmd_config_edit,
};
use hoard::sources::ManualSource;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    match cli.command {
        Commands::Add {
            name,
            description,
            category,
            source,
            install_cmd,
            binary,
            installed,
        } => cmd_add(&db, name, description, category, source, install_cmd, binary, installed),

        Commands::List {
            installed,
            category,
            label,
            format,
        } => cmd_list(&db, installed, category, label, &format),

        Commands::Search { query } => cmd_search(&db, &query),

        Commands::Show { name } => cmd_show(&db, &name),

        Commands::Remove { name, force } => cmd_remove(&db, &name, force),

        Commands::Scan { dry_run } => cmd_scan(&db, dry_run),

        Commands::Sync { dry_run } => cmd_sync(&db, dry_run),

        Commands::FetchDescriptions { dry_run } => cmd_fetch_descriptions(&db, dry_run),

        Commands::Suggest { category } => cmd_suggest(category),

        Commands::Stats => cmd_stats(&db),

        Commands::Info => cmd_info(),

        Commands::Categories => cmd_categories(&db),

        Commands::Updates { source, cross, tracked, all_versions } => {
            cmd_updates(&db, source, cross, tracked, all_versions)
        }

        Commands::Install { name, source, version, force } => {
            cmd_install(&db, &name, source, version, force)
        }

        Commands::Uninstall { name, remove, force } => {
            cmd_uninstall(&db, &name, remove, force)
        }

        Commands::Upgrade { name, to, version, force } => {
            cmd_upgrade(&db, &name, to, version, force)
        }

        Commands::Bundle(subcmd) => match subcmd {
            BundleCommands::Create { name, tools, description } => {
                cmd_bundle_create(&db, &name, tools, description)
            }
            BundleCommands::List => cmd_bundle_list(&db),
            BundleCommands::Show { name } => cmd_bundle_show(&db, &name),
            BundleCommands::Install { name, force } => cmd_bundle_install(&db, &name, force),
            BundleCommands::Add { name, tools } => cmd_bundle_add(&db, &name, tools),
            BundleCommands::Remove { name, tools } => cmd_bundle_remove(&db, &name, tools),
            BundleCommands::Delete { name, force } => cmd_bundle_delete(&db, &name, force),
            BundleCommands::Update { name, yes } => cmd_bundle_update(&db, &name, yes),
        },

        Commands::Config(subcmd) => match subcmd {
            ConfigCommands::Link { name, target, source, tool } => {
                cmd_config_link(&db, &name, &target, &source, tool)
            }
            ConfigCommands::Unlink { name, remove_symlink, force } => {
                cmd_config_unlink(&db, &name, remove_symlink, force)
            }
            ConfigCommands::List { broken, format } => cmd_config_list(&db, broken, &format),
            ConfigCommands::Show { name } => cmd_config_show(&db, &name),
            ConfigCommands::Sync { dry_run, force } => cmd_config_sync(&db, dry_run, force),
            ConfigCommands::Status => cmd_config_status(&db),
            ConfigCommands::Edit { name, target, source, tool } => {
                cmd_config_edit(&db, &name, target, source, tool)
            }
        },

        Commands::Ai(subcmd) => match subcmd {
            AiCommands::Set { provider } => cmd_ai_set(&provider),
            AiCommands::Show => cmd_ai_show(),
            AiCommands::Test => cmd_ai_test(),
            AiCommands::Categorize { dry_run } => cmd_ai_categorize(dry_run),
            AiCommands::SuggestBundle { count } => cmd_ai_suggest_bundle(count),
            AiCommands::Describe { dry_run, limit } => cmd_ai_describe(dry_run, limit),
        },

        Commands::Gh(subcmd) => match subcmd {
            GhCommands::Sync { dry_run, limit, delay } => cmd_gh_sync(&db, dry_run, limit, delay),
            GhCommands::RateLimit => cmd_gh_rate_limit(),
            GhCommands::Backfill { dry_run } => cmd_gh_backfill(&db, dry_run),
            GhCommands::Fetch { name } => cmd_gh_fetch(&db, &name),
            GhCommands::Search { query, limit } => cmd_gh_search(&query, limit),
            GhCommands::Info { name } => cmd_gh_info(&db, &name),
        },

        Commands::Labels => cmd_labels(&db),

        Commands::Usage(subcmd) => match subcmd {
            UsageCommands::Scan { dry_run, reset } => cmd_usage_scan(&db, dry_run, reset),
            UsageCommands::Show { limit } => cmd_usage_show(&db, limit),
            UsageCommands::Tool { name } => cmd_usage_tool(&db, &name),
        },

        Commands::Unused => cmd_unused(&db),
        Commands::Recommend { count } => cmd_recommend(&db, count),
        Commands::Export { output, format, installed } => cmd_export(&db, output, &format, installed),
        Commands::Import { file, skip_existing, dry_run } => cmd_import(&db, &file, skip_existing, dry_run),
        Commands::Doctor { fix } => cmd_doctor(&db, fix),
        Commands::Edit { name } => cmd_edit(&db, &name),
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_add(
    db: &Database,
    name: String,
    description: Option<String>,
    category: Option<String>,
    source: Option<String>,
    install_cmd: Option<String>,
    binary: Option<String>,
    installed: bool,
) -> Result<()> {
    // Check if tool already exists
    if db.get_tool_by_name(&name)?.is_some() {
        println!("{} Tool '{}' already exists", "!".yellow(), name);
        return Ok(());
    }

    let mut tool = Tool::new(&name);

    if let Some(desc) = description {
        tool = tool.with_description(desc);
    }
    if let Some(cat) = category {
        tool = tool.with_category(cat);
    }
    if let Some(src) = source {
        tool = tool.with_source(InstallSource::from(src.as_str()));
    }
    if let Some(cmd) = install_cmd {
        tool = tool.with_install_command(cmd);
    }
    if let Some(bin) = binary {
        tool = tool.with_binary(bin);
    }
    if installed {
        tool = tool.installed();
    }

    db.insert_tool(&tool)?;
    println!("{} Added '{}'", "+".green(), name);

    Ok(())
}

fn cmd_list(db: &Database, installed_only: bool, category: Option<String>, label: Option<String>, format: &str) -> Result<()> {
    // If filtering by label, use the label-specific query
    let tools = if let Some(lbl) = &label {
        db.list_tools_by_label(lbl)?
    } else {
        db.list_tools(installed_only, category.as_deref())?
    };

    if tools.is_empty() {
        println!("No tools found");
        return Ok(());
    }

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&tools)?);
        }
        _ => {
            // Table format
            println!(
                "{:20} {:12} {:10} {:8} {}",
                "NAME".bold(),
                "CATEGORY".bold(),
                "SOURCE".bold(),
                "STATUS".bold(),
                "DESCRIPTION".bold()
            );
            println!("{}", "-".repeat(80));

            for tool in tools {
                let status = if tool.is_installed {
                    "installed".green()
                } else {
                    "missing".red()
                };

                println!(
                    "{:20} {:12} {:10} {:8} {}",
                    tool.name,
                    tool.category.as_deref().unwrap_or("-"),
                    tool.source.to_string(),
                    status,
                    tool.description.as_deref().unwrap_or("").chars().take(30).collect::<String>()
                );
            }
        }
    }

    Ok(())
}

fn cmd_search(db: &Database, query: &str) -> Result<()> {
    let tools = db.search_tools(query)?;

    if tools.is_empty() {
        println!("No tools found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} tool(s):\n", tools.len());

    for tool in tools {
        let status = if tool.is_installed {
            "installed".green()
        } else {
            "missing".red()
        };

        println!(
            "  {} {} [{}]",
            tool.name.bold(),
            status,
            tool.category.as_deref().unwrap_or("uncategorized")
        );
        if let Some(desc) = &tool.description {
            println!("    {}", desc.dimmed());
        }
    }

    Ok(())
}

fn cmd_show(db: &Database, name: &str) -> Result<()> {
    match db.get_tool_by_name(name)? {
        Some(tool) => {
            println!("{}", tool.name.bold());
            println!("{}", "=".repeat(tool.name.len()));

            if let Some(desc) = &tool.description {
                println!("\n{}", desc);
            }

            println!("\n{}: {}", "Category".bold(), tool.category.as_deref().unwrap_or("-"));
            println!("{}: {}", "Source".bold(), tool.source);

            let status = if tool.is_installed {
                "installed".green()
            } else {
                "not installed".red()
            };
            println!("{}: {}", "Status".bold(), status);

            if let Some(bin) = &tool.binary_name {
                println!("{}: {}", "Binary".bold(), bin);
            }

            if let Some(cmd) = &tool.install_command {
                println!("{}: {}", "Install".bold(), cmd);
            }

            if let Some(notes) = &tool.notes {
                println!("\n{}", "Notes:".bold());
                println!("{}", notes);
            }

            println!("\n{}: {}", "Added".dimmed(), tool.created_at.format("%Y-%m-%d %H:%M"));
        }
        None => {
            println!("Tool '{}' not found", name);
        }
    }

    Ok(())
}

fn cmd_remove(db: &Database, name: &str, force: bool) -> Result<()> {
    if !force {
        print!("Remove tool '{}'? [y/N] ", name);
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    if db.delete_tool(name)? {
        println!("{} Removed '{}'", "-".red(), name);
    } else {
        println!("Tool '{}' not found", name);
    }

    Ok(())
}

fn cmd_scan(db: &Database, dry_run: bool) -> Result<()> {
    let mut added = 0;
    let mut skipped = 0;
    let mut tracked_binaries: HashSet<String> = HashSet::new();

    // Collect binaries already in database
    for tool in db.list_tools(false, None)? {
        if let Some(bin) = tool.binary_name {
            tracked_binaries.insert(bin);
        }
        tracked_binaries.insert(tool.name);
    }

    // Helper to process tools from any source
    let mut process_tools = |tools: Vec<Tool>, source_name: &str, track: bool| -> Result<()> {
        if tools.is_empty() {
            return Ok(());
        }

        println!("{} {} tools:", ">".cyan(), source_name);

        for tool in tools {
            // Track binary for PATH scan exclusion
            if track {
                if let Some(ref bin) = tool.binary_name {
                    tracked_binaries.insert(bin.clone());
                }
                tracked_binaries.insert(tool.name.clone());
            }

            // Check if already in database
            if db.get_tool_by_name(&tool.name)?.is_some() {
                skipped += 1;
                continue;
            }

            println!(
                "  {} {} ({})",
                "+".green(),
                tool.name,
                tool.category.as_deref().unwrap_or("?")
            );

            if !dry_run {
                db.insert_tool(&tool)?;
            }
            added += 1;
        }
        println!();
        Ok(())
    };

    // 1. Scan known tools (curated list with good metadata)
    process_tools(scan_known_tools(), "Known", true)?;

    // 2. Scan all package sources using the trait-based system
    for source in all_sources() {
        // Skip manual source in the main scan loop
        if source.name() == "manual" {
            continue;
        }

        match source.scan() {
            Ok(tools) => {
                let label = format!("{} ({})", source.name(), tools.len());
                process_tools(tools, &label, true)?;
            }
            Err(e) => {
                // Skip silently if source not installed (e.g., brew)
                let err_str = e.to_string();
                if !err_str.contains("No such file") && !err_str.contains("not found") {
                    eprintln!("  {} {} scan: {}", "!".yellow(), source.name(), e);
                }
            }
        }
    }

    // 7. Scan PATH for untracked binaries (go tools, manual installs, etc.)
    match scan_path_tools(&tracked_binaries) {
        Ok(tools) if !tools.is_empty() => {
            println!("{} PATH (untracked) tools:", ">".cyan());
            for tool in tools {
                if db.get_tool_by_name(&tool.name)?.is_some() {
                    skipped += 1;
                    continue;
                }
                println!(
                    "  {} {} ({})",
                    "+".green(),
                    tool.name,
                    tool.category.as_deref().unwrap_or("?")
                );
                if !dry_run {
                    db.insert_tool(&tool)?;
                }
                added += 1;
            }
            println!();
        }
        Ok(_) => {}
        Err(e) => eprintln!("  {} path scan: {}", "!".yellow(), e),
    }

    // Summary
    if added == 0 && skipped == 0 {
        println!("No tools found on system");
    } else if dry_run {
        println!(
            "{} Would add {} tools ({} already tracked)",
            "i".cyan(),
            added,
            skipped
        );
    } else {
        println!(
            "{} Added {} tools ({} already tracked)",
            "+".green(),
            added,
            skipped
        );
    }

    Ok(())
}

fn cmd_sync(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Syncing database with system...\n", ">".cyan());

    let tools = db.list_tools(false, None)?;

    if tools.is_empty() {
        println!("No tools in database. Run 'hoard scan' first.");
        return Ok(());
    }

    let mut changed = 0;

    for tool in tools {
        // Determine binary to check
        let binary = tool.binary_name.as_deref().unwrap_or(&tool.name);
        let currently_installed = is_installed(binary);

        if currently_installed != tool.is_installed {
            let status = if currently_installed {
                "installed".green()
            } else {
                "missing".red()
            };

            println!("  {} {} -> {}", "~".yellow(), tool.name, status);

            if !dry_run {
                db.set_tool_installed(&tool.name, currently_installed)?;
            }
            changed += 1;
        }
    }

    println!();
    if changed == 0 {
        println!("{} Database is in sync", "+".green());
    } else if dry_run {
        println!("{} Would update {} tools", "i".cyan(), changed);
    } else {
        println!("{} Updated {} tools", "+".green(), changed);
    }

    Ok(())
}

/// Extract package name from install command (e.g., "cargo install git-delta" -> "git-delta")
fn extract_package_from_install_cmd(cmd: &str) -> Option<String> {
    // Common prefixes to strip
    let prefixes = [
        "cargo install ",
        "pip install ",
        "npm install -g ",
        "brew install ",
        "sudo apt install ",
    ];

    for prefix in prefixes {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            let pkg = rest.split_whitespace().next().unwrap_or("");
            if !pkg.is_empty() && !pkg.starts_with('-') {
                return Some(pkg.to_string());
            }
        }
    }
    None
}

/// Fetch description for a single tool, trying multiple sources
fn fetch_tool_description(tool: &Tool) -> Option<(String, &'static str)> {
    let binary = tool.binary_name.as_deref().unwrap_or(&tool.name);

    // Extract actual package name from install command if available
    let pkg = tool
        .install_command
        .as_ref()
        .and_then(|c| extract_package_from_install_cmd(c))
        .unwrap_or_else(|| tool.name.clone());

    // Try package registry first based on source
    if let Some(source) = source_for(&tool.source)
        && let Some(desc) = source.fetch_description(&pkg) {
            return Some((desc, source.name()));
        }

    // Fallback to man page, then --help
    ManualSource::fetch_man_description(binary)
        .map(|d| (d, "man"))
        .or_else(|| ManualSource::fetch_help_description(binary).map(|d| (d, "--help")))
}

fn cmd_fetch_descriptions(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Fetching missing descriptions...\n", ">".cyan());

    let tools = db.list_tools(false, None)?;

    // Filter tools without descriptions
    let tools_without_desc: Vec<_> = tools
        .into_iter()
        .filter(|t| t.description.is_none())
        .collect();

    if tools_without_desc.is_empty() {
        println!("{} All tools already have descriptions", "+".green());
        return Ok(());
    }

    let count = tools_without_desc.len();
    println!("  Found {} tools without descriptions", count);
    println!("  Fetching in parallel...\n");

    // Fetch descriptions in parallel using scoped threads
    let results: Vec<_> = thread::scope(|s| {
        let handles: Vec<_> = tools_without_desc
            .iter()
            .map(|tool| {
                s.spawn(move || {
                    let desc = fetch_tool_description(tool);
                    (tool.name.clone(), desc)
                })
            })
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Process results and update database
    let mut updated = 0;

    for (name, result) in results {
        if let Some((desc, source)) = result {
            println!(
                "  {} {} [{}]: {}",
                "+".green(),
                name,
                source.dimmed(),
                desc.chars().take(60).collect::<String>()
            );

            if !dry_run {
                db.update_tool_description(&name, &desc)?;
            }
            updated += 1;
        } else {
            println!("  {} {}: no description found", "-".dimmed(), name.dimmed());
        }
    }

    println!();
    if updated == 0 {
        println!("{} No descriptions found to update", "i".cyan());
    } else if dry_run {
        println!("{} Would update {} descriptions", "i".cyan(), updated);
    } else {
        println!("{} Updated {} descriptions", "+".green(), updated);
    }

    Ok(())
}

fn cmd_suggest(category: Option<String>) -> Result<()> {
    println!("{} Tools you might want to try:\n", ">".cyan());

    let missing = scan_missing_tools();

    let filtered: Vec<_> = if let Some(ref cat) = category {
        missing
            .into_iter()
            .filter(|t| t.category.as_deref() == Some(cat.as_str()))
            .collect()
    } else {
        missing
    };

    if filtered.is_empty() {
        println!("No suggestions available");
        return Ok(());
    }

    // Group by category
    let mut by_category: std::collections::HashMap<&str, Vec<&Tool>> = std::collections::HashMap::new();
    for tool in &filtered {
        let cat = tool.category.as_deref().unwrap_or("other");
        by_category.entry(cat).or_default().push(tool);
    }

    for (cat, tools) in by_category.iter() {
        println!("{}:", cat.bold());
        for tool in tools {
            println!(
                "  {} - {}",
                tool.name.cyan(),
                tool.description.as_deref().unwrap_or("")
            );
            if let Some(cmd) = &tool.install_command {
                println!("    {}", cmd.dimmed());
            }
        }
        println!();
    }

    Ok(())
}

fn cmd_stats(db: &Database) -> Result<()> {
    let (total, installed, favorites) = db.get_stats()?;
    let categories = db.get_categories()?;

    println!("{}", "Hoard Statistics".bold());
    println!("{}", "=".repeat(20));
    println!();
    println!("Total tools:     {}", total);
    println!("Installed:       {}", installed.to_string().green());
    println!("Missing:         {}", (total - installed).to_string().red());
    println!("Favorites:       {}", favorites.to_string().yellow());
    println!("Categories:      {}", categories.len());
    println!();
    println!("Known tools:     {}", KNOWN_TOOLS.len());

    Ok(())
}

fn cmd_info() -> Result<()> {
    let db_path = Database::db_path()?;

    println!("{}", "Hoard Info".bold());
    println!("{}", "=".repeat(20));
    println!();
    println!("Database: {}", db_path.display());

    if db_path.exists() {
        let metadata = std::fs::metadata(&db_path)?;
        let size = metadata.len();
        println!("Size:     {} bytes", size);
    } else {
        println!("Status:   {} (will be created on first use)", "not created".yellow());
    }

    Ok(())
}

fn cmd_categories(db: &Database) -> Result<()> {
    let category_counts = db.get_category_counts()?;

    if category_counts.is_empty() {
        println!("No categories found. Add some tools first.");
        return Ok(());
    }

    println!("{}", "Categories".bold());
    println!();
    for (cat, count) in category_counts {
        println!("  {} ({})", cat, count);
    }

    Ok(())
}

fn cmd_updates(
    db: &Database,
    source_filter: Option<String>,
    cross: bool,
    tracked: bool,
    all_versions: bool,
) -> Result<()> {
    use hoard::updates::*;

    if cross {
        return cmd_updates_cross(db);
    }

    // If --tracked or --all-versions, use the tracked tools mode
    if tracked || all_versions {
        return cmd_updates_tracked(db, source_filter, all_versions);
    }

    println!("{} Checking for updates...\n", ">".cyan());

    let mut total_updates = 0;

    let check_source = |name: &str, check_fn: fn() -> Result<Vec<Update>>| -> Result<usize> {
        print!("  {} {}... ", ">".cyan(), name);
        std::io::Write::flush(&mut std::io::stdout())?;

        match check_fn() {
            Ok(updates) if updates.is_empty() => {
                println!("{}", "up to date".green());
                Ok(0)
            }
            Ok(updates) => {
                println!("{} available", updates.len().to_string().yellow());
                for update in &updates {
                    println!(
                        "    {} {} -> {}",
                        update.name.bold(),
                        update.current.dimmed(),
                        update.latest.green()
                    );
                }
                Ok(updates.len())
            }
            Err(e) => {
                println!("{} ({})", "skipped".dimmed(), e.to_string().chars().take(30).collect::<String>());
                Ok(0)
            }
        }
    };

    #[allow(clippy::type_complexity)]
    let sources: Vec<(&str, fn() -> Result<Vec<Update>>)> = vec![
        ("cargo", check_cargo_updates),
        ("pip", check_pip_updates),
        ("npm", check_npm_updates),
        ("apt", check_apt_updates),
        ("brew", check_brew_updates),
    ];

    for (name, check_fn) in sources {
        if let Some(ref filter) = source_filter
            && filter != name {
                continue;
            }
        total_updates += check_source(name, check_fn)?;
    }

    println!();
    if total_updates == 0 {
        println!("{} All tools are up to date!", "+".green());
    } else {
        println!(
            "{} {} update(s) available",
            "!".yellow(),
            total_updates
        );
    }

    Ok(())
}

fn cmd_updates_tracked(
    db: &Database,
    source_filter: Option<String>,
    all_versions: bool,
) -> Result<()> {
    use hoard::updates::*;

    println!(
        "{} Checking tracked tools for updates{}...\n",
        ">".cyan(),
        if all_versions { " (all versions)" } else { "" }
    );

    // Get all installed tools from database
    let tools = db.list_tools(true, None)?;

    // Filter by source if specified
    let tools: Vec<_> = tools
        .into_iter()
        .filter(|t| {
            if let Some(ref filter) = source_filter {
                t.source.to_string() == *filter
            } else {
                // Only check sources we can query (cargo, pip, npm)
                matches!(t.source.to_string().as_str(), "cargo" | "pip" | "npm")
            }
        })
        .collect();

    if tools.is_empty() {
        println!("No tracked tools found for the specified source(s).");
        println!("  Note: Only cargo, pip, and npm tools can be checked for updates.");
        return Ok(());
    }

    let mut updates_found = 0;

    for tool in &tools {
        let source = tool.source.to_string();

        // Get current installed version
        let current = match get_installed_version(&tool.name, &source) {
            Some(v) => v,
            None => continue,
        };

        if all_versions {
            // Get all newer versions
            let versions = get_available_versions(&tool.name, &source, &current);
            if !versions.is_empty() {
                updates_found += 1;
                println!(
                    "  {} ({}) {} -> ",
                    tool.name.bold(),
                    source.cyan(),
                    current.dimmed()
                );
                for (i, ver) in versions.iter().enumerate() {
                    let marker = if i == versions.len() - 1 { "(latest)" } else { "" };
                    println!("    {} {}", ver.green(), marker.dimmed());
                }
            }
        } else {
            // Just check for latest
            let latest = match &source[..] {
                "cargo" => get_crates_io_latest(&tool.name),
                "pip" => get_pypi_latest(&tool.name),
                "npm" => get_npm_latest(&tool.name),
                _ => None,
            };

            if let Some(latest) = latest
                && version_is_newer(&latest, &current) {
                    updates_found += 1;
                    println!(
                        "  {} ({}) {} -> {}",
                        tool.name.bold(),
                        source.cyan(),
                        current.dimmed(),
                        latest.green()
                    );
                }
        }
    }

    println!();
    if updates_found == 0 {
        println!("{} All tracked tools are up to date!", "+".green());
    } else {
        println!(
            "{} {} tool(s) have updates available",
            "!".yellow(),
            updates_found
        );
        if all_versions {
            println!(
                "  Use {} to install a specific version",
                "hoard upgrade <tool> --version <ver>".cyan()
            );
        }
    }

    Ok(())
}

fn cmd_updates_cross(db: &Database) -> Result<()> {
    use hoard::updates::*;

    println!("{} Checking apt/snap tools for newer versions on other sources...\n", ">".cyan());

    // Get all apt/snap tools from database with their versions
    let tools = db.list_tools(true, None)?;
    let apt_snap_tools: Vec<(String, String, String)> = tools
        .into_iter()
        .filter(|t| {
            let source = t.source.to_string();
            source == "apt" || source == "snap"
        })
        .filter_map(|t| {
            // Get current installed version
            let version = get_apt_version(&t.name)?;
            Some((t.name, version, t.source.to_string()))
        })
        .collect();

    if apt_snap_tools.is_empty() {
        println!("No apt/snap tools found in database.");
        return Ok(());
    }

    println!("  Checking {} apt/snap tools...\n", apt_snap_tools.len());

    let upgrades = check_cross_source_upgrades(&apt_snap_tools);

    if upgrades.is_empty() {
        println!("{} No cross-source upgrades found.", "+".green());
        println!("  All apt/snap tools are either up-to-date or not available on cargo/pip/npm.");
    } else {
        println!("{} {} tool(s) have newer versions on other sources:\n", "!".yellow(), upgrades.len());

        for upgrade in &upgrades {
            println!(
                "  {} ({} {}) -> {} {} {}",
                upgrade.name.bold(),
                upgrade.current_source.dimmed(),
                upgrade.current_version.dimmed(),
                upgrade.better_source.cyan(),
                upgrade.better_version.green(),
                format!("({})", upgrade.better_source).dimmed()
            );

            // Show install command
            let install_cmd = match upgrade.better_source.as_str() {
                "cargo" => format!("cargo install {}", upgrade.name),
                "pip" => format!("pip install {}", upgrade.name),
                "npm" => format!("npm install -g {}", upgrade.name),
                _ => String::new(),
            };
            if !install_cmd.is_empty() {
                println!("    {}", install_cmd.dimmed());
            }
        }
    }

    Ok(())
}

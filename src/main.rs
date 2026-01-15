use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use colored::Colorize;

use std::collections::HashSet;
use std::thread;

use hoards::sources::ManualSource;
use hoards::{
    AiCommands, AiConfigCommands, BundleCommands, Cli, Commands, CompletionsCommands,
    ConfigCommands, Database, DiscoverCommands, GhCommands, HoardConfig, InsightsCommands,
    InstallSource, KNOWN_TOOLS, Tool, UsageCommands, all_sources, cmd_ai_analyze,
    cmd_ai_bundle_cheatsheet, cmd_ai_categorize, cmd_ai_cheatsheet, cmd_ai_describe,
    cmd_ai_discover, cmd_ai_extract, cmd_ai_set, cmd_ai_show, cmd_ai_suggest_bundle, cmd_ai_test,
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install, cmd_bundle_list,
    cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update, cmd_completions_install,
    cmd_completions_status, cmd_completions_uninstall, cmd_config_edit, cmd_config_link,
    cmd_config_list, cmd_config_show, cmd_config_status, cmd_config_sync, cmd_config_unlink,
    cmd_doctor, cmd_edit, cmd_export, cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info,
    cmd_gh_rate_limit, cmd_gh_search, cmd_gh_sync, cmd_import, cmd_install, cmd_labels,
    cmd_recommend, cmd_uninstall, cmd_unused, cmd_upgrade, cmd_usage_config, cmd_usage_init,
    cmd_usage_log, cmd_usage_reset, cmd_usage_scan, cmd_usage_show, cmd_usage_tool, is_installed,
    scan_known_tools, scan_missing_tools, scan_path_tools, source_for,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::open()?;

    match cli.command {
        // ============================================
        // CORE COMMANDS
        // ============================================
        Commands::Add {
            name,
            description,
            category,
            source,
            install_cmd,
            binary,
            installed,
        } => cmd_add(
            &db,
            name,
            description,
            category,
            source,
            install_cmd,
            binary,
            installed,
        ),

        Commands::Show { name } => cmd_show(&db, &name),

        Commands::Remove { name, force } => cmd_remove(&db, &name, force),

        Commands::Edit { name } => cmd_edit(&db, &name),

        // ============================================
        // SYNC - Unified sync command
        // ============================================
        Commands::Sync {
            dry_run,
            scan,
            github,
            usage,
            descriptions,
            all,
            limit,
            delay,
        } => {
            let do_scan = scan || all;
            let do_github = github || all;
            let do_usage = usage || all;
            let do_descriptions = descriptions || all;

            // Always sync installation status
            cmd_sync_status(&db, dry_run)?;

            // Optional: scan for new tools
            if do_scan {
                println!();
                cmd_scan(&db, dry_run)?;
            }

            // Optional: fetch descriptions
            if do_descriptions {
                println!();
                cmd_fetch_descriptions(&db, dry_run)?;
            }

            // Optional: GitHub sync
            if do_github {
                println!();
                cmd_gh_sync(&db, dry_run, limit, delay)?;
            }

            // Optional: usage scan
            if do_usage {
                println!();
                cmd_usage_scan(&db, dry_run, false)?;
            }

            Ok(())
        }

        // ============================================
        // DISCOVER - Tool discovery commands
        // ============================================
        Commands::Discover(subcmd) => match subcmd {
            DiscoverCommands::List {
                installed,
                category,
                label,
                format,
            } => cmd_list(&db, installed, category, label, &format),
            DiscoverCommands::Search {
                query,
                github,
                limit,
            } => {
                cmd_search(&db, &query)?;
                if github {
                    println!("\n{}", "GitHub Results:".bold());
                    cmd_gh_search(&query, limit)?;
                }
                Ok(())
            }
            DiscoverCommands::Categories => cmd_categories(&db),
            DiscoverCommands::Labels => cmd_labels(&db),
            DiscoverCommands::Missing { category } => cmd_suggest(category),
            DiscoverCommands::Recommended { count } => cmd_recommend(&db, count),
            DiscoverCommands::Similar { tool } => cmd_similar(&db, &tool),
            DiscoverCommands::Trending { category, limit } => cmd_trending(&db, category, limit),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // INSIGHTS - Analytics and health commands
        // ============================================
        Commands::Insights(subcmd) => match subcmd {
            InsightsCommands::Usage { tool, limit } => {
                if let Some(name) = tool {
                    cmd_usage_tool(&db, &name)
                } else {
                    cmd_usage_show(&db, limit)
                }
            }
            InsightsCommands::Unused => cmd_unused(&db),
            InsightsCommands::Health { fix } => cmd_doctor(&db, fix),
            InsightsCommands::Stats => cmd_stats(&db),
            InsightsCommands::Overview => cmd_overview(&db),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // AI - AI-powered features
        // ============================================
        Commands::Ai(subcmd) => match subcmd {
            AiCommands::Config(config_cmd) => match config_cmd {
                AiConfigCommands::Set { provider } => cmd_ai_set(&provider),
                AiConfigCommands::Show => cmd_ai_show(),
                AiConfigCommands::Test => cmd_ai_test(),
                _ => unreachable!("all variants covered"),
            },
            AiCommands::Enrich {
                categorize,
                describe,
                all,
                dry_run,
                limit,
            } => {
                let do_categorize = categorize || all;
                let do_describe = describe || all;

                if !do_categorize && !do_describe {
                    println!(
                        "{} Specify --categorize, --describe, or --all",
                        "!".yellow()
                    );
                    println!("  Example: hoard ai enrich --all");
                    return Ok(());
                }

                if do_categorize {
                    cmd_ai_categorize(dry_run)?;
                }
                if do_describe {
                    if do_categorize {
                        println!();
                    }
                    cmd_ai_describe(dry_run, limit)?;
                }
                Ok(())
            }
            AiCommands::SuggestBundle { count } => cmd_ai_suggest_bundle(count),
            AiCommands::Extract {
                urls,
                yes,
                dry_run,
                delay,
            } => cmd_ai_extract(&db, urls, yes, dry_run, delay),
            AiCommands::Cheatsheet {
                tool,
                bundle,
                refresh,
            } => {
                match (tool, bundle) {
                    (Some(t), None) => cmd_ai_cheatsheet(&t, refresh),
                    (None, Some(b)) => cmd_ai_bundle_cheatsheet(&b, refresh),
                    (None, None) => {
                        eprintln!("Error: Either --tool or --bundle must be specified");
                        std::process::exit(1);
                    }
                    (Some(_), Some(_)) => unreachable!(), // conflicts_with handles this
                }
            }
            AiCommands::Discover {
                query,
                limit,
                no_stars,
                dry_run,
            } => cmd_ai_discover(&db, &query, limit, no_stars, dry_run),
            AiCommands::Analyze {
                json,
                no_ai,
                min_uses,
            } => cmd_ai_analyze(&db, json, no_ai, min_uses),
            // Backward compatibility aliases
            AiCommands::Set { provider } => cmd_ai_set(&provider),
            AiCommands::ShowConfig => cmd_ai_show(),
            AiCommands::Test => cmd_ai_test(),
            AiCommands::Categorize { dry_run } => cmd_ai_categorize(dry_run),
            AiCommands::Describe { dry_run, limit } => cmd_ai_describe(dry_run, limit),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // WORKFLOW COMMANDS
        // ============================================
        Commands::Init { auto } => cmd_init(&db, auto),

        Commands::Maintain { auto, dry_run } => cmd_maintain(&db, auto, dry_run),

        Commands::Cleanup { force, dry_run } => cmd_cleanup(&db, force, dry_run),

        // ============================================
        // INSTALL/UNINSTALL/UPGRADE
        // ============================================
        Commands::Install {
            name,
            source,
            version,
            force,
        } => cmd_install(&db, &name, source, version, force),

        Commands::Uninstall {
            name,
            remove,
            force,
        } => cmd_uninstall(&db, &name, remove, force),

        Commands::Upgrade {
            name,
            to,
            version,
            force,
        } => cmd_upgrade(&db, &name, to, version, force),

        Commands::Updates {
            source,
            cross,
            tracked,
            all_versions,
        } => cmd_updates(&db, source, cross, tracked, all_versions),

        // ============================================
        // BUNDLES & CONFIG
        // ============================================
        Commands::Bundle(subcmd) => match subcmd {
            BundleCommands::Create {
                name,
                tools,
                description,
            } => cmd_bundle_create(&db, &name, tools, description),
            BundleCommands::List => cmd_bundle_list(&db),
            BundleCommands::Show { name } => cmd_bundle_show(&db, &name),
            BundleCommands::Install { name, force } => cmd_bundle_install(&db, &name, force),
            BundleCommands::Add { name, tools } => cmd_bundle_add(&db, &name, tools),
            BundleCommands::Remove { name, tools } => cmd_bundle_remove(&db, &name, tools),
            BundleCommands::Delete { name, force } => cmd_bundle_delete(&db, &name, force),
            BundleCommands::Update { name, yes } => cmd_bundle_update(&db, &name, yes),
            _ => unreachable!("all variants covered"),
        },

        Commands::Config(subcmd) => match subcmd {
            ConfigCommands::Link {
                name,
                target,
                source,
                tool,
            } => cmd_config_link(&db, &name, &target, &source, tool),
            ConfigCommands::Unlink {
                name,
                remove_symlink,
                force,
            } => cmd_config_unlink(&db, &name, remove_symlink, force),
            ConfigCommands::List { broken, format } => cmd_config_list(&db, broken, &format),
            ConfigCommands::Show { name } => cmd_config_show(&db, &name),
            ConfigCommands::Sync { dry_run, force } => cmd_config_sync(&db, dry_run, force),
            ConfigCommands::Status => cmd_config_status(&db),
            ConfigCommands::Edit {
                name,
                target,
                source,
                tool,
            } => cmd_config_edit(&db, &name, target, source, tool),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // IMPORT/EXPORT
        // ============================================
        Commands::Export {
            output,
            format,
            installed,
        } => cmd_export(&db, output, &format, installed),
        Commands::Import {
            file,
            skip_existing,
            dry_run,
        } => cmd_import(&db, &file, skip_existing, dry_run),

        // ============================================
        // GITHUB (power user)
        // ============================================
        Commands::Gh(subcmd) => match subcmd {
            GhCommands::Sync {
                dry_run,
                limit,
                delay,
            } => cmd_gh_sync(&db, dry_run, limit, delay),
            GhCommands::RateLimit => cmd_gh_rate_limit(),
            GhCommands::Backfill { dry_run } => cmd_gh_backfill(&db, dry_run),
            GhCommands::Fetch { name } => cmd_gh_fetch(&db, &name),
            GhCommands::Search { query, limit } => cmd_gh_search(&query, limit),
            GhCommands::Info { name } => cmd_gh_info(&db, &name),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // SHELL COMPLETIONS
        // ============================================
        Commands::Completions(subcmd) => match subcmd {
            CompletionsCommands::Generate { shell } => {
                let mut cmd = hoards::Cli::command();
                generate(shell, &mut cmd, "hoards", &mut std::io::stdout());
                Ok(())
            }
            CompletionsCommands::Install { shell, force } => cmd_completions_install(shell, force),
            CompletionsCommands::Status => cmd_completions_status(),
            CompletionsCommands::Uninstall { shell } => cmd_completions_uninstall(shell),
            _ => unreachable!("all variants covered"),
        },

        // ============================================
        // BACKWARD COMPATIBILITY ALIASES
        // ============================================
        Commands::List {
            installed,
            category,
            label,
            format,
        } => cmd_list(&db, installed, category, label, &format),

        Commands::Search { query } => cmd_search(&db, &query),

        Commands::Scan { dry_run } => cmd_scan(&db, dry_run),

        Commands::FetchDescriptions { dry_run } => cmd_fetch_descriptions(&db, dry_run),

        Commands::Suggest { category } => cmd_suggest(category),

        Commands::Stats => cmd_stats(&db),

        Commands::Info => cmd_info(),

        Commands::Categories => cmd_categories(&db),

        Commands::Labels => cmd_labels(&db),

        Commands::Usage(subcmd) => match subcmd {
            UsageCommands::Scan { dry_run, reset } => cmd_usage_scan(&db, dry_run, reset),
            UsageCommands::Show { limit } => cmd_usage_show(&db, limit),
            UsageCommands::Tool { name } => cmd_usage_tool(&db, &name),
            UsageCommands::Log { command } => cmd_usage_log(&db, &command),
            UsageCommands::Init { shell } => {
                let config = HoardConfig::load()?;
                cmd_usage_init(&config, shell)
            }
            UsageCommands::Config { mode } => {
                let mut config = HoardConfig::load()?;
                cmd_usage_config(&mut config, mode)
            }
            UsageCommands::Reset { force } => cmd_usage_reset(&db, force),
            _ => unreachable!("all variants covered"),
        },

        Commands::Unused => cmd_unused(&db),
        Commands::Recommend { count } => cmd_recommend(&db, count),
        Commands::Doctor { fix } => cmd_doctor(&db, fix),
        _ => unreachable!("all variants covered"),
    }
}

// ============================================
// CORE COMMAND IMPLEMENTATIONS
// ============================================

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

fn cmd_list(
    db: &Database,
    installed_only: bool,
    category: Option<String>,
    label: Option<String>,
    format: &str,
) -> Result<()> {
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };
    use hoards::icons::{category_icon, print_legend_compact, source_icon, status_icon};

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
            let term_width = terminal_size::terminal_size()
                .map(|(w, _)| w.0)
                .unwrap_or(120);

            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .apply_modifier(UTF8_ROUND_CORNERS)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_width(term_width)
                .set_header(vec![
                    Cell::new("Name").fg(Color::Cyan),
                    Cell::new("Cat").fg(Color::Cyan),
                    Cell::new("Src").fg(Color::Cyan),
                    Cell::new("✓").fg(Color::Cyan),
                    Cell::new("Description").fg(Color::Cyan),
                ]);

            for tool in &tools {
                let cat = tool.category.as_deref().unwrap_or("-");
                let cat_display = format!("{} {}", category_icon(cat), cat);

                let src = tool.source.to_string();
                let src_display = source_icon(&src).to_string();

                let status_cell = if tool.is_installed {
                    Cell::new(status_icon(true)).fg(Color::Green)
                } else {
                    Cell::new(status_icon(false)).fg(Color::Red)
                };

                let desc = tool.description.as_deref().unwrap_or("");

                table.add_row(vec![
                    Cell::new(&tool.name),
                    Cell::new(cat_display),
                    Cell::new(src_display),
                    status_cell,
                    Cell::new(desc),
                ]);
            }

            println!("{table}");
            print_legend_compact();
            println!("{} {} tools", ">".cyan(), tools.len());
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

            println!(
                "\n{}: {}",
                "Category".bold(),
                tool.category.as_deref().unwrap_or("-")
            );
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

            // Show GitHub info if available
            if let Ok(Some(gh_info)) = db.get_github_info(&tool.name) {
                println!("\n{}", "GitHub:".bold());
                println!("  Repo: {}/{}", gh_info.repo_owner, gh_info.repo_name);
                println!("  Stars: {}", gh_info.stars.to_string().yellow());
            }

            // Show usage if available
            if let Ok(Some(usage)) = db.get_usage(&tool.name)
                && usage.use_count > 0
            {
                println!(
                    "\n{}: {} times",
                    "Usage".bold(),
                    usage.use_count.to_string().cyan()
                );
            }

            if let Some(notes) = &tool.notes {
                println!("\n{}", "Notes:".bold());
                println!("{}", notes);
            }

            println!(
                "\n{}: {}",
                "Added".dimmed(),
                tool.created_at.format("%Y-%m-%d %H:%M")
            );
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

// ============================================
// SYNC COMMAND IMPLEMENTATIONS
// ============================================

fn cmd_sync_status(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Syncing installation status...\n", ">".cyan());

    let tools = db.list_tools(false, None)?;

    if tools.is_empty() {
        println!("No tools in database. Run 'hoards sync --scan' first.");
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

    if changed == 0 {
        println!("{} Database is in sync", "+".green());
    } else if dry_run {
        println!("{} Would update {} tools", "i".cyan(), changed);
    } else {
        println!("{} Updated {} tools", "+".green(), changed);
    }

    Ok(())
}

fn cmd_scan(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Scanning for new tools...\n", ">".cyan());

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

    // Scan PATH for untracked binaries (go tools, manual installs, etc.)
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
        println!("No new tools found on system");
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
        && let Some(desc) = source.fetch_description(&pkg)
    {
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

// ============================================
// DISCOVER COMMAND IMPLEMENTATIONS
// ============================================

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
    let mut by_category: std::collections::HashMap<&str, Vec<&Tool>> =
        std::collections::HashMap::new();
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

fn cmd_similar(db: &Database, tool_name: &str) -> Result<()> {
    let tool = match db.get_tool_by_name(tool_name)? {
        Some(t) => t,
        None => {
            println!("Tool '{}' not found", tool_name);
            return Ok(());
        }
    };

    println!("{} Tools similar to '{}':\n", ">".cyan(), tool_name.bold());

    // Find tools in the same category
    let mut similar: Vec<Tool> = Vec::new();

    if let Some(ref cat) = tool.category {
        let same_category = db.list_tools(false, Some(cat))?;
        for t in same_category {
            if t.name != tool_name {
                similar.push(t);
            }
        }
    }

    if similar.is_empty() {
        println!("No similar tools found");
        return Ok(());
    }

    // Sort alphabetically
    similar.sort_by(|a, b| a.name.cmp(&b.name));

    for t in similar.iter().take(10) {
        let status = if t.is_installed {
            "installed".green()
        } else {
            "not installed".dimmed()
        };

        println!("  {} {} [{}]", t.name.bold(), status, t.source);
        if let Some(desc) = &t.description {
            println!("    {}", desc.dimmed());
        }
    }

    Ok(())
}

fn cmd_trending(db: &Database, category: Option<String>, limit: usize) -> Result<()> {
    println!("{} Trending tools by GitHub stars:\n", ">".cyan());

    let tools = db.list_tools(false, category.as_deref())?;

    // Collect tools with their GitHub star counts
    let mut tools_with_stars: Vec<(Tool, i64)> = Vec::new();
    for tool in tools {
        if let Ok(Some(gh_info)) = db.get_github_info(&tool.name) {
            tools_with_stars.push((tool, gh_info.stars));
        }
    }

    // Sort by stars descending
    tools_with_stars.sort_by(|a, b| b.1.cmp(&a.1));

    if tools_with_stars.is_empty() {
        println!("No tools with GitHub star data found.");
        println!("Run 'hoards sync --github' to fetch star counts.");
        return Ok(());
    }

    for (tool, stars) in tools_with_stars.iter().take(limit) {
        let status = if tool.is_installed {
            "✓".green()
        } else {
            " ".normal()
        };

        println!(
            "  {} {:>6} ★  {}  [{}]",
            status,
            stars.to_string().yellow(),
            tool.name.bold(),
            tool.category.as_deref().unwrap_or("-")
        );
    }

    Ok(())
}

// ============================================
// INSIGHTS COMMAND IMPLEMENTATIONS
// ============================================

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
        println!(
            "Status:   {} (will be created on first use)",
            "not created".yellow()
        );
    }

    Ok(())
}

fn cmd_overview(db: &Database) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "         HOARD OVERVIEW DASHBOARD       ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Stats
    let (total, installed, _favorites) = db.get_stats()?;
    println!(
        "📦 {} tools tracked ({} installed, {} missing)",
        total.to_string().bold(),
        installed.to_string().green(),
        (total - installed).to_string().red()
    );
    println!();

    // Top used tools (get from usage table)
    println!("{}", "📊 Top Used Tools:".bold());
    let tools = db.list_tools(true, None)?;

    // Collect usage data for each tool
    let mut tools_with_usage: Vec<(String, i64)> = Vec::new();
    for tool in &tools {
        if let Ok(Some(usage)) = db.get_usage(&tool.name)
            && usage.use_count > 0
        {
            tools_with_usage.push((tool.name.clone(), usage.use_count));
        }
    }
    tools_with_usage.sort_by(|a, b| b.1.cmp(&a.1));

    if tools_with_usage.is_empty() {
        println!("   (no usage data - run 'hoards sync --usage')");
    } else {
        for (name, count) in tools_with_usage.iter().take(5) {
            println!("   {:20} {} uses", name, count.to_string().cyan());
        }
    }
    println!();

    // Health check
    println!("{}", "🔍 Quick Health Check:".bold());

    let db_path = Database::db_path()?;
    if db_path.exists() {
        let metadata = std::fs::metadata(&db_path)?;
        println!(
            "   Database: {} ({} KB)",
            "OK".green(),
            metadata.len() / 1024
        );
    }

    let missing_desc: usize = tools.iter().filter(|t| t.description.is_none()).count();
    if missing_desc > 0 {
        println!(
            "   {} tools missing descriptions (run 'hoards sync --descriptions')",
            missing_desc.to_string().yellow()
        );
    } else {
        println!("   Descriptions: {}", "All tools have descriptions".green());
    }

    let uncategorized: usize = tools.iter().filter(|t| t.category.is_none()).count();
    if uncategorized > 0 {
        println!(
            "   {} tools uncategorized (run 'hoards ai enrich --categorize')",
            uncategorized.to_string().yellow()
        );
    } else {
        println!("   Categories: {}", "All tools categorized".green());
    }

    println!();

    Ok(())
}

// ============================================
// WORKFLOW COMMAND IMPLEMENTATIONS
// ============================================

fn cmd_init(db: &Database, auto: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD FIRST-TIME SETUP          ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Scan for tools
    println!("{} Scanning system for installed tools...", "1.".bold());
    cmd_scan(db, false)?;

    // Step 2: Sync status
    println!("\n{} Syncing installation status...", "2.".bold());
    cmd_sync_status(db, false)?;

    // Step 3: Fetch descriptions
    println!("\n{} Fetching descriptions from registries...", "3.".bold());
    cmd_fetch_descriptions(db, false)?;

    // Step 4: Install shell completions
    println!("\n{} Installing shell completions...", "4.".bold());
    if let Err(e) = cmd_completions_install(None, false) {
        println!("  {} Failed to install completions: {}", "!".yellow(), e);
    }

    if !auto {
        // Step 5: Optional GitHub sync
        print!("\n{} Sync GitHub data (stars, topics)? [y/N] ", "5.".bold());
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            println!();
            cmd_gh_sync(db, false, None, 2000)?;
        }

        // Step 6: Optional AI categorization
        print!("\n{} Auto-categorize tools with AI? [y/N] ", "6.".bold());
        std::io::Write::flush(&mut std::io::stdout())?;

        input.clear();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            println!();
            cmd_ai_categorize(false)?;
        }
    }

    println!();
    println!("{}", "═══════════════════════════════════════".green());
    println!("{}", "        SETUP COMPLETE!                 ".bold());
    println!("{}", "═══════════════════════════════════════".green());
    println!();
    println!("Next steps:");
    println!("  {} - see overview", "hoards insights overview".cyan());
    println!(
        "  {} - find tools you're missing",
        "hoards discover missing".cyan()
    );
    println!("  {} - keep things up to date", "hoards maintain".cyan());

    Ok(())
}

fn cmd_maintain(db: &Database, auto: bool, dry_run: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD MAINTENANCE               ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Sync status
    println!("{} Syncing installation status...", "1.".bold());
    cmd_sync_status(db, dry_run)?;

    // Step 2: Check for updates
    println!("\n{} Checking for updates...", "2.".bold());
    cmd_updates(db, None, false, true, false)?;

    // Step 3: Scan usage
    println!("\n{} Scanning shell history for usage...", "3.".bold());
    cmd_usage_scan(db, dry_run, false)?;

    // Step 4: Health check
    println!("\n{} Running health check...", "4.".bold());
    cmd_doctor(db, false)?;

    if !auto && !dry_run {
        println!();
        println!("{} Maintenance complete!", "+".green());
    } else if dry_run {
        println!();
        println!("{} Dry run complete - no changes made", "i".cyan());
    }

    Ok(())
}

fn cmd_cleanup(db: &Database, force: bool, dry_run: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD CLEANUP WIZARD            ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Show unused tools
    println!("{} Unused installed tools:", "1.".bold());
    let unused = db.get_unused_tools()?;

    if unused.is_empty() {
        println!("   {} No unused tools found", "+".green());
    } else {
        println!(
            "   Found {} installed tools with no recorded usage:\n",
            unused.len()
        );
        for tool in &unused {
            println!("   {} {} ({})", "-".yellow(), tool.name, tool.source);
        }
    }

    // Step 2: Check for orphaned entries (not installed, not in usage table)
    println!(
        "\n{} Checking for orphaned database entries...",
        "2.".bold()
    );
    let all_tools = db.list_tools(false, None)?;
    let orphaned: Vec<_> = all_tools
        .iter()
        .filter(|t| !t.is_installed)
        .filter(|t| db.get_usage(&t.name).ok().flatten().is_none())
        .collect();

    if orphaned.is_empty() {
        println!("   {} No orphaned entries found", "+".green());
    } else {
        println!(
            "   Found {} tools not installed with no usage:\n",
            orphaned.len()
        );
        for tool in orphaned.iter().take(10) {
            println!("   {} {}", "-".dimmed(), tool.name.dimmed());
        }
        if orphaned.len() > 10 {
            println!("   ... and {} more", orphaned.len() - 10);
        }

        if !dry_run && (force || confirm("Remove orphaned entries?")?) {
            for tool in &orphaned {
                db.delete_tool(&tool.name)?;
            }
            println!(
                "   {} Removed {} orphaned entries",
                "+".green(),
                orphaned.len()
            );
        }
    }

    // Step 3: Run health fix
    println!("\n{} Running health checks...", "3.".bold());
    cmd_doctor(db, !dry_run && force)?;

    println!();
    if dry_run {
        println!("{} Dry run complete - no changes made", "i".cyan());
    } else {
        println!("{} Cleanup complete!", "+".green());
    }

    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N] ", prompt);
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

// ============================================
// UPDATES COMMAND
// ============================================

fn cmd_updates(
    db: &Database,
    source_filter: Option<String>,
    cross: bool,
    tracked: bool,
    all_versions: bool,
) -> Result<()> {
    use hoards::updates::*;

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
                println!(
                    "{} ({})",
                    "skipped".dimmed(),
                    e.to_string().chars().take(30).collect::<String>()
                );
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
            && filter != name
        {
            continue;
        }
        total_updates += check_source(name, check_fn)?;
    }

    println!();
    if total_updates == 0 {
        println!("{} All tools are up to date!", "+".green());
    } else {
        println!("{} {} update(s) available", "!".yellow(), total_updates);
    }

    Ok(())
}

fn cmd_updates_tracked(
    db: &Database,
    source_filter: Option<String>,
    all_versions: bool,
) -> Result<()> {
    use hoards::updates::*;

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
                    let marker = if i == versions.len() - 1 {
                        "(latest)"
                    } else {
                        ""
                    };
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
                && version_is_newer(&latest, &current)
            {
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
                "hoards upgrade <tool> --version <ver>".cyan()
            );
        }
    }

    Ok(())
}

fn cmd_updates_cross(db: &Database) -> Result<()> {
    use hoards::updates::*;

    println!(
        "{} Checking apt/snap tools for newer versions on other sources...\n",
        ">".cyan()
    );

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
        println!(
            "{} {} tool(s) have newer versions on other sources:\n",
            "!".yellow(),
            upgrades.len()
        );

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

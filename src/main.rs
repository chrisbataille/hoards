//! Hoards CLI - AI-powered tool manager with usage analytics
//!
//! This file contains only CLI dispatch logic. All command implementations
//! are in the `commands/` module.

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

use hoards::{
    AiCommands,
    AiConfigCommands,
    BundleCommands,
    Cli,
    Commands,
    CompletionsCommands,
    ConfigCommands,
    Database,
    DiscoverCommands,
    GhCommands,
    HoardConfig,
    InsightsCommands,
    UsageCommands,
    // Core commands
    cmd_add,
    // AI commands
    cmd_ai_analyze,
    cmd_ai_bundle_cheatsheet,
    cmd_ai_categorize,
    cmd_ai_cheatsheet,
    cmd_ai_describe,
    cmd_ai_discover,
    cmd_ai_extract,
    cmd_ai_migrate,
    cmd_ai_model,
    cmd_ai_set,
    cmd_ai_show,
    cmd_ai_suggest_bundle,
    cmd_ai_test,
    // Bundle commands
    cmd_bundle_add,
    cmd_bundle_create,
    cmd_bundle_delete,
    cmd_bundle_install,
    cmd_bundle_list,
    cmd_bundle_remove,
    cmd_bundle_show,
    cmd_bundle_update,
    // Discover commands
    cmd_categories,
    // Workflow commands
    cmd_cleanup,
    // Completions commands
    cmd_completions_install,
    cmd_completions_status,
    cmd_completions_uninstall,
    // Config commands
    cmd_config_edit,
    cmd_config_link,
    cmd_config_list,
    cmd_config_show,
    cmd_config_status,
    cmd_config_sync,
    cmd_config_unlink,
    // Misc commands
    cmd_doctor,
    cmd_edit,
    cmd_export,
    // Sync commands
    cmd_fetch_descriptions,
    // GitHub commands
    cmd_gh_backfill,
    cmd_gh_fetch,
    cmd_gh_info,
    cmd_gh_rate_limit,
    cmd_gh_search,
    cmd_gh_sync,
    cmd_import,
    // Insights commands
    cmd_info,
    cmd_init,
    // Install commands
    cmd_install,
    // Usage commands
    cmd_labels,
    cmd_list,
    cmd_maintain,
    cmd_overview,
    cmd_recommend,
    cmd_remove,
    cmd_scan,
    cmd_search,
    cmd_show,
    cmd_similar,
    cmd_stats,
    cmd_suggest,
    cmd_sync_status,
    cmd_trending,
    cmd_uninstall,
    cmd_unused,
    // Updates commands
    cmd_updates,
    cmd_upgrade,
    cmd_usage_config,
    cmd_usage_init,
    cmd_usage_log,
    cmd_usage_reset,
    cmd_usage_scan,
    cmd_usage_show,
    cmd_usage_tool,
    ensure_usage_configured,
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

            if do_scan {
                println!();
                cmd_scan(&db, dry_run)?;
            }

            if do_descriptions {
                println!();
                cmd_fetch_descriptions(&db, dry_run)?;
            }

            if do_github {
                println!();
                cmd_gh_sync(&db, dry_run, limit, delay)?;
            }

            if do_usage {
                println!();
                let mut config = HoardConfig::load()?;
                ensure_usage_configured(&mut config)?;
                cmd_usage_scan(&db, dry_run, false)?;
            }

            Ok(())
        }

        // ============================================
        // DISCOVER COMMANDS
        // ============================================
        Commands::Discover(command) => match command {
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
                    println!();
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
            _ => unreachable!("all DiscoverCommands variants covered"),
        },

        // ============================================
        // INSIGHTS COMMANDS
        // ============================================
        Commands::Insights(command) => match command {
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
            _ => unreachable!("all InsightsCommands variants covered"),
        },

        // ============================================
        // UPDATES
        // ============================================
        Commands::Updates {
            source,
            cross,
            tracked,
            all_versions,
        } => cmd_updates(&db, source, cross, tracked, all_versions),

        // ============================================
        // WORKFLOW COMMANDS
        // ============================================
        Commands::Init { auto } => cmd_init(&db, auto),
        Commands::Maintain { auto, dry_run } => cmd_maintain(&db, auto, dry_run),
        Commands::Cleanup { force, dry_run } => cmd_cleanup(&db, force, dry_run),

        // ============================================
        // TUI
        // ============================================
        Commands::Tui => hoards::tui::run(&db),

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

        // ============================================
        // GITHUB (advanced)
        // ============================================
        Commands::Gh(command) => match command {
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
            _ => unreachable!("all GhCommands variants covered"),
        },

        // ============================================
        // AI COMMANDS
        // ============================================
        Commands::Ai(command) => match command {
            AiCommands::Config(config_cmd) => match config_cmd {
                AiConfigCommands::Set { provider } => cmd_ai_set(&provider),
                AiConfigCommands::Model { model } => cmd_ai_model(&model),
                AiConfigCommands::Show => cmd_ai_show(),
                AiConfigCommands::Test => cmd_ai_test(),
                _ => unreachable!("all AiConfigCommands variants covered"),
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
                if do_categorize {
                    cmd_ai_categorize(dry_run)?;
                }
                if do_describe {
                    println!();
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
                if let Some(bundle_name) = bundle {
                    cmd_ai_bundle_cheatsheet(&bundle_name, refresh)
                } else if let Some(tool_name) = tool {
                    cmd_ai_cheatsheet(&tool_name, refresh)
                } else {
                    anyhow::bail!("Either --tool or --bundle must be specified")
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
            AiCommands::Migrate {
                from,
                to,
                dry_run,
                json,
                no_ai,
            } => cmd_ai_migrate(&db, from, to, dry_run, json, no_ai),
            // Hidden backward compatibility aliases
            AiCommands::Set { provider } => cmd_ai_set(&provider),
            AiCommands::ShowConfig => cmd_ai_show(),
            AiCommands::Test => cmd_ai_test(),
            AiCommands::Categorize { dry_run } => cmd_ai_categorize(dry_run),
            AiCommands::Describe { dry_run, limit } => cmd_ai_describe(dry_run, limit),
            _ => unreachable!("all AiCommands variants covered"),
        },

        // ============================================
        // BUNDLES
        // ============================================
        Commands::Bundle(command) => match command {
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
            _ => unreachable!("all BundleCommands variants covered"),
        },

        // ============================================
        // USAGE
        // ============================================
        Commands::Usage(command) => match command {
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
            _ => unreachable!("all UsageCommands variants covered"),
        },

        // ============================================
        // CONFIG (dotfiles management)
        // ============================================
        Commands::Config(command) => match command {
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
            _ => unreachable!("all ConfigCommands variants covered"),
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
        // COMPLETIONS
        // ============================================
        Commands::Completions(command) => match command {
            CompletionsCommands::Generate { shell } => {
                let mut cmd = Cli::command();
                let name = cmd.get_name().to_string();
                generate(shell, &mut cmd, name, &mut std::io::stdout());
                Ok(())
            }
            CompletionsCommands::Install { shell, force } => cmd_completions_install(shell, force),
            CompletionsCommands::Uninstall { shell } => cmd_completions_uninstall(shell),
            CompletionsCommands::Status => cmd_completions_status(),
            _ => unreachable!("all CompletionsCommands variants covered"),
        },

        // ============================================
        // HIDDEN BACKWARD COMPATIBILITY ALIASES
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
        Commands::Unused => cmd_unused(&db),
        Commands::Recommend { count } => cmd_recommend(&db, count),
        Commands::Doctor { fix } => cmd_doctor(&db, fix),

        _ => unreachable!("all variants covered"),
    }
}

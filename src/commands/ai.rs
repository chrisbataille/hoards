//! AI command implementations
//!
//! Commands for AI-assisted tool management using various AI providers.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::{AiProvider, Database, HoardConfig};

/// Set the AI provider
pub fn cmd_ai_set(provider: &str) -> Result<()> {
    let ai_provider = AiProvider::from(provider);

    if ai_provider == AiProvider::None {
        println!(
            "{} Unknown provider '{}'. Valid options: claude, gemini, codex, opencode",
            "!".yellow(),
            provider
        );
        return Ok(());
    }

    // Check if the CLI tool is installed
    if !ai_provider.is_installed() {
        println!(
            "{} Warning: '{}' CLI not found in PATH",
            "!".yellow(),
            ai_provider.command().unwrap_or("unknown")
        );
        println!("  The provider will be saved, but AI features won't work until installed.");
    }

    let mut config = HoardConfig::load()?;
    config.set_ai_provider(ai_provider.clone());
    config.save()?;

    println!("{} AI provider set to '{}'", "+".green(), ai_provider);
    println!(
        "  Config saved to: {}",
        HoardConfig::config_path()?.display()
    );

    Ok(())
}

/// Show current AI configuration
pub fn cmd_ai_show() -> Result<()> {
    let config = HoardConfig::load()?;

    println!("{}", "AI Configuration".bold());
    println!("{}", "=".repeat(30));
    println!();

    let provider = &config.ai.provider;
    let status = if provider == &AiProvider::None {
        "not configured".red().to_string()
    } else if provider.is_installed() {
        "installed".green().to_string()
    } else {
        "not installed".yellow().to_string()
    };

    println!("Provider: {} [{}]", provider.to_string().cyan(), status);

    if let Some(cmd) = provider.command() {
        println!("Command:  {}", cmd);
    }

    println!();
    println!("Config file: {}", HoardConfig::config_path()?.display());

    Ok(())
}

/// Test the AI provider
pub fn cmd_ai_test() -> Result<()> {
    let config = HoardConfig::load()?;

    if config.ai.provider == AiProvider::None {
        println!("{} No AI provider configured", "!".yellow());
        println!("  Use {} to set one", "hoards ai set <provider>".cyan());
        return Ok(());
    }

    let provider = &config.ai.provider;
    let cmd = match provider.command() {
        Some(c) => c,
        None => {
            println!("{} No command for provider '{}'", "!".red(), provider);
            return Ok(());
        }
    };

    println!("{} Testing {} CLI...", ">".cyan(), provider);

    // Check if command exists
    if !provider.is_installed() {
        println!("{} '{}' not found in PATH", "!".red(), cmd);
        return Ok(());
    }

    // Try to get version or help to verify it works
    let output = Command::new(cmd).arg("--version").output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout);
            let version = version.trim();
            if version.is_empty() {
                println!("{} {} is available", "+".green(), cmd);
            } else {
                println!("{} {} - {}", "+".green(), cmd, version.dimmed());
            }
        }
        Ok(_) => {
            // --version might not be supported, try --help
            let help_out = Command::new(cmd).arg("--help").output();
            match help_out {
                Ok(h) if h.status.success() || !h.stdout.is_empty() => {
                    println!("{} {} is available", "+".green(), cmd);
                }
                _ => {
                    println!(
                        "{} {} found but may not be working correctly",
                        "!".yellow(),
                        cmd
                    );
                }
            }
        }
        Err(e) => {
            println!("{} Failed to run '{}': {}", "!".red(), cmd, e);
        }
    }

    Ok(())
}

/// Categorize tools using AI
pub fn cmd_ai_categorize(dry_run: bool) -> Result<()> {
    use crate::ai::{categorize_prompt, invoke_ai, parse_categorize_response};

    let db = Database::open()?;

    // Get tools without categories
    let all_tools = db.list_tools(false, None)?;
    let uncategorized: Vec<_> = all_tools
        .iter()
        .filter(|t| t.category.is_none())
        .cloned()
        .collect();

    if uncategorized.is_empty() {
        println!("{} All tools are already categorized", "+".green());
        return Ok(());
    }

    println!(
        "{} Found {} uncategorized tool{}",
        ">".cyan(),
        uncategorized.len(),
        if uncategorized.len() == 1 { "" } else { "s" }
    );

    // Get existing categories
    let categories: Vec<String> = all_tools
        .iter()
        .filter_map(|t| t.category.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Generate prompt and call AI
    let prompt = categorize_prompt(&uncategorized, &categories);

    println!("{} Asking AI to categorize...", ">".cyan());
    let response = invoke_ai(&prompt)?;

    // Parse response
    let categorizations = parse_categorize_response(&response)?;

    if categorizations.is_empty() {
        println!("{} AI returned no categorizations", "!".yellow());
        return Ok(());
    }

    // Apply or show results
    println!();
    for (tool_name, category) in &categorizations {
        if dry_run {
            println!(
                "  {} {} -> {}",
                "[dry]".yellow(),
                tool_name,
                category.cyan()
            );
        } else if let Err(e) = db.update_tool_category(tool_name, category) {
            println!("  {} {} : {}", "!".red(), tool_name, e);
        } else {
            println!("  {} {} -> {}", "+".green(), tool_name, category.cyan());
        }
    }

    if dry_run {
        println!();
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!();
        println!(
            "{} Categorized {} tool{}",
            "+".green(),
            categorizations.len(),
            if categorizations.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

/// Suggest bundles using AI
pub fn cmd_ai_suggest_bundle(count: usize) -> Result<()> {
    use crate::ai::{invoke_ai, parse_bundle_response, suggest_bundle_prompt};

    let db = Database::open()?;

    // Get all tools and existing bundles
    let tools = db.list_tools(false, None)?;
    let bundles = db.list_bundles()?;

    // Count tools already in bundles
    let bundled_tools: std::collections::HashSet<&str> = bundles
        .iter()
        .flat_map(|b| b.tools.iter().map(|s| s.as_str()))
        .collect();
    let unbundled_count = tools
        .iter()
        .filter(|t| !bundled_tools.contains(t.name.as_str()))
        .count();

    if unbundled_count < 3 {
        println!(
            "{} Not enough unbundled tools to suggest bundles (need at least 3, have {})",
            "!".yellow(),
            unbundled_count
        );
        return Ok(());
    }

    println!(
        "{} Analyzing {} unbundled tools for bundle suggestions...",
        ">".cyan(),
        unbundled_count
    );

    if !bundles.is_empty() {
        println!(
            "  {} Excluding {} tool{} already in {} bundle{}",
            ">".dimmed(),
            bundled_tools.len(),
            if bundled_tools.len() == 1 { "" } else { "s" },
            bundles.len(),
            if bundles.len() == 1 { "" } else { "s" }
        );
    }

    // Generate prompt and call AI
    let prompt = suggest_bundle_prompt(&tools, &bundles, count);
    let response = invoke_ai(&prompt)?;

    // Parse response
    let suggestions = parse_bundle_response(&response)?;

    if suggestions.is_empty() {
        println!("{} AI returned no bundle suggestions", "!".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "Suggested Bundles:".bold());
    println!();

    for (i, suggestion) in suggestions.iter().enumerate() {
        println!(
            "{}. {} - {}",
            i + 1,
            suggestion.name.cyan().bold(),
            suggestion.description.dimmed()
        );
        for tool in &suggestion.tools {
            println!("   - {}", tool);
        }
        println!();
    }

    println!(
        "{} Create a bundle with: {}",
        ">".cyan(),
        "hoards bundle create <name> -d \"description\"".yellow()
    );
    println!(
        "  Then add tools with: {}",
        "hoards bundle add <bundle> <tool>".yellow()
    );

    Ok(())
}

/// Generate descriptions for tools using AI
pub fn cmd_ai_describe(dry_run: bool, limit: Option<usize>) -> Result<()> {
    use crate::ai::{describe_prompt, invoke_ai, parse_describe_response};

    let db = Database::open()?;

    // Get tools without descriptions
    let all_tools = db.list_tools(false, None)?;
    let mut no_description: Vec<_> = all_tools
        .iter()
        .filter(|t| {
            t.description.is_none()
                || t.description
                    .as_ref()
                    .map(|d| d.is_empty())
                    .unwrap_or(false)
        })
        .cloned()
        .collect();

    if no_description.is_empty() {
        println!("{} All tools already have descriptions", "+".green());
        return Ok(());
    }

    // Apply limit if specified
    if let Some(max) = limit {
        no_description.truncate(max);
    }

    println!(
        "{} Found {} tool{} without descriptions",
        ">".cyan(),
        no_description.len(),
        if no_description.len() == 1 { "" } else { "s" }
    );

    // Generate prompt and call AI
    let prompt = describe_prompt(&no_description);

    println!("{} Asking AI to generate descriptions...", ">".cyan());
    let response = invoke_ai(&prompt)?;

    // Parse response
    let descriptions = parse_describe_response(&response)?;

    if descriptions.is_empty() {
        println!("{} AI returned no descriptions", "!".yellow());
        return Ok(());
    }

    // Apply or show results
    println!();
    for (tool_name, description) in &descriptions {
        if dry_run {
            println!("  {} {}", "[dry]".yellow(), tool_name.cyan());
            println!("       {}", description.dimmed());
        } else if let Err(e) = db.update_tool_description(tool_name, description) {
            println!("  {} {} : {}", "!".red(), tool_name, e);
        } else {
            println!("  {} {}", "+".green(), tool_name.cyan());
            println!("       {}", description.dimmed());
        }
    }

    if dry_run {
        println!();
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!();
        println!(
            "{} Added descriptions for {} tool{}",
            "+".green(),
            descriptions.len(),
            if descriptions.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

/// Extract tool info from GitHub README using AI
pub fn cmd_ai_extract(
    db: &Database,
    urls: Vec<String>,
    yes: bool,
    dry_run: bool,
    delay_ms: u64,
) -> Result<()> {
    use crate::ai::{
        ExtractedTool, extract_prompt, fetch_readme, fetch_repo_version, invoke_ai,
        parse_extract_response, parse_github_url,
    };
    use crate::db::CachedExtraction;
    use crate::{InstallSource, Tool};
    use dialoguer::Confirm;
    use std::thread;
    use std::time::Duration;

    if urls.is_empty() {
        println!("{} No URLs provided", "!".yellow());
        return Ok(());
    }

    println!(
        "{} Extracting tool info from {} URL{}...",
        ">".cyan(),
        urls.len(),
        if urls.len() == 1 { "" } else { "s" }
    );
    println!();

    let mut extracted: Vec<(String, String, ExtractedTool)> = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();

    for (i, url) in urls.iter().enumerate() {
        // Rate limiting for batch mode
        if i > 0 && delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }

        // Parse URL
        let (owner, repo) = match parse_github_url(url) {
            Ok(parsed) => parsed,
            Err(e) => {
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        println!("{} {}/{}", ">".cyan(), owner, repo);

        // Check cache first
        let version = match fetch_repo_version(&owner, &repo) {
            Ok(v) => v,
            Err(e) => {
                println!("  {} Failed to get version: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        if let Ok(Some(cached)) = db.get_cached_extraction(&owner, &repo, &version) {
            println!("  {} Using cached extraction", "+".green());
            let tool = ExtractedTool {
                name: cached.name,
                binary: cached.binary,
                source: cached.source,
                install_command: cached.install_command,
                description: cached.description,
                category: cached.category,
            };
            extracted.push((owner, repo, tool));
            continue;
        }

        // Fetch README
        let readme = match fetch_readme(&owner, &repo) {
            Ok(r) => r,
            Err(e) => {
                println!("  {} Failed to fetch README: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        // Extract using AI
        let prompt = extract_prompt(&readme);
        println!("  {} Asking AI to extract...", ">".dimmed());

        let response = match invoke_ai(&prompt) {
            Ok(r) => r,
            Err(e) => {
                println!("  {} AI extraction failed: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        let tool = match parse_extract_response(&response) {
            Ok(t) => t,
            Err(e) => {
                println!("  {} Failed to parse response: {}", "!".red(), e);
                errors.push((url.clone(), e.to_string()));
                continue;
            }
        };

        // Cache the result
        let cached = CachedExtraction {
            repo_owner: owner.clone(),
            repo_name: repo.clone(),
            version: version.clone(),
            name: tool.name.clone(),
            binary: tool.binary.clone(),
            source: tool.source.clone(),
            install_command: tool.install_command.clone(),
            description: tool.description.clone(),
            category: tool.category.clone(),
            extracted_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = db.cache_extraction(&cached) {
            println!("  {} Cache write failed: {}", "!".yellow(), e);
        }

        println!("  {} Extracted successfully", "+".green());
        extracted.push((owner, repo, tool));
    }

    // Show results
    if !extracted.is_empty() {
        println!();
        println!("{}", "Extracted Tools:".bold());
        println!("{}", "=".repeat(50));

        for (owner, repo, tool) in &extracted {
            println!();
            println!("{} (from {}/{})", tool.name.cyan().bold(), owner, repo);
            if let Some(bin) = &tool.binary {
                println!("  Binary:      {}", bin);
            }
            println!("  Source:      {}", tool.source);
            if let Some(cmd) = &tool.install_command {
                println!("  Install:     {}", cmd);
            }
            println!("  Category:    {}", tool.category);
            println!("  Description: {}", tool.description.dimmed());
        }
    }

    // Handle errors
    if !errors.is_empty() {
        println!();
        println!("{}", "Errors:".red().bold());
        for (url, err) in &errors {
            println!("  {} {}: {}", "!".red(), url, err);
        }
    }

    // Add to database
    if !extracted.is_empty() && !dry_run {
        println!();

        let should_add = if yes {
            true
        } else {
            Confirm::new()
                .with_prompt(format!(
                    "Add {} tool{} to database?",
                    extracted.len(),
                    if extracted.len() == 1 { "" } else { "s" }
                ))
                .default(true)
                .interact()?
        };

        if should_add {
            let mut added = 0;
            for (_owner, _repo, ext) in &extracted {
                // Check if tool already exists
                if db.get_tool_by_name(&ext.name)?.is_some() {
                    println!("  {} {} already exists, skipping", "!".yellow(), ext.name);
                    continue;
                }

                let source = InstallSource::from(ext.source.as_str());
                let tool = Tool::new(&ext.name)
                    .with_source(source)
                    .with_description(&ext.description)
                    .with_category(&ext.category)
                    .with_binary(ext.binary.as_deref().unwrap_or(&ext.name))
                    .with_install_command(ext.install_command.as_deref().unwrap_or(""));

                if let Err(e) = db.insert_tool(&tool) {
                    println!("  {} Failed to add {}: {}", "!".red(), ext.name, e);
                } else {
                    println!("  {} Added {}", "+".green(), ext.name);
                    added += 1;
                }
            }

            println!();
            println!(
                "{} Added {} tool{} to database",
                "+".green(),
                added,
                if added == 1 { "" } else { "s" }
            );
        }
    } else if dry_run && !extracted.is_empty() {
        println!();
        println!(
            "{} Run without {} to add to database",
            ">".cyan(),
            "--dry-run".yellow()
        );
    }

    Ok(())
}

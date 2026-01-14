//! GitHub command implementations
//!
//! Commands for fetching and syncing GitHub repository information.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;

use crate::Database;

/// Sync GitHub info for tools without it
pub fn cmd_gh_sync(db: &Database, dry_run: bool, limit: Option<usize>, delay_ms: u64) -> Result<()> {
    use crate::github::{find_repo, get_all_rate_limits, is_gh_available, topics_to_category, TopicMapping};

    if !is_gh_available() {
        println!("{} GitHub CLI (gh) is not installed", "!".red());
        println!("  Install it with: {}", "brew install gh".cyan());
        return Ok(());
    }

    // Check both core and search rate limits
    let limits = get_all_rate_limits()?;

    println!(
        "{} Core API:   {}/{} remaining (resets in {} min)",
        ">".cyan(),
        limits.core.remaining,
        limits.core.limit,
        limits.core.reset_minutes()
    );
    println!(
        "{} Search API: {}/{} remaining (resets in {} sec)",
        ">".cyan(),
        limits.search.remaining,
        limits.search.limit,
        limits.search.reset_seconds()
    );

    // Search API is the bottleneck (30/minute vs 5000/hour)
    // Each tool needs 1 search call + 1 core API call
    if limits.search.remaining == 0 {
        println!(
            "\n{} Search API quota exhausted! Wait {} seconds before retrying.",
            "!".red(),
            limits.search.reset_seconds()
        );
        return Ok(());
    }

    // Get tools without GitHub info
    let mut tools_to_sync = db.get_tools_without_github()?;

    if tools_to_sync.is_empty() {
        println!("{} All tools already have GitHub info", "+".green());
        return Ok(());
    }

    // Limit based on Search API (the stricter limit)
    // Each tool needs 1 search call
    let search_limited_max = limits.search.remaining as usize;
    // Also check core API (each tool needs 1 core call for repo details)
    let core_limited_max = limits.core.remaining as usize;
    let rate_limited_max = search_limited_max.min(core_limited_max);

    if let Some(max) = limit {
        tools_to_sync.truncate(max.min(rate_limited_max));
    } else if tools_to_sync.len() > rate_limited_max {
        println!(
            "{} Limiting to {} tools (search quota: {}/min)",
            "!".yellow(),
            rate_limited_max,
            limits.search.limit
        );
        tools_to_sync.truncate(rate_limited_max);
    }

    if tools_to_sync.is_empty() {
        println!(
            "{} Not enough API quota. Wait {} sec or use --limit",
            "!".red(),
            limits.search.reset_seconds()
        );
        return Ok(());
    }

    // Warn if delay is too short for search API (30/min = 2000ms between calls)
    let min_safe_delay = 2000;
    if delay_ms < min_safe_delay && tools_to_sync.len() > 1 {
        println!(
            "{} Warning: {}ms delay may hit search rate limit (30/min). Use --delay {} for safety.",
            "!".yellow(),
            delay_ms,
            min_safe_delay
        );
    }

    println!(
        "{} Syncing {} tool{} ({}ms delay between searches)...",
        ">".cyan(),
        tools_to_sync.len(),
        if tools_to_sync.len() == 1 { "" } else { "s" },
        delay_ms
    );

    let mapping = TopicMapping::load();
    let mut synced = 0;
    let mut not_found = 0;
    let delay = std::time::Duration::from_millis(delay_ms);

    for (i, tool_name) in tools_to_sync.iter().enumerate() {
        // Add delay between requests (except first)
        if i > 0 && delay_ms > 0 {
            std::thread::sleep(delay);
        }

        // Get tool's source to improve search accuracy (e.g., cargo -> language:rust)
        let source = db.get_tool_by_name(tool_name)?
            .map(|t| t.source.to_string());

        print!("  {} {}... ", ">".dimmed(), tool_name);

        match find_repo(tool_name, source.as_deref()) {
            Ok(Some(info)) => {
                if dry_run {
                    println!("{}", "[dry] found".yellow());
                    println!(
                        "       {} ({} stars)",
                        info.full_name.dimmed(),
                        info.stars
                    );
                    if !info.topics.is_empty() {
                        println!("       topics: {}", info.topics.join(", ").dimmed());
                    }
                } else {
                    // Store GitHub info
                    db.set_github_info(
                        tool_name,
                        crate::db::GitHubInfoInput {
                            repo_owner: &info.owner.login,
                            repo_name: &info.name,
                            description: info.description.as_deref(),
                            stars: info.stars,
                            language: info.language.as_deref(),
                            homepage: info.homepage.as_deref(),
                        },
                    )?;

                    // Add topics as labels
                    let labels: Vec<String> = info.topics.iter().map(|t| t.to_lowercase()).collect();
                    if !labels.is_empty() {
                        db.add_labels(tool_name, &labels)?;
                    }

                    // Auto-fill description and category if missing
                    if let Some(tool) = db.get_tool_by_name(tool_name)? {
                        let mut updates = Vec::new();

                        // Copy description from GitHub if tool has none
                        if tool.description.is_none()
                            && let Some(desc) = &info.description {
                                db.update_tool_description(tool_name, desc)?;
                                updates.push("desc".to_string());
                            }

                        // Auto-categorize from topics if uncategorized
                        if tool.category.is_none()
                            && let Some(category) = topics_to_category(&info.topics, &mapping) {
                                db.update_tool_category(tool_name, &category)?;
                                updates.push(format!("→ {}", category));
                            }

                        if updates.is_empty() {
                            println!("{}", "+".green());
                        } else {
                            println!("{} {}", "+".green(), updates.join(", ").cyan());
                        }
                    }

                    synced += 1;
                }
            }
            Ok(None) => {
                println!("{}", "not found".dimmed());
                not_found += 1;
            }
            Err(e) => {
                println!("{} {}", "!".red(), e);
            }
        }
    }

    println!();
    if dry_run {
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!(
            "{} Synced {} tool{}, {} not found on GitHub",
            "+".green(),
            synced,
            if synced == 1 { "" } else { "s" },
            not_found
        );
    }

    Ok(())
}

/// Show GitHub API rate limits
pub fn cmd_gh_rate_limit() -> Result<()> {
    use crate::github::{get_all_rate_limits, is_gh_available};

    if !is_gh_available() {
        println!("{} GitHub CLI (gh) is not installed", "!".red());
        return Ok(());
    }

    let limits = get_all_rate_limits()?;

    println!("{}", "Core API (5000/hour):".bold());
    println!("  Limit:     {}", limits.core.limit);
    println!("  Used:      {}", limits.core.used);
    println!("  Remaining: {}", limits.core.remaining);
    println!("  Resets in: {} minutes", limits.core.reset_minutes());

    println!();
    println!("{}", "Search API (30/minute):".bold());
    println!("  Limit:     {}", limits.search.limit);
    println!("  Used:      {}", limits.search.used);
    println!("  Remaining: {}", limits.search.remaining);
    println!("  Resets in: {} seconds", limits.search.reset_seconds());

    // Warning for search API (the bottleneck)
    if limits.search.remaining < 10 {
        println!();
        println!(
            "{} Search API quota low! Wait {} sec before syncing",
            "!".yellow(),
            limits.search.reset_seconds()
        );
    } else if limits.core.remaining < 100 {
        println!();
        println!(
            "{} Core API quota low! Wait {} min before syncing",
            "!".yellow(),
            limits.core.reset_minutes()
        );
    }

    Ok(())
}

/// Backfill descriptions from cached GitHub data
pub fn cmd_gh_backfill(db: &Database, dry_run: bool) -> Result<()> {
    let tools = db.get_tools_needing_description_backfill()?;

    if tools.is_empty() {
        println!("{} All tools with GitHub info already have descriptions", "+".green());
        return Ok(());
    }

    println!(
        "{} Found {} tool{} with cached GitHub descriptions to backfill",
        ">".cyan(),
        tools.len(),
        if tools.len() == 1 { "" } else { "s" }
    );

    for (name, description) in &tools {
        if dry_run {
            println!("  {} {} → {}", "[dry]".yellow(), name, description.chars().take(50).collect::<String>());
        } else {
            db.update_tool_description(name, description)?;
            println!("  {} {} → {}", "+".green(), name, description.chars().take(50).collect::<String>());
        }
    }

    println!();
    if dry_run {
        println!(
            "{} Run without {} to apply changes",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!(
            "{} Updated {} tool description{}",
            "+".green(),
            tools.len(),
            if tools.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}

/// Fetch GitHub info for a specific tool
pub fn cmd_gh_fetch(db: &Database, name: &str) -> Result<()> {
    use crate::github::{find_repo, is_gh_available, topics_to_category, TopicMapping};

    if !is_gh_available() {
        println!("{} GitHub CLI (gh) is not installed", "!".red());
        return Ok(());
    }

    // Check if tool exists in DB
    let tool = db.get_tool_by_name(name)?;
    if tool.is_none() {
        println!("{} Tool '{}' not found in database", "!".yellow(), name);
        println!("  Add it first with: {}", format!("hoard add {}", name).cyan());
        return Ok(());
    }
    let source = tool.map(|t| t.source.to_string());

    println!("{} Fetching GitHub info for '{}'...", ">".cyan(), name);

    match find_repo(name, source.as_deref())? {
        Some(info) => {
            // Store GitHub info
            db.set_github_info(
                name,
                crate::db::GitHubInfoInput {
                    repo_owner: &info.owner.login,
                    repo_name: &info.name,
                    description: info.description.as_deref(),
                    stars: info.stars,
                    language: info.language.as_deref(),
                    homepage: info.homepage.as_deref(),
                },
            )?;

            // Add topics as labels
            let labels: Vec<String> = info.topics.iter().map(|t| t.to_lowercase()).collect();
            if !labels.is_empty() {
                db.clear_labels(name)?;
                db.add_labels(name, &labels)?;
            }

            // Auto-categorize if uncategorized
            let mapping = TopicMapping::load();
            if let Some(tool) = db.get_tool_by_name(name)?
                && tool.category.is_none()
                    && let Some(category) = topics_to_category(&info.topics, &mapping) {
                        db.update_tool_category(name, &category)?;
                    }

            // Always update description from GitHub on explicit fetch
            if let Some(desc) = &info.description {
                db.update_tool_description(name, desc)?;
            }

            println!();
            println!("{}", "GitHub Info:".bold());
            println!("  Repo:     {}", info.full_name.cyan());
            println!("  Stars:    {}", info.stars);
            if let Some(desc) = &info.description {
                println!("  Desc:     {}", desc.dimmed());
            }
            if let Some(lang) = &info.language {
                println!("  Language: {}", lang);
            }
            if !info.topics.is_empty() {
                println!("  Topics:   {}", info.topics.join(", "));
            }
            println!();
            println!("{} GitHub info saved", "+".green());
        }
        None => {
            println!("{} '{}' not found on GitHub", "!".yellow(), name);
        }
    }

    Ok(())
}

/// Search GitHub repositories
pub fn cmd_gh_search(query: &str, limit: usize) -> Result<()> {
    use crate::github::is_gh_available;

    if !is_gh_available() {
        println!("{} GitHub CLI (gh) is not installed", "!".red());
        return Ok(());
    }

    println!("{} Searching GitHub for '{}'...", ">".cyan(), query);

    let output = Command::new("gh")
        .args([
            "search",
            "repos",
            query,
            "--json",
            "name,fullName,description,stargazersCount",
            "--limit",
            &limit.to_string(),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("{} Search failed: {}", "!".red(), stderr);
        return Ok(());
    }

    #[derive(serde::Deserialize)]
    struct SearchResult {
        #[allow(dead_code)]
        name: String,
        #[serde(rename = "fullName")]
        full_name: String,
        description: Option<String>,
        #[serde(rename = "stargazersCount")]
        stars: i64,
    }

    let results: Vec<SearchResult> = serde_json::from_slice(&output.stdout)?;

    if results.is_empty() {
        println!("{} No results found", "!".yellow());
        return Ok(());
    }

    println!();
    for result in results {
        println!(
            "  {} {} ({})",
            "*".cyan(),
            result.full_name.bold(),
            format!("{} stars", result.stars).dimmed()
        );
        if let Some(desc) = result.description {
            println!("    {}", desc.dimmed());
        }
    }

    Ok(())
}

/// Show cached GitHub info for a tool
pub fn cmd_gh_info(db: &Database, name: &str) -> Result<()> {
    // Check if tool exists
    let tool = db.get_tool_by_name(name)?;
    if tool.is_none() {
        println!("{} Tool '{}' not found in database", "!".yellow(), name);
        return Ok(());
    }

    // Get cached GitHub info
    match db.get_github_info(name)? {
        Some(info) => {
            println!("{}", "GitHub Info:".bold());
            println!("  Repo:     {}/{}", info.repo_owner, info.repo_name);
            println!("  Stars:    {}", info.stars);
            if let Some(desc) = &info.description {
                println!("  Desc:     {}", desc.dimmed());
            }
            if let Some(lang) = &info.language {
                println!("  Language: {}", lang);
            }
            if let Some(hp) = &info.homepage {
                println!("  Homepage: {}", hp);
            }

            // Show labels
            let labels = db.get_labels(name)?;
            if !labels.is_empty() {
                println!("  Labels:   {}", labels.join(", ").cyan());
            }
        }
        None => {
            println!("{} No GitHub info cached for '{}'", "!".yellow(), name);
            println!(
                "  Fetch it with: {}",
                format!("hoard gh fetch {}", name).cyan()
            );
        }
    }

    Ok(())
}

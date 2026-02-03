pub mod cli;
pub mod commands;
pub mod db;
pub mod helpers;
pub mod id;
pub mod models;

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

use cli::{Cli, Commands, GoalCommands, TaskCommands};
use db::Database;

pub const RADIAL_DIR: &str = ".radial";
pub const DB_FILE: &str = "radial.db";
pub const REDIRECT_FILE: &str = "redirect";

/// Finds the `.radial/` directory by walking up from the current directory.
/// Returns `None` if no `.radial/` directory is found.
pub fn find_radial_dir() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let mut dir = current_dir.as_path();

    loop {
        let radial_path = dir.join(RADIAL_DIR);
        if radial_path.is_dir() {
            return Some(radial_path);
        }

        dir = dir.parent()?;
    }
}

/// Resolves the final radial directory, following any redirect file.
/// A redirect file contains a path (absolute or relative) to another `.radial/` directory.
pub fn resolve_radial_dir() -> Option<PathBuf> {
    let radial_dir = find_radial_dir()?;
    let redirect_path = radial_dir.join(REDIRECT_FILE);

    if redirect_path.is_file() {
        let target = std::fs::read_to_string(&redirect_path).ok()?;
        let target = target.trim();

        let target_path = if PathBuf::from(target).is_absolute() {
            PathBuf::from(target)
        } else {
            radial_dir.parent()?.join(target)
        };

        if target_path.is_dir() {
            return Some(target_path);
        }
    }

    Some(radial_dir)
}

fn get_db_path() -> Option<PathBuf> {
    resolve_radial_dir().map(|dir| dir.join(DB_FILE))
}

fn ensure_initialized() -> Result<Database> {
    let db_path =
        get_db_path().ok_or_else(|| anyhow!("Radial not initialized. Run 'radial init' first."))?;

    if !db_path.exists() {
        return Err(anyhow!("Radial not initialized. Run 'radial init' first."));
    }

    Database::open(&db_path).context("Failed to open database")
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { stealth } => commands::init::run(stealth),
        Commands::Goal(goal_cmd) => {
            let db = ensure_initialized()?;
            match goal_cmd {
                GoalCommands::Create { description, json } => {
                    commands::goal::create(description, json, &db)
                }
                GoalCommands::List { json } => commands::goal::list(json, &db),
            }
        }
        Commands::Task(task_cmd) => {
            let db = ensure_initialized()?;
            match task_cmd {
                TaskCommands::Create {
                    goal_id,
                    description,
                    receives,
                    produces,
                    verify,
                    blocked_by,
                    json,
                } => commands::task::create(
                    goal_id,
                    description,
                    receives,
                    produces,
                    verify,
                    blocked_by,
                    json,
                    &db,
                ),
                TaskCommands::List { goal_id, json } => commands::task::list(goal_id, json, &db),
                TaskCommands::Start { task_id } => commands::task::start(task_id, &db),
                TaskCommands::Complete {
                    task_id,
                    result,
                    artifacts,
                    tokens,
                    elapsed,
                } => commands::task::complete(task_id, result, artifacts, tokens, elapsed, &db),
                TaskCommands::Fail { task_id } => commands::task::fail(task_id, &db),
                TaskCommands::Retry { task_id } => commands::task::retry(task_id, &db),
            }
        }
        Commands::Status { goal, task, json } => {
            let db = ensure_initialized()?;
            commands::status::run(goal, task, json, &db)
        }
        Commands::Ready { goal_id, json } => {
            let db = ensure_initialized()?;
            commands::ready::run(goal_id, json, &db)
        }
    }
}

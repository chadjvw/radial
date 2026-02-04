use anyhow::{anyhow, Result};

use crate::db::Database;
use crate::models::{Task, TaskState};

pub fn run(goal_id: String, db: &Database) -> Result<Vec<Task>> {
    let _goal = db
        .get_goal(&goal_id)?
        .ok_or_else(|| anyhow!("Goal not found: {goal_id}"))?;

    let tasks = db.list_tasks(&goal_id)?;

    Ok(tasks
        .into_iter()
        .filter(|t| t.state == TaskState::Pending && t.contract.is_some())
        .collect())
}

use anyhow::{Result, anyhow};

use crate::db::Database;
use crate::models::TaskState;

pub fn run(goal_id: String, json: bool, db: &Database) -> Result<()> {
    let goal = db
        .get_goal(&goal_id)?
        .ok_or_else(|| anyhow!("Goal not found: {}", goal_id))?;

    let tasks = db.list_tasks(&goal_id)?;

    let ready_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| t.state == TaskState::Pending && t.contract.is_some())
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&ready_tasks)?);
        return Ok(());
    }

    println!(
        "Ready tasks for goal: {} [{}]",
        goal.id,
        goal.state.as_str()
    );
    println!("  {}", goal.description);
    println!();

    if ready_tasks.is_empty() {
        println!("No tasks ready to start.");
        return Ok(());
    }

    println!("{} task(s) ready:", ready_tasks.len());
    println!();

    for task in ready_tasks {
        println!("{}", task.id);
        println!("  Description: {}", task.description);
        if let Some(ref contract) = task.contract {
            println!("  Receives: {}", contract.receives);
            println!("  Produces: {}", contract.produces);
            println!("  Verify: {}", contract.verify);
        }
        println!();
    }

    Ok(())
}

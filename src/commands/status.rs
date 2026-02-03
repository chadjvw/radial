use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::db::Database;
use crate::models::{Goal, Metrics, Task};

#[derive(Serialize)]
struct GoalStatus {
    #[serde(flatten)]
    goal: Goal,
    tasks: Vec<Task>,
}

#[derive(Serialize)]
struct GoalSummary {
    #[serde(flatten)]
    goal: Goal,
    computed_metrics: Metrics,
}

pub fn run(
    goal_id: Option<String>,
    task_id: Option<String>,
    json: bool,
    db: &Database,
) -> Result<()> {
    if let Some(tid) = task_id {
        return show_task(&tid, json, db);
    }

    if let Some(gid) = goal_id {
        return show_goal(&gid, json, db);
    }

    show_all_goals(json, db)
}

fn show_task(task_id: &str, json: bool, db: &Database) -> Result<()> {
    let task = db
        .get_task(task_id)?
        .ok_or_else(|| anyhow!("Task not found: {}", task_id))?;

    if json {
        let output = serde_json::to_string_pretty(&task)?;
        println!("{output}");
        return Ok(());
    }

    println!("Task: {} [{}]", task.id, task.state.as_str());
    println!("  Goal: {}", task.goal_id);
    println!("  Description: {}", task.description);
    println!("  Created: {}", task.created_at);
    println!("  Updated: {}", task.updated_at);
    println!();
    if let Some(ref contract) = task.contract {
        println!("Contract:");
        println!("  Receives: {}", contract.receives);
        println!("  Produces: {}", contract.produces);
        println!("  Verify: {}", contract.verify);
    } else {
        println!("Contract: (not set)");
    }

    if let Some(blocked_by) = &task.blocked_by {
        println!();
        println!("Blocked by: {}", blocked_by.join(", "));
    }

    if let Some(result) = &task.result {
        println!();
        println!("Result:");
        println!("  Summary: {}", result.summary);
        if !result.artifacts.is_empty() {
            println!("  Artifacts:");
            for artifact in &result.artifacts {
                println!("    - {}", artifact);
            }
        }
    }

    println!();
    println!("Metrics:");
    println!("  Tokens: {}", task.metrics.tokens);
    println!("  Elapsed: {}ms", task.metrics.elapsed_ms);
    println!("  Retries: {}", task.metrics.retry_count);

    Ok(())
}

fn show_goal(goal_id: &str, json: bool, db: &Database) -> Result<()> {
    let goal = db
        .get_goal(goal_id)?
        .ok_or_else(|| anyhow!("Goal not found: {}", goal_id))?;

    let tasks = db.list_tasks(goal_id)?;

    if json {
        let status = GoalStatus { goal, tasks };
        let output = serde_json::to_string_pretty(&status)?;
        println!("{output}");
        return Ok(());
    }

    let metrics = db.compute_goal_metrics(goal_id)?;

    println!("Goal: {} [{}]", goal.id, goal.state.as_str());
    println!("  Description: {}", goal.description);
    println!("  Created: {}", goal.created_at);
    println!("  Updated: {}", goal.updated_at);
    if let Some(completed_at) = goal.completed_at {
        println!("  Completed: {}", completed_at);
    }
    println!();
    println!("Metrics:");
    println!(
        "  Tasks: {} total, {} completed, {} failed",
        metrics.task_count, metrics.tasks_completed, metrics.tasks_failed
    );
    println!("  Tokens: {}", metrics.total_tokens);
    println!("  Elapsed: {}ms", metrics.elapsed_ms);

    if !tasks.is_empty() {
        println!();
        println!("Tasks:");
        for task in tasks {
            println!(
                "  {} [{}] - {}",
                task.id,
                task.state.as_str(),
                task.description
            );
        }
    }

    Ok(())
}

fn show_all_goals(json: bool, db: &Database) -> Result<()> {
    let goals = db.list_goals()?;

    if json {
        let summaries: Vec<GoalSummary> = goals
            .into_iter()
            .map(|goal| {
                let computed_metrics = db.compute_goal_metrics(&goal.id)?;
                Ok(GoalSummary {
                    goal,
                    computed_metrics,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let output = serde_json::to_string_pretty(&summaries)?;
        println!("{output}");
        return Ok(());
    }

    if goals.is_empty() {
        println!("No goals found.");
        return Ok(());
    }

    println!("All Goals:");
    println!();
    for goal in goals {
        let metrics = db.compute_goal_metrics(&goal.id)?;
        println!("{} [{}]", goal.id, goal.state.as_str());
        println!("  Description: {}", goal.description);
        println!(
            "  Tasks: {} total, {} completed, {} failed",
            metrics.task_count, metrics.tasks_completed, metrics.tasks_failed
        );
        println!();
    }

    Ok(())
}

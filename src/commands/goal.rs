use anyhow::Result;
use chrono::Utc;

use crate::db::Database;
use crate::id::generate_id;
use crate::models::{Goal, GoalState, Metrics};

pub fn create(description: String, json: bool, db: &mut Database) -> Result<()> {
    let goal = Goal {
        id: generate_id(),
        parent_id: None,
        description: description.clone(),
        state: GoalState::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        metrics: Metrics::default(),
    };

    db.create_goal(&goal)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&goal)?);
    } else {
        println!("Created goal: {}", goal.id);
        println!("  Description: {}", goal.description);
    }
    Ok(())
}

pub fn list(json: bool, db: &Database) -> Result<()> {
    let goals = db.list_goals()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&goals)?);
        return Ok(());
    }

    if goals.is_empty() {
        println!("No goals found.");
        return Ok(());
    }

    for goal in goals {
        println!("{} [{}]", goal.id, goal.state.as_str());
        println!("  Description: {}", goal.description);
        println!(
            "  Tasks: {} total, {} completed, {} failed",
            goal.metrics.task_count, goal.metrics.tasks_completed, goal.metrics.tasks_failed
        );
        println!();
    }

    Ok(())
}

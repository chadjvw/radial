use anyhow::Result;
use jiff::Timestamp;

use crate::db::Database;
use crate::id::generate_id;
use crate::models::{Goal, GoalState, Metrics};

pub fn create(description: String, db: &mut Database) -> Result<Goal> {
    let now = Timestamp::now();
    let goal = Goal {
        id: generate_id(),
        parent_id: None,
        description,
        state: GoalState::Pending,
        created_at: now,
        updated_at: now,
        completed_at: None,
        metrics: Metrics::default(),
    };

    db.create_goal(&goal)?;
    Ok(goal)
}

pub fn list(db: &Database) -> Result<Vec<Goal>> {
    db.list_goals()
}

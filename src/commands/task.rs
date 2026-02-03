use anyhow::{Result, anyhow};
use chrono::Utc;

use crate::db::Database;
use crate::helpers::find_similar_id;
use crate::id::generate_id;
use crate::models::{Contract, GoalState, Task, TaskMetrics, TaskState};

pub fn create(
    goal_id: String,
    description: String,
    receives: Option<String>,
    produces: Option<String>,
    verify: Option<String>,
    blocked_by: Option<Vec<String>>,
    json: bool,
    db: &mut Database,
) -> Result<()> {
    let goal = db.get_goal(&goal_id)?;

    if goal.is_none() {
        let all_goals = db.list_goals()?;
        let goal_ids: Vec<String> = all_goals.iter().map(|g| g.id.clone()).collect();

        return if let Some(suggestion) = find_similar_id(&goal_id, &goal_ids) {
            Err(anyhow!(
                "Goal not found: {}\nDid you mean: {}",
                goal_id,
                suggestion
            ))
        } else {
            Err(anyhow!("Goal not found: {}", goal_id))
        };
    }

    let goal = goal.unwrap();

    // Validate blocked_by task IDs exist
    if let Some(ref task_ids) = blocked_by {
        let all_tasks = db.list_tasks(&goal.id)?;
        let existing_task_ids: Vec<String> = all_tasks.iter().map(|t| t.id.clone()).collect();

        for task_id in task_ids {
            if !existing_task_ids.contains(task_id) {
                return if let Some(suggestion) = find_similar_id(task_id, &existing_task_ids) {
                    Err(anyhow!(
                        "Task not found in blocked-by list: {}\nDid you mean: {}",
                        task_id,
                        suggestion
                    ))
                } else {
                    Err(anyhow!(
                        "Task not found in blocked-by list: {}\nTask must exist in the same goal.",
                        task_id
                    ))
                };
            }
        }
    }

    // Build contract if any contract fields are provided
    let contract = if receives.is_some() || produces.is_some() || verify.is_some() {
        Some(Contract {
            receives: receives.unwrap_or_default(),
            produces: produces.unwrap_or_default(),
            verify: verify.unwrap_or_default(),
        })
    } else {
        None
    };

    let task = Task {
        id: generate_id(),
        goal_id: goal.id.clone(),
        description,
        contract,
        state: if blocked_by.is_some() {
            TaskState::Blocked
        } else {
            TaskState::Pending
        },
        blocked_by,
        result: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
        metrics: TaskMetrics::default(),
    };

    db.create_task(&task)?;

    let mut updated_goal = goal;
    updated_goal.updated_at = Utc::now();
    if updated_goal.state == GoalState::Pending {
        updated_goal.state = GoalState::InProgress;
    }
    db.update_goal(&updated_goal)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!("Created task: {}", task.id);
        println!("  Description: {}", task.description);
        println!("  State: {}", task.state.as_str());
        if task.contract.is_none() {
            println!("  Contract: (not set - required before starting)");
        }
    }
    Ok(())
}

pub fn list(goal_id: String, json: bool, db: &Database) -> Result<()> {
    let goal = db
        .get_goal(&goal_id)?
        .ok_or_else(|| anyhow!("Goal not found: {}", goal_id))?;

    let tasks = db.list_tasks(&goal_id)?;

    if json {
        let output = serde_json::to_string_pretty(&tasks)?;
        println!("{output}");
        return Ok(());
    }

    println!("Tasks for goal: {} [{}]", goal.id, goal.state.as_str());
    println!("  {}", goal.description);
    println!();

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    for task in tasks {
        println!("{} [{}]", task.id, task.state.as_str());
        println!("  Description: {}", task.description);
        if let Some(ref contract) = task.contract {
            println!("  Contract:");
            println!("    Receives: {}", contract.receives);
            println!("    Produces: {}", contract.produces);
            println!("    Verify: {}", contract.verify);
        } else {
            println!("  Contract: (not set)");
        }
        if let Some(blocked_by) = &task.blocked_by {
            println!("  Blocked by: {}", blocked_by.join(", "));
        }
        if let Some(result) = &task.result {
            println!("  Result: {}", result.summary);
            if !result.artifacts.is_empty() {
                println!("  Artifacts: {}", result.artifacts.join(", "));
            }
        }
        println!();
    }

    Ok(())
}

pub fn start(task_id: String, db: &mut Database) -> Result<()> {
    let task = db.get_task(&task_id)?;

    if task.is_none() {
        // Get all tasks across all goals for suggestions
        let all_goals = db.list_goals()?;
        let mut all_task_ids = Vec::new();
        for goal in all_goals {
            let tasks = db.list_tasks(&goal.id)?;
            all_task_ids.extend(tasks.iter().map(|t| t.id.clone()));
        }

        return if let Some(suggestion) = find_similar_id(&task_id, &all_task_ids) {
            Err(anyhow!(
                "Task not found: {}\nDid you mean: {}",
                task_id,
                suggestion
            ))
        } else {
            Err(anyhow!("Task not found: {}", task_id))
        };
    }

    let task = task.unwrap();

    if task.contract.is_none() {
        return Err(anyhow!(
            "Task has no contract. Set a contract before starting.\nUse: radial task contract {} --receives \"...\" --produces \"...\" --verify \"...\"",
            task.id
        ));
    }

    if task.state == TaskState::Blocked {
        if let Some(ref blocked_by) = task.blocked_by {
            return Err(anyhow!(
                "Task is blocked by: {}\nComplete those tasks first, or use --force to override.",
                blocked_by.join(", ")
            ));
        }
    }

    if task.state != TaskState::Pending {
        return Err(anyhow!(
            "Task must be in 'pending' state to start. Current state: {}",
            task.state.as_str()
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let transitioned = db.transition_task_state(
        &task.id,
        &TaskState::Pending,
        &TaskState::InProgress,
        &updated_at,
    )?;

    if !transitioned {
        return Err(anyhow!(
            "Failed to start task: another process may have already started it"
        ));
    }

    println!("Started task: {}", task.id);
    println!("  Description: {}", task.description);
    Ok(())
}

pub fn complete(
    task_id: String,
    result_summary: String,
    artifacts: Option<Vec<String>>,
    tokens: Option<i64>,
    elapsed: Option<i64>,
    db: &mut Database,
) -> Result<()> {
    let task = db.get_task(&task_id)?;

    if task.is_none() {
        // Get all tasks across all goals for suggestions
        let all_goals = db.list_goals()?;
        let mut all_task_ids = Vec::new();
        for goal in all_goals {
            let tasks = db.list_tasks(&goal.id)?;
            all_task_ids.extend(tasks.iter().map(|t| t.id.clone()));
        }

        return if let Some(suggestion) = find_similar_id(&task_id, &all_task_ids) {
            Err(anyhow!(
                "Task not found: {}\nDid you mean: {}",
                task_id,
                suggestion
            ))
        } else {
            Err(anyhow!("Task not found: {}", task_id))
        };
    }

    let task = task.unwrap();

    if task.state != TaskState::InProgress {
        return Err(anyhow!(
            "Task must be in 'in_progress' state to complete. Current state: {}",
            task.state.as_str()
        ));
    }

    let now = Utc::now();
    let updated_at = now.to_rfc3339();
    let completed_at = now.to_rfc3339();
    let artifacts_list = artifacts.unwrap_or_default();
    let artifacts_json = serde_json::to_string(&artifacts_list)?;

    let transitioned = db.complete_task(
        &task.id,
        &result_summary,
        Some(&artifacts_json),
        tokens.unwrap_or(0),
        elapsed.unwrap_or(0),
        &updated_at,
        &completed_at,
    )?;

    if !transitioned {
        return Err(anyhow!(
            "Failed to complete task: another process may have changed its state"
        ));
    }

    // Re-fetch for subsequent logic
    let task = db.get_task(&task_id)?.unwrap();

    let mut goal = db
        .get_goal(&task.goal_id)?
        .ok_or_else(|| anyhow!("Goal not found: {}", task.goal_id))?;

    goal.updated_at = Utc::now();

    let all_tasks = db.list_tasks(&goal.id)?;

    // Unblock tasks that were waiting on this task
    let completed_task_id = task.id.clone();
    let mut unblocked_tasks = Vec::new();

    for mut dependent_task in all_tasks.iter().cloned() {
        if dependent_task.state == TaskState::Blocked {
            if let Some(ref blocked_by) = dependent_task.blocked_by {
                if blocked_by.contains(&completed_task_id) {
                    // Check if all blocking tasks are now completed
                    let all_blockers_done = blocked_by.iter().all(|blocker_id| {
                        all_tasks
                            .iter()
                            .any(|t| t.id == *blocker_id && t.state == TaskState::Completed)
                    });

                    if all_blockers_done {
                        dependent_task.state = TaskState::Pending;
                        dependent_task.updated_at = Utc::now();
                        db.update_task(&dependent_task)?;
                        unblocked_tasks.push(dependent_task.id.clone());
                    }
                }
            }
        }
    }

    // Refresh task list after unblocking
    let all_tasks = db.list_tasks(&goal.id)?;
    let all_completed = all_tasks.iter().all(|t| t.state == TaskState::Completed);
    let any_failed = all_tasks.iter().any(|t| t.state == TaskState::Failed);

    if all_completed {
        goal.state = GoalState::Completed;
        goal.completed_at = Some(Utc::now());
    } else if any_failed {
        goal.state = GoalState::Failed;
    }

    db.update_goal(&goal)?;

    println!("Completed task: {}", task.id);
    println!("  Result: {}", task.result.as_ref().unwrap().summary);

    if !unblocked_tasks.is_empty() {
        println!("\nUnblocked tasks:");
        for unblocked_id in unblocked_tasks {
            println!("  - {}", unblocked_id);
        }
    }

    Ok(())
}

pub fn fail(task_id: String, db: &mut Database) -> Result<()> {
    let task = db.get_task(&task_id)?;

    if task.is_none() {
        let all_goals = db.list_goals()?;
        let mut all_task_ids = Vec::new();
        for goal in all_goals {
            let tasks = db.list_tasks(&goal.id)?;
            all_task_ids.extend(tasks.iter().map(|t| t.id.clone()));
        }

        return if let Some(suggestion) = find_similar_id(&task_id, &all_task_ids) {
            Err(anyhow!(
                "Task not found: {}\nDid you mean: {}",
                task_id,
                suggestion
            ))
        } else {
            Err(anyhow!("Task not found: {}", task_id))
        };
    }

    let task = task.unwrap();

    if task.state != TaskState::InProgress && task.state != TaskState::Verifying {
        return Err(anyhow!(
            "Task must be in 'in_progress' or 'verifying' state to fail. Current state: {}",
            task.state.as_str()
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let transitioned = db.transition_task_state_from_any(
        &task.id,
        &[&TaskState::InProgress, &TaskState::Verifying],
        &TaskState::Failed,
        &updated_at,
    )?;

    if !transitioned {
        return Err(anyhow!(
            "Failed to mark task as failed: state may have changed"
        ));
    }

    println!("Failed task: {}", task.id);
    println!("  Description: {}", task.description);
    Ok(())
}

pub fn retry(task_id: String, db: &mut Database) -> Result<()> {
    let task = db.get_task(&task_id)?;

    if task.is_none() {
        let all_goals = db.list_goals()?;
        let mut all_task_ids = Vec::new();
        for goal in all_goals {
            let tasks = db.list_tasks(&goal.id)?;
            all_task_ids.extend(tasks.iter().map(|t| t.id.clone()));
        }

        return if let Some(suggestion) = find_similar_id(&task_id, &all_task_ids) {
            Err(anyhow!(
                "Task not found: {}\nDid you mean: {}",
                task_id,
                suggestion
            ))
        } else {
            Err(anyhow!("Task not found: {}", task_id))
        };
    }

    let task = task.unwrap();

    if task.state != TaskState::Failed {
        return Err(anyhow!(
            "Task must be in 'failed' state to retry. Current state: {}",
            task.state.as_str()
        ));
    }

    let updated_at = Utc::now().to_rfc3339();
    let transitioned = db.retry_task(&task.id, &updated_at)?;

    if !transitioned {
        return Err(anyhow!("Failed to retry task: state may have changed"));
    }

    // Re-fetch to get updated retry_count
    let task = db.get_task(&task_id)?.unwrap();

    println!("Retrying task: {}", task.id);
    println!("  Description: {}", task.description);
    println!("  Retry count: {}", task.metrics.retry_count);
    Ok(())
}

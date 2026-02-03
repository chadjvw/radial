use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

use crate::models::{Contract, Goal, GoalState, Metrics, Outcome, Task, TaskMetrics, TaskState};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database connection")?;
        Ok(Self { conn })
    }

    pub fn init_schema(&self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS goals (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                description TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                total_tokens INTEGER DEFAULT 0,
                prompt_tokens INTEGER DEFAULT 0,
                completion_tokens INTEGER DEFAULT 0,
                elapsed_ms INTEGER DEFAULT 0,
                task_count INTEGER DEFAULT 0,
                tasks_completed INTEGER DEFAULT 0,
                tasks_failed INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                goal_id TEXT NOT NULL,
                description TEXT NOT NULL,
                contract_receives TEXT,
                contract_produces TEXT,
                contract_verify TEXT,
                state TEXT NOT NULL,
                blocked_by TEXT,
                result_summary TEXT,
                result_artifacts TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                tokens INTEGER DEFAULT 0,
                elapsed_ms INTEGER DEFAULT 0,
                retry_count INTEGER DEFAULT 0,
                FOREIGN KEY(goal_id) REFERENCES goals(id)
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_goal_id ON tasks(goal_id);
            CREATE INDEX IF NOT EXISTS idx_goals_state ON goals(state);
            CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);
            "#,
            )
            .context("Failed to initialize database schema")?;
        Ok(())
    }

    pub fn create_goal(&self, goal: &Goal) -> Result<()> {
        self.conn
            .execute(
                r#"
            INSERT INTO goals (
                id, parent_id, description, state, created_at, updated_at, completed_at,
                total_tokens, prompt_tokens, completion_tokens, elapsed_ms,
                task_count, tasks_completed, tasks_failed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
                params![
                    goal.id,
                    goal.parent_id,
                    goal.description,
                    goal.state.as_str(),
                    goal.created_at.to_rfc3339(),
                    goal.updated_at.to_rfc3339(),
                    goal.completed_at.map(|dt| dt.to_rfc3339()),
                    goal.metrics.total_tokens,
                    goal.metrics.prompt_tokens,
                    goal.metrics.completion_tokens,
                    goal.metrics.elapsed_ms,
                    goal.metrics.task_count,
                    goal.metrics.tasks_completed,
                    goal.metrics.tasks_failed,
                ],
            )
            .context("Failed to insert goal")?;
        Ok(())
    }

    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, parent_id, description, state, created_at, updated_at, completed_at,
                   total_tokens, prompt_tokens, completion_tokens, elapsed_ms,
                   task_count, tasks_completed, tasks_failed
            FROM goals WHERE id = ?1
            "#,
        )?;

        let goal = stmt
            .query_row(params![id], |row| {
                Ok(Goal {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    description: row.get(2)?,
                    state: GoalState::from_str(&row.get::<_, String>(3)?).unwrap(),
                    created_at: row.get::<_, String>(4)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(5)?.parse::<DateTime<Utc>>().unwrap(),
                    completed_at: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    metrics: Metrics {
                        total_tokens: row.get(7)?,
                        prompt_tokens: row.get(8)?,
                        completion_tokens: row.get(9)?,
                        elapsed_ms: row.get(10)?,
                        task_count: row.get(11)?,
                        tasks_completed: row.get(12)?,
                        tasks_failed: row.get(13)?,
                    },
                })
            })
            .optional()?;

        Ok(goal)
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, parent_id, description, state, created_at, updated_at, completed_at,
                   total_tokens, prompt_tokens, completion_tokens, elapsed_ms,
                   task_count, tasks_completed, tasks_failed
            FROM goals ORDER BY created_at DESC
            "#,
        )?;

        let goals = stmt
            .query_map([], |row| {
                Ok(Goal {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    description: row.get(2)?,
                    state: GoalState::from_str(&row.get::<_, String>(3)?).unwrap(),
                    created_at: row.get::<_, String>(4)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(5)?.parse::<DateTime<Utc>>().unwrap(),
                    completed_at: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    metrics: Metrics {
                        total_tokens: row.get(7)?,
                        prompt_tokens: row.get(8)?,
                        completion_tokens: row.get(9)?,
                        elapsed_ms: row.get(10)?,
                        task_count: row.get(11)?,
                        tasks_completed: row.get(12)?,
                        tasks_failed: row.get(13)?,
                    },
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(goals)
    }

    pub fn update_goal(&self, goal: &Goal) -> Result<()> {
        self.conn
            .execute(
                r#"
            UPDATE goals SET
                parent_id = ?2,
                description = ?3,
                state = ?4,
                updated_at = ?5,
                completed_at = ?6,
                total_tokens = ?7,
                prompt_tokens = ?8,
                completion_tokens = ?9,
                elapsed_ms = ?10,
                task_count = ?11,
                tasks_completed = ?12,
                tasks_failed = ?13
            WHERE id = ?1
            "#,
                params![
                    goal.id,
                    goal.parent_id,
                    goal.description,
                    goal.state.as_str(),
                    goal.updated_at.to_rfc3339(),
                    goal.completed_at.map(|dt| dt.to_rfc3339()),
                    goal.metrics.total_tokens,
                    goal.metrics.prompt_tokens,
                    goal.metrics.completion_tokens,
                    goal.metrics.elapsed_ms,
                    goal.metrics.task_count,
                    goal.metrics.tasks_completed,
                    goal.metrics.tasks_failed,
                ],
            )
            .context("Failed to update goal")?;
        Ok(())
    }

    pub fn create_task(&self, task: &Task) -> Result<()> {
        let blocked_by_json = task
            .blocked_by
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let result_artifacts_json = task
            .result
            .as_ref()
            .map(|r| serde_json::to_string(&r.artifacts).unwrap());

        self.conn.execute(
            r#"
            INSERT INTO tasks (
                id, goal_id, description, contract_receives, contract_produces, contract_verify,
                state, blocked_by, result_summary, result_artifacts, created_at, updated_at, completed_at,
                tokens, elapsed_ms, retry_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            "#,
            params![
                task.id,
                task.goal_id,
                task.description,
                task.contract.as_ref().map(|c| c.receives.clone()),
                task.contract.as_ref().map(|c| c.produces.clone()),
                task.contract.as_ref().map(|c| c.verify.clone()),
                task.state.as_str(),
                blocked_by_json,
                task.result.as_ref().map(|r| r.summary.clone()),
                result_artifacts_json,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.metrics.tokens,
                task.metrics.elapsed_ms,
                task.metrics.retry_count,
            ],
        ).context("Failed to insert task")?;
        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, goal_id, description, contract_receives, contract_produces, contract_verify,
                   state, blocked_by, result_summary, result_artifacts, created_at, updated_at, completed_at,
                   tokens, elapsed_ms, retry_count
            FROM tasks WHERE id = ?1
            "#,
        )?;

        let task = stmt
            .query_row(params![id], |row| {
                let blocked_by_json: Option<String> = row.get(7)?;
                let result_artifacts_json: Option<String> = row.get(9)?;
                let result_summary: Option<String> = row.get(8)?;
                let contract_receives: Option<String> = row.get(3)?;
                let contract_produces: Option<String> = row.get(4)?;
                let contract_verify: Option<String> = row.get(5)?;

                let contract = if contract_receives.is_some()
                    || contract_produces.is_some()
                    || contract_verify.is_some()
                {
                    Some(Contract {
                        receives: contract_receives.unwrap_or_default(),
                        produces: contract_produces.unwrap_or_default(),
                        verify: contract_verify.unwrap_or_default(),
                    })
                } else {
                    None
                };

                Ok(Task {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    description: row.get(2)?,
                    contract,
                    state: TaskState::from_str(&row.get::<_, String>(6)?).unwrap(),
                    blocked_by: blocked_by_json.and_then(|s| serde_json::from_str(&s).ok()),
                    result: result_summary.map(|summary| Outcome {
                        summary,
                        artifacts: result_artifacts_json
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                    }),
                    created_at: row.get::<_, String>(10)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(11)?.parse::<DateTime<Utc>>().unwrap(),
                    completed_at: row
                        .get::<_, Option<String>>(12)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    metrics: TaskMetrics {
                        tokens: row.get(13)?,
                        elapsed_ms: row.get(14)?,
                        retry_count: row.get(15)?,
                    },
                })
            })
            .optional()?;

        Ok(task)
    }

    pub fn list_tasks(&self, goal_id: &str) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, goal_id, description, contract_receives, contract_produces, contract_verify,
                   state, blocked_by, result_summary, result_artifacts, created_at, updated_at, completed_at,
                   tokens, elapsed_ms, retry_count
            FROM tasks WHERE goal_id = ?1 ORDER BY created_at ASC
            "#,
        )?;

        let tasks = stmt
            .query_map(params![goal_id], |row| {
                let blocked_by_json: Option<String> = row.get(7)?;
                let result_artifacts_json: Option<String> = row.get(9)?;
                let result_summary: Option<String> = row.get(8)?;
                let contract_receives: Option<String> = row.get(3)?;
                let contract_produces: Option<String> = row.get(4)?;
                let contract_verify: Option<String> = row.get(5)?;

                let contract = if contract_receives.is_some()
                    || contract_produces.is_some()
                    || contract_verify.is_some()
                {
                    Some(Contract {
                        receives: contract_receives.unwrap_or_default(),
                        produces: contract_produces.unwrap_or_default(),
                        verify: contract_verify.unwrap_or_default(),
                    })
                } else {
                    None
                };

                Ok(Task {
                    id: row.get(0)?,
                    goal_id: row.get(1)?,
                    description: row.get(2)?,
                    contract,
                    state: TaskState::from_str(&row.get::<_, String>(6)?).unwrap(),
                    blocked_by: blocked_by_json.and_then(|s| serde_json::from_str(&s).ok()),
                    result: result_summary.map(|summary| Outcome {
                        summary,
                        artifacts: result_artifacts_json
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                    }),
                    created_at: row.get::<_, String>(10)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(11)?.parse::<DateTime<Utc>>().unwrap(),
                    completed_at: row
                        .get::<_, Option<String>>(12)?
                        .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                    metrics: TaskMetrics {
                        tokens: row.get(13)?,
                        elapsed_ms: row.get(14)?,
                        retry_count: row.get(15)?,
                    },
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    pub fn update_task(&self, task: &Task) -> Result<()> {
        let blocked_by_json = task
            .blocked_by
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let result_artifacts_json = task
            .result
            .as_ref()
            .map(|r| serde_json::to_string(&r.artifacts).unwrap());

        self.conn
            .execute(
                r#"
            UPDATE tasks SET
                goal_id = ?2,
                description = ?3,
                contract_receives = ?4,
                contract_produces = ?5,
                contract_verify = ?6,
                state = ?7,
                blocked_by = ?8,
                result_summary = ?9,
                result_artifacts = ?10,
                updated_at = ?11,
                completed_at = ?12,
                tokens = ?13,
                elapsed_ms = ?14,
                retry_count = ?15
            WHERE id = ?1
            "#,
                params![
                    task.id,
                    task.goal_id,
                    task.description,
                    task.contract.as_ref().map(|c| c.receives.clone()),
                    task.contract.as_ref().map(|c| c.produces.clone()),
                    task.contract.as_ref().map(|c| c.verify.clone()),
                    task.state.as_str(),
                    blocked_by_json,
                    task.result.as_ref().map(|r| r.summary.clone()),
                    result_artifacts_json,
                    task.updated_at.to_rfc3339(),
                    task.completed_at.map(|dt| dt.to_rfc3339()),
                    task.metrics.tokens,
                    task.metrics.elapsed_ms,
                    task.metrics.retry_count,
                ],
            )
            .context("Failed to update task")?;
        Ok(())
    }

    /// Atomically transition a task from one state to another.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in the expected state.
    pub fn transition_task_state(
        &self,
        task_id: &str,
        from_state: &TaskState,
        to_state: &TaskState,
        updated_at: &str,
    ) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE tasks SET state = ?1, updated_at = ?2 WHERE id = ?3 AND state = ?4",
                params![to_state.as_str(), updated_at, task_id, from_state.as_str()],
            )
            .context("Failed to transition task state")?;

        Ok(rows_affected > 0)
    }

    /// Atomically transition a task from one of several states to a new state.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in any of the expected states.
    pub fn transition_task_state_from_any(
        &self,
        task_id: &str,
        from_states: &[&TaskState],
        to_state: &TaskState,
        updated_at: &str,
    ) -> Result<bool> {
        let state_list: Vec<&str> = from_states.iter().map(|s| s.as_str()).collect();
        let placeholders: Vec<String> = (0..state_list.len())
            .map(|i| format!("?{}", i + 4))
            .collect();
        let query = format!(
            "UPDATE tasks SET state = ?1, updated_at = ?2 WHERE id = ?3 AND state IN ({})",
            placeholders.join(", ")
        );

        let to_state_str = to_state.as_str();
        let mut stmt = self.conn.prepare(&query)?;
        let mut param_values: Vec<&dyn rusqlite::ToSql> =
            vec![&to_state_str, &updated_at, &task_id];
        for state in &state_list {
            param_values.push(state);
        }

        let rows_affected = stmt.execute(param_values.as_slice())?;
        Ok(rows_affected > 0)
    }

    /// Atomically complete a task: transition from InProgress to Completed with result and metrics.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in InProgress state.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_task(
        &self,
        task_id: &str,
        result_summary: &str,
        result_artifacts: Option<&str>,
        tokens: i64,
        elapsed_ms: i64,
        updated_at: &str,
        completed_at: &str,
    ) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute(
                r"UPDATE tasks SET
                    state = ?1,
                    result_summary = ?2,
                    result_artifacts = ?3,
                    tokens = ?4,
                    elapsed_ms = ?5,
                    updated_at = ?6,
                    completed_at = ?7
                WHERE id = ?8 AND state = ?9",
                params![
                    TaskState::Completed.as_str(),
                    result_summary,
                    result_artifacts,
                    tokens,
                    elapsed_ms,
                    updated_at,
                    completed_at,
                    task_id,
                    TaskState::InProgress.as_str()
                ],
            )
            .context("Failed to complete task")?;

        Ok(rows_affected > 0)
    }

    /// Atomically retry a failed task: transition from Failed to InProgress and increment retry_count.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in Failed state.
    pub fn retry_task(&self, task_id: &str, updated_at: &str) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute(
                "UPDATE tasks SET state = ?1, retry_count = retry_count + 1, updated_at = ?2 WHERE id = ?3 AND state = ?4",
                params![TaskState::InProgress.as_str(), updated_at, task_id, TaskState::Failed.as_str()],
            )
            .context("Failed to retry task")?;

        Ok(rows_affected > 0)
    }

    pub fn compute_goal_metrics(&self, goal_id: &str) -> Result<Metrics> {
        let tasks = self.list_tasks(goal_id)?;

        let total_tokens: i64 = tasks.iter().map(|t| t.metrics.tokens).sum();
        let elapsed_ms: i64 = tasks.iter().map(|t| t.metrics.elapsed_ms).sum();
        let task_count = i64::try_from(tasks.len()).unwrap_or(0);
        let tasks_completed = i64::try_from(
            tasks
                .iter()
                .filter(|t| t.state == TaskState::Completed)
                .count(),
        )
        .unwrap_or(0);
        let tasks_failed = i64::try_from(
            tasks
                .iter()
                .filter(|t| t.state == TaskState::Failed)
                .count(),
        )
        .unwrap_or(0);

        Ok(Metrics {
            total_tokens,
            prompt_tokens: 0,
            completion_tokens: 0,
            elapsed_ms,
            task_count,
            tasks_completed,
            tasks_failed,
        })
    }
}

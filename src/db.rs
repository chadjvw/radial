use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use jiff::Timestamp;

use crate::models::{Goal, Metrics, Outcome, Task, TaskState};

const GOALS_FILE: &str = "goals.jsonl";
const TASKS_FILE: &str = "tasks.jsonl";

pub struct Database {
    path: PathBuf,
    goals: HashMap<String, Goal>,
    tasks: HashMap<String, Task>,
    tasks_by_goal: HashMap<String, Vec<String>>,
}

impl Database {
    /// Open an existing database from the given directory
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            bail!("Database directory does not exist: {}", path.display());
        }

        let mut db = Self {
            path,
            goals: HashMap::new(),
            tasks: HashMap::new(),
            tasks_by_goal: HashMap::new(),
        };

        db.load()?;
        Ok(db)
    }

    /// Initialize a new database (creates empty files)
    pub fn init_schema(&self) -> Result<()> {
        let goals_path = self.path.join(GOALS_FILE);
        let tasks_path = self.path.join(TASKS_FILE);

        if !goals_path.exists() {
            File::create(&goals_path).context("Failed to create goals.jsonl")?;
        }

        if !tasks_path.exists() {
            File::create(&tasks_path).context("Failed to create tasks.jsonl")?;
        }

        Ok(())
    }

    /// Load all data from JSONL files into memory
    fn load(&mut self) -> Result<()> {
        self.load_goals()?;
        self.load_tasks()?;
        self.rebuild_indexes();
        Ok(())
    }

    fn load_goals(&mut self) -> Result<()> {
        let goals_path = self.path.join(GOALS_FILE);
        if !goals_path.exists() {
            return Ok(());
        }

        let file = File::open(&goals_path).context("Failed to open goals.jsonl")?;
        let reader = BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.context("Failed to read line from goals.jsonl")?;
            if line.trim().is_empty() {
                continue;
            }

            let goal: Goal = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse goal at line {}", line_num + 1))?;

            self.goals.insert(goal.id.clone(), goal);
        }

        Ok(())
    }

    fn load_tasks(&mut self) -> Result<()> {
        let tasks_path = self.path.join(TASKS_FILE);
        if !tasks_path.exists() {
            return Ok(());
        }

        let file = File::open(&tasks_path).context("Failed to open tasks.jsonl")?;
        let reader = BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.context("Failed to read line from tasks.jsonl")?;
            if line.trim().is_empty() {
                continue;
            }

            let task: Task = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse task at line {}", line_num + 1))?;

            self.tasks.insert(task.id.clone(), task);
        }

        Ok(())
    }

    fn rebuild_indexes(&mut self) {
        self.tasks_by_goal.clear();

        for task in self.tasks.values() {
            self.tasks_by_goal
                .entry(task.goal_id.clone())
                .or_default()
                .push(task.id.clone());
        }
    }

    /// Write all goals to disk atomically
    fn persist_goals(&self) -> Result<()> {
        let temp_path = self.path.join("goals.jsonl.tmp");
        let final_path = self.path.join(GOALS_FILE);

        let mut file = File::create(&temp_path).context("Failed to create temporary goals file")?;

        file.lock_exclusive()
            .context("Failed to acquire lock on goals file")?;

        for goal in self.goals.values() {
            serde_json::to_writer(&mut file, goal)?;
            writeln!(file)?;
        }

        file.sync_all().context("Failed to sync goals file")?;
        file.unlock().context("Failed to unlock goals file")?;

        fs::rename(&temp_path, &final_path).context("Failed to rename goals file")?;

        Ok(())
    }

    /// Write all tasks to disk atomically
    fn persist_tasks(&self) -> Result<()> {
        let temp_path = self.path.join("tasks.jsonl.tmp");
        let final_path = self.path.join(TASKS_FILE);

        let mut file = File::create(&temp_path).context("Failed to create temporary tasks file")?;

        file.lock_exclusive()
            .context("Failed to acquire lock on tasks file")?;

        for task in self.tasks.values() {
            serde_json::to_writer(&mut file, task)?;
            writeln!(file)?;
        }

        file.sync_all().context("Failed to sync tasks file")?;
        file.unlock().context("Failed to unlock tasks file")?;

        fs::rename(&temp_path, &final_path).context("Failed to rename tasks file")?;

        Ok(())
    }

    // Goal operations

    pub fn create_goal(&mut self, goal: &Goal) -> Result<()> {
        if self.goals.contains_key(&goal.id) {
            bail!("Goal already exists: {}", goal.id);
        }

        self.goals.insert(goal.id.clone(), goal.clone());
        self.persist_goals()?;

        Ok(())
    }

    pub fn get_goal(&self, id: &str) -> Result<Option<Goal>> {
        Ok(self.goals.get(id).cloned())
    }

    pub fn list_goals(&self) -> Result<Vec<Goal>> {
        let mut goals: Vec<Goal> = self.goals.values().cloned().collect();
        goals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(goals)
    }

    pub fn update_goal(&mut self, goal: &Goal) -> Result<()> {
        if !self.goals.contains_key(&goal.id) {
            bail!("Goal not found: {}", goal.id);
        }

        self.goals.insert(goal.id.clone(), goal.clone());
        self.persist_goals()?;

        Ok(())
    }

    // Task operations

    pub fn create_task(&mut self, task: &Task) -> Result<()> {
        if self.tasks.contains_key(&task.id) {
            bail!("Task already exists: {}", task.id);
        }

        self.tasks_by_goal
            .entry(task.goal_id.clone())
            .or_default()
            .push(task.id.clone());

        self.tasks.insert(task.id.clone(), task.clone());
        self.persist_tasks()?;

        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        Ok(self.tasks.get(id).cloned())
    }

    pub fn list_tasks(&self, goal_id: &str) -> Result<Vec<Task>> {
        let task_ids = self.tasks_by_goal.get(goal_id);

        match task_ids {
            Some(ids) => {
                let mut tasks: Vec<Task> = ids
                    .iter()
                    .filter_map(|id| self.tasks.get(id).cloned())
                    .collect();
                tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
                Ok(tasks)
            }
            None => Ok(Vec::new()),
        }
    }

    pub fn update_task(&mut self, task: &Task) -> Result<()> {
        if !self.tasks.contains_key(&task.id) {
            bail!("Task not found: {}", task.id);
        }

        self.tasks.insert(task.id.clone(), task.clone());
        self.persist_tasks()?;

        Ok(())
    }

    /// Atomically transition a task from one state to another.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in the expected state.
    pub fn transition_task_state(
        &mut self,
        task_id: &str,
        from_state: &TaskState,
        to_state: &TaskState,
        updated_at: &str,
    ) -> Result<bool> {
        let Some(task) = self.tasks.get_mut(task_id) else {
            return Ok(false);
        };

        if task.state != *from_state {
            return Ok(false);
        }

        task.state = to_state.clone();
        task.updated_at = updated_at.parse().unwrap_or_else(|_| Timestamp::now());

        self.persist_tasks()?;

        Ok(true)
    }

    /// Atomically transition a task from one of several states to a new state.
    /// Returns Ok(true) if the transition succeeded, Ok(false) if the task was not in any of the expected states.
    pub fn transition_task_state_from_any(
        &mut self,
        task_id: &str,
        from_states: &[&TaskState],
        to_state: &TaskState,
        updated_at: &str,
    ) -> Result<bool> {
        let Some(task) = self.tasks.get_mut(task_id) else {
            return Ok(false);
        };

        if !from_states.iter().any(|s| task.state == **s) {
            return Ok(false);
        }

        task.state = to_state.clone();
        task.updated_at = updated_at.parse().unwrap_or_else(|_| Timestamp::now());

        self.persist_tasks()?;
        Ok(true)
    }

    /// Atomically complete a task: transition from `InProgress` to `Completed` with result and metrics.
    /// Returns `Ok(true)` if the transition succeeded, `Ok(false)` if the task was not in `InProgress` state.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_task(
        &mut self,
        task_id: &str,
        result_summary: &str,
        result_artifacts: Option<&str>,
        tokens: i64,
        elapsed_ms: i64,
        updated_at: &str,
        completed_at: &str,
    ) -> Result<bool> {
        let Some(task) = self.tasks.get_mut(task_id) else {
            return Ok(false);
        };

        if task.state != TaskState::InProgress {
            return Ok(false);
        }

        task.state = TaskState::Completed;
        task.result = Some(Outcome {
            summary: result_summary.to_string(),
            artifacts: result_artifacts
                .map(|a| serde_json::from_str(a).unwrap_or_default())
                .unwrap_or_default(),
        });
        task.metrics.tokens = tokens;
        task.metrics.elapsed_ms = elapsed_ms;
        task.updated_at = updated_at.parse().unwrap_or_else(|_| Timestamp::now());
        task.completed_at = Some(completed_at.parse().unwrap_or_else(|_| Timestamp::now()));

        self.persist_tasks()?;

        Ok(true)
    }

    /// Atomically retry a failed task: transition from `Failed` to `InProgress` and increment `retry_count`.
    /// Returns `Ok(true)` if the transition succeeded, `Ok(false)` if the task was not in `Failed` state.
    pub fn retry_task(&mut self, task_id: &str, updated_at: &str) -> Result<bool> {
        let Some(task) = self.tasks.get_mut(task_id) else {
            return Ok(false);
        };

        if task.state != TaskState::Failed {
            return Ok(false);
        }

        task.state = TaskState::InProgress;
        task.metrics.retry_count += 1;
        task.updated_at = updated_at.parse().unwrap_or_else(|_| Timestamp::now());

        self.persist_tasks()?;

        Ok(true)
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

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fs2::FileExt;

use crate::models::{Goal, Metrics, Task, TaskState};

/// Atomically write content to a file using a temporary file + rename.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let temp = path.with_extension("toml.tmp");
    let mut file = File::create(&temp)
        .with_context(|| format!("Failed to create temporary file: {}", temp.display()))?;
    file.lock_exclusive()
        .context("Failed to acquire file lock")?;
    file.write_all(content)
        .context("Failed to write file content")?;
    file.sync_all().context("Failed to sync file")?;
    file.unlock().context("Failed to unlock file")?;
    fs::rename(&temp, path).with_context(|| format!("Failed to rename to {}", path.display()))?;
    Ok(())
}

pub struct Database {
    path: PathBuf,
    goals: HashMap<String, Goal>,
    tasks: HashMap<String, Task>,
}

impl Database {
    /// Open an existing database from the given directory.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            bail!("Database directory does not exist: {}", path.display());
        }

        let mut db = Self {
            path,
            goals: HashMap::new(),
            tasks: HashMap::new(),
        };

        db.load()?;
        Ok(db)
    }

    /// Initialize a new database. The `.radial/` directory must already exist.
    pub fn init_schema(&self) -> Result<()> {
        Ok(())
    }

    /// The base path for the `.radial/` directory.
    pub fn base_path(&self) -> &Path {
        &self.path
    }

    /// Load all data from the per-entity TOML files into memory.
    fn load(&mut self) -> Result<()> {
        let dir = fs::read_dir(&self.path).context("Failed to read .radial directory")?;

        for entry in dir {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let goal_toml_path = path.join("goal.toml");
            if !goal_toml_path.exists() {
                continue;
            }

            let goal_content = fs::read_to_string(&goal_toml_path)
                .with_context(|| format!("Failed to read {}", goal_toml_path.display()))?;
            let goal: Goal = toml::from_str(&goal_content)
                .with_context(|| format!("Failed to parse {}", goal_toml_path.display()))?;

            let goal_id = goal.id.clone();
            self.goals.insert(goal_id, goal);

            let task_dir = fs::read_dir(&path)
                .with_context(|| format!("Failed to read goal directory: {}", path.display()))?;

            for task_entry in task_dir {
                let task_entry = task_entry.context("Failed to read task entry")?;
                let task_path = task_entry.path();

                if task_path.file_name() == Some(std::ffi::OsStr::new("goal.toml")) {
                    continue;
                }

                if task_path.extension() != Some(std::ffi::OsStr::new("toml")) {
                    continue;
                }

                let task_content = fs::read_to_string(&task_path)
                    .with_context(|| format!("Failed to read {}", task_path.display()))?;
                let task: Task = toml::from_str(&task_content)
                    .with_context(|| format!("Failed to parse {}", task_path.display()))?;

                self.tasks.insert(task.id.clone(), task);
            }
        }

        Ok(())
    }

    // Goal operations

    pub fn create_goal(&mut self, goal: Goal) -> Result<()> {
        if self.goals.contains_key(&goal.id) {
            bail!("Goal already exists: {}", goal.id);
        }

        let goal_dir = self.path.join(&goal.id);
        fs::create_dir_all(&goal_dir).context("Failed to create goal directory")?;

        goal.write_file(&self.path)?;
        self.goals.insert(goal.id.clone(), goal);

        Ok(())
    }

    pub fn get_goal(&self, id: &str) -> Option<&Goal> {
        self.goals.get(id)
    }

    pub fn get_goal_mut(&mut self, id: &str) -> Option<&mut Goal> {
        self.goals.get_mut(id)
    }

    pub fn list_goals(&self) -> Vec<&Goal> {
        let mut goals: Vec<&Goal> = self.goals.values().collect();
        goals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        goals
    }

    // Task operations

    pub fn create_task(&mut self, task: Task) -> Result<()> {
        if self.tasks.contains_key(&task.id) {
            bail!("Task already exists: {}", task.id);
        }

        task.write_file(&self.path)?;
        self.tasks.insert(task.id.clone(), task);

        Ok(())
    }

    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    pub fn list_tasks(&self, goal_id: &str) -> Vec<&Task> {
        let mut tasks: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| t.goal_id == goal_id)
            .collect();
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        tasks
    }

    pub fn compute_goal_metrics(&self, goal_id: &str) -> Metrics {
        let tasks = self.list_tasks(goal_id);

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

        Metrics {
            total_tokens,
            prompt_tokens: 0,
            completion_tokens: 0,
            elapsed_ms,
            task_count,
            tasks_completed,
            tasks_failed,
        }
    }
}

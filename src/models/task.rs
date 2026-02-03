use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Contract, Outcome};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Pending,
    Blocked,
    InProgress,
    Verifying,
    Completed,
    Failed,
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Pending => "pending",
            TaskState::Blocked => "blocked",
            TaskState::InProgress => "in_progress",
            TaskState::Verifying => "verifying",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TaskState::Pending),
            "blocked" => Some(TaskState::Blocked),
            "in_progress" => Some(TaskState::InProgress),
            "verifying" => Some(TaskState::Verifying),
            "completed" => Some(TaskState::Completed),
            "failed" => Some(TaskState::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub tokens: i64,
    pub elapsed_ms: i64,
    pub retry_count: i64,
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            tokens: 0,
            elapsed_ms: 0,
            retry_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub goal_id: String,
    pub description: String,
    pub contract: Option<Contract>,
    pub state: TaskState,
    pub blocked_by: Option<Vec<String>>,
    pub result: Option<Outcome>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metrics: TaskMetrics,
}

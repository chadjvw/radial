use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GoalState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl GoalState {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalState::Pending => "pending",
            GoalState::InProgress => "in_progress",
            GoalState::Completed => "completed",
            GoalState::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(GoalState::Pending),
            "in_progress" => Some(GoalState::InProgress),
            "completed" => Some(GoalState::Completed),
            "failed" => Some(GoalState::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub elapsed_ms: i64,
    pub task_count: i64,
    pub tasks_completed: i64,
    pub tasks_failed: i64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            elapsed_ms: 0,
            task_count: 0,
            tasks_completed: 0,
            tasks_failed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub parent_id: Option<String>,
    pub description: String,
    pub state: GoalState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metrics: Metrics,
}

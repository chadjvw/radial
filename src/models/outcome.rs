use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub summary: String,
    pub artifacts: Vec<String>,
}

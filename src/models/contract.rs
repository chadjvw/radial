use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub receives: String,
    pub produces: String,
    pub verify: String,
}

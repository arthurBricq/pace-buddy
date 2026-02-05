use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStream {
    pub activity_id: Uuid,
    pub stream_type: String,
    pub data_json: String,
}

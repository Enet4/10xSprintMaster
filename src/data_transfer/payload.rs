use serde::{Deserialize, Serialize};

use crate::components::{stage::StageId, task::TaskKind};

/// The data transferred when dragging a task to a drop zone.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TaskTransfer {
    /// the unique task ID
    pub id: u32,
    pub kind: TaskKind,
    /// where it came from
    pub from: StageId,
    pub progress: f64,
}

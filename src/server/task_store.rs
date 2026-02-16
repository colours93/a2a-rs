//! Task store — persistence layer for A2A tasks.
//!
//! Mirrors Python SDK's `TaskStore(ABC)` and `InMemoryTaskStore` from
//! `a2a.server.tasks.task_store` and `a2a.server.tasks.inmemory_task_store`.
//!
//! The task store is responsible for persisting and retrieving [`Task`] objects.
//! The [`InMemoryTaskStore`] is provided for development and testing; production
//! deployments should implement the [`TaskStore`] trait backed by a database.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::A2AResult;
use crate::types::{Task, TaskState};

/// Parameters for listing tasks with optional filtering and pagination.
#[derive(Debug, Clone, Default)]
pub struct TaskListParams {
    /// Filter tasks by context ID.
    pub context_id: Option<String>,

    /// Filter tasks by state.
    pub status: Option<Vec<TaskState>>,

    /// Maximum number of tasks to return per page.
    pub page_size: Option<usize>,

    /// Opaque token for pagination — the task ID to start after.
    pub page_token: Option<String>,
}

/// Response for a task listing request.
#[derive(Debug, Clone)]
pub struct TaskListResponse {
    /// The tasks matching the query.
    pub tasks: Vec<Task>,

    /// Token for the next page, if more results are available.
    pub next_page_token: Option<String>,
}

/// Trait for persisting and retrieving A2A tasks.
///
/// Implementations must be `Send + Sync` for use in async server contexts.
/// All methods take `&self` and use interior mutability for thread safety.
///
/// # Provided implementations
///
/// - [`InMemoryTaskStore`] — simple in-memory store (data lost on restart)
#[async_trait]
pub trait TaskStore: Send + Sync {
    /// Save or update a task in the store.
    ///
    /// If a task with the same ID already exists, it is overwritten.
    async fn save(&self, task: Task) -> A2AResult<()>;

    /// Retrieve a task by its ID.
    ///
    /// Returns `None` if the task does not exist.
    async fn get(&self, task_id: &str) -> A2AResult<Option<Task>>;

    /// Delete a task by its ID.
    ///
    /// Silently succeeds if the task does not exist.
    async fn delete(&self, task_id: &str) -> A2AResult<()>;

    /// List tasks matching the given parameters.
    ///
    /// Supports filtering by context ID and status, and pagination via
    /// `page_size` and `page_token`.
    async fn list(&self, params: &TaskListParams) -> A2AResult<TaskListResponse>;
}

/// In-memory task store backed by a `HashMap`.
///
/// Suitable for development, testing, and short-lived server instances.
/// All task data is lost when the process exits.
///
/// Thread-safe via `tokio::sync::RwLock`.
#[derive(Debug)]
pub struct InMemoryTaskStore {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
    /// Insertion order for deterministic listing/pagination.
    insertion_order: Arc<RwLock<Vec<String>>>,
}

impl InMemoryTaskStore {
    /// Create a new empty in-memory task store.
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            insertion_order: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStore for InMemoryTaskStore {
    async fn save(&self, task: Task) -> A2AResult<()> {
        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().await;
        let is_new = !tasks.contains_key(&task_id);
        tasks.insert(task_id.clone(), task);

        if is_new {
            let mut order = self.insertion_order.write().await;
            order.push(task_id.clone());
        }

        debug!(task_id = %task_id, is_new = is_new, "Task saved");
        Ok(())
    }

    async fn get(&self, task_id: &str) -> A2AResult<Option<Task>> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(task_id).cloned();
        debug!(task_id = %task_id, found = task.is_some(), "Task lookup");
        Ok(task)
    }

    async fn delete(&self, task_id: &str) -> A2AResult<()> {
        let mut tasks = self.tasks.write().await;
        if tasks.remove(task_id).is_some() {
            let mut order = self.insertion_order.write().await;
            order.retain(|id| id != task_id);
            debug!(task_id = %task_id, "Task deleted");
        } else {
            warn!(task_id = %task_id, "Attempted to delete non-existent task");
        }
        Ok(())
    }

    async fn list(&self, params: &TaskListParams) -> A2AResult<TaskListResponse> {
        let tasks = self.tasks.read().await;
        let order = self.insertion_order.read().await;

        // Determine the starting position based on page_token.
        let start_idx = if let Some(ref token) = params.page_token {
            // page_token is the last task ID from the previous page.
            // Find its position and start after it.
            match order.iter().position(|id| id == token) {
                Some(pos) => pos + 1,
                None => {
                    // Invalid token — start from the beginning.
                    warn!(page_token = %token, "Invalid page token, starting from beginning");
                    0
                }
            }
        } else {
            0
        };

        let page_size = params.page_size.unwrap_or(usize::MAX);
        let mut result_tasks = Vec::new();
        let mut last_id: Option<String> = None;

        for id in order.iter().skip(start_idx) {
            if result_tasks.len() >= page_size {
                break;
            }

            if let Some(task) = tasks.get(id) {
                // Apply context_id filter.
                if let Some(ref ctx_id) = params.context_id {
                    if task.context_id != *ctx_id {
                        continue;
                    }
                }

                // Apply status filter.
                if let Some(ref statuses) = params.status {
                    if !statuses.contains(&task.status.state) {
                        continue;
                    }
                }

                last_id = Some(id.clone());
                result_tasks.push(task.clone());
            }
        }

        // Determine if there are more results.
        let next_page_token = if result_tasks.len() == page_size {
            // Check if there are more tasks after the last returned one.
            if let Some(ref last) = last_id {
                let last_pos = order.iter().position(|id| id == last).unwrap_or(0);
                if last_pos + 1 < order.len() {
                    Some(last.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        debug!(
            count = result_tasks.len(),
            has_more = next_page_token.is_some(),
            "Listed tasks"
        );

        Ok(TaskListResponse {
            tasks: result_tasks,
            next_page_token,
        })
    }
}

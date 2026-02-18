//! File-based task store — persists tasks as JSON files.
//!
//! This implementation writes each task as a separate JSON file in a directory,
//! inspired by Claude Code's task system. Suitable for development, debugging,
//! and visualization with external tools (e.g., a TUI watching the directory).
//!
//! Thread-safe via `tokio::sync::RwLock`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::error::{A2AError, A2AResult};
use crate::server::task_store::{TaskListParams, TaskListResponse, TaskStore};
use crate::types::Task;

/// File-based task store that persists tasks as individual JSON files.
///
/// Each task is saved as `{task_id}.json` in the specified directory.
/// This allows external tools (like a TUI) to watch the directory and
/// visualize task updates in real-time.
///
/// Thread-safe via `tokio::sync::RwLock`.
#[derive(Debug, Clone)]
pub struct FileTaskStore {
    /// Directory where task JSON files are stored.
    tasks_dir: PathBuf,
    /// In-memory cache for faster listing/filtering.
    /// Maps task_id -> Task
    cache: Arc<RwLock<HashMap<String, Task>>>,
    /// Insertion order for deterministic listing/pagination.
    insertion_order: Arc<RwLock<Vec<String>>>,
}

impl FileTaskStore {
    /// Create a new file-based task store.
    ///
    /// Creates the directory if it doesn't exist.
    ///
    /// # Arguments
    /// * `tasks_dir` - Directory path where task JSON files will be stored
    ///
    /// # Example
    /// ```no_run
    /// use a2a_rs::server::FileTaskStore;
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = FileTaskStore::new(PathBuf::from("./tasks")).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(tasks_dir: PathBuf) -> A2AResult<Self> {
        // Create directory if it doesn't exist
        fs::create_dir_all(&tasks_dir).await.map_err(|e| {
            A2AError::InternalError {
                message: format!("Failed to create tasks directory: {}", e),
                data: None,
            }
        })?;

        let store = Self {
            tasks_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            insertion_order: Arc::new(RwLock::new(Vec::new())),
        };

        // Load existing tasks from disk into cache
        store.load_from_disk().await?;

        Ok(store)
    }

    /// Get the path to a task's JSON file.
    fn task_file_path(&self, task_id: &str) -> PathBuf {
        self.tasks_dir.join(format!("{}.json", task_id))
    }

    /// Load all existing tasks from disk into the cache.
    async fn load_from_disk(&self) -> A2AResult<()> {
        let mut entries = fs::read_dir(&self.tasks_dir).await.map_err(|e| {
            A2AError::InternalError {
                message: format!("Failed to read tasks directory: {}", e),
                data: None,
            }
        })?;

        let mut cache = self.cache.write().await;
        let mut order = self.insertion_order.write().await;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            
            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match fs::read_to_string(&path).await {
                Ok(contents) => match serde_json::from_str::<Task>(&contents) {
                    Ok(task) => {
                        let task_id = task.id.clone();
                        if !cache.contains_key(&task_id) {
                            order.push(task_id.clone());
                        }
                        cache.insert(task_id, task);
                    }
                    Err(e) => {
                        warn!(path = ?path, error = %e, "Failed to parse task JSON file");
                    }
                },
                Err(e) => {
                    warn!(path = ?path, error = %e, "Failed to read task file");
                }
            }
        }

        debug!(count = cache.len(), "Loaded tasks from disk");
        Ok(())
    }

    /// Write a task to disk as JSON.
    async fn write_to_disk(&self, task: &Task) -> A2AResult<()> {
        let path = self.task_file_path(&task.id);
        let json = serde_json::to_string_pretty(task).map_err(|e| {
            A2AError::InternalError {
                message: format!("Failed to serialize task: {}", e),
                data: None,
            }
        })?;

        fs::write(&path, json).await.map_err(|e| {
            A2AError::InternalError {
                message: format!("Failed to write task file: {}", e),
                data: None,
            }
        })?;

        debug!(task_id = %task.id, path = ?path, "Task written to disk");
        Ok(())
    }

    /// Delete a task file from disk.
    async fn delete_from_disk(&self, task_id: &str) -> A2AResult<()> {
        let path = self.task_file_path(task_id);
        
        match fs::remove_file(&path).await {
            Ok(_) => {
                debug!(task_id = %task_id, path = ?path, "Task file deleted");
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, that's fine
                Ok(())
            }
            Err(e) => Err(A2AError::InternalError {
                message: format!("Failed to delete task file: {}", e),
                data: None,
            }),
        }
    }
}

#[async_trait]
impl TaskStore for FileTaskStore {
    async fn save(&self, task: Task) -> A2AResult<()> {
        let task_id = task.id.clone();
        
        // Write to disk first
        self.write_to_disk(&task).await?;

        // Update cache
        let mut cache = self.cache.write().await;
        let is_new = !cache.contains_key(&task_id);
        cache.insert(task_id.clone(), task);

        if is_new {
            let mut order = self.insertion_order.write().await;
            order.push(task_id.clone());
        }

        debug!(task_id = %task_id, is_new = is_new, "Task saved");
        Ok(())
    }

    async fn get(&self, task_id: &str) -> A2AResult<Option<Task>> {
        let cache = self.cache.read().await;
        let task = cache.get(task_id).cloned();
        debug!(task_id = %task_id, found = task.is_some(), "Task lookup");
        Ok(task)
    }

    async fn delete(&self, task_id: &str) -> A2AResult<()> {
        // Delete from disk first
        self.delete_from_disk(task_id).await?;

        // Update cache
        let mut cache = self.cache.write().await;
        if cache.remove(task_id).is_some() {
            let mut order = self.insertion_order.write().await;
            order.retain(|id| id != task_id);
            debug!(task_id = %task_id, "Task deleted");
        } else {
            warn!(task_id = %task_id, "Attempted to delete non-existent task");
        }
        Ok(())
    }

    async fn list(&self, params: &TaskListParams) -> A2AResult<TaskListResponse> {
        let cache = self.cache.read().await;
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

            if let Some(task) = cache.get(id) {
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

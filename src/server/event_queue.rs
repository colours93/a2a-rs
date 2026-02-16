//! Event queue — broadcast channel for streaming A2A events.
//!
//! Mirrors Python SDK's `EventQueue` from `a2a.server.events.event_queue`.
//!
//! The event queue connects agent executors (producers) to request handlers
//! (consumers). Agents publish [`StreamResponse`] events, and the server
//! framework delivers them to SSE streams or collects them for synchronous
//! responses.
//!
//! Also provides [`QueueManager`] trait and [`InMemoryQueueManager`] for
//! managing per-task event queues (mirrors Python SDK's `QueueManager` and
//! `InMemoryQueueManager`), and [`EventConsumer`] for consuming events from
//! a queue (mirrors Python SDK's `EventConsumer`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

use crate::error::{A2AError, A2AResult};
use crate::types::{StreamResponse, TaskState};

/// Default channel capacity for the event queue.
const DEFAULT_CAPACITY: usize = 1024;

/// Event queue for publishing and subscribing to A2A streaming events.
///
/// Built on top of a `tokio::sync::broadcast` channel, allowing multiple
/// consumers to independently receive events from a single producer.
///
/// Mirrors Python SDK's `EventQueue` with close semantics and child queue
/// (tap) support.
///
/// # Usage
///
/// ```rust,ignore
/// let queue = EventQueue::new(256);
/// let mut rx = queue.subscribe();
///
/// // In agent executor:
/// queue.publish(event)?;
///
/// // In request handler / SSE stream:
/// while let Ok(event) = rx.recv().await {
///     // process event
/// }
///
/// // Close the queue when done:
/// queue.close();
/// ```
#[derive(Debug, Clone)]
pub struct EventQueue {
    tx: broadcast::Sender<StreamResponse>,
    closed: Arc<AtomicBool>,
    children: Arc<Mutex<Vec<EventQueue>>>,
}

impl EventQueue {
    /// Create a new event queue with the given channel capacity.
    ///
    /// The capacity determines how many events can be buffered before
    /// slow consumers start missing events (receiving `RecvError::Lagged`).
    ///
    /// Mirrors Python SDK's `EventQueue.__init__(max_queue_size)`.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be greater than 0");
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            tx,
            closed: Arc::new(AtomicBool::new(false)),
            children: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new event queue with the default capacity (1024).
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// Subscribe to events on this queue.
    ///
    /// Returns a receiver that will receive all events published after
    /// this subscription was created. Multiple subscribers can exist
    /// simultaneously, each receiving an independent copy of events.
    pub fn subscribe(&self) -> broadcast::Receiver<StreamResponse> {
        self.tx.subscribe()
    }

    /// Publish an event to all subscribers and child queues.
    ///
    /// If the queue is closed, the event is silently dropped (matching
    /// Python SDK's behavior where closed queues log a warning and return).
    ///
    /// Mirrors Python SDK's `EventQueue.enqueue_event(event)`.
    pub async fn enqueue_event(&self, event: StreamResponse) -> A2AResult<()> {
        if self.closed.load(Ordering::Acquire) {
            warn!("Queue is closed. Event will not be enqueued.");
            return Ok(());
        }

        debug!("Enqueuing event to queue");

        match self.tx.send(event.clone()) {
            Ok(count) => {
                debug!(subscriber_count = count, "Published event to queue");
            }
            Err(_) => {
                warn!("Failed to publish event (no subscribers)");
                // Not fatal — subscriber may have disconnected.
            }
        }

        // Forward to child queues (mirrors Python SDK's child forwarding).
        let children = self.children.lock().await;
        for child in children.iter() {
            // Box::pin to allow recursion in async.
            Box::pin(child.enqueue_event(event.clone())).await?;
        }

        Ok(())
    }

    /// Publish an event to all subscribers (sync version, no child forwarding).
    ///
    /// This is a simpler API for when you don't need child queue support.
    /// Prefer [`enqueue_event`](Self::enqueue_event) for full Python SDK parity.
    pub fn publish(&self, event: StreamResponse) -> A2AResult<()> {
        if self.closed.load(Ordering::Acquire) {
            warn!("Queue is closed. Event will not be published.");
            return Ok(());
        }

        match self.tx.send(event) {
            Ok(count) => {
                debug!(subscriber_count = count, "Published event to queue");
                Ok(())
            }
            Err(_) => {
                warn!("Failed to publish event (no subscribers)");
                Ok(())
            }
        }
    }

    /// Returns the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Create a child queue that receives all future events from this queue.
    ///
    /// Mirrors Python SDK's `EventQueue.tap()`. The child queue will receive
    /// all events enqueued to this parent queue from this point forward
    /// (via the `enqueue_event` method's child forwarding).
    ///
    /// Returns a new `EventQueue` instance.
    pub async fn tap(&self) -> EventQueue {
        debug!("Tapping EventQueue to create a child queue.");
        let child = EventQueue::with_default_capacity();
        let mut children = self.children.lock().await;
        children.push(child.clone());
        child
    }

    /// Close the queue, preventing future events from being enqueued.
    ///
    /// Also closes all child queues. Once closed, `enqueue_event` and
    /// `publish` will silently drop events.
    ///
    /// Mirrors Python SDK's `EventQueue.close()`.
    pub async fn close(&self) {
        debug!("Closing EventQueue.");
        self.closed.store(true, Ordering::Release);

        // Close all children.
        let children = self.children.lock().await;
        for child in children.iter() {
            Box::pin(child.close()).await;
        }
    }

    /// Check if the queue has been closed.
    ///
    /// Mirrors Python SDK's `EventQueue.is_closed()`.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

// ---------------------------------------------------------------------------
// QueueManager — per-task event queue management
// ---------------------------------------------------------------------------

/// Error raised when attempting to add a queue for a task ID that already exists.
///
/// Mirrors Python SDK's `TaskQueueExists`.
#[derive(Debug, Clone, thiserror::Error)]
#[error("Queue already exists for this task")]
pub struct TaskQueueExists;

/// Error raised when accessing/closing a queue for a task ID that does not exist.
///
/// Mirrors Python SDK's `NoTaskQueue`.
#[derive(Debug, Clone, thiserror::Error)]
#[error("No queue exists for this task")]
pub struct NoTaskQueue;

/// Interface for managing per-task event queue lifecycles.
///
/// Mirrors Python SDK's `QueueManager(ABC)` from
/// `a2a.server.events.queue_manager`.
#[async_trait]
pub trait QueueManager: Send + Sync {
    /// Add a new event queue for a task ID.
    ///
    /// Returns `Err(TaskQueueExists)` if a queue already exists for this task.
    async fn add(&self, task_id: &str, queue: EventQueue) -> Result<(), TaskQueueExists>;

    /// Retrieve the event queue for a task ID.
    ///
    /// Returns `None` if no queue exists.
    async fn get(&self, task_id: &str) -> Option<EventQueue>;

    /// Create a child (tap) of the event queue for a task ID.
    ///
    /// Returns `None` if no queue exists for the task.
    async fn tap(&self, task_id: &str) -> Option<EventQueue>;

    /// Close and remove the event queue for a task ID.
    ///
    /// Returns `Err(NoTaskQueue)` if no queue exists.
    async fn close(&self, task_id: &str) -> Result<(), NoTaskQueue>;

    /// Create a new queue if one doesn't exist, otherwise tap the existing one.
    ///
    /// Returns the new or child `EventQueue`.
    async fn create_or_tap(&self, task_id: &str) -> EventQueue;
}

/// In-memory implementation of [`QueueManager`].
///
/// Suitable for single-instance deployments. All incoming interactions for a
/// given task ID must hit the same process.
///
/// Mirrors Python SDK's `InMemoryQueueManager` from
/// `a2a.server.events.in_memory_queue_manager`.
pub struct InMemoryQueueManager {
    queues: Mutex<HashMap<String, EventQueue>>,
}

impl InMemoryQueueManager {
    /// Create a new empty queue manager.
    pub fn new() -> Self {
        Self {
            queues: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueueManager for InMemoryQueueManager {
    async fn add(&self, task_id: &str, queue: EventQueue) -> Result<(), TaskQueueExists> {
        let mut queues = self.queues.lock().await;
        if queues.contains_key(task_id) {
            return Err(TaskQueueExists);
        }
        queues.insert(task_id.to_string(), queue);
        Ok(())
    }

    async fn get(&self, task_id: &str) -> Option<EventQueue> {
        let queues = self.queues.lock().await;
        queues.get(task_id).cloned()
    }

    async fn tap(&self, task_id: &str) -> Option<EventQueue> {
        let queues = self.queues.lock().await;
        if let Some(queue) = queues.get(task_id) {
            Some(queue.tap().await)
        } else {
            None
        }
    }

    async fn close(&self, task_id: &str) -> Result<(), NoTaskQueue> {
        let mut queues = self.queues.lock().await;
        if let Some(queue) = queues.remove(task_id) {
            queue.close().await;
            Ok(())
        } else {
            Err(NoTaskQueue)
        }
    }

    async fn create_or_tap(&self, task_id: &str) -> EventQueue {
        let mut queues = self.queues.lock().await;
        if let Some(existing) = queues.get(task_id) {
            existing.tap().await
        } else {
            let queue = EventQueue::with_default_capacity();
            queues.insert(task_id.to_string(), queue.clone());
            queue
        }
    }
}

// ---------------------------------------------------------------------------
// EventConsumer — consumes events from the queue
// ---------------------------------------------------------------------------

/// Consumer that reads events from an agent's event queue.
///
/// Mirrors Python SDK's `EventConsumer` from
/// `a2a.server.events.event_consumer`.
///
/// Provides `consume_one` for non-streaming responses and `consume_all`
/// for streaming (SSE) responses. The consumer handles final-event detection,
/// timeout-based polling, and exception propagation from the agent task.
pub struct EventConsumer {
    rx: broadcast::Receiver<StreamResponse>,
    queue: EventQueue,
    timeout: Duration,
    /// If the agent task sets an error, it's stored here for re-raising.
    exception: Arc<Mutex<Option<A2AError>>>,
}

impl EventConsumer {
    /// Create a new event consumer for the given queue.
    ///
    /// Mirrors Python SDK's `EventConsumer.__init__(queue)`.
    pub fn new(queue: EventQueue) -> Self {
        let rx = queue.subscribe();
        Self {
            rx,
            queue,
            timeout: Duration::from_millis(500),
            exception: Arc::new(Mutex::new(None)),
        }
    }

    /// Consume one event from the queue (non-blocking).
    ///
    /// Returns an error if no event is immediately available.
    ///
    /// Mirrors Python SDK's `EventConsumer.consume_one()`.
    pub async fn consume_one(&mut self) -> A2AResult<StreamResponse> {
        debug!("Attempting to consume one event.");
        match self.rx.try_recv() {
            Ok(event) => {
                debug!("Consumed one event.");
                Ok(event)
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                warn!("Event queue was empty in consume_one.");
                Err(A2AError::InternalError {
                    message: "Agent did not return any response".to_string(),
                    data: None,
                })
            }
            Err(broadcast::error::TryRecvError::Closed) => Err(A2AError::InternalError {
                message: "Event queue closed before producing a response".to_string(),
                data: None,
            }),
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                warn!(missed = n, "Consumer lagged in consume_one");
                Err(A2AError::InternalError {
                    message: format!("Consumer lagged, missed {} events", n),
                    data: None,
                })
            }
        }
    }

    /// Consume all events from the queue until a final event is received.
    ///
    /// Yields events as they become available. Detects final events
    /// (terminal `TaskStatusUpdateEvent`, `Message`, or terminal `Task`)
    /// and closes the queue after the final event.
    ///
    /// Also checks for exceptions set via `set_exception` (from the agent
    /// task callback).
    ///
    /// Mirrors Python SDK's `EventConsumer.consume_all()`.
    pub async fn consume_all(&mut self) -> Vec<StreamResponse> {
        debug!("Starting to consume all events from the queue.");
        let mut events = Vec::new();

        loop {
            // Check for agent exception.
            {
                let exc = self.exception.lock().await;
                if let Some(ref e) = *exc {
                    warn!("Agent exception detected: {}", e);
                    break;
                }
            }

            // Use timeout to allow periodic exception checking (mirrors Python).
            match tokio::time::timeout(self.timeout, self.rx.recv()).await {
                Ok(Ok(event)) => {
                    debug!("Dequeued event in consume_all.");

                    let is_final = Self::is_final_event(&event);

                    if is_final {
                        debug!("Stopping event consumption in consume_all.");
                        self.queue.close().await;
                        events.push(event);
                        break;
                    }

                    events.push(event);
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    // Queue closed — confirm and break.
                    if self.queue.is_closed() {
                        break;
                    }
                    // Channel dropped but queue not explicitly closed.
                    break;
                }
                Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                    warn!(missed = n, "Event consumer lagged");
                    continue;
                }
                Err(_timeout) => {
                    // Timeout — continue polling (allows exception check).
                    continue;
                }
            }
        }

        events
    }

    /// Consume all events as an async stream (yields one at a time).
    ///
    /// This is the streaming equivalent of `consume_all`, suitable for SSE.
    /// Returns `None` when the stream is finished.
    pub async fn next_event(&mut self) -> Option<StreamResponse> {
        loop {
            // Check for agent exception.
            {
                let exc = self.exception.lock().await;
                if exc.is_some() {
                    return None;
                }
            }

            match tokio::time::timeout(self.timeout, self.rx.recv()).await {
                Ok(Ok(event)) => {
                    let is_final = Self::is_final_event(&event);

                    if is_final {
                        self.queue.close().await;
                    }

                    return Some(event);
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    return None;
                }
                Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                    warn!(missed = n, "Event consumer lagged");
                    continue;
                }
                Err(_timeout) => {
                    continue;
                }
            }
        }
    }

    /// Set an exception from the agent's execution task.
    ///
    /// This is called from an agent task callback when the agent errors.
    /// The consumer loop will detect this and stop consuming.
    ///
    /// Mirrors Python SDK's `EventConsumer.agent_task_callback(agent_task)`.
    pub async fn set_exception(&self, error: A2AError) {
        let mut exc = self.exception.lock().await;
        *exc = Some(error);
    }

    /// Get a clone of the exception handle for use in task callbacks.
    ///
    /// This allows external code (like a spawned agent task) to set the
    /// exception without holding a mutable reference to the consumer.
    pub fn exception_handle(&self) -> Arc<Mutex<Option<A2AError>>> {
        Arc::clone(&self.exception)
    }

    /// Check if an event is a final event (should stop consumption).
    ///
    /// Mirrors Python SDK's `is_final_event` logic in `consume_all`.
    fn is_final_event(event: &StreamResponse) -> bool {
        match event {
            StreamResponse::StatusUpdate(update) => update.r#final,
            StreamResponse::Message(_) => true,
            StreamResponse::Task(task) => matches!(
                task.status.state,
                TaskState::Completed
                    | TaskState::Canceled
                    | TaskState::Failed
                    | TaskState::Rejected
                    | TaskState::Unknown
                    | TaskState::InputRequired
            ),
            StreamResponse::ArtifactUpdate(_) => false,
        }
    }
}

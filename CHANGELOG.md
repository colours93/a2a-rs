# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-11

### Added

#### Core Protocol Support
- Complete implementation of A2A protocol v0.3 specification
- All types matching the official protobuf schema at `a2a.proto`
- JSON-RPC 2.0 request/response serialization with camelCase field names
- Full support for all A2A JSON-RPC methods:
  - `message/send` — Send a message and receive a task
  - `message/stream` — Send a message with SSE streaming
  - `tasks/get` — Retrieve task by ID with optional history
  - `tasks/list` — List tasks with filtering by context/status
  - `tasks/cancel` — Cancel running tasks
  - `tasks/subscribe` — Subscribe to task update events via SSE

#### Client Features
- `A2AClient` — High-level client for calling A2A agents
- `CardResolver` — Automatic agent card discovery from `/.well-known/agent.json`
- `JsonRpcTransport` — HTTP transport with JSON-RPC 2.0 encoding
- `SseStream` — Server-Sent Events stream parser for real-time updates
- Convenience methods:
  - `send_text()` — Send simple text messages
  - `send_text_stream()` — Send text with streaming
  - `send_text_in_context()` — Multi-turn conversations with context tracking
  - `send_text_with_config()` — Configure output modes and blocking behavior
- Support for custom transport implementations via `Transport` trait

#### Server Features
- `AgentExecutor` trait — Define agent logic with `execute()` and `cancel()` methods
- `TaskStore` trait — Pluggable task persistence layer
- `InMemoryTaskStore` — Reference implementation for task storage
- `DefaultRequestHandler` — Complete JSON-RPC 2.0 request handling
- `TaskUpdater` — Thread-safe helper for publishing status/artifact updates
- `EventQueue` — Broadcast channel for SSE event streaming
- `a2a_router()` — Ready-made axum routes:
  - `POST /a2a` — JSON-RPC 2.0 dispatch endpoint
  - `GET /.well-known/agent.json` — Agent card discovery endpoint
- Full SSE streaming support with automatic event serialization

#### Type System
- Complete type definitions for:
  - `Task` / `A2ATask` — Task lifecycle with status, history, artifacts
  - `Message` — Messages with text/file/data parts
  - `Part` — Content parts (text, file, structured data)
  - `TaskState` — State machine (submitted → working → completed/failed/canceled)
  - `StreamResponse` — SSE event types (status updates, artifact updates)
  - `AgentCard` — Agent metadata and capabilities
  - `AgentInterface` — Protocol binding configuration
  - `AgentSkill` — Agent skill definitions
  - `SecurityScheme` — OAuth2, API key, HTTP auth support
- Ergonomic builder patterns:
  - `AgentCardBuilder` — Construct agent cards with sensible defaults
  - `ClientBuilder` — Configure client with custom transport/timeout
  - `ServerBuilder` — Fluent API for server configuration

#### Error Handling
- `A2AError` — Unified error type with all A2A error codes:
  - `-32700` — Parse error (invalid JSON)
  - `-32600` — Invalid request
  - `-32601` — Method not found
  - `-32602` — Invalid params
  - `-32603` — Internal error
  - `-32001` — Task not found
  - `-32002` — Task not cancelable
  - `-32003` — Push notification not supported
  - `-32004` — Unsupported operation
  - `-32005` — Content type not supported
  - `-32006` — Invalid agent response
  - `-32007` — Authenticated extended card not configured
- Transport error variants for HTTP failures, timeouts, invalid JSON
- Automatic mapping from `A2AError` to `JsonRpcError` for server responses

#### Feature Flags
- `client` (default) — HTTP client for calling A2A agents
- `server` (default) — Server traits and axum integration
- `full` — Enable all features

#### Documentation
- Comprehensive crate-level documentation with quick start guides
- Module-level docs for client, server, types, error
- Doc comments on all public types and methods
- `prelude` module for easy imports of common types
- README with architecture overview and usage examples
- Examples directory (structure defined, awaiting implementation):
  - `echo_agent.rs` — Minimal server example
  - `hello_client.rs` — Simple client example
  - `streaming_client.rs` — SSE streaming example
  - `multi_turn.rs` — Multi-turn conversation example

### Implementation Notes
- Built on tokio async runtime with full `async/await` support
- Uses `axum` 0.8 for HTTP server framework
- Uses `reqwest` 0.12 for HTTP client with SSE support
- Uses `serde`/`serde_json` for zero-copy JSON serialization
- Uses `thiserror` for ergonomic error handling
- Minimum Rust version: 1.70

### Protocol Compliance
- Matches A2A v0.3 protobuf specification byte-for-byte
- Field naming follows JSON-RPC camelCase convention (matches Python/JS SDKs)
- Task state transitions validated per specification
- SSE event format matches reference implementation
- Agent card schema validated against official examples

[Unreleased]: https://github.com/colours93/a2a-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/colours93/a2a-rs/releases/tag/v0.1.0

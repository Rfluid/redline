//! redline: inline review comments between artifacts and coding-agent sessions.
//!
//! The binary (`src/main.rs`) is a thin wrapper over [`cli::run`]. Modules:
//! - [`daemon`]: local HTTP/WebSocket broker (`redline serve`).
//! - [`store`]: `SQLite` persistence (Diesel).
//! - [`mcp`]: MCP server on stdio (`redline mcp`).
//! - [`api`]: blocking agent-side client of the daemon; starts it on demand.
//! - [`lifecycle`]: start, replace (after an update) and stop the daemon.
//! - [`strip`]: remove embed snippets before shipping.
//! - [`install`]: per-agent skill links and MCP registration.
//! - [`update`]: release checks, update notices and `redline update`.

pub mod api;
pub mod cli;
pub mod daemon;
pub mod ids;
pub mod install;
pub mod lifecycle;
pub mod mcp;
pub mod model;
pub mod paths;
pub mod store;
pub mod strip;
pub mod update;

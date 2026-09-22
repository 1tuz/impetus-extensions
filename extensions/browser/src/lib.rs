//! Impetus Browser extension — Browser Provider Protocol `0.1` wire shapes + MCP tools.
//!
//! Re-implements documented protocol semantics as serde types. Does **not** depend on
//! `impetus-core`. See Impetus `docs/reference/browser-provider-protocol.md`.

pub mod backend;
pub mod protocol;
pub mod tools;

pub use backend::{Backend, CdpBackend, MockBackend, SharedBackend};
pub use protocol::*;
pub use tools::{build_handler, mock_shared, tool_defs};

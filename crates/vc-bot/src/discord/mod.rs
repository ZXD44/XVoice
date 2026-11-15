//! Discord module facade.
//!
//! Modular architecture:
//! - `rest`: REST API client (API v10)
//! - `bridge`: Bridge runtime state, caching, account linking
//! - `worker`: Background workers (command, dry-run, apply_layout)
//! - `handler`: Serenity gateway event handler

pub mod rest;
pub mod bridge;
pub mod worker;
pub mod handler;

pub use bridge::Bridge;
pub use handler::client;
pub use worker::{command_worker, dry_run_worker};

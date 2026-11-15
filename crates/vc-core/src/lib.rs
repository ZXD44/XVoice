//! ZirconX VoiceChat core: proximity graph + Discord room layout.
//!
//! Deliberately free of I/O so it can be unit-tested and reasoned about
//! without a Discord token.

pub mod engine;
pub mod model;
pub mod planner;

pub use engine::{Engine, EngineConfig, RadiusMode};
pub use model::{
    normalize, Group, PairKey, PlayerSample, Snapshot, Update, DEFAULT_RADIUS, MAX_RADIUS,
    MIN_RADIUS,
};
pub use planner::{DesiredRoom, Layout, PlannerConfig, RoomPlanner, Slot};

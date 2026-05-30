//! Cross-room fleet coordination for the PLATO nervous system.
//!
//! When room A detects an anomaly and room B detects something related,
//! the fleet coordinator figures out if they're connected and how to respond.

mod correlation;
mod engine;
mod fleet;
mod types;

pub use correlation::{CorrelationMatrix, CorrelationStrength};
pub use engine::{
    Comparison, CoordinationConfig, CoordinationDecision, CoordinationEngine, Correlation,
    CorrelationReason, CorrelationRule, RuleCondition,
};
pub use fleet::{FleetState, RoomHealth};
pub use types::{FleetEvent, RoomId, Tile};

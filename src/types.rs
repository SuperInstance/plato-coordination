use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a room in the fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

/// A sensor tile / measurement category.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    Temperature,
    Smoke,
    Humidity,
    Pressure,
    Vibration,
    AirQuality,
    Power,
    Custom(String),
}

/// An event emitted from a room sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetEvent {
    pub room: RoomId,
    pub tile: Tile,
    pub value: f64,
    pub confidence: f64,
    pub timestamp: u64, // epoch seconds
}

impl FleetEvent {
    pub fn new(room: RoomId, tile: Tile, value: f64, confidence: f64, timestamp: u64) -> Self {
        Self {
            room,
            tile,
            value,
            confidence,
            timestamp,
        }
    }
}

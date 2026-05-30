use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::correlation::CorrelationMatrix;
use crate::types::{FleetEvent, RoomId, Tile};

/// Health score and status for a single room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomHealth {
    pub room: RoomId,
    /// 0.0 (critical) to 1.0 (perfect)
    pub health_score: f64,
    /// Currently active alert tiles.
    pub active_alerts: Vec<Tile>,
    /// Last known sensor readings by tile.
    pub last_readings: HashMap<String, f64>,
    /// Timestamp of last event.
    pub last_event_time: Option<u64>,
}

impl RoomHealth {
    pub fn new(room: RoomId) -> Self {
        Self {
            room,
            health_score: 1.0,
            active_alerts: Vec::new(),
            last_readings: HashMap::new(),
            last_event_time: None,
        }
    }

    pub fn update_with_event(&mut self, event: &FleetEvent) {
        self.last_event_time = Some(event.timestamp);
        let tile_key = format!("{:?}", event.tile);
        self.last_readings.insert(tile_key, event.value);

        if event.confidence > 0.7 {
            self.health_score = (self.health_score - 0.05 * event.confidence).max(0.0);
            if !self.active_alerts.contains(&event.tile) {
                self.active_alerts.push(event.tile.clone());
            }
        }
    }
}

/// Current state of all rooms in the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetState {
    pub(crate) rooms: HashMap<RoomId, RoomHealth>,
}

impl FleetState {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    /// Register a room in the fleet.
    pub fn register_room(&mut self, room: RoomId) {
        self.rooms
            .entry(room)
            .or_insert_with(|| RoomHealth::new(room));
    }

    /// Update room state with a new event.
    pub fn update(&mut self, event: &FleetEvent) {
        self.rooms
            .entry(event.room)
            .or_insert_with(|| RoomHealth::new(event.room))
            .update_with_event(event);
    }

    /// Get a room's health info.
    pub fn get_room(&self, room: &RoomId) -> Option<&RoomHealth> {
        self.rooms.get(room)
    }

    /// Get all room IDs.
    pub fn room_ids(&self) -> Vec<RoomId> {
        self.rooms.keys().copied().collect()
    }

    /// Number of registered rooms.
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Check for correlated events in other rooms given a new event.
    pub fn check_correlations(
        &self,
        event: &FleetEvent,
        matrix: &CorrelationMatrix,
        temporal_window_secs: u64,
    ) -> Vec<crate::engine::Correlation> {
        let mut results = Vec::new();

        for (room_id, health) in &self.rooms {
            if *room_id == event.room {
                continue;
            }

            // Temporal: events within N seconds in different rooms
            if let Some(last_time) = health.last_event_time {
                if event.timestamp.abs_diff(last_time) <= temporal_window_secs
                    && !health.active_alerts.is_empty()
                {
                    results.push(crate::engine::Correlation::temporal(
                        *room_id,
                        event.tile.clone(),
                        temporal_window_secs as f64,
                    ));
                }
            }

            // Statistical: rooms that historically correlate above threshold
            let score = matrix.get_correlation(event.room, *room_id);
            if score >= 0.5 {
                results.push(crate::engine::Correlation::statistical(*room_id, score));
            }
        }

        results
    }
}

impl Default for FleetState {
    fn default() -> Self {
        Self::new()
    }
}

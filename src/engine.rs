use serde::{Deserialize, Serialize};

use crate::correlation::CorrelationMatrix;
use crate::fleet::FleetState;
use crate::types::{FleetEvent, RoomId, Tile};

/// A detected correlation with another room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correlation {
    pub room: RoomId,
    pub reason: CorrelationReason,
    pub strength: f64,
}

impl Correlation {
    pub fn temporal(room: RoomId, tile: Tile, window_secs: f64) -> Self {
        Self {
            room,
            reason: CorrelationReason::Temporal { tile, window_secs },
            strength: 0.6,
        }
    }

    pub fn statistical(room: RoomId, score: f64) -> Self {
        Self {
            room,
            reason: CorrelationReason::Statistical { score },
            strength: score,
        }
    }

    pub fn rule_based(room: RoomId, rule_name: String, confidence: f64) -> Self {
        Self {
            room,
            reason: CorrelationReason::RuleBased {
                rule_name,
                confidence,
            },
            strength: confidence,
        }
    }

    pub fn spatial(room: RoomId, distance: f64) -> Self {
        Self {
            room,
            reason: CorrelationReason::Spatial { distance },
            strength: 1.0 / (1.0 + distance),
        }
    }
}

/// Why two rooms' events are considered correlated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrelationReason {
    Temporal { tile: Tile, window_secs: f64 },
    Statistical { score: f64 },
    RuleBased { rule_name: String, confidence: f64 },
    Spatial { distance: f64 },
}

/// Coordination decision: are events related or independent?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationDecision {
    Related {
        rooms: Vec<RoomId>,
        correlations: Vec<Correlation>,
        action: String,
    },
    Independent,
}

/// Configuration for the coordination engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationConfig {
    /// Minimum correlation score to consider rooms related.
    pub correlation_threshold: f64,
    /// Maximum rooms to include in a single coordinated response.
    pub max_rooms: usize,
    /// Cooldown period in seconds between similar alerts.
    pub cooldown_period_secs: u64,
    /// Temporal window for correlating events.
    pub temporal_window_secs: u64,
    /// Physically adjacent rooms (pairs).
    pub adjacent_rooms: Vec<(RoomId, RoomId)>,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            correlation_threshold: 0.5,
            max_rooms: 10,
            cooldown_period_secs: 300,
            temporal_window_secs: 30,
            adjacent_rooms: Vec::new(),
        }
    }
}

impl CoordinationConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.correlation_threshold < 0.0 || self.correlation_threshold > 1.0 {
            return Err("correlation_threshold must be between 0.0 and 1.0".into());
        }
        if self.max_rooms == 0 {
            return Err("max_rooms must be at least 1".into());
        }
        Ok(())
    }
}

/// A manual correlation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationRule {
    pub name: String,
    pub conditions: Vec<RuleCondition>,
    pub action: String,
    pub confidence: f64,
}

impl CorrelationRule {
    pub fn new(name: impl Into<String>, action: impl Into<String>, confidence: f64) -> Self {
        Self {
            name: name.into(),
            conditions: Vec::new(),
            action: action.into(),
            confidence,
        }
    }

    pub fn when(mut self, condition: RuleCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn matches(&self, fleet_state: &FleetState, trigger_event: &FleetEvent) -> bool {
        for cond in &self.conditions {
            if !cond.matches(fleet_state, trigger_event) {
                return false;
            }
        }
        !self.conditions.is_empty()
    }
}

/// A single condition in a correlation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub tile: Tile,
    pub comparison: Comparison,
    pub threshold: f64,
    pub room: Option<RoomId>,
}

impl RuleCondition {
    pub fn new(tile: Tile, comparison: Comparison, threshold: f64) -> Self {
        Self {
            tile,
            comparison,
            threshold,
            room: None,
        }
    }

    pub fn in_room(mut self, room: RoomId) -> Self {
        self.room = Some(room);
        self
    }

    pub fn matches(&self, fleet_state: &FleetState, trigger_event: &FleetEvent) -> bool {
        if self.tile == trigger_event.tile && self.room.map_or(true, |r| r == trigger_event.room) {
            if self.comparison.compare(trigger_event.value, self.threshold) {
                return true;
            }
        }

        for (_, health) in &fleet_state.rooms {
            if let Some(room_id) = self.room {
                if health.room != room_id {
                    continue;
                }
            }
            let tile_key = format!("{:?}", self.tile);
            if let Some(&value) = health.last_readings.get(&tile_key) {
                if self.comparison.compare(value, self.threshold) {
                    return true;
                }
            }
        }
        false
    }
}

/// Comparison operators for rule conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    GreaterThan,
    LessThan,
    Equal,
    GreaterOrEqual,
    LessOrEqual,
}

impl Comparison {
    pub fn compare(&self, value: f64, threshold: f64) -> bool {
        match self {
            Comparison::GreaterThan => value > threshold,
            Comparison::LessThan => value < threshold,
            Comparison::Equal => (value - threshold).abs() < f64::EPSILON,
            Comparison::GreaterOrEqual => value >= threshold,
            Comparison::LessOrEqual => value <= threshold,
        }
    }
}

/// The coordination engine — main entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationEngine {
    pub config: CoordinationConfig,
    pub matrix: CorrelationMatrix,
    pub rules: Vec<CorrelationRule>,
}

impl CoordinationEngine {
    pub fn new(config: CoordinationConfig) -> Self {
        Self {
            config,
            matrix: CorrelationMatrix::new(),
            rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: CorrelationRule) {
        self.rules.push(rule);
    }

    /// Main coordination logic.
    pub fn coordinate(&self, event: &FleetEvent, fleet_state: &FleetState) -> CoordinationDecision {
        let mut correlated_rooms: std::collections::HashMap<RoomId, Correlation> =
            std::collections::HashMap::new();

        // 1. Rule-based correlations
        for rule in &self.rules {
            if rule.matches(fleet_state, event) {
                for (room_id, health) in &fleet_state.rooms {
                    if *room_id == event.room {
                        continue;
                    }
                    for cond in &rule.conditions {
                        if cond.room.map_or(true, |r| r == *room_id) {
                            let tile_key = format!("{:?}", cond.tile);
                            if let Some(&val) = health.last_readings.get(&tile_key) {
                                if cond.comparison.compare(val, cond.threshold) {
                                    correlated_rooms.entry(*room_id).or_insert_with(|| {
                                        Correlation::rule_based(
                                            *room_id,
                                            rule.name.clone(),
                                            rule.confidence,
                                        )
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Temporal + statistical correlations
        let corrs =
            fleet_state.check_correlations(event, &self.matrix, self.config.temporal_window_secs);
        for corr in corrs {
            correlated_rooms
                .entry(corr.room)
                .and_modify(|existing| {
                    if corr.strength > existing.strength {
                        *existing = corr.clone();
                    }
                })
                .or_insert(corr);
        }

        // 3. Spatial correlations (adjacent rooms)
        for (a, b) in &self.config.adjacent_rooms {
            let other = if *a == event.room {
                Some(*b)
            } else if *b == event.room {
                Some(*a)
            } else {
                None
            };
            if let Some(other_room) = other {
                if fleet_state.get_room(&other_room).is_some() {
                    let corr = Correlation::spatial(other_room, 1.0);
                    correlated_rooms.entry(other_room).or_insert(corr);
                }
            }
        }

        // Filter by correlation threshold
        correlated_rooms.retain(|_, corr| corr.strength >= self.config.correlation_threshold);

        if correlated_rooms.is_empty() {
            return CoordinationDecision::Independent;
        }

        let mut rooms_vec: Vec<(RoomId, Correlation)> = correlated_rooms.into_iter().collect();
        rooms_vec.sort_by(|a, b| b.1.strength.partial_cmp(&a.1.strength).unwrap());
        rooms_vec.truncate(self.config.max_rooms);

        let rooms: Vec<RoomId> = rooms_vec.iter().map(|(r, _)| *r).collect();
        let correlations: Vec<Correlation> = rooms_vec.into_iter().map(|(_, c)| c).collect();
        let action = self.determine_action(&rooms, &correlations);

        CoordinationDecision::Related {
            rooms,
            correlations,
            action,
        }
    }

    fn determine_action(&self, rooms: &[RoomId], correlations: &[Correlation]) -> String {
        let strong_count = correlations.iter().filter(|c| c.strength >= 0.8).count();
        if strong_count >= 2 {
            "escalate_fleet_alert".to_string()
        } else if !rooms.is_empty() {
            "cross_room_monitor".to_string()
        } else {
            "log_and_continue".to_string()
        }
    }
}

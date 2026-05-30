use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::types::RoomId;

/// Strength of correlation between two rooms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CorrelationStrength {
    None,
    Weak,
    Moderate,
    Strong,
}

impl CorrelationStrength {
    pub fn from_score(score: f64) -> Self {
        if score < 0.2 {
            CorrelationStrength::None
        } else if score < 0.5 {
            CorrelationStrength::Weak
        } else if score < 0.8 {
            CorrelationStrength::Moderate
        } else {
            CorrelationStrength::Strong
        }
    }
}

/// Tracks which rooms' events tend to correlate (RoomId × RoomId → f64).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    /// Serialized as an array of {a, b, score, observations} entries.
    #[serde(
        serialize_with = "serialize_matrix",
        deserialize_with = "deserialize_matrix"
    )]
    scores: HashMap<(RoomId, RoomId), f64>,
    observations: HashMap<(RoomId, RoomId), u64>,
}

fn ordered_pair(a: RoomId, b: RoomId) -> (RoomId, RoomId) {
    if a.0 <= b.0 {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Serialize, Deserialize)]
struct MatrixEntry {
    a: RoomId,
    b: RoomId,
    score: f64,
    observations: u64,
}

fn serialize_matrix<S: Serializer>(
    scores: &HashMap<(RoomId, RoomId), f64>,
    s: S,
) -> Result<S::Ok, S::Error> {
    let entries: Vec<MatrixEntry> = scores
        .iter()
        .map(|(&(a, b), &score)| MatrixEntry {
            a,
            b,
            score,
            observations: 0, // We'll skip observations in serialization for simplicity
        })
        .collect();
    entries.serialize(s)
}

fn deserialize_matrix<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<HashMap<(RoomId, RoomId), f64>, D::Error> {
    let entries: Vec<MatrixEntry> = Vec::deserialize(d)?;
    let mut map = HashMap::new();
    for entry in entries {
        let key = ordered_pair(entry.a, entry.b);
        map.insert(key, entry.score);
    }
    Ok(map)
}

impl CorrelationMatrix {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            observations: HashMap::new(),
        }
    }

    /// Strengthen/weaken correlation between two rooms based on co-occurring events.
    /// If `correlated` is true, the score moves toward 1.0; otherwise toward 0.0.
    pub fn learn(&mut self, room_a: RoomId, room_b: RoomId, correlated: bool) {
        if room_a == room_b {
            return;
        }
        let key = ordered_pair(room_a, room_b);
        let current = self.scores.entry(key).or_insert(0.5);
        let obs = self.observations.entry(key).or_insert(0);
        *obs += 1;
        // Exponential moving average toward target
        let alpha = 0.1;
        let target = if correlated { 1.0 } else { 0.0 };
        *current = *current * (1.0 - alpha) + target * alpha;
    }

    /// Get the correlation score between two rooms (0.0–1.0).
    pub fn get_correlation(&self, room_a: RoomId, room_b: RoomId) -> f64 {
        if room_a == room_b {
            return 1.0;
        }
        let key = ordered_pair(room_a, room_b);
        self.scores.get(&key).copied().unwrap_or(0.0)
    }

    /// Get the correlation strength category.
    pub fn get_strength(&self, room_a: RoomId, room_b: RoomId) -> CorrelationStrength {
        CorrelationStrength::from_score(self.get_correlation(room_a, room_b))
    }

    /// Get all rooms that have at least `threshold` correlation with the given room.
    pub fn correlated_rooms(&self, room: RoomId, threshold: f64) -> Vec<RoomId> {
        let mut result = Vec::new();
        for &(a, b) in self.scores.keys() {
            let score = self.get_correlation(a, b);
            if score >= threshold {
                if a == room {
                    result.push(b);
                } else if b == room {
                    result.push(a);
                }
            }
        }
        result
    }

    /// Get the number of room pairs being tracked.
    pub fn pair_count(&self) -> usize {
        self.scores.len()
    }
}

impl Default for CorrelationMatrix {
    fn default() -> Self {
        Self::new()
    }
}

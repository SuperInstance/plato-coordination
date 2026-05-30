use plato_coordination::*;
use plato_coordination::{Comparison, CorrelationReason, CorrelationRule, RuleCondition};

fn make_event(room: RoomId, tile: Tile, value: f64, confidence: f64, ts: u64) -> FleetEvent {
    FleetEvent::new(room, tile, value, confidence, ts)
}

// ─── RoomId ───

#[test]
fn room_id_new_is_unique() {
    let a = RoomId::new();
    let b = RoomId::new();
    assert_ne!(a, b);
}

#[test]
fn room_id_default() {
    let id = RoomId::default();
    // default() calls new(), so it generates a random UUID
    assert_ne!(*id.as_uuid(), uuid::Uuid::nil());
}

// ─── FleetState ───

#[test]
fn fleet_state_new_is_empty() {
    let state = FleetState::new();
    assert_eq!(state.room_count(), 0);
}

#[test]
fn fleet_state_register_and_update() {
    let mut state = FleetState::new();
    let room = RoomId::new();
    state.register_room(room);

    assert_eq!(state.room_count(), 1);
    assert!(state.get_room(&room).is_some());

    let event = make_event(room, Tile::Temperature, 98.6, 0.9, 1000);
    state.update(&event);

    let health = state.get_room(&room).unwrap();
    assert_eq!(health.last_event_time, Some(1000));
    assert!(health.health_score < 1.0); // degraded by high-confidence event
}

#[test]
fn fleet_state_auto_registers_on_update() {
    let mut state = FleetState::new();
    let room = RoomId::new();
    let event = make_event(room, Tile::Humidity, 45.0, 0.5, 2000);
    state.update(&event);
    assert_eq!(state.room_count(), 1);
}

#[test]
fn fleet_state_room_ids() {
    let mut state = FleetState::new();
    let r1 = RoomId::new();
    let r2 = RoomId::new();
    state.register_room(r1);
    state.register_room(r2);
    let ids = state.room_ids();
    assert_eq!(ids.len(), 2);
}

// ─── CorrelationMatrix ───

#[test]
fn correlation_matrix_new_is_empty() {
    let m = CorrelationMatrix::new();
    assert_eq!(m.pair_count(), 0);
}

#[test]
fn correlation_matrix_learn_and_get() {
    let mut m = CorrelationMatrix::new();
    let a = RoomId::new();
    let b = RoomId::new();

    // Initial: 0.5 default → after learning correlated, moves toward 1.0
    assert_eq!(m.get_correlation(a, b), 0.0);

    m.learn(a, b, true);
    // After one learn, score = 0.5 * 0.9 + 1.0 * 0.1 = 0.55
    let score = m.get_correlation(a, b);
    assert!(score > 0.5, "score should be > 0.5 after positive learn, got {score}");

    // Symmetric
    assert_eq!(m.get_correlation(a, b), m.get_correlation(b, a));
}

#[test]
fn correlation_matrix_self_correlation_is_one() {
    let m = CorrelationMatrix::new();
    let a = RoomId::new();
    assert_eq!(m.get_correlation(a, a), 1.0);
}

#[test]
fn correlation_matrix_learn_negative() {
    let mut m = CorrelationMatrix::new();
    let a = RoomId::new();
    let b = RoomId::new();

    m.learn(a, b, true);
    m.learn(a, b, false);
    m.learn(a, b, false);
    // Should be trending down
    let score = m.get_correlation(a, b);
    assert!(score < 0.55, "score should decrease after negative learns, got {score}");
}

#[test]
fn correlation_matrix_correlated_rooms() {
    let mut m = CorrelationMatrix::new();
    let a = RoomId::new();
    let b = RoomId::new();
    let c = RoomId::new();

    for _ in 0..50 {
        m.learn(a, b, true);
    }
    let related = m.correlated_rooms(a, 0.5);
    assert!(related.contains(&b));
    assert!(!related.contains(&c));
}

// ─── CorrelationRule ───

#[test]
fn rule_matches_simple_condition() {
    let room = RoomId::new();
    let mut state = FleetState::new();
    state.register_room(room);

    let event = make_event(room, Tile::Temperature, 225.0, 0.9, 1000);
    state.update(&event);

    let rule = CorrelationRule::new("hot_engine", "alert", 0.9)
        .when(RuleCondition::new(Tile::Temperature, Comparison::GreaterThan, 220.0));

    assert!(rule.matches(&state, &event));
}

#[test]
fn rule_no_match_wrong_tile() {
    let room = RoomId::new();
    let state = FleetState::new();
    let event = make_event(room, Tile::Temperature, 225.0, 0.9, 1000);

    let rule = CorrelationRule::new("smoke_rule", "alert", 0.9)
        .when(RuleCondition::new(Tile::Smoke, Comparison::GreaterThan, 0.5));

    assert!(!rule.matches(&state, &event));
}

#[test]
fn rule_no_match_no_conditions() {
    let room = RoomId::new();
    let state = FleetState::new();
    let event = make_event(room, Tile::Temperature, 225.0, 0.9, 1000);

    let rule = CorrelationRule::new("empty", "noop", 0.5);
    assert!(!rule.matches(&state, &event));
}

// ─── CoordinationConfig ───

#[test]
fn config_default_validates() {
    let config = CoordinationConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn config_invalid_threshold() {
    let mut config = CoordinationConfig::default();
    config.correlation_threshold = -0.1;
    assert!(config.validate().is_err());
}

#[test]
fn config_zero_max_rooms() {
    let mut config = CoordinationConfig::default();
    config.max_rooms = 0;
    assert!(config.validate().is_err());
}

// ─── CoordinationEngine ───

#[test]
fn engine_independent_when_single_room() {
    let engine = CoordinationEngine::new(CoordinationConfig::default());
    let room = RoomId::new();
    let mut state = FleetState::new();
    state.register_room(room);

    let event = make_event(room, Tile::Temperature, 200.0, 0.8, 1000);
    let decision = engine.coordinate(&event, &state);
    assert!(matches!(decision, CoordinationDecision::Independent));
}

#[test]
fn engine_detects_temporal_correlation() {
    let config = CoordinationConfig {
        temporal_window_secs: 60,
        correlation_threshold: 0.3,
        ..Default::default()
    };
    let engine = CoordinationEngine::new(config);

    let room_a = RoomId::new();
    let room_b = RoomId::new();
    let mut state = FleetState::new();
    state.register_room(room_a);
    state.register_room(room_b);

    // Room B has recent event
    let event_b = make_event(room_b, Tile::Smoke, 1.0, 0.9, 1000);
    state.update(&event_b);

    // Room A event at same time
    let event_a = make_event(room_a, Tile::Temperature, 250.0, 0.8, 1010);
    let decision = engine.coordinate(&event_a, &state);

    match decision {
        CoordinationDecision::Related { rooms, .. } => {
            assert!(rooms.contains(&room_b));
        }
        CoordinationDecision::Independent => panic!("Expected correlated decision"),
    }
}

#[test]
fn engine_detects_statistical_correlation() {
    let mut engine = CoordinationEngine::new(CoordinationConfig {
        correlation_threshold: 0.3,
        ..Default::default()
    });

    let room_a = RoomId::new();
    let room_b = RoomId::new();

    // Teach strong correlation
    for _ in 0..50 {
        engine.matrix.learn(room_a, room_b, true);
    }

    let mut state = FleetState::new();
    state.register_room(room_a);
    state.register_room(room_b);

    // Room B has recent event with alerts so statistical correlation is detected
    let event_b = make_event(room_b, Tile::Smoke, 1.0, 0.9, 1000);
    state.update(&event_b);

    let event_a = make_event(room_a, Tile::Temperature, 250.0, 0.8, 2000);
    let decision = engine.coordinate(&event_a, &state);

    match decision {
        CoordinationDecision::Related { correlations, .. } => {
            let has_stat = correlations.iter().any(|c| matches!(
                c.reason,
                CorrelationReason::Statistical { .. }
            ));
            assert!(has_stat);
        }
        CoordinationDecision::Independent => panic!("Expected correlated decision"),
    }
}

#[test]
fn engine_detects_spatial_correlation() {
    let room_a = RoomId::new();
    let room_b = RoomId::new();

    let config = CoordinationConfig {
        correlation_threshold: 0.3,
        adjacent_rooms: vec![(room_a, room_b)],
        ..Default::default()
    };
    let engine = CoordinationEngine::new(config);

    let mut state = FleetState::new();
    state.register_room(room_a);
    state.register_room(room_b);

    let event_a = make_event(room_a, Tile::Temperature, 250.0, 0.8, 1000);
    let decision = engine.coordinate(&event_a, &state);

    match decision {
        CoordinationDecision::Related { rooms, correlations, .. } => {
            assert!(rooms.contains(&room_b));
            let has_spatial = correlations.iter().any(|c| matches!(
                c.reason,
                CorrelationReason::Spatial { .. }
            ));
            assert!(has_spatial);
        }
        CoordinationDecision::Independent => panic!("Expected correlated decision"),
    }
}

#[test]
fn engine_rule_based_correlation() {
    let room_a = RoomId::new();
    let room_b = RoomId::new();

    let mut engine = CoordinationEngine::new(CoordinationConfig {
        correlation_threshold: 0.1,
        ..Default::default()
    });

    engine.add_rule(
        CorrelationRule::new("fire_correlated", "escalate", 0.95)
            .when(RuleCondition::new(Tile::Temperature, Comparison::GreaterThan, 220.0))
            .when(RuleCondition::new(Tile::Smoke, Comparison::GreaterThan, 0.5)),
    );

    let mut state = FleetState::new();
    state.register_room(room_a);
    state.register_room(room_b);

    // Room B has smoke
    let event_b = make_event(room_b, Tile::Smoke, 1.0, 0.9, 1000);
    state.update(&event_b);

    // Room A has high temp — triggers rule
    let event_a = make_event(room_a, Tile::Temperature, 225.0, 0.9, 1010);
    let decision = engine.coordinate(&event_a, &state);

    match decision {
        CoordinationDecision::Related { rooms, correlations, .. } => {
            assert!(rooms.contains(&room_b));
            let has_rule = correlations.iter().any(|c| matches!(
                c.reason,
                CorrelationReason::RuleBased { .. }
            ));
            assert!(has_rule);
        }
        CoordinationDecision::Independent => panic!("Expected correlated decision"),
    }
}

#[test]
fn engine_max_rooms_limit() {
    let rooms: Vec<RoomId> = (0..20).map(|_| RoomId::new()).collect();
    let trigger = rooms[0];

    let config = CoordinationConfig {
        correlation_threshold: 0.3,
        max_rooms: 3,
        adjacent_rooms: rooms[1..].iter().map(|r| (trigger, *r)).collect(),
        ..Default::default()
    };
    let engine = CoordinationEngine::new(config);

    let mut state = FleetState::new();
    for r in &rooms {
        state.register_room(*r);
    }

    let event = make_event(trigger, Tile::Temperature, 250.0, 0.8, 1000);
    let decision = engine.coordinate(&event, &state);

    match decision {
        CoordinationDecision::Related { rooms: result_rooms, .. } => {
            assert_eq!(result_rooms.len(), 3);
        }
        CoordinationDecision::Independent => panic!("Expected correlated"),
    }
}

#[test]
fn engine_escalates_with_multiple_strong() {
    let room_a = RoomId::new();
    let room_b = RoomId::new();
    let room_c = RoomId::new();

    let mut engine = CoordinationEngine::new(CoordinationConfig {
        correlation_threshold: 0.3,
        temporal_window_secs: 60,
        ..Default::default()
    });

    for _ in 0..50 {
        engine.matrix.learn(room_a, room_b, true);
        engine.matrix.learn(room_a, room_c, true);
    }

    let mut state = FleetState::new();
    state.register_room(room_a);
    state.register_room(room_b);
    state.register_room(room_c);

    let event_b = make_event(room_b, Tile::Smoke, 1.0, 0.95, 990);
    let event_c = make_event(room_c, Tile::Smoke, 0.8, 0.95, 995);
    state.update(&event_b);
    state.update(&event_c);

    let event = make_event(room_a, Tile::Temperature, 300.0, 0.95, 1000);
    let decision = engine.coordinate(&event, &state);

    match decision {
        CoordinationDecision::Related { action, correlations, .. } => {
            let strong_count = correlations.iter().filter(|c| c.strength >= 0.8).count();
            assert!(strong_count >= 2);
            assert_eq!(action, "escalate_fleet_alert");
        }
        CoordinationDecision::Independent => panic!("Expected correlated"),
    }
}

// ─── Serialization round-trips ───

#[test]
fn fleet_event_roundtrip() {
    let room = RoomId::new();
    let event = make_event(room, Tile::Temperature, 98.6, 0.75, 12345);
    let json = serde_json::to_string(&event).unwrap();
    let back: FleetEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.room, event.room);
    assert_eq!(back.tile, event.tile);
    assert!((back.value - event.value).abs() < f64::EPSILON);
}

#[test]
fn correlation_matrix_roundtrip() {
    let mut m = CorrelationMatrix::new();
    let a = RoomId::new();
    let b = RoomId::new();
    m.learn(a, b, true);
    let score_before = m.get_correlation(a, b);

    // Test that the matrix itself round-trips through Clone
    let cloned = m.clone();
    assert!((cloned.get_correlation(a, b) - score_before).abs() < f64::EPSILON);
    assert_eq!(cloned.pair_count(), m.pair_count());
}

#[test]
fn coordination_decision_roundtrip() {
    let room = RoomId::new();
    let decision = CoordinationDecision::Related {
        rooms: vec![room],
        correlations: vec![],
        action: "test".into(),
    };
    let json = serde_json::to_string(&decision).unwrap();
    let back: CoordinationDecision = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, CoordinationDecision::Related { .. }));
}

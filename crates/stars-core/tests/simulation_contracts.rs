//! PR4 contract acceptance tests: deterministic commits, atomic rejection,
//! canonical state encoding, versioned RNG streams, explicit phase scaffolding
//! and player-filtered views.

use std::collections::BTreeSet;

use stars_core::model::{
    BatchId, Design, DesignId, EntityVisibility, Fleet, FleetId, FleetLocation, GalacticPosition,
    GameState, Planet, PlanetId, Player, PlayerId, PlayerRole, STATE_FORMAT_VERSION,
};
use stars_core::orders::{Command, CommandAction, CommandId, OrderBatch, OrderError};
use stars_core::rng::{RNG_ALGORITHM_VERSION, RngStreamKey, RngSubsystem};
use stars_core::turn::{
    EventKind, EventVisibility, GameEvent, GameSession, Phase, PhaseCapability, PhasePlan,
    PhasePlanSource,
};
use stars_core::view::{PlayerView, ViewError, ViewLocation};

struct Fixture {
    state: GameState,
    human: PlayerId,
    rival: PlayerId,
    home: PlanetId,
    rival_home: PlanetId,
    hidden_planet: PlanetId,
    human_design: DesignId,
    rival_design: DesignId,
    hidden_design: DesignId,
    human_fleet: FleetId,
    rival_fleet: FleetId,
    hidden_fleet: FleetId,
}

fn fixture() -> Fixture {
    let mut state = GameState::new(2400, 0x5eed_1234_abcd_0001);

    let human = state.allocate_player_id().expect("player id");
    state.players.insert(
        human,
        Player {
            id: human,
            name: "Human".to_owned(),
            role: PlayerRole::Human,
            ready: false,
        },
    );
    let rival = state.allocate_player_id().expect("player id");
    state.players.insert(
        rival,
        Player {
            id: rival,
            name: "Turindrones".to_owned(),
            role: PlayerRole::Computer,
            ready: false,
        },
    );

    let home = state.allocate_planet_id().expect("planet id");
    state.planets.insert(
        home,
        Planet {
            id: home,
            name: "Home".to_owned(),
            position: GalacticPosition::new(100, 200),
            owner: Some(human),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let rival_home = state.allocate_planet_id().expect("planet id");
    state.planets.insert(
        rival_home,
        Planet {
            id: rival_home,
            name: "Rival Home".to_owned(),
            position: GalacticPosition::new(-300, 400),
            owner: Some(rival),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let hidden_planet = state.allocate_planet_id().expect("planet id");
    state.planets.insert(
        hidden_planet,
        Planet {
            id: hidden_planet,
            name: "Hidden World".to_owned(),
            position: GalacticPosition::new(900, 900),
            owner: None,
            visibility: EntityVisibility::Hidden,
        },
    );

    let human_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        human_design,
        Design {
            id: human_design,
            owner: human,
            name: "Scout".to_owned(),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let rival_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        rival_design,
        Design {
            id: rival_design,
            owner: rival,
            name: "Raider".to_owned(),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let hidden_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        hidden_design,
        Design {
            id: hidden_design,
            owner: human,
            name: "Secret".to_owned(),
            visibility: EntityVisibility::Hidden,
        },
    );

    let human_fleet = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        human_fleet,
        Fleet {
            id: human_fleet,
            owner: human,
            name: "Alpha".to_owned(),
            design: human_design,
            ships: 3,
            location: FleetLocation::AtPlanet(home),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let rival_fleet = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        rival_fleet,
        Fleet {
            id: rival_fleet,
            owner: rival,
            name: "Beta".to_owned(),
            design: rival_design,
            ships: 5,
            location: FleetLocation::InSpace(GalacticPosition::new(0, 0)),
            visibility: EntityVisibility::OwnerOnly,
        },
    );
    let hidden_fleet = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        hidden_fleet,
        Fleet {
            id: hidden_fleet,
            owner: human,
            name: "Ghost".to_owned(),
            design: hidden_design,
            ships: 1,
            location: FleetLocation::InSpace(GalacticPosition::new(10, 10)),
            visibility: EntityVisibility::Hidden,
        },
    );

    Fixture {
        state,
        human,
        rival,
        home,
        rival_home,
        hidden_planet,
        human_design,
        rival_design,
        hidden_design,
        human_fleet,
        rival_fleet,
        hidden_fleet,
    }
}

fn batch(id: u64, player: PlayerId, expected_turn: u32) -> OrderBatch {
    OrderBatch::new(BatchId::from_raw(id), player, expected_turn)
}

fn human_batch(fixture: &Fixture, id: u64) -> OrderBatch {
    batch(id, fixture.human, fixture.state.turn)
        .with_command(Command::new(
            CommandId::from_raw(1),
            CommandAction::SetReady { ready: true },
        ))
        .with_command(Command::new(
            CommandId::from_raw(2),
            CommandAction::RenameFleet {
                fleet: fixture.human_fleet,
                name: "Alpha Renamed".to_owned(),
            },
        ))
}

#[test]
fn identical_state_seed_and_orders_produce_identical_state_and_event_order() {
    let fixture = fixture();
    let batch = human_batch(&fixture, 1);

    let mut first_session = GameSession::new(fixture.state.clone()).expect("valid fixture state");
    let first = first_session.commit(&batch).expect("batch commits");
    let mut second_session = GameSession::new(fixture.state.clone()).expect("valid fixture state");
    let second = second_session.commit(&batch).expect("batch commits");

    assert_eq!(
        first_session.state().encode_canonical(),
        second_session.state().encode_canonical()
    );
    assert_eq!(first.events, second.events);
    assert_eq!(first.view, second.view);
    assert_eq!(first.turn, 1);
    assert_eq!(first.year, 2401);

    let mut first_rng = first_session.state().rng.clone();
    let mut second_rng = second_session.state().rng.clone();
    let key = RngStreamKey::for_player(fixture.human, RngSubsystem::Movement);
    for _ in 0..16 {
        assert_eq!(first_rng.draw_u64(key), second_rng.draw_u64(key));
    }
}

#[test]
fn identical_seed_and_stream_serialization_continues_identically() {
    let fixture = fixture();
    let mut state = fixture.state.clone();
    let key = RngStreamKey::for_player(fixture.human, RngSubsystem::Combat);
    for _ in 0..3 {
        state.rng.draw_u64(key);
    }

    let encoded = state.encode_canonical();
    let mut restored = GameState::decode_canonical(&encoded).expect("state decodes");
    assert_eq!(restored, state);
    assert_eq!(restored.rng.algorithm_version(), RNG_ALGORITHM_VERSION);
    assert_eq!(restored.rng.stream_keys(), vec![key]);
    assert_eq!(restored.rng.draws(key), 3);

    let mut original = state;
    for _ in 0..8 {
        assert_eq!(original.rng.draw_u64(key), restored.rng.draw_u64(key));
    }
    assert_eq!(original.encode_canonical(), restored.encode_canonical());
}

#[test]
fn invalid_batches_are_rejected_without_changing_state() {
    let fixture = fixture();
    let original = fixture.state.encode_canonical();

    type CaseCheck = fn(&OrderError) -> bool;
    let cases: [(&str, OrderBatch, CaseCheck); 5] = [
        (
            "unknown player",
            batch(1, PlayerId::from_raw(999), 0),
            |error: &OrderError| matches!(error, OrderError::UnknownPlayer(_)),
        ),
        (
            "stale turn",
            batch(1, fixture.human, 7),
            |error: &OrderError| matches!(error, OrderError::StaleTurn { .. }),
        ),
        (
            "duplicate command id",
            batch(1, fixture.human, 0)
                .with_command(Command::new(
                    CommandId::from_raw(4),
                    CommandAction::SetReady { ready: true },
                ))
                .with_command(Command::new(
                    CommandId::from_raw(4),
                    CommandAction::SetReady { ready: false },
                )),
            |error: &OrderError| matches!(error, OrderError::DuplicateCommand(_)),
        ),
        (
            "unknown fleet",
            batch(1, fixture.human, 0).with_command(Command::new(
                CommandId::from_raw(5),
                CommandAction::RenameFleet {
                    fleet: FleetId::from_raw(999),
                    name: "Nowhere".to_owned(),
                },
            )),
            |error: &OrderError| matches!(error, OrderError::UnknownFleet(_)),
        ),
        (
            "unauthorized fleet",
            batch(1, fixture.human, 0).with_command(Command::new(
                CommandId::from_raw(6),
                CommandAction::RenameFleet {
                    fleet: fixture.rival_fleet,
                    name: "Stolen".to_owned(),
                },
            )),
            |error: &OrderError| matches!(error, OrderError::UnauthorizedFleet { .. }),
        ),
    ];

    for (label, batch, expected) in cases {
        let mut session = GameSession::new(fixture.state.clone()).expect("valid fixture");
        let error = session.commit(&batch).expect_err(label);
        assert!(expected(&error), "{label}: unexpected error {error}");
        assert_eq!(
            session.state().encode_canonical(),
            original,
            "{label} changed authoritative state"
        );
        assert!(session.last_events().is_empty(), "{label} produced events");
    }
}

#[test]
fn a_later_invalid_command_rolls_back_earlier_commands_in_the_batch() {
    let fixture = fixture();
    let original = fixture.state.encode_canonical();
    let rejected = batch(1, fixture.human, 0)
        .with_command(Command::new(
            CommandId::from_raw(1),
            CommandAction::SetReady { ready: true },
        ))
        .with_command(Command::new(
            CommandId::from_raw(2),
            CommandAction::RenameFleet {
                fleet: fixture.human_fleet,
                name: "Should Not Apply".to_owned(),
            },
        ))
        .with_command(Command::new(
            CommandId::from_raw(3),
            CommandAction::RenameFleet {
                fleet: FleetId::from_raw(404),
                name: "Missing".to_owned(),
            },
        ));

    let mut session = GameSession::new(fixture.state.clone()).expect("valid fixture");
    assert!(matches!(
        session.commit(&rejected),
        Err(OrderError::UnknownFleet(_))
    ));
    assert_eq!(session.state().encode_canonical(), original);
    assert!(!session.state().players[&fixture.human].ready);
    assert_eq!(session.state().fleets[&fixture.human_fleet].name, "Alpha");
}

#[test]
fn duplicate_and_stale_batches_are_rejected_after_a_commit() {
    let fixture = fixture();
    let mut session = GameSession::new(fixture.state.clone()).expect("valid fixture");
    session
        .commit(&human_batch(&fixture, 5))
        .expect("first commit");
    let after = session.state().encode_canonical();

    let duplicate = batch(5, fixture.human, 1);
    assert!(matches!(
        session.commit(&duplicate),
        Err(OrderError::DuplicateOrStaleBatch {
            last_applied: Some(last),
            ..
        }) if last == BatchId::from_raw(5)
    ));
    assert_eq!(session.state().encode_canonical(), after);

    let lower = batch(4, fixture.human, 1);
    assert!(matches!(
        session.commit(&lower),
        Err(OrderError::DuplicateOrStaleBatch { .. })
    ));

    let stale_turn = batch(6, fixture.human, 0);
    assert!(matches!(
        session.commit(&stale_turn),
        Err(OrderError::StaleTurn {
            expected: 0,
            actual: 1
        })
    ));
    assert_eq!(session.state().encode_canonical(), after);
}

#[test]
fn phase_progress_follows_the_recorded_designed_order() {
    let fixture = fixture();
    let batch = human_batch(&fixture, 9);
    let mut session = GameSession::new(fixture.state.clone()).expect("valid fixture");
    let mut progress = Vec::new();
    let outcome = session
        .commit_observed(&batch, |step| progress.push(step))
        .expect("commit");

    let plan = session.state().phase_plan.clone();
    assert_eq!(plan.version(), 1);
    assert_eq!(plan.source(), PhasePlanSource::DesignedChoice);
    assert_eq!(plan.phases(), &Phase::ALL_V1);
    assert_eq!(
        progress.iter().map(|step| step.phase).collect::<Vec<_>>(),
        Phase::ALL_V1
    );
    for (index, step) in progress.iter().enumerate() {
        assert_eq!(step.index, index as u32);
        assert_eq!(step.total, Phase::ALL_V1.len() as u32);
        let expected = if matches!(step.phase, Phase::OrderIntake | Phase::Snapshot) {
            PhaseCapability::Implemented
        } else {
            PhaseCapability::NotImplemented
        };
        assert_eq!(step.capability, expected, "phase {}", step.phase.name());
    }

    let implemented = outcome
        .events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::PhaseCompleted { .. }))
        .count();
    let unimplemented = outcome
        .events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::PhaseNotImplemented { .. }))
        .count();
    assert_eq!(implemented, 2);
    assert_eq!(unimplemented, Phase::ALL_V1.len() - 2);
    assert!(matches!(
        outcome.events.last().map(|event| &event.kind),
        Some(EventKind::TurnCommitted { .. })
    ));
    assert!(
        outcome
            .events
            .windows(2)
            .all(|pair| pair[0].sequence < pair[1].sequence)
    );
}

#[test]
fn phase_plan_is_serialized_and_replaceable_but_not_empty() {
    let fixture = fixture();
    let encoded = fixture.state.encode_canonical();
    let decoded = GameState::decode_canonical(&encoded).expect("state decodes");
    assert_eq!(decoded.phase_plan, fixture.state.phase_plan);
    assert_eq!(decoded.rng.algorithm_version(), RNG_ALGORITHM_VERSION);

    assert!(PhasePlan::from_phases(Vec::new()).is_err());

    let mut state = fixture.state.clone();
    let reordered =
        PhasePlan::from_phases(vec![Phase::OrderIntake, Phase::Snapshot]).expect("valid plan");
    state
        .replace_phase_plan(reordered.clone())
        .expect("replace");
    assert_eq!(state.phase_plan, reordered);
    let restored = GameState::decode_canonical(&state.encode_canonical()).expect("decode");
    assert_eq!(restored.phase_plan, reordered);
}

#[test]
fn canonical_state_round_trips_and_rejects_malformed_input() {
    let fixture = fixture();
    let encoded = fixture.state.encode_canonical();
    assert_eq!(&encoded[..4], b"STC4");
    assert_eq!(
        u16::from_le_bytes([encoded[4], encoded[5]]),
        STATE_FORMAT_VERSION
    );
    assert_eq!(
        GameState::decode_canonical(&encoded).expect("valid state"),
        fixture.state
    );

    let mut bad_magic = encoded.clone();
    bad_magic[0] = b'X';
    assert!(GameState::decode_canonical(&bad_magic).is_err());

    let mut bad_version = encoded.clone();
    bad_version[4] = 0xff;
    assert!(GameState::decode_canonical(&bad_version).is_err());

    assert!(GameState::decode_canonical(&[]).is_err());
    assert!(GameState::decode_canonical(&encoded[..encoded.len() - 1]).is_err());
    let mut trailing = encoded;
    trailing.push(0);
    assert!(GameState::decode_canonical(&trailing).is_err());
}

#[test]
fn allocator_resumes_after_serialization() {
    let fixture = fixture();
    let encoded = fixture.state.encode_canonical();
    let mut restored = GameState::decode_canonical(&encoded).expect("state decodes");
    let next = restored.allocate_planet_id().expect("planet id");
    assert_eq!(next, PlanetId::from_raw(4));
    assert_eq!(
        restored.allocate_player_id().expect("player id"),
        PlayerId::from_raw(3)
    );
    assert_eq!(
        restored.allocate_design_id().expect("design id"),
        DesignId::from_raw(4)
    );
    assert_eq!(
        restored.allocate_fleet_id().expect("fleet id"),
        FleetId::from_raw(4)
    );
}

#[test]
fn player_views_omit_hidden_and_foreign_entities_and_events() {
    let fixture = fixture();
    let mut hidden_players = BTreeSet::new();
    hidden_players.insert(fixture.human);
    let events = vec![
        GameEvent {
            sequence: 0,
            turn: 0,
            visibility: EventVisibility::Public,
            kind: EventKind::TurnStarted { year: 2400 },
        },
        GameEvent {
            sequence: 1,
            turn: 0,
            visibility: EventVisibility::Players(hidden_players),
            kind: EventKind::PhaseCompleted {
                phase: Phase::OrderIntake,
            },
        },
        GameEvent {
            sequence: 2,
            turn: 0,
            visibility: EventVisibility::Players(BTreeSet::from([fixture.rival])),
            kind: EventKind::PhaseCompleted {
                phase: Phase::OrderIntake,
            },
        },
        GameEvent {
            sequence: 3,
            turn: 0,
            visibility: EventVisibility::Hidden,
            kind: EventKind::TurnCommitted {
                batch: BatchId::from_raw(1),
                year: 2401,
            },
        },
    ];

    let human_view = PlayerView::build(&fixture.state, fixture.human, &events).expect("human view");
    assert_eq!(human_view.player_name, "Human");
    assert_eq!(human_view.role, PlayerRole::Human);
    assert_eq!(human_view.planets.len(), 1);
    assert_eq!(human_view.planets[0].id, fixture.home);
    assert!(
        !human_view
            .planets
            .iter()
            .any(|p| p.id == fixture.hidden_planet)
    );
    assert!(
        !human_view
            .planets
            .iter()
            .any(|p| p.id == fixture.rival_home)
    );
    assert_eq!(human_view.designs.len(), 1);
    assert_eq!(human_view.designs[0].id, fixture.human_design);
    assert!(
        !human_view
            .designs
            .iter()
            .any(|d| d.id == fixture.hidden_design)
    );
    assert!(
        !human_view
            .designs
            .iter()
            .any(|d| d.id == fixture.rival_design)
    );
    assert_eq!(human_view.fleets.len(), 1);
    assert_eq!(human_view.fleets[0].id, fixture.human_fleet);
    assert!(
        !human_view
            .fleets
            .iter()
            .any(|f| f.id == fixture.hidden_fleet)
    );
    assert!(
        !human_view
            .fleets
            .iter()
            .any(|f| f.id == fixture.rival_fleet)
    );
    assert_eq!(human_view.events.len(), 2);
    assert_eq!(human_view.events[0].sequence, 0);
    assert_eq!(human_view.events[1].sequence, 1);

    let rival_view = PlayerView::build(&fixture.state, fixture.rival, &events).expect("rival view");
    assert_eq!(rival_view.player_name, "Turindrones");
    assert_eq!(rival_view.planets[0].id, fixture.rival_home);
    assert_eq!(rival_view.designs[0].id, fixture.rival_design);
    assert_eq!(rival_view.fleets[0].id, fixture.rival_fleet);
    assert_eq!(
        rival_view
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
}

#[test]
fn visible_fleet_orbiting_a_hidden_planet_does_not_leak_the_planet() {
    let fixture = fixture();
    let mut state = fixture.state;
    let scout = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        scout,
        Fleet {
            id: scout,
            owner: fixture.human,
            name: "Probe".to_owned(),
            design: fixture.human_design,
            ships: 1,
            location: FleetLocation::AtPlanet(fixture.hidden_planet),
            visibility: EntityVisibility::OwnerOnly,
        },
    );

    let view = PlayerView::build(&state, fixture.human, &[]).expect("human view");
    let fleet = view
        .fleets
        .iter()
        .find(|fleet| fleet.id == scout)
        .expect("visible fleet");
    assert_eq!(fleet.location, ViewLocation::UnknownOrbit);
    assert!(
        !view
            .planets
            .iter()
            .any(|planet| planet.id == fixture.hidden_planet)
    );
}

#[test]
fn visible_fleet_does_not_leak_a_hidden_design() {
    let fixture = fixture();
    let mut state = fixture.state;
    let probe = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        probe,
        Fleet {
            id: probe,
            owner: fixture.human,
            name: "Mystery".to_owned(),
            design: fixture.hidden_design,
            ships: 1,
            location: FleetLocation::InSpace(GalacticPosition::new(1, 2)),
            visibility: EntityVisibility::OwnerOnly,
        },
    );

    let view = PlayerView::build(&state, fixture.human, &[]).expect("human view");
    let fleet = view
        .fleets
        .iter()
        .find(|fleet| fleet.id == probe)
        .expect("visible fleet");
    assert_eq!(fleet.design, None);
    assert!(
        !view
            .designs
            .iter()
            .any(|design| design.id == fixture.hidden_design)
    );
}

#[test]
fn exhausted_event_sequence_is_rejected_as_invalid_state() {
    let fixture = fixture();
    let mut state = fixture.state;
    state.next_event_sequence = u64::MAX;
    assert_eq!(
        state.validate_integrity(),
        Err(stars_core::model::StateValidationError::EventSequenceExhausted)
    );
}

#[test]
fn entity_id_mismatch_with_its_map_key_is_rejected() {
    let fixture = fixture();
    let mut state = fixture.state;
    let id = state
        .planets
        .keys()
        .copied()
        .next()
        .expect("fixture has a planet");
    let planet = state.planets.get_mut(&id).expect("planet present");
    planet.id = PlanetId::from_raw(id.raw() + 1);
    assert!(matches!(
        state.validate_integrity(),
        Err(stars_core::model::StateValidationError::EntityIdMismatch { .. })
    ));
}

#[test]
fn committed_events_are_filtered_per_player() {
    let fixture = fixture();
    let mut session = GameSession::new(fixture.state.clone()).expect("valid fixture");
    session.commit(&human_batch(&fixture, 1)).expect("commit");

    let human_view = session.view_for(fixture.human).expect("human view");
    let rival_view = session.view_for(fixture.rival).expect("rival view");

    assert!(human_view.events.iter().any(|event| matches!(
        event.kind,
        EventKind::PlayerReadinessChanged {
            player,
            ready: true
        } if player == fixture.human
    )));
    assert!(
        !rival_view
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::PlayerReadinessChanged { .. }))
    );
    assert!(
        rival_view
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::TurnCommitted { .. }))
    );
}

#[test]
fn empty_games_and_dangling_references_are_reported() {
    let empty = GameState::new(2400, 1);
    assert_eq!(
        PlayerView::build(&empty, PlayerId::from_raw(1), &[]),
        Err(ViewError::UnknownPlayer(PlayerId::from_raw(1)))
    );

    let fixture = fixture();

    let mut missing_design = fixture.state.clone();
    missing_design.designs.remove(&fixture.human_design);
    assert!(matches!(
        PlayerView::build(&missing_design, fixture.human, &[]),
        Err(ViewError::DanglingDesign { .. })
    ));

    let mut missing_planet = fixture.state.clone();
    missing_planet.planets.remove(&fixture.home);
    assert!(matches!(
        PlayerView::build(&missing_planet, fixture.human, &[]),
        Err(ViewError::DanglingPlanet { .. })
    ));

    let mut missing_player = fixture.state.clone();
    missing_player.players.remove(&fixture.rival);
    assert!(matches!(
        PlayerView::build(&missing_player, fixture.human, &[]),
        Err(ViewError::DanglingPlayer { .. })
    ));
}

#[test]
fn session_rejects_invalid_initial_state() {
    let fixture = fixture();
    let mut broken = fixture.state;
    broken.designs.remove(&fixture.human_design);
    assert!(GameSession::new(broken).is_err());
}

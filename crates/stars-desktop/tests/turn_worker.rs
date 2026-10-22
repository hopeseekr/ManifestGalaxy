//! PR4 acceptance tests for the desktop turn-worker adapter.
//!
//! These tests use the real worker thread. They verify ordered phase progress,
//! atomic rejection with no state loss, stale request handling and the
//! UI-side rules for ignoring delayed or superseded completions.

use std::sync::mpsc::Receiver;
use std::time::Duration;

use stars_core::model::{
    BatchId, Design, EntityVisibility, Fleet, FleetId, FleetLocation, GalacticPosition, GameState,
    Planet, Player, PlayerId, PlayerRole,
};
use stars_core::orders::{Command, CommandAction, CommandId, OrderBatch, OrderError};
use stars_core::turn::{GameSession, Phase, TurnOutcome};
use stars_desktop::turn_worker::{
    TurnActivity, TurnApplyOutcome, TurnSessionModel, TurnWorkerError, TurnWorkerHandle,
    TurnWorkerMessage,
};
use stars_desktop::worker::RequestId;

struct Fixture {
    state: GameState,
    human: PlayerId,
    human_fleet: FleetId,
    rival_fleet: FleetId,
}

fn fixture() -> Fixture {
    let mut state = GameState::new(2400, 0xfeed_0000_0000_0042);

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
            position: GalacticPosition::new(0, 0),
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
            position: GalacticPosition::new(500, 500),
            owner: Some(rival),
            visibility: EntityVisibility::OwnerOnly,
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

    let human_fleet = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        human_fleet,
        Fleet {
            id: human_fleet,
            owner: human,
            name: "Alpha".to_owned(),
            design: human_design,
            ships: 2,
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
            ships: 4,
            location: FleetLocation::AtPlanet(rival_home),
            visibility: EntityVisibility::OwnerOnly,
        },
    );

    Fixture {
        state,
        human,
        human_fleet,
        rival_fleet,
    }
}

fn valid_batch(fixture: &Fixture, id: u64, expected_turn: u32) -> OrderBatch {
    OrderBatch::new(BatchId::from_raw(id), fixture.human, expected_turn)
        .with_command(Command::new(
            CommandId::from_raw(1),
            CommandAction::SetReady { ready: true },
        ))
        .with_command(Command::new(
            CommandId::from_raw(2),
            CommandAction::RenameFleet {
                fleet: fixture.human_fleet,
                name: "Alpha Prime".to_owned(),
            },
        ))
}

fn invalid_batch(fixture: &Fixture, id: u64, expected_turn: u32) -> OrderBatch {
    OrderBatch::new(BatchId::from_raw(id), fixture.human, expected_turn).with_command(Command::new(
        CommandId::from_raw(3),
        CommandAction::RenameFleet {
            fleet: FleetId::from_raw(999),
            name: "Missing".to_owned(),
        },
    ))
}

fn collect_until_completed(messages: &Receiver<TurnWorkerMessage>) -> (Vec<Phase>, TurnOutcome) {
    let mut phases = Vec::new();
    loop {
        match messages
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|error| panic!("worker did not complete: {error}"))
        {
            TurnWorkerMessage::Progress { progress, .. } => phases.push(progress.phase),
            TurnWorkerMessage::Completed { outcome, .. } => return (phases, *outcome),
            TurnWorkerMessage::Failed { error, .. } => panic!("unexpected failure: {error}"),
        }
    }
}

#[test]
fn worker_commits_a_turn_with_ordered_progress_and_a_filtered_view() {
    let fixture = fixture();
    let (worker, messages) = TurnWorkerHandle::spawn(fixture.state.clone()).expect("worker starts");
    let request = RequestId::new(1);

    worker
        .submit(request, valid_batch(&fixture, 1, 0))
        .expect("request submitted");
    let (phases, outcome) = collect_until_completed(&messages);
    worker.shutdown();

    assert_eq!(phases, Phase::ALL_V1);
    assert_eq!(outcome.batch, BatchId::from_raw(1));
    assert_eq!(outcome.player, fixture.human);
    assert_eq!(outcome.turn, 1);
    assert_eq!(outcome.year, 2401);
    assert_eq!(
        outcome.state.fleets[&fixture.human_fleet].name,
        "Alpha Prime"
    );
    assert!(outcome.state.players[&fixture.human].ready);
    assert_eq!(outcome.view.player, fixture.human);
    assert_eq!(outcome.view.fleets.len(), 1);
    assert_eq!(outcome.view.fleets[0].id, fixture.human_fleet);
    assert!(
        !outcome
            .view
            .fleets
            .iter()
            .any(|fleet| fleet.id == fixture.rival_fleet)
    );
    assert!(outcome.view.events.iter().any(|event| matches!(
        event.kind,
        stars_core::turn::EventKind::TurnCommitted { .. }
    )));
}

#[test]
fn worker_failure_leaves_authoritative_state_unchanged_and_continues() {
    let fixture = fixture();
    let (worker, messages) = TurnWorkerHandle::spawn(fixture.state.clone()).expect("worker starts");

    worker
        .submit(RequestId::new(1), invalid_batch(&fixture, 1, 0))
        .expect("request submitted");
    let message = messages
        .recv_timeout(Duration::from_secs(10))
        .expect("worker responds");
    match message {
        TurnWorkerMessage::Failed {
            request_id,
            error: TurnWorkerError::Order(OrderError::UnknownFleet(_)),
        } => assert_eq!(request_id, RequestId::new(1)),
        other => panic!("unexpected response to invalid batch: {other:?}"),
    }

    // The next request still starts from turn 0 and therefore commits turn 1.
    worker
        .submit(RequestId::new(2), valid_batch(&fixture, 2, 0))
        .expect("request submitted");
    let (_, outcome) = collect_until_completed(&messages);
    assert_eq!(outcome.turn, 1);
    assert_eq!(outcome.year, 2401);
    assert_eq!(
        outcome.state.fleets[&fixture.human_fleet].name,
        "Alpha Prime"
    );
    worker.shutdown();
}

#[test]
fn worker_rejects_non_monotonic_request_ids_without_running_the_batch() {
    let fixture = fixture();
    let (worker, messages) = TurnWorkerHandle::spawn(fixture.state.clone()).expect("worker starts");

    worker
        .submit(RequestId::new(10), valid_batch(&fixture, 1, 0))
        .expect("request submitted");
    let (_, first) = collect_until_completed(&messages);
    assert_eq!(first.turn, 1);

    worker
        .submit(RequestId::new(9), valid_batch(&fixture, 2, 1))
        .expect("request submitted");
    let message = messages
        .recv_timeout(Duration::from_secs(10))
        .expect("worker responds");
    match message {
        TurnWorkerMessage::Failed {
            request_id,
            error:
                TurnWorkerError::StaleRequest {
                    request,
                    last_request: Some(last),
                },
        } => {
            assert_eq!(request_id, RequestId::new(9));
            assert_eq!(request, RequestId::new(9));
            assert_eq!(last, RequestId::new(10));
        }
        other => panic!("unexpected message for stale request: {other:?}"),
    }

    // The rejected stale request did not consume the batch or advance the year.
    worker
        .submit(RequestId::new(11), valid_batch(&fixture, 2, 1))
        .expect("request submitted");
    let (_, second) = collect_until_completed(&messages);
    assert_eq!(second.turn, 2);
    assert_eq!(second.year, 2402);
    worker.shutdown();
}

#[test]
fn delayed_completion_cannot_overwrite_a_newer_request() {
    let fixture = fixture();
    let old = make_outcome(&fixture.state, &valid_batch(&fixture, 1, 0));
    let newer = make_outcome(&old.state, &valid_batch(&fixture, 2, 1));

    let mut model = TurnSessionModel::new();
    let old_state = old.state.clone();
    assert!(model.begin_request(RequestId::new(1)));
    assert!(model.begin_request(RequestId::new(2)));

    assert_eq!(
        model.apply(TurnWorkerMessage::Completed {
            request_id: RequestId::new(1),
            outcome: Box::new(old),
        }),
        TurnApplyOutcome::Applied
    );
    // The worker committed request 1, so its snapshot is applied, but the
    // newer active request stays active.
    assert_eq!(model.snapshot(), Some(&old_state));
    assert_eq!(model.active_request(), Some(RequestId::new(2)));
    assert_eq!(model.last_accepted_request(), Some(RequestId::new(1)));

    assert_eq!(
        model.apply(TurnWorkerMessage::Completed {
            request_id: RequestId::new(2),
            outcome: Box::new(newer.clone()),
        }),
        TurnApplyOutcome::Applied
    );
    assert_eq!(model.snapshot(), Some(&newer.state));
    assert_eq!(model.view(), Some(&newer.view));
    assert_eq!(model.last_accepted_request(), Some(RequestId::new(2)));
}

#[test]
fn model_failure_retains_the_last_committed_snapshot() {
    let fixture = fixture();
    let accepted = make_outcome(&fixture.state, &valid_batch(&fixture, 1, 0));

    let mut model = TurnSessionModel::new();
    assert!(model.begin_request(RequestId::new(1)));
    assert_eq!(
        model.apply(TurnWorkerMessage::Completed {
            request_id: RequestId::new(1),
            outcome: Box::new(accepted.clone()),
        }),
        TurnApplyOutcome::Applied
    );

    assert!(model.begin_request(RequestId::new(2)));
    assert_eq!(
        model.apply(TurnWorkerMessage::Failed {
            request_id: RequestId::new(2),
            error: TurnWorkerError::Order(OrderError::UnknownFleet(FleetId::from_raw(77))),
        }),
        TurnApplyOutcome::Applied
    );

    assert_eq!(model.snapshot(), Some(&accepted.state));
    assert_eq!(model.view(), Some(&accepted.view));
    assert_eq!(model.last_accepted_request(), Some(RequestId::new(1)));
    assert_eq!(model.active_request(), None);
    assert!(matches!(
        model.activity(),
        TurnActivity::Failed { request_id, .. } if *request_id == RequestId::new(2)
    ));
}

#[test]
fn stale_progress_is_ignored_and_request_ids_must_increase() {
    let mut model = TurnSessionModel::new();
    let first = RequestId::new(1);
    let second = RequestId::new(2);

    assert!(model.begin_request(first));
    assert!(model.begin_request(second));
    assert!(!model.begin_request(second));
    assert!(!model.begin_request(first));

    let progress = stars_core::turn::PhaseProgress {
        phase: Phase::OrderIntake,
        index: 1,
        total: 18,
        capability: stars_core::turn::PhaseCapability::Implemented,
    };
    assert_eq!(
        model.apply(TurnWorkerMessage::Progress {
            request_id: first,
            progress,
        }),
        TurnApplyOutcome::IgnoredStale
    );
    assert_eq!(
        model.apply(TurnWorkerMessage::Progress {
            request_id: second,
            progress,
        }),
        TurnApplyOutcome::Applied
    );
    assert_eq!(
        model.activity(),
        &TurnActivity::Running {
            request_id: second,
            phase: Some(Phase::OrderIntake),
        }
    );
}

fn make_outcome(state: &GameState, batch: &OrderBatch) -> TurnOutcome {
    GameSession::new(state.clone())
        .expect("valid fixture state")
        .commit(batch)
        .expect("batch commits")
}

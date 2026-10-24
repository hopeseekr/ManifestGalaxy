//! PR6 acceptance tests for the desktop new-game form and worker.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use eframe::egui;

use stars_core::race::{PrimaryTrait, RaceSpec};
use stars_core::universe::{
    Density, Difficulty, GalaxySize, PlayerPositions, UniverseSettings, VictoryConditionKind,
};
use stars_desktop::race_file::encode_race_json;
use stars_desktop::ui::new_game::{
    NewGameActivity, NewGameError, NewGameState, UniverseJob, UniverseWorkerError,
    UniverseWorkerHandle, UniverseWorkerMessage,
};
use stars_desktop::worker::RequestId;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_race_directory() -> PathBuf {
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "stars-new-game-race-{}-{sequence}",
        std::process::id()
    ))
}

fn valid_race() -> RaceSpec {
    let mut race = RaceSpec::humanoids_anchor();
    race.primary = PrimaryTrait::JackOfAllTrades;
    race
}

fn context_input(size: egui::Vec2) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    }
}

fn render(state: &mut NewGameState, context: &egui::Context, size: egui::Vec2) {
    let output = context.run_ui(context_input(size), |ui| state.draw(ui));
    output.drop_without_applying_deltas();
}

fn configured() -> NewGameState {
    let mut state = NewGameState::new();
    state.set_race(Some(valid_race()));
    state.set_seed(0x1234_5678);
    state.settings_mut().game_name = "Desktop Acceptance".to_owned();
    state.settings_mut().opponent_count = 2;
    state
}

#[test]
fn form_renders_every_setting_at_multiple_scales_and_proportions() {
    let context = egui::Context::default();
    let mut state = configured();

    for (scale, width, height) in [
        (1.0_f32, 720.0_f32, 520.0_f32),
        (1.5, 1100.0, 760.0),
        (2.0, 900.0, 1000.0),
    ] {
        context.set_zoom_factor(scale);
        for size in GalaxySize::ALL {
            state.settings_mut().size = size;
            for density in Density::ALL {
                state.settings_mut().density = density;
                for clumping in [false, true] {
                    state.settings_mut().clumping = clumping;
                    render(&mut state, &context, egui::vec2(width, height));
                }
            }
        }
        for position in PlayerPositions::ALL {
            state.settings_mut().player_positions = position;
            render(&mut state, &context, egui::vec2(width, height));
        }
        for difficulty in Difficulty::ALL {
            state.settings_mut().difficulty = difficulty;
            render(&mut state, &context, egui::vec2(width, height));
        }
        for kind in VictoryConditionKind::ALL {
            state
                .settings_mut()
                .victory
                .condition_mut(kind)
                .set_enabled(true);
            render(&mut state, &context, egui::vec2(width, height));
        }
    }

    assert_eq!(state.activity(), &NewGameActivity::Idle);
}

#[test]
fn valid_generation_commits_a_universe_and_reports_progress() {
    let mut state = configured();
    state.generate_blocking().expect("universe generated");
    let universe = state.universe().expect("committed universe");
    assert_eq!(universe.settings.seed, 0x1234_5678);
    assert_eq!(universe.state.players.len(), 3);
    assert!(universe.planet_count() > 1);
    assert_eq!(state.errors(), &[]);
    assert!(matches!(
        state.activity(),
        NewGameActivity::Generated { .. }
    ));
}

#[test]
fn invalid_race_and_seed_retain_the_previous_universe() {
    let mut state = configured();
    state.generate_blocking().expect("initial universe");
    let committed = state.universe().cloned().expect("committed universe");
    let committed_planets = committed.planet_count();

    let mut broken = valid_race();
    broken.growth_rate = 0;
    state.set_race(Some(broken));
    let errors = state.generate_blocking().expect_err("invalid race");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, NewGameError::InvalidRace(_)))
    );
    assert_eq!(
        state
            .universe()
            .map(stars_core::universe::GeneratedUniverse::planet_count),
        Some(committed_planets),
        "a rejected race must not replace the active game"
    );

    state.set_race(Some(valid_race()));
    *state.seed_text_mut() = "not-a-seed".to_owned();
    let errors = state.generate_blocking().expect_err("invalid seed");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, NewGameError::SeedNotANumber(_)))
    );
    assert_eq!(
        state
            .universe()
            .map(stars_core::universe::GeneratedUniverse::planet_count),
        Some(committed_planets)
    );
}

#[test]
fn invalid_settings_fail_before_submission_and_keep_the_old_game() {
    let mut state = configured();
    state.generate_blocking().expect("initial universe");
    let committed = state.universe().cloned().expect("committed universe");

    state.settings_mut().opponent_count = 0;
    assert!(state.generate_blocking().is_err());
    assert!(state.errors().iter().any(|error| matches!(
        error,
        NewGameError::InvalidSettings(errors)
            if errors.iter().any(|value| matches!(
                value,
                stars_core::universe::UniverseSettingsError::OpponentCountOutOfRange { .. }
            ))
    )));
    assert_eq!(state.universe(), Some(&committed));
}

#[test]
fn worker_failure_retains_the_previous_universe() {
    let mut state = configured();
    state.generate_blocking().expect("initial universe");
    let committed = state.universe().cloned().expect("committed universe");

    let request = RequestId::new(99);
    state.apply(UniverseWorkerMessage::Failed {
        request_id: request,
        error: UniverseWorkerError::SpawnFailed {
            message: "injected".to_owned(),
        },
    });
    assert_eq!(state.universe(), Some(&committed));

    // A failure for the active request is applied and retains the snapshot.
    state.set_race(Some(valid_race()));
    let request = state.generate().expect("request submitted");
    state.apply(UniverseWorkerMessage::Failed {
        request_id: request,
        error: UniverseWorkerError::Generation(
            stars_core::universe::UniverseGenerationError::HomeworldPlacementFailed {
                player_index: 3,
            },
        ),
    });
    assert_eq!(state.universe(), Some(&committed));
    assert!(matches!(
        state.activity(),
        NewGameActivity::Failed { request_id, .. } if *request_id == request
    ));
}

#[test]
fn four_opponents_have_distinct_identities() {
    let mut state = configured();
    state.settings_mut().opponent_count = 4;
    state.settings_mut().difficulty = Difficulty::Tough;
    state.generate_blocking().expect("universe generated");
    let universe = state.universe().expect("committed universe");

    let names: BTreeSet<&str> = universe
        .state
        .players
        .values()
        .map(|player| player.name.as_str())
        .collect();
    assert_eq!(names.len(), 5);
    assert_eq!(universe.opponents.len(), 4);
    let ids: BTreeSet<_> = universe.opponents.iter().copied().collect();
    assert_eq!(ids.len(), 4);
}

#[test]
fn worker_rejects_a_stale_request_identifier() {
    let (worker, messages) = UniverseWorkerHandle::spawn().expect("worker");
    let settings = UniverseSettings::new();
    let race = valid_race();

    worker
        .submit(UniverseJob {
            request_id: RequestId::new(2),
            settings: settings.clone(),
            race: race.clone(),
        })
        .expect("newer request");
    worker
        .submit(UniverseJob {
            request_id: RequestId::new(1),
            settings,
            race,
        })
        .expect("older request queued");

    let mut completed = false;
    let mut stale = false;
    for _ in 0..12 {
        match messages
            .recv_timeout(Duration::from_secs(10))
            .expect("worker message")
        {
            UniverseWorkerMessage::Progress { .. } => {}
            UniverseWorkerMessage::Completed { request_id, .. } => {
                assert_eq!(request_id, RequestId::new(2));
                completed = true;
            }
            UniverseWorkerMessage::Failed { request_id, error } => {
                assert_eq!(request_id, RequestId::new(1));
                assert!(matches!(error, UniverseWorkerError::StaleRequest { .. }));
                stale = true;
            }
        }
        if completed && stale {
            break;
        }
    }
    worker.shutdown();
    assert!(completed && stale);
}

#[test]
fn stale_completion_cannot_replace_the_active_request() {
    let mut state = configured();
    state.generate_blocking().expect("initial universe");
    let committed = state.universe().cloned().expect("committed universe");
    let accepted = state.last_accepted_request();

    let older = state.generate().expect("older request submitted");
    let newer = state.generate().expect("newer request submitted");
    assert_ne!(older, newer);

    let mut stale_settings = state.settings().clone();
    stale_settings.seed ^= 0xffff;
    let stale = stars_core::universe::generate_universe(
        &stale_settings,
        state.race().expect("validated race"),
    )
    .expect("stale universe");

    state.apply(UniverseWorkerMessage::Completed {
        request_id: older,
        universe: Box::new(stale),
    });

    assert_eq!(
        state.universe(),
        Some(&committed),
        "a completion for a superseded request must not replace the active game"
    );
    assert!(
        matches!(
            state.activity(),
            NewGameActivity::Generating { request_id, .. } if *request_id == newer
        ),
        "the newer request must remain active after an older completion arrives"
    );
    assert_eq!(state.last_accepted_request(), accepted);
}

#[test]
fn loads_a_saved_validated_race_file_and_uses_it_for_generation() {
    let directory = temp_race_directory();
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("alteran.json");
    let mut saved = valid_race();
    saved.name = "Alteran".to_owned();
    saved.plural_name = "Alterans".to_owned();
    std::fs::write(&path, encode_race_json(&saved).expect("encode race")).expect("write race");

    let mut state = NewGameState::new();
    state.refresh_race_files_in(&directory);
    assert_eq!(state.available_race_files(), std::slice::from_ref(&path));
    assert!(state.race_files_loaded());

    state.load_race_file(&path).expect("load race");
    assert!(state.race_from_file());
    assert_eq!(state.race().map(|race| race.name.as_str()), Some("Alteran"));

    // A later builder race must not silently override an explicitly loaded file.
    let mut builder_race = valid_race();
    builder_race.name = "Builder Race".to_owned();
    state.set_builder_race(Some(builder_race));
    assert_eq!(state.race().map(|race| race.name.as_str()), Some("Alteran"));

    state.settings_mut().opponent_count = 1;
    state.generate_blocking().expect("generated from file");
    let universe = state.universe().expect("universe");
    assert_eq!(
        universe
            .state
            .players
            .get(&universe.human)
            .and_then(|player| player.race.as_ref())
            .map(|race| race.name.as_str()),
        Some("Alteran")
    );

    state.clear_race_file_source();
    assert!(!state.race_from_file());
    assert!(state.race().is_none());
    state.set_builder_race(Some(valid_race()));
    assert_eq!(
        state.race().map(|race| race.name.as_str()),
        Some("Humanoids")
    );

    std::fs::remove_file(&path).expect("remove race");
    std::fs::remove_dir(&directory).expect("remove directory");
}

#[test]
fn an_invalid_race_file_retains_the_current_universe_and_race() {
    let directory = temp_race_directory();
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("broken.json");
    std::fs::write(&path, "{ not a race").expect("write invalid race");

    let mut state = configured();
    state.generate_blocking().expect("initial universe");
    let committed = state.universe().cloned().expect("universe");
    let race_name = state.race().map(|race| race.name.clone());

    let error = state.load_race_file(&path).expect_err("invalid file");
    assert!(matches!(error, NewGameError::RaceFile { .. }));
    assert_eq!(state.universe(), Some(&committed));
    assert_eq!(state.race().map(|race| race.name.clone()), race_name);
    assert!(
        state
            .errors()
            .iter()
            .any(|error| matches!(error, NewGameError::RaceFile { .. }))
    );

    std::fs::remove_file(&path).expect("remove race");
    std::fs::remove_dir(&directory).expect("remove directory");
}

#[test]
fn invalid_race_files_are_reported_without_replacing_the_race() {
    let mut state = configured();
    let race_name = state.race().map(|race| race.name.clone());
    let error = state
        .load_selected_race_file()
        .expect_err("empty path is rejected");
    assert!(matches!(error, NewGameError::RaceFile { .. }));
    assert_eq!(state.race().map(|race| race.name.clone()), race_name);
}

//! PR7 acceptance tests for reports, inspector, linked selection and drafts.

use std::collections::BTreeSet;

use eframe::egui;

use stars_core::model::{
    BatchId, Design, EntityVisibility, Fleet, FleetId, FleetLocation, GalacticPosition, GameState,
    Planet, PlanetEnvironment, PlanetId, Player, PlayerId, PlayerRole,
};
use stars_core::orders::{Command, CommandAction, CommandId, OrderBatch, OrderError};
use stars_core::race::{HabitabilityRange, LesserTrait, PrimaryTrait, RaceSpec};
use stars_core::turn::GameSession;
use stars_core::view::{PlayerView, ViewLocation};
use stars_desktop::ui::map::Selection;
use stars_desktop::ui::orders::{DraftStatus, OrderDraft};
use stars_desktop::ui::reports::{OwnerFilter, PlanetSortKey, ReportState};
use stars_desktop::ui::{inspector, map::MapState};

struct Fixture {
    state: GameState,
    human: PlayerId,
    rival: PlayerId,
    home: PlanetId,
    rival_home: PlanetId,
    neutral: PlanetId,
    human_fleet: FleetId,
    rival_fleet: FleetId,
    hidden_planet: PlanetId,
    hidden_fleet: FleetId,
}

fn race(primary: PrimaryTrait) -> RaceSpec {
    let mut race = RaceSpec::humanoids_anchor();
    race.primary = primary;
    race.habitability = [HabitabilityRange::new(15, 85); 3];
    race.lesser.insert(LesserTrait::TotalTerraforming);
    race
}

fn planet(
    state: &mut GameState,
    name: &str,
    position: GalacticPosition,
    owner: Option<PlayerId>,
    population: u32,
    visibility: EntityVisibility,
) -> PlanetId {
    let id = state.allocate_planet_id().expect("planet id");
    state.planets.insert(
        id,
        Planet {
            id,
            name: name.to_owned(),
            position,
            owner,
            visibility,
            environment: PlanetEnvironment::new(45, 55, 65),
            mineral_concentration: [90, 80, 70],
            surface_minerals: [1_200, 800, 400],
            population,
            factories: 12,
            mines: 7,
            defenses: 3,
            homeworld: population > 0,
            starbase: population > 0,
        },
    );
    id
}

fn fixture() -> Fixture {
    let mut state = GameState::new(2400, 0x5eed_0007);
    let human = state.allocate_player_id().expect("human id");
    state.players.insert(
        human,
        Player {
            id: human,
            name: "Human".to_owned(),
            role: PlayerRole::Human,
            ready: false,
            race: Some(race(PrimaryTrait::AlternateReality)),
            starting_technology: [3, 2, 1, 4, 5, 6],
        },
    );
    let rival = state.allocate_player_id().expect("rival id");
    state.players.insert(
        rival,
        Player {
            id: rival,
            name: "Turindrones".to_owned(),
            role: PlayerRole::Computer,
            ready: false,
            race: Some(race(PrimaryTrait::WarMonger)),
            starting_technology: [1; 6],
        },
    );

    let home = planet(
        &mut state,
        "A Home",
        GalacticPosition::new(100, 100),
        Some(human),
        25_000,
        EntityVisibility::Public,
    );
    let rival_home = planet(
        &mut state,
        "M Rival",
        GalacticPosition::new(500, 500),
        Some(rival),
        5_000,
        EntityVisibility::Public,
    );
    let neutral = planet(
        &mut state,
        "Z Neutral",
        GalacticPosition::new(300, 300),
        None,
        0,
        EntityVisibility::Public,
    );
    let hidden_planet = planet(
        &mut state,
        "Hidden World",
        GalacticPosition::new(900, 900),
        Some(rival),
        1_000,
        EntityVisibility::Hidden,
    );

    let human_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        human_design,
        Design {
            id: human_design,
            owner: human,
            name: "Scout".to_owned(),
            visibility: EntityVisibility::Public,
        },
    );
    let rival_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        rival_design,
        Design {
            id: rival_design,
            owner: rival,
            name: "Raider".to_owned(),
            visibility: EntityVisibility::Public,
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
            visibility: EntityVisibility::Public,
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
            location: FleetLocation::InSpace(GalacticPosition::new(450, 460)),
            visibility: EntityVisibility::Public,
        },
    );
    let hidden_design = state.allocate_design_id().expect("design id");
    state.designs.insert(
        hidden_design,
        Design {
            id: hidden_design,
            owner: rival,
            name: "Secret".to_owned(),
            visibility: EntityVisibility::Hidden,
        },
    );
    let hidden_fleet = state.allocate_fleet_id().expect("fleet id");
    state.fleets.insert(
        hidden_fleet,
        Fleet {
            id: hidden_fleet,
            owner: rival,
            name: "Ghost".to_owned(),
            design: hidden_design,
            ships: 9,
            location: FleetLocation::InSpace(GalacticPosition::new(800, 800)),
            visibility: EntityVisibility::Hidden,
        },
    );

    Fixture {
        state,
        human,
        rival,
        home,
        rival_home,
        neutral,
        human_fleet,
        rival_fleet,
        hidden_planet,
        hidden_fleet,
    }
}

fn view(fixture: &Fixture) -> PlayerView {
    PlayerView::build(&fixture.state, fixture.human, &[]).expect("player view")
}

#[test]
fn reports_sort_search_filter_and_preserve_selection_by_identifier() {
    let fixture = fixture();
    let view = view(&fixture);
    let mut reports = ReportState::new();

    let names: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.name.as_str())
        .collect();
    assert_eq!(names, ["A Home", "M Rival", "Z Neutral"]);

    reports.sort_planets_by(PlanetSortKey::Population);
    let ascending: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(
        ascending,
        vec![fixture.neutral, fixture.rival_home, fixture.home]
    );

    reports.sort_planets_by(PlanetSortKey::Population);
    let descending: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(
        descending,
        vec![fixture.home, fixture.rival_home, fixture.neutral]
    );

    let mut selection = Selection::Planet(fixture.rival_home);
    reports.sort_planets_by(PlanetSortKey::Name);
    reports.retain_selection(&view, &mut selection);
    assert_eq!(selection, Selection::Planet(fixture.rival_home));

    reports.set_query("rival");
    let matches: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(matches, vec![fixture.rival_home]);

    reports.set_query("human");
    let matches: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(matches, vec![fixture.home], "search matches owner labels");

    reports.set_query("");
    reports.set_owner_filter(OwnerFilter::Mine);
    let mine: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(mine, vec![fixture.home]);

    reports.set_owner_filter(OwnerFilter::Opponents);
    let opponents: Vec<_> = reports
        .visible_planets(&view)
        .into_iter()
        .map(|planet| planet.id)
        .collect();
    assert_eq!(opponents, vec![fixture.rival_home]);

    reports.set_owner_filter(OwnerFilter::Unowned);
    assert_eq!(
        reports
            .visible_planets(&view)
            .into_iter()
            .map(|planet| planet.id)
            .collect::<Vec<_>>(),
        vec![fixture.neutral]
    );
}

#[test]
fn hidden_entities_and_designs_never_reach_the_management_view() {
    let fixture = fixture();
    let view = view(&fixture);
    let reports = ReportState::new();

    assert_eq!(view.planets.len(), 3);
    assert_eq!(view.fleets.len(), 2);
    assert!(
        view.planets
            .iter()
            .all(|planet| planet.id != fixture.hidden_planet)
    );
    assert!(
        view.fleets
            .iter()
            .all(|fleet| fleet.id != fixture.hidden_fleet)
    );
    assert!(view.designs.iter().all(|design| design.name != "Secret"));
    assert!(
        reports
            .visible_planets(&view)
            .iter()
            .all(|planet| planet.name != "Hidden World")
    );
    assert!(
        reports
            .visible_fleets(&view)
            .iter()
            .all(|fleet| fleet.name != "Ghost")
    );
}

#[test]
fn selection_disappears_cleanly_after_a_turn_removes_the_entity() {
    let fixture = fixture();
    let mut view = view(&fixture);
    let selection = Selection::Fleet(fixture.rival_fleet);
    assert_eq!(selection.retain(&view), selection);

    view.fleets.retain(|fleet| fleet.id != fixture.rival_fleet);
    assert_eq!(selection.retain(&view), Selection::None);

    let mut map = MapState::new();
    map.select(Selection::Planet(fixture.rival_home));
    let mut without = view.clone();
    without
        .planets
        .retain(|planet| planet.id != fixture.rival_home);
    let context = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_280.0, 720.0),
        )),
        ..Default::default()
    };
    let output = context.run_ui(input, |ui| map.draw(ui, &without));
    output.drop_without_applying_deltas();
    assert_eq!(map.selection(), Selection::None);
}

#[test]
fn draft_orders_build_stable_commands_and_report_core_errors() {
    let fixture = fixture();
    let session = GameSession::new(fixture.state.clone()).expect("session");
    let view = session.view_for(fixture.human).expect("view");
    let mut draft = OrderDraft::new();
    draft.sync(&view, Selection::Fleet(fixture.human_fleet));
    assert!(!draft.has_changes(&view));

    draft.set_ready(true);
    draft.rename_text_mut().push_str(" II");
    let batch = draft.build_batch(&view);
    assert_eq!(batch.player, fixture.human);
    assert_eq!(batch.expected_turn, view.turn);
    assert_eq!(batch.commands.len(), 2);
    assert_eq!(
        batch.commands[0].action,
        CommandAction::SetReady { ready: true }
    );
    assert!(matches!(
        batch.commands[1].action,
        CommandAction::RenameFleet { fleet, ref name }
            if fleet == fixture.human_fleet && name == "Alpha II"
    ));
    assert_eq!(batch.commands[0].id, CommandId::from_raw(1));
    assert_eq!(batch.commands[1].id, CommandId::from_raw(2));

    draft.validate(&view, &session).expect("valid draft");
    assert_eq!(draft.status(), DraftStatus::Valid);
    assert!(draft.errors().is_empty());

    let hijack =
        OrderBatch::new(BatchId::from_raw(1), fixture.human, view.turn).with_command(Command::new(
            CommandId::from_raw(1),
            CommandAction::RenameFleet {
                fleet: fixture.rival_fleet,
                name: "Hijacked".to_owned(),
            },
        ));
    let error = session
        .validate_orders(&hijack)
        .expect_err("core rejects foreign fleet");
    assert!(matches!(
        error,
        OrderError::UnauthorizedFleet {
            fleet,
            owner,
            player
        } if fleet == fixture.rival_fleet && owner == fixture.rival && player == fixture.human
    ));

    let stale = OrderBatch::new(BatchId::from_raw(2), fixture.human, view.turn + 5);
    assert!(matches!(
        session.validate_orders(&stale),
        Err(OrderError::StaleTurn { .. })
    ));
}

#[test]
fn management_panels_render_at_required_windows_and_text_scales() {
    let fixture = fixture();
    let session = GameSession::new(fixture.state.clone()).expect("session");
    let view = session.view_for(fixture.human).expect("view");
    let context = egui::Context::default();
    let mut reports = ReportState::new();
    let mut draft = OrderDraft::new();
    let mut selection = Selection::Planet(fixture.home);
    let mut map = MapState::new();

    for (scale, width, height) in [
        (1.0_f32, 1_280.0_f32, 720.0_f32),
        (1.5, 1_280.0, 720.0),
        (2.0, 1_920.0, 1_080.0),
        (1.0, 640.0, 420.0),
    ] {
        context.set_zoom_factor(scale);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            ..Default::default()
        };
        let output = context.run_ui(input, |ui| {
            egui::Panel::left("test_reports")
                .resizable(true)
                .default_size(320.0)
                .min_size(200.0)
                .show(ui, |ui| reports.draw(ui, &view, &mut selection));
            egui::Panel::right("test_inspector")
                .resizable(true)
                .default_size(320.0)
                .min_size(200.0)
                .show(ui, |ui| {
                    inspector::draw(ui, &view, selection);
                    let _ = draft.draw(ui, &view, selection);
                });
            egui::CentralPanel::default().show(ui, |ui| map.draw(ui, &view));
        });
        output.drop_without_applying_deltas();
    }

    assert_eq!(selection.retain(&view), selection);
}

#[test]
fn every_racial_trait_reaches_the_view_and_unusual_planets_keep_their_facts() {
    let fixture = fixture();
    let mut state = fixture.state.clone();

    for primary in PrimaryTrait::ALL {
        if let Some(player) = state.players.get_mut(&fixture.human)
            && let Some(race) = &mut player.race
        {
            race.primary = primary;
            race.lesser.clear();
        }
        let view = PlayerView::build(&state, fixture.human, &[]).expect("view");
        let summary = view.race.as_ref().expect("race summary");
        assert_eq!(summary.primary, primary);
        assert_eq!(summary.starting_technology, [3, 2, 1, 4, 5, 6]);
        assert_eq!(
            view.planets
                .iter()
                .find(|planet| planet.id == fixture.home)
                .map(|planet| (planet.population, planet.factories, planet.mines)),
            Some((25_000, 12, 7)),
            "Alternate Reality planet facts pass through unchanged"
        );
    }

    let lesser_set: BTreeSet<_> = LesserTrait::ALL.into_iter().collect();
    if let Some(player) = state.players.get_mut(&fixture.human)
        && let Some(race) = &mut player.race
    {
        race.lesser.clone_from(&lesser_set);
    }
    let view = PlayerView::build(&state, fixture.human, &[]).expect("view");
    let summary = view.race.as_ref().expect("race summary");
    assert_eq!(summary.lesser, LesserTrait::ALL.to_vec());
    let reports = ReportState::new();
    assert_eq!(
        reports.visible_planets(&view).len(),
        view.planets.len(),
        "management tables never invent hidden planets"
    );
    assert_eq!(
        view.fleets
            .iter()
            .find(|fleet| fleet.id == fixture.human_fleet)
            .map(|fleet| fleet.location),
        Some(ViewLocation::AtPlanet {
            planet: fixture.home,
            position: GalacticPosition::new(100, 100)
        })
    );
}

#[test]
fn stable_identifiers_survive_sorting_and_filter_changes() {
    let fixture = fixture();
    let view = view(&fixture);
    let mut reports = ReportState::new();
    let selected = Selection::Planet(fixture.neutral);

    for key in [
        PlanetSortKey::Name,
        PlanetSortKey::Owner,
        PlanetSortKey::Population,
        PlanetSortKey::Gravity,
        PlanetSortKey::Temperature,
        PlanetSortKey::Radiation,
        PlanetSortKey::Ironium,
        PlanetSortKey::Boranium,
        PlanetSortKey::Germanium,
        PlanetSortKey::Factories,
        PlanetSortKey::Mines,
        PlanetSortKey::Defenses,
    ] {
        reports.sort_planets_by(key);
        let mut current = selected;
        reports.retain_selection(&view, &mut current);
        assert_eq!(current, selected, "{key:?} changed the selected identifier");
    }

    reports.set_query("nothing matches");
    let mut current = selected;
    reports.retain_selection(&view, &mut current);
    assert_eq!(
        current, selected,
        "a filtered-out entity stays selected until it leaves the view"
    );
    assert_eq!(reports.visible_planets(&view).len(), 0);
}

#[test]
fn the_desktop_app_commits_a_generated_universe_into_the_management_view() {
    use stars_desktop::StarsApp;

    let mut app = StarsApp::new().expect("app starts");
    let mut race = RaceSpec::humanoids_anchor();
    race.primary = PrimaryTrait::JackOfAllTrades;
    app.new_game_mut().set_race(Some(race));
    app.new_game_mut().settings_mut().opponent_count = 1;
    app.new_game_mut().settings_mut().game_name = "PR7 View Acceptance".to_owned();
    app.new_game_mut()
        .generate_blocking()
        .expect("universe generated");

    let context = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1_280.0, 720.0),
        )),
        ..Default::default()
    };
    let output = context.run_ui(input, |ui| app.draw(ui));
    output.drop_without_applying_deltas();

    let view = app.view().expect("management view");
    assert_eq!(view.player_name, "Humanoids");
    assert!(view.planets.len() > 1);
    assert_eq!(view.fleets.len(), 0);
    assert!(app.map().transform().viewport().width() > 0.0);
    assert!(app.reports().visible_planets(view).len() == view.planets.len());
}

//! PR7 acceptance tests for map projection, zoom, pan, culling and hit testing.

use eframe::egui;

use stars_core::model::{
    DesignId, EntityVisibility, FleetId, GalacticPosition, PlanetEnvironment, PlanetId, PlayerId,
    PlayerRole,
};
use stars_core::view::{FleetView, PlanetView, PlayerView, ViewLocation};
use stars_desktop::ui::map::{MapState, MapTransform, Selection, WorldBounds};

fn planet(id: u64, name: &str, x: i32, y: i32, owner: Option<PlayerId>) -> PlanetView {
    PlanetView {
        id: PlanetId::from_raw(id),
        name: name.to_owned(),
        position: GalacticPosition::new(x, y),
        owner,
        environment: PlanetEnvironment::new(50, 60, 70),
        mineral_concentration: [100, 90, 80],
        surface_minerals: [1_000, 2_000, 3_000],
        population: 25_000,
        factories: 10,
        mines: 8,
        defenses: 4,
        homeworld: id == 1,
        starbase: id == 1,
    }
}

fn fleet(id: u64, name: &str, owner: PlayerId, location: ViewLocation, ships: u32) -> FleetView {
    FleetView {
        id: FleetId::from_raw(id),
        owner,
        name: name.to_owned(),
        design: Some(DesignId::from_raw(1)),
        ships,
        location,
    }
}

fn fixture() -> PlayerView {
    let human = PlayerId::from_raw(1);
    let rival = PlayerId::from_raw(2);
    PlayerView {
        player: human,
        player_name: "Human".to_owned(),
        role: PlayerRole::Human,
        race: None,
        ready: false,
        year: 2400,
        turn: 0,
        designs: Vec::new(),
        planets: vec![
            planet(1, "Near", 100, 100, Some(human)),
            planet(2, "Middle", 300, 420, None),
            planet(3, "Far", 5_000, 5_000, Some(rival)),
        ],
        fleets: vec![
            fleet(
                1,
                "Alpha",
                human,
                ViewLocation::AtPlanet {
                    planet: PlanetId::from_raw(1),
                    position: GalacticPosition::new(100, 100),
                },
                3,
            ),
            fleet(
                2,
                "Beta",
                rival,
                ViewLocation::InSpace(GalacticPosition::new(5_200, 5_100)),
                5,
            ),
        ],
        events: Vec::new(),
    }
}

fn viewport(width: f32, height: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height))
}

fn fitted(width: f32, height: f32) -> MapState {
    let view = fixture();
    let bounds = WorldBounds::from_view(&view).expect("bounds");
    let mut map = MapState::new();
    *map.transform_mut() = MapTransform::fit(bounds, viewport(width, height));
    map
}

#[test]
fn fit_projects_every_world_object_inside_the_viewport() {
    let view = fixture();
    let map = fitted(1_280.0, 720.0);
    let rect = map.transform().viewport();

    for planet in &view.planets {
        let point = map.transform().world_to_screen(planet.position);
        assert!(
            rect.contains(point),
            "{} projected outside {rect:?} at {point:?}",
            planet.name
        );
    }
    for fleet in &view.fleets {
        if let ViewLocation::InSpace(position) | ViewLocation::AtPlanet { position, .. } =
            fleet.location
        {
            assert!(rect.contains(map.transform().world_to_screen(position)));
        }
    }
}

#[test]
fn zoom_keeps_the_world_point_under_the_cursor_fixed() {
    let mut map = fitted(1_920.0, 1_080.0);
    let cursor = egui::pos2(640.0, 400.0);
    let before = map.transform().screen_to_world(cursor);
    let zoom_before = map.transform().zoom();

    map.transform_mut().zoom_at(cursor, 2.5);
    let after = map.transform().screen_to_world(cursor);

    assert!(map.transform().zoom() > zoom_before);
    assert!((before.x - after.x).abs() < 0.01);
    assert!((before.y - after.y).abs() < 0.01);
}

#[test]
fn hit_testing_follows_zoom_pan_and_high_dpi_viewports() {
    let view = fixture();
    let near = view.planets[0].clone();
    let mut map = fitted(1_280.0, 720.0);

    let pointer = map.transform().world_to_screen(near.position);
    assert_eq!(
        map.hit_test(&view, pointer),
        Selection::Planet(near.id),
        "initial hit"
    );

    map.transform_mut().zoom_at(pointer, 3.0);
    let zoomed = map.transform().world_to_screen(near.position);
    assert_eq!(
        map.hit_test(&view, zoomed),
        Selection::Planet(near.id),
        "hit after cursor-centred zoom"
    );

    map.transform_mut().pan_by(egui::vec2(140.0, -80.0));
    let panned = map.transform().world_to_screen(near.position);
    assert_eq!(map.hit_test(&view, pointer), Selection::None);
    assert_eq!(
        map.hit_test(&view, panned),
        Selection::Planet(near.id),
        "hit after pan"
    );

    for (width, height) in [(1_280.0, 720.0), (1_920.0, 1_080.0)] {
        let dpi_map = fitted(width, height);
        let point = dpi_map.transform().world_to_screen(near.position);
        assert_eq!(
            dpi_map.hit_test(&view, point),
            Selection::Planet(near.id),
            "{width}x{height}"
        );
    }
}

#[test]
fn a_fleet_in_orbit_has_a_distinct_hit_target_from_its_planet() {
    let view = fixture();
    let map = fitted(1_280.0, 720.0);
    let fleet = &view.fleets[0];
    let ViewLocation::AtPlanet { position, .. } = fleet.location else {
        panic!("fixture fleet should be in orbit");
    };
    let fleet_pointer = map.transform().world_to_screen(position) + egui::vec2(9.0, -9.0);

    assert_eq!(
        map.hit_test(&view, fleet_pointer),
        Selection::Fleet(fleet.id)
    );
    assert_eq!(
        map.hit_test(&view, map.transform().world_to_screen(position)),
        Selection::Planet(view.planets[0].id)
    );
}

#[test]
fn offscreen_objects_are_culled_from_rendering_and_bounds_endpoints_are_inclusive() {
    let view = fixture();
    let near_bounds = WorldBounds::from_view(&PlayerView {
        planets: vec![view.planets[0].clone()],
        fleets: vec![view.fleets[0].clone()],
        ..view.clone()
    })
    .expect("near bounds");
    let mut map = MapState::new();
    *map.transform_mut() = MapTransform::fit(near_bounds, viewport(900.0, 600.0));

    assert_eq!(map.render_counts(&view), (1, 1));
    let ids: Vec<_> = map.visible_planets(&view).map(|planet| planet.id).collect();
    assert_eq!(ids, vec![view.planets[0].id]);

    let bounds = map.transform().visible_world_bounds();
    assert!(bounds.min_x <= 100.0 && bounds.max_x >= 100.0);
    assert!(bounds.min_y <= 100.0 && bounds.max_y >= 100.0);
}

#[test]
fn selection_is_stable_by_identifier_and_clears_when_the_entity_disappears() {
    let view = fixture();
    let selected = Selection::Planet(view.planets[0].id);
    assert_eq!(selected.retain(&view), selected);

    let mut without = view.clone();
    without
        .planets
        .retain(|planet| planet.id != view.planets[0].id);
    assert_eq!(selected.retain(&without), Selection::None);

    let mut map = fitted(800.0, 600.0);
    map.select(selected);
    map.prepare(&without, viewport(800.0, 600.0));
    assert_eq!(map.selection(), Selection::None);
}

#[test]
fn zoom_is_bounded_and_malformed_input_is_ignored() {
    let mut map = fitted(800.0, 600.0);
    let start = map.transform().zoom();
    map.transform_mut().zoom_at(egui::Pos2::ZERO, 0.0);
    map.transform_mut().zoom_at(egui::Pos2::ZERO, f32::NAN);
    assert!((map.transform().zoom() - start).abs() < f32::EPSILON);

    map.transform_mut().zoom_at(egui::Pos2::ZERO, 1e9);
    assert!((map.transform().zoom() - stars_desktop::ui::map::MAX_ZOOM).abs() < 0.01);
    map.transform_mut().zoom_at(egui::Pos2::ZERO, 1e-9);
    assert!((map.transform().zoom() - stars_desktop::ui::map::MIN_ZOOM).abs() < 0.0001);
}

#[test]
fn hit_test_ignores_a_fleet_at_an_unknown_orbit() {
    let human = PlayerId::from_raw(1);
    let mut view = fixture();
    view.fleets
        .push(fleet(9, "Ghost", human, ViewLocation::UnknownOrbit, 1));
    let map = fitted(1_280.0, 720.0);

    for x in (0..1_280).step_by(80) {
        for y in (0..720).step_by(80) {
            assert_ne!(
                map.hit_test(&view, egui::pos2(x as f32, y as f32)),
                Selection::Fleet(FleetId::from_raw(9))
            );
        }
    }
}

#[test]
fn stable_entity_visibility_is_not_decided_by_the_map() {
    let view = fixture();
    assert_eq!(view.planets[0].id, PlanetId::from_raw(1));
    assert_eq!(view.planets[2].owner, Some(PlayerId::from_raw(2)));
    assert!(EntityVisibility::Public.is_visible_to(None, view.player));
}

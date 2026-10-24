//! PR6 acceptance tests for deterministic universe and new-game generation.
//!
//! Expected counts and distributions are tied to the recovered `GenerateWorld()`
//! formulas and the January 1998 help topics. The random stream is the
//! documented remake stream, so tests assert reproduction and invariants rather
//! than invented original seeds.

use std::collections::BTreeSet;

use stars_core::model::{PlanetId, PlayerId, PlayerRole};
use stars_core::race::{
    Efficiency, HabitabilityRange, LesserTrait, PrimaryTrait, RaceSpec, ReferenceHabitability,
    ResearchCost, evaluate_race, validate_race,
};
use stars_core::rng::{RngStreamKey, RngSubsystem};
use stars_core::universe::{
    Density, Difficulty, GalaxySize, GameOptions, PlayerPositions, TURINDRONES_NAME,
    UniverseGenerationError, UniverseSettings, UniverseSettingsError, generate_universe,
    starting_technology, turindrones_race,
};

fn valid_race(primary: PrimaryTrait) -> RaceSpec {
    let mut race = RaceSpec::humanoids_anchor();
    race.primary = primary;
    race.growth_rate = 1;
    race.habitability = [HabitabilityRange::new(0, 5); 3];
    race.efficiency = Efficiency {
        resource_generation: 25,
        factory_production: 5,
        factory_cost: 25,
        factory_operation: 5,
        mine_production: 5,
        mine_cost: 15,
        mine_operation: 5,
    };
    race.lesser = [
        LesserTrait::GeneralizedResearch,
        LesserTrait::NoRamscoopEngines,
        LesserTrait::CheapEngines,
        LesserTrait::OnlyBasicRemoteMining,
        LesserTrait::NoAdvancedScanners,
        LesserTrait::BleedingEdgeTechnology,
        LesserTrait::RegeneratingShields,
    ]
    .into_iter()
    .collect();
    race.research.costs = [ResearchCost::Extra; 6];
    race.research.tech3 = false;
    let evaluation = evaluate_race(&race, &ReferenceHabitability);
    assert!(
        evaluation.displayed >= 0,
        "{primary} fixture is overspent: {}",
        evaluation.displayed
    );
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
    race
}

fn settings(size: GalaxySize, density: Density, seed: u64) -> UniverseSettings {
    let mut settings = UniverseSettings::new();
    settings.game_name = "PR6 Acceptance".to_owned();
    settings.seed = seed;
    settings.size = size;
    settings.density = density;
    settings.opponent_count = 1;
    settings
}

#[test]
fn same_seed_and_settings_reproduce_the_world_and_a_changed_seed_changes_it() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut first_settings = settings(GalaxySize::Medium, Density::Normal, 0xfeed_1234);
    first_settings.clumping = true;
    let first = generate_universe(&first_settings, &race).expect("first world");
    let second = generate_universe(&first_settings, &race).expect("second world");
    assert_eq!(first, second);

    let mut changed_settings = first_settings.clone();
    changed_settings.seed ^= 1;
    let changed = generate_universe(&changed_settings, &race).expect("changed world");
    assert_ne!(first.state.planets, changed.state.planets);
}

#[test]
fn every_size_density_and_clumping_option_produces_a_bounded_usable_world() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    for size in GalaxySize::ALL {
        for density in Density::ALL {
            for clumping in [false, true] {
                let mut world_settings = settings(size, density, 0x1234_5678);
                world_settings.clumping = clumping;
                let universe =
                    generate_universe(&world_settings, &race).expect("supported settings");

                let target = usize::from(world_settings.planet_count());
                assert!(universe.planet_count() >= usize::from(world_settings.player_count()));
                assert!(
                    universe.planet_count() <= target,
                    "{size:?} {density:?}: {} planets above target {target}",
                    universe.planet_count()
                );
                assert!(
                    universe.planet_count() * 4 >= target * 3,
                    "{size:?} {density:?}: too few planets ({}) for target {target}",
                    universe.planet_count()
                );

                let span = size.span_ly();
                for planet in universe.state.planets.values() {
                    assert!(
                        (1_010..=1_010 + span - 20).contains(&planet.position.x),
                        "{size:?}: {} outside x bounds",
                        planet.position.x
                    );
                    assert!(
                        (1_010..=1_010 + span - 20).contains(&planet.position.y),
                        "{size:?}: {} outside y bounds",
                        planet.position.y
                    );
                }

                let homes: BTreeSet<_> = universe.homeworlds.values().copied().collect();
                assert_eq!(homes.len(), usize::from(world_settings.player_count()));
                for (player, home) in &universe.homeworlds {
                    let planet = universe.state.planets.get(home).expect("homeworld exists");
                    assert_eq!(planet.owner, Some(*player));
                    assert!(planet.homeworld);
                    assert!(planet.starbase);
                    assert!(planet.population > 0);
                    assert!(
                        planet
                            .mineral_concentration
                            .iter()
                            .all(|value| *value >= 30)
                    );
                }
            }
        }
    }
}

#[test]
fn recovered_density_ordering_and_huge_packed_match_the_documented_relationship() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    for size in GalaxySize::ALL {
        let mut counts = Vec::new();
        for density in Density::ALL {
            let universe = generate_universe(&settings(size, density, 77), &race).expect("world");
            counts.push(universe.planet_count());
        }
        assert!(
            counts[0] < counts[1],
            "{size:?}: sparse {} is not below normal {}",
            counts[0],
            counts[1]
        );
        assert!(
            counts[1] < counts[2],
            "{size:?}: normal {} is not below dense {}",
            counts[1],
            counts[2]
        );
        assert!(
            counts[2] <= counts[3],
            "{size:?}: packed {} is below dense {}",
            counts[3],
            counts[2]
        );
    }

    let dense = generate_universe(&settings(GalaxySize::Huge, Density::Dense, 9), &race)
        .expect("huge dense");
    let packed = generate_universe(&settings(GalaxySize::Huge, Density::Packed, 9), &race)
        .expect("huge packed");
    assert_eq!(
        dense.planet_count(),
        packed.planet_count(),
        "the help says Packed is Dense in Huge"
    );
}

#[test]
fn clumping_reduces_the_mean_nearest_neighbour_distance() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut loose_total = 0_i64;
    let mut clumped_total = 0_i64;

    for seed in 0..12_u64 {
        let mut loose_settings = settings(GalaxySize::Medium, Density::Normal, seed);
        loose_settings.clumping = false;
        let loose = generate_universe(&loose_settings, &race).expect("loose world");
        loose_settings.clumping = true;
        let clumped = generate_universe(&loose_settings, &race).expect("clumped world");
        loose_total += nearest_neighbour_total(&loose);
        clumped_total += nearest_neighbour_total(&clumped);
    }
    assert!(
        clumped_total < loose_total,
        "clumped nearest-neighbour total {clumped_total} is not below {loose_total}"
    );
}

fn nearest_neighbour_total(universe: &stars_core::universe::GeneratedUniverse) -> i64 {
    let planets: Vec<_> = universe.state.planets.values().collect();
    let mut total = 0_i64;
    for (index, planet) in planets.iter().enumerate() {
        let mut best = i64::MAX;
        for (other_index, other) in planets.iter().enumerate() {
            if index == other_index {
                continue;
            }
            let dx = i64::from(planet.position.x - other.position.x);
            let dy = i64::from(planet.position.y - other.position.y);
            best = best.min(dx * dx + dy * dy);
        }
        if best != i64::MAX {
            total += best;
        }
    }
    total
}

#[test]
fn distant_player_positions_separate_homeworlds_more_than_close_positions() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut close_total = 0_i64;
    let mut distant_total = 0_i64;
    for seed in 0..20_u64 {
        let mut close = settings(GalaxySize::Medium, Density::Normal, seed);
        close.player_positions = PlayerPositions::Close;
        let close_world = generate_universe(&close, &race).expect("close world");
        close_total += homeworld_separation(&close_world);

        let mut distant = close.clone();
        distant.player_positions = PlayerPositions::Distant;
        let distant_world = generate_universe(&distant, &race).expect("distant world");
        distant_total += homeworld_separation(&distant_world);
    }
    assert!(
        distant_total > close_total,
        "distant homeworlds ({distant_total}) not farther than close ({close_total})"
    );
}

fn homeworld_separation(universe: &stars_core::universe::GeneratedUniverse) -> i64 {
    let homes: Vec<_> = universe
        .homeworlds
        .values()
        .filter_map(|id| universe.state.planets.get(id))
        .collect();
    let mut total = 0_i64;
    for (index, first) in homes.iter().enumerate() {
        for second in homes.iter().skip(index + 1) {
            let dx = i64::from(first.position.x - second.position.x);
            let dy = i64::from(first.position.y - second.position.y);
            total += dx * dx + dy * dy;
        }
    }
    total
}

#[test]
fn maximum_settings_place_sixteen_distinct_players_without_ownership_overlap() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut settings = settings(GalaxySize::Huge, Density::Packed, 0xc0ff_ee00);
    settings.opponent_count = 15;
    settings.difficulty = Difficulty::Expert;
    settings.player_positions = PlayerPositions::Distant;
    let universe = generate_universe(&settings, &race).expect("maximum game");

    assert_eq!(universe.state.players.len(), 16);
    assert_eq!(universe.opponents.len(), 15);
    let ids: BTreeSet<PlayerId> = universe.state.players.keys().copied().collect();
    assert_eq!(ids.len(), 16);
    let names: BTreeSet<&str> = universe
        .state
        .players
        .values()
        .map(|player| player.name.as_str())
        .collect();
    assert_eq!(names.len(), 16, "opponents must have distinct identities");

    let homes: Vec<PlanetId> = universe.homeworlds.values().copied().collect();
    let unique: BTreeSet<_> = homes.iter().copied().collect();
    assert_eq!(homes.len(), unique.len(), "homeworlds must not overlap");

    for opponent in &universe.opponents {
        let player = universe.state.players.get(opponent).expect("opponent");
        assert_eq!(player.role, PlayerRole::Computer);
        assert_eq!(
            player.race.as_ref().map(|race| race.name.as_str()),
            Some(TURINDRONES_NAME)
        );
    }
}

#[test]
fn opponent_seed_streams_are_distinct() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut settings = settings(GalaxySize::Small, Density::Normal, 0xabc);
    settings.opponent_count = 3;
    let universe = generate_universe(&settings, &race).expect("world");
    let mut rng = universe.state.rng.clone();
    let values: BTreeSet<u64> = universe
        .opponents
        .iter()
        .map(|opponent| {
            rng.draw_u64(RngStreamKey::for_player(
                *opponent,
                RngSubsystem::AiPlanning,
            ))
        })
        .collect();
    assert_eq!(values.len(), universe.opponents.len());
}

#[test]
fn every_primary_trait_and_low_starting_population_have_recovered_starting_conditions() {
    let expected_technology = |primary| match primary {
        PrimaryTrait::HyperExpansion | PrimaryTrait::InnerStrength => [0, 0, 1, 0, 0, 0],
        PrimaryTrait::SuperStealth => [0, 0, 1, 0, 5, 0],
        PrimaryTrait::WarMonger => [1, 6, 2, 0, 0, 0],
        PrimaryTrait::ClaimAdjuster => [1, 1, 2, 2, 0, 6],
        PrimaryTrait::SpaceDemolition => [0, 0, 3, 0, 0, 2],
        PrimaryTrait::PacketPhysics => [4, 0, 1, 0, 0, 0],
        PrimaryTrait::InterstellarTraveler => [0, 0, 6, 5, 0, 0],
        PrimaryTrait::AlternateReality => [1, 0, 1, 0, 0, 0],
        PrimaryTrait::JackOfAllTrades => [3, 3, 4, 3, 3, 3],
    };

    for primary in PrimaryTrait::ALL {
        let race = valid_race(primary);
        let mut world_settings = settings(GalaxySize::Medium, Density::Normal, 0x51);
        world_settings.opponent_count = 1;
        let universe = generate_universe(&world_settings, &race).expect("primary world");
        let home = universe.homeworld(universe.human).expect("human homeworld");
        let human = universe.state.players.get(&universe.human).expect("human");
        assert_eq!(
            home.environment.signed_values(),
            race.habitability.map(|band| i32::from(band.ideal()))
        );
        let expected_population = if matches!(
            primary,
            PrimaryTrait::PacketPhysics | PrimaryTrait::InterstellarTraveler
        ) {
            20_000
        } else {
            25_000
        };
        assert_eq!(home.population, expected_population, "{primary}");
        assert!(
            home.mineral_concentration.iter().all(|value| *value >= 30),
            "{primary} homeworld concentration"
        );
        assert_eq!(
            human.starting_technology,
            expected_technology(primary),
            "{primary} starting technology"
        );
        assert_eq!(
            home.factories == 0 && home.mines == 0 && home.defenses == 0,
            primary == PrimaryTrait::AlternateReality,
            "{primary} planetary installations"
        );
        if matches!(
            primary,
            PrimaryTrait::PacketPhysics | PrimaryTrait::InterstellarTraveler
        ) {
            assert_eq!(universe.owned_planets(universe.human).count(), 2);
        } else {
            assert_eq!(universe.owned_planets(universe.human).count(), 1);
        }
    }

    let mut low = valid_race(PrimaryTrait::JackOfAllTrades);
    low.lesser.insert(LesserTrait::LowStartingPopulation);
    assert!(validate_race(&low, &ReferenceHabitability).is_ok());
    let universe = generate_universe(&settings(GalaxySize::Medium, Density::Normal, 0x52), &low)
        .expect("low-population world");
    assert_eq!(
        universe
            .homeworld(universe.human)
            .expect("human homeworld")
            .population,
        17_500
    );
}

#[test]
fn accelerated_bbs_and_maximum_minerals_apply_to_every_homeworld() {
    let race = valid_race(PrimaryTrait::HyperExpansion);
    let mut world_settings = settings(GalaxySize::Small, Density::Normal, 0x53);
    world_settings.opponent_count = 2;
    world_settings.options = GameOptions {
        maximum_minerals: true,
        accelerated_bbs: true,
        ..GameOptions::default()
    };
    let universe = generate_universe(&world_settings, &race).expect("options world");
    for home in universe
        .homeworlds
        .values()
        .filter_map(|id| universe.state.planets.get(id))
    {
        assert_eq!(home.mineral_concentration, [100; 3]);
        assert!(home.surface_minerals.iter().all(|value| *value > 0));
        assert!(home.population > 25_000);
    }
}

#[test]
fn turindrones_profiles_are_distinct_and_from_the_recovered_table() {
    let easy = turindrones_race(Difficulty::Easy);
    let expert = turindrones_race(Difficulty::Expert);
    assert_ne!(easy.growth_rate, expert.growth_rate);
    assert!(expert.cheap_factories);
    assert!(expert.research.tech3);
    assert!(!easy.cheap_factories);
    assert_eq!(starting_technology(&easy)[4], 5);
}

#[test]
fn invalid_settings_and_races_fail_before_any_state_is_returned() {
    let race = valid_race(PrimaryTrait::JackOfAllTrades);
    let mut bad_settings = UniverseSettings::new();
    bad_settings.opponent_count = 0;
    assert!(matches!(
        generate_universe(&bad_settings, &race),
        Err(UniverseGenerationError::InvalidSettings(errors))
            if errors.contains(&UniverseSettingsError::OpponentCountOutOfRange { count: 0 })
    ));

    let mut bad_race = race.clone();
    bad_race.growth_rate = 0;
    assert!(matches!(
        generate_universe(&settings(GalaxySize::Tiny, Density::Sparse, 1), &bad_race),
        Err(UniverseGenerationError::InvalidRace(_))
    ));

    let mut overspent = RaceSpec::humanoids_anchor();
    overspent.lesser = [
        LesserTrait::ImprovedFuelEfficiency,
        LesserTrait::UltimateRecycling,
    ]
    .into_iter()
    .collect();
    assert!(matches!(
        generate_universe(&settings(GalaxySize::Tiny, Density::Sparse, 1), &overspent),
        Err(UniverseGenerationError::InvalidRace(_))
    ));
}

#[test]
fn pr7_fixture_is_stable() {
    let race = RaceSpec::humanoids_anchor();
    let mut fixture = UniverseSettings::new();
    fixture.game_name = "PR7 Fixture".to_owned();
    fixture.seed = 0x5052_375F_4649_5854;
    fixture.size = GalaxySize::Medium;
    fixture.density = Density::Normal;
    fixture.player_positions = PlayerPositions::Moderate;
    fixture.clumping = false;
    fixture.opponent_count = 2;
    fixture.difficulty = Difficulty::Standard;

    let universe = generate_universe(&fixture, &race).expect("fixture");
    assert_eq!(universe.planet_count(), 288);
    assert_eq!(universe.state.players.len(), 3);
    assert_eq!(
        universe.homeworlds,
        [
            (PlayerId::from_raw(1), PlanetId::from_raw(146)),
            (PlayerId::from_raw(2), PlanetId::from_raw(235)),
            (PlayerId::from_raw(3), PlanetId::from_raw(226)),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(generate_universe(&fixture, &race).expect("again"), universe);
}

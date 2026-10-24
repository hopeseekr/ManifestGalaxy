//! PR5 acceptance tests for the custom-race builder rules.
//!
//! These tests exercise the recovered point formula, field validation, all 24
//! trait selections, and the validated-race state contract. The formula is
//! statically derived from `stars-decompile/race.c`; no original-game execution
//! is claimed.

use std::collections::BTreeSet;

use stars_core::model::{
    EntityVisibility, GalacticPosition, GameState, Planet, PlanetId, Player, PlayerId, PlayerRole,
};
use stars_core::race::{
    FixedHabitability, LesserTrait, PrimaryTrait, RaceDraft, RaceField, RaceSpec,
    RaceValidationError, ReferenceHabitability, evaluate_race, trait_effect_coverage,
    validate_race, validate_race_fields,
};

fn funded_race() -> RaceSpec {
    let mut race = RaceSpec::humanoids_anchor();
    race.name = "Funded".to_owned();
    race.plural_name = "Funded".to_owned();
    // Low growth pays enough points to fund the expensive traits under test.
    race.growth_rate = 1;
    race
}

#[test]
fn every_primary_and_lesser_trait_is_selectable() {
    assert_eq!(PrimaryTrait::ALL.len(), 10);
    assert_eq!(LesserTrait::ALL.len(), 14);

    let mut race = RaceSpec::humanoids_anchor();
    for primary in PrimaryTrait::ALL {
        race.primary = primary;
        assert_eq!(race.primary, primary);
    }

    race.lesser.extend(LesserTrait::ALL);
    assert_eq!(race.lesser.len(), 14);
    for lesser in LesserTrait::ALL {
        assert!(race.has_lesser(lesser), "{}", lesser.id());
    }
}

#[test]
fn trait_effect_checklist_covers_all_24_rows_without_claiming_runtime_effects() {
    let rows: Vec<_> = trait_effect_coverage().collect();
    assert_eq!(rows.len(), 24);
    let ids: BTreeSet<_> = rows.iter().map(|row| row.trait_id).collect();
    assert_eq!(ids.len(), 24);
    for row in rows {
        assert!(row.builder_supported, "{}", row.trait_id);
        assert!(!row.runtime_implemented, "{}", row.trait_id);
        assert!(!row.owner_prs.is_empty(), "{}", row.trait_id);
    }
}

#[test]
fn humanoids_reference_race_matches_the_documented_25_points() {
    let evaluation = validate_race(&RaceSpec::humanoids_anchor(), &ReferenceHabitability)
        .expect("Humanoids anchor is affordable");
    assert_eq!(evaluation.displayed, 25);
    assert_eq!(evaluation.raw, 75);
}

#[test]
fn total_terraforming_is_selectable_and_priced() {
    let mut race = funded_race();
    assert_eq!(
        evaluate_race(&race, &FixedHabitability(0)).term_raw(LesserTrait::TotalTerraforming.id()),
        0,
        "disabled trait has no term"
    );

    race.lesser.insert(LesserTrait::TotalTerraforming);
    let evaluation = evaluate_race(&race, &FixedHabitability(0));
    assert_eq!(evaluation.term_raw("LRT.TT"), -25);
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
}

#[test]
fn ultimate_recycling_is_selectable_and_uses_the_inferred_price() {
    let mut race = funded_race();
    race.lesser.insert(LesserTrait::UltimateRecycling);
    let evaluation = evaluate_race(&race, &FixedHabitability(0));
    assert_eq!(evaluation.term_raw("LRT.UR"), -240);
    assert_eq!(
        LesserTrait::UltimateRecycling.cost().evidence,
        stars_core::race::CostEvidence::Inferred
    );
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
}

#[test]
fn mineral_alchemy_is_selectable_and_priced() {
    let mut race = funded_race();
    race.lesser.insert(LesserTrait::MineralAlchemy);
    let evaluation = evaluate_race(&race, &FixedHabitability(0));
    assert_eq!(evaluation.term_raw("LRT.MA"), -155);
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
}

#[test]
fn the_three_special_traits_compose_without_becoming_research_purchases() {
    let mut race = funded_race();
    race.lesser.extend([
        LesserTrait::TotalTerraforming,
        LesserTrait::UltimateRecycling,
        LesserTrait::MineralAlchemy,
    ]);
    let evaluation = evaluate_race(&race, &FixedHabitability(0));
    assert_eq!(
        evaluation.term_raw("LRT.TT")
            + evaluation.term_raw("LRT.UR")
            + evaluation.term_raw("LRT.MA"),
        -420
    );
    // Research choices are a separate Step 6 input and are not trait points.
    assert_eq!(evaluation.term_raw("research_cost"), 0);
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
}

#[test]
fn point_breakdown_terms_sum_to_the_raw_accumulator() {
    let mut race = funded_race();
    race.lesser.insert(LesserTrait::ImprovedFuelEfficiency);
    race.lesser.insert(LesserTrait::CheapEngines);
    race.research.costs[2] = stars_core::race::ResearchCost::Extra;
    let evaluation = evaluate_race(&race, &ReferenceHabitability);
    let sum: i64 = evaluation.terms.iter().map(|term| term.raw).sum();
    assert_eq!(for_each_term_sum(&evaluation), sum);
    assert_eq!(sum, evaluation.raw);
    assert_eq!(evaluation.displayed, (evaluation.raw / 3) as i32);
    assert!(evaluation.terms.iter().all(|term| !term.rule_id.is_empty()));
}

fn for_each_term_sum(evaluation: &stars_core::race::RaceEvaluation) -> i64 {
    evaluation.terms.iter().map(|term| term.raw).sum()
}

#[test]
fn missing_primary_trait_is_a_structural_draft_error() {
    let draft = RaceDraft::neutral();
    let errors = draft.to_spec().expect_err("draft is incomplete");
    assert_eq!(errors, vec![RaceValidationError::MissingPrimaryTrait]);
    assert_eq!(errors[0].field(), RaceField::Primary);
}

#[test]
fn invalid_fields_produce_field_specific_errors() {
    let mut race = funded_race();
    race.name.clear();
    race.plural_name = "Plural".to_owned();
    race.growth_rate = 0;
    race.habitability[0].min = 80;
    race.habitability[0].max = 20;
    race.efficiency.mine_cost = 99;

    let errors = validate_race_fields(&race);
    let keys: BTreeSet<_> = errors.iter().map(|error| error.field_key()).collect();
    assert!(keys.contains("name"));
    assert!(keys.contains("growth_rate"));
    assert!(keys.contains("gravity"));
    assert!(keys.contains("mine_cost"));

    let result = validate_race(&race, &ReferenceHabitability);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .iter()
            .all(|error| !error.to_string().is_empty())
    );
}

#[test]
fn overspent_budget_is_rejected_with_the_advantage_point_field() {
    let mut race = RaceSpec::humanoids_anchor();
    race.lesser.insert(LesserTrait::UltimateRecycling);
    let errors = validate_race(&race, &ReferenceHabitability).expect_err("overspent");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field(), RaceField::AdvantagePoints);
    assert!(matches!(
        errors[0],
        RaceValidationError::NegativeBalance { displayed } if displayed < 0
    ));
}

#[test]
fn habitability_band_requires_at_least_five_points() {
    let mut race = RaceSpec::humanoids_anchor();
    race.habitability[0] = stars_core::race::HabitabilityRange::new(20, 24);
    let errors = validate_race_fields(&race);
    assert!(errors.iter().any(|error| {
        error.field() == RaceField::Gravity
            && matches!(
                error,
                RaceValidationError::HabitabilitySpanTooSmall { minimum: 5, .. }
            )
    }));

    race.habitability[0] = stars_core::race::HabitabilityRange::new(20, 25);
    assert!(
        validate_race_fields(&race)
            .iter()
            .all(|error| error.field() != RaceField::Gravity)
    );

    // Immunity ignores the stored band width.
    race.habitability[0] = stars_core::race::HabitabilityRange::immune_with_bounds(20, 21);
    assert!(
        validate_race_fields(&race)
            .iter()
            .all(|error| error.field() != RaceField::Gravity)
    );
}

#[test]
fn legal_boundary_values_are_accepted() {
    let mut race = RaceSpec::humanoids_anchor();
    race.name = "A".repeat(31);
    race.plural_name = "Boundary Races".to_owned();
    race.emblem = 31;
    race.growth_rate = 1;
    race.habitability = [
        stars_core::race::HabitabilityRange::new(0, 100),
        stars_core::race::HabitabilityRange::new(0, 100),
        stars_core::race::HabitabilityRange::new(0, 100),
    ];
    race.efficiency.resource_generation = 7;
    race.efficiency.factory_production = 5;
    race.efficiency.factory_cost = 25;
    race.efficiency.factory_operation = 25;
    race.efficiency.mine_production = 25;
    race.efficiency.mine_cost = 15;
    race.efficiency.mine_operation = 25;

    assert!(validate_race_fields(&race).is_empty());
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());

    let mut fastest = race.clone();
    fastest.growth_rate = 20;
    assert!(validate_race_fields(&fastest).is_empty());
}

#[test]
fn name_too_long_and_bad_emblem_are_rejected() {
    let mut race = RaceSpec::humanoids_anchor();
    race.name = "X".repeat(32);
    race.emblem = 32;
    let keys: BTreeSet<_> = validate_race_fields(&race)
        .iter()
        .map(|error| error.field_key())
        .collect();
    assert!(keys.contains("name"));
    assert!(keys.contains("emblem"));
}

#[test]
fn a_validated_race_round_trips_in_authoritative_state() {
    let mut state = GameState::new(2400, 0x1234_5678_9abc_def0);
    let player_id = state.allocate_player_id().expect("player id");
    state.players.insert(
        player_id,
        Player {
            id: player_id,
            name: "Funded".to_owned(),
            role: PlayerRole::Human,
            ready: false,
            race: Some(funded_race()),
        },
    );
    let planet_id = state.allocate_planet_id().expect("planet id");
    state.planets.insert(
        planet_id,
        Planet {
            id: planet_id,
            name: "Home".to_owned(),
            position: GalacticPosition::new(0, 0),
            owner: Some(player_id),
            visibility: EntityVisibility::OwnerOnly,
        },
    );

    let bytes = state.encode_canonical();
    let decoded = GameState::decode_canonical(&bytes).expect("state decodes");
    assert_eq!(decoded, state);
    assert_eq!(
        decoded
            .players
            .get(&player_id)
            .and_then(|player| player.race.as_ref())
            .map(|race| race.name.as_str()),
        Some("Funded")
    );
}

#[test]
fn unsupported_core_contract_and_state_versions_are_rejected() {
    let fixture = GameState::new(2400, 1);

    let mut bytes = fixture.encode_canonical();
    // State format version is at bytes 4..6 in the canonical envelope.
    bytes[4] = 0xff;
    assert!(GameState::decode_canonical(&bytes).is_err());

    let mut contract_bytes = fixture.encode_canonical();
    // Core contract version is at bytes 6..10 in the canonical envelope.
    contract_bytes[6] = 0xff;
    let error = GameState::decode_canonical(&contract_bytes).expect_err("contract version");
    assert!(matches!(
        error.kind(),
        stars_core::model::StateDecodeErrorKind::UnsupportedContractVersion(_)
    ));
}

#[test]
fn invalid_races_cannot_enter_loaded_state() {
    let mut state = GameState::new(2400, 7);
    let player_id = state.allocate_player_id().expect("player id");
    let mut race = funded_race();
    race.growth_rate = 0;
    state.players.insert(
        player_id,
        Player {
            id: player_id,
            name: "Broken".to_owned(),
            role: PlayerRole::Human,
            ready: false,
            race: Some(race),
        },
    );

    assert!(matches!(
        state.validate_integrity(),
        Err(stars_core::model::StateValidationError::InvalidRace {
            field: "growth_rate",
            ..
        })
    ));
    assert!(GameState::decode_canonical(&state.encode_canonical()).is_err());

    let player = state.players.get_mut(&player_id).expect("player");
    let race = player.race.as_mut().expect("race");
    race.growth_rate = 1;
    race.habitability[0] = stars_core::race::HabitabilityRange::new(90, 10);
    assert!(matches!(
        state.validate_integrity(),
        Err(stars_core::model::StateValidationError::InvalidRace {
            field: "gravity",
            ..
        })
    ));
}

#[test]
fn race_validation_does_not_reject_an_unrecovered_exclusion_pair() {
    // The exclusion matrix is unknown; the product permits any priced
    // combination. This pair is only a regression that no guessed rule was
    // added. The traits themselves remain independently priced.
    let mut race = funded_race();
    race.lesser.extend([
        LesserTrait::ImprovedFuelEfficiency,
        LesserTrait::NoRamscoopEngines,
    ]);
    let evaluation = evaluate_race(&race, &FixedHabitability(0));
    assert_eq!(evaluation.term_raw("LRT.IFE"), -235);
    assert_eq!(evaluation.term_raw("LRT.NRSE"), 160);
    assert!(validate_race(&race, &ReferenceHabitability).is_ok());
}

#[test]
fn primary_trait_costs_match_the_recovered_table() {
    for (primary, expected) in [
        (PrimaryTrait::HyperExpansion, 40),
        (PrimaryTrait::SuperStealth, 95),
        (PrimaryTrait::WarMonger, 45),
        (PrimaryTrait::ClaimAdjuster, 10),
        (PrimaryTrait::InnerStrength, -100),
        (PrimaryTrait::SpaceDemolition, -150),
        (PrimaryTrait::PacketPhysics, 120),
        (PrimaryTrait::InterstellarTraveler, 180),
        (PrimaryTrait::AlternateReality, 90),
        (PrimaryTrait::JackOfAllTrades, -66),
    ] {
        assert_eq!(primary.point_cost_raw(), expected, "{primary}");
    }
}

#[test]
fn a_negative_raw_balance_truncates_toward_zero() {
    let mut race = RaceSpec::humanoids_anchor();
    race.lesser.extend(LesserTrait::ALL);
    race.research.costs = [stars_core::race::ResearchCost::Cheaper; 6];
    let evaluation = evaluate_race(&race, &ReferenceHabitability);
    assert!(evaluation.raw < 0);
    assert_eq!(evaluation.displayed, (evaluation.raw / 3) as i32);
}

#[test]
fn player_race_field_is_optional_for_computer_slots() {
    let player = Player {
        id: PlayerId::from_raw(1),
        name: "Turindrones".to_owned(),
        role: PlayerRole::Computer,
        ready: false,
        race: None,
    };
    assert!(player.race.is_none());
    let _ = PlanetId::from_raw(1);
}

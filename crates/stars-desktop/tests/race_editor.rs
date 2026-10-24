//! PR5 acceptance tests for the desktop custom-race editor.
//!
//! The tests use the same headless model and egui layout entry point as the
//! native window. No display or GPU device is required.

use eframe::egui;

use stars_core::race::{
    HabitabilityRange, LesserTrait, RaceDraft, RaceField, RaceSpec, ReferenceHabitability,
    ResearchCost, evaluate_race, validate_race,
};
use stars_desktop::ui::{RaceBuilderState, RacePage};

fn context_input(size: egui::Vec2) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    }
}

fn render(builder: &mut RaceBuilderState, context: &egui::Context, size: egui::Vec2) {
    let output = context.run_ui(context_input(size), |ui| builder.draw(ui));
    output.drop_without_applying_deltas();
}

#[test]
fn editor_runs_at_multiple_scales_and_window_proportions() {
    let context = egui::Context::default();
    let mut builder = RaceBuilderState::new();
    builder.draft_mut().primary = Some(stars_core::race::PrimaryTrait::ClaimAdjuster);

    for (scale, width, height) in [
        (1.0_f32, 640.0_f32, 420.0_f32),
        (1.5, 1100.0, 700.0),
        (2.0, 900.0, 1000.0),
    ] {
        context.set_zoom_factor(scale);
        for page in RacePage::ALL {
            builder.set_page(page);
            render(&mut builder, &context, egui::vec2(width, height));
        }
    }

    assert_eq!(builder.page(), RacePage::Review);
}

#[test]
fn stepping_backward_preserves_unrelated_selections() {
    let mut builder = RaceBuilderState::new();
    builder.draft_mut().name = "Keepers".to_owned();
    builder.draft_mut().primary = Some(stars_core::race::PrimaryTrait::SuperStealth);
    builder.set_page(RacePage::Primary);
    builder.next_page();
    builder
        .draft_mut()
        .lesser
        .insert(LesserTrait::TotalTerraforming);
    builder
        .draft_mut()
        .lesser
        .insert(LesserTrait::MineralAlchemy);
    builder.next_page();
    builder.draft_mut().growth_rate = 7;

    builder.back_page();
    assert_eq!(builder.page(), RacePage::Lesser);
    assert_eq!(builder.draft().name, "Keepers");
    assert_eq!(
        builder.draft().primary,
        Some(stars_core::race::PrimaryTrait::SuperStealth)
    );
    assert!(
        builder
            .draft()
            .lesser
            .contains(&LesserTrait::TotalTerraforming)
    );
    assert!(
        builder
            .draft()
            .lesser
            .contains(&LesserTrait::MineralAlchemy)
    );
    assert_eq!(builder.draft().growth_rate, 7);
}

#[test]
fn reset_clears_the_draft_and_validation_state() {
    let mut builder = RaceBuilderState::new();
    *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
    builder.draft_mut().name = "Disposable".to_owned();
    builder.set_page(RacePage::Research);
    builder.finish().expect("valid race before reset");

    builder.reset();
    assert_eq!(builder.page(), RacePage::Basic);
    assert_eq!(builder.draft(), &RaceDraft::neutral());
    assert!(builder.errors().is_empty());
    assert!(builder.validated_race().is_none());
}

#[test]
fn keyboard_shortcuts_move_pages_and_reset() {
    let context = egui::Context::default();
    let mut builder = RaceBuilderState::new();

    let press = |key: egui::Key, modifiers: egui::Modifiers| {
        let mut events = Vec::new();
        if modifiers != egui::Modifiers::NONE {
            events.push(egui::Event::ModifiersChanged(modifiers));
        }
        events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events,
            ..Default::default()
        }
    };

    let output = context.run_ui(press(egui::Key::PageDown, egui::Modifiers::NONE), |ui| {
        builder.draw(ui);
    });
    output.drop_without_applying_deltas();
    assert_eq!(builder.page(), RacePage::Primary);

    let output = context.run_ui(press(egui::Key::PageUp, egui::Modifiers::NONE), |ui| {
        builder.draw(ui);
    });
    output.drop_without_applying_deltas();
    assert_eq!(builder.page(), RacePage::Basic);

    builder.draft_mut().primary = Some(stars_core::race::PrimaryTrait::WarMonger);
    let output = context.run_ui(press(egui::Key::R, egui::Modifiers::COMMAND), |ui| {
        builder.draw(ui)
    });
    output.drop_without_applying_deltas();
    assert!(builder.draft().primary.is_none());
}

#[test]
fn overview_values_and_core_evaluation_agree() {
    let mut builder = RaceBuilderState::new();
    builder.draft_mut().primary = Some(stars_core::race::PrimaryTrait::PacketPhysics);
    builder
        .draft_mut()
        .lesser
        .insert(LesserTrait::NoAdvancedScanners);
    builder.draft_mut().growth_rate = 9;
    builder.draft_mut().research.costs[4] = ResearchCost::Cheaper;

    let spec = builder.draft().to_spec().expect("primary selected");
    let core = evaluate_race(&spec, &ReferenceHabitability);
    let ui = builder.evaluation().expect("live evaluation");
    assert_eq!(ui.raw, core.raw);
    assert_eq!(ui.displayed, core.displayed);
    assert_eq!(
        ui.term_raw("no_advanced_scanners"),
        -280,
        "Packet Physics and No Advanced Scanners pairing"
    );
}

#[test]
fn finish_rejects_an_incomplete_draft_with_a_primary_field_error() {
    let mut builder = RaceBuilderState::new();
    let errors = builder.finish().expect_err("no primary selected");
    assert_eq!(
        errors,
        vec![stars_core::race::RaceValidationError::MissingPrimaryTrait]
    );
    assert_eq!(errors[0].field(), RaceField::Primary);
    assert!(builder.validated_race().is_none());
}

#[test]
fn finish_accepts_a_validated_race_and_exposes_it_to_the_new_game_flow() {
    let mut builder = RaceBuilderState::new();
    *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
    let spec = builder.finish().expect("Humanoids anchor is valid");
    assert_eq!(spec.name, "Humanoids");
    assert_eq!(
        builder.validated_race().map(|race| race.primary),
        Some(stars_core::race::PrimaryTrait::JackOfAllTrades)
    );
    let evaluation = validate_race(&spec, &ReferenceHabitability).expect("validated");
    assert_eq!(evaluation.displayed, 25);
}

#[test]
fn finish_rejects_an_overspent_race_and_reports_the_advantage_point_field() {
    let mut builder = RaceBuilderState::new();
    let mut race = RaceSpec::humanoids_anchor();
    race.lesser.insert(LesserTrait::UltimateRecycling);
    *builder.draft_mut() = RaceDraft::from_spec(&race);

    let errors = builder.finish().expect_err("overspent");
    assert_eq!(errors[0].field(), RaceField::AdvantagePoints);
    assert!(builder.validated_race().is_none());
    assert_eq!(builder.errors(), errors);
}

#[test]
fn finish_rejects_a_habitability_band_narrower_than_five_points() {
    let mut builder = RaceBuilderState::new();
    *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
    builder.draft_mut().habitability[1] = HabitabilityRange::new(40, 44);

    let errors = builder.finish().expect_err("band is too narrow");
    assert!(errors.iter().any(|error| {
        error.field() == RaceField::Temperature
            && matches!(
                error,
                stars_core::race::RaceValidationError::HabitabilitySpanTooSmall { minimum: 5, .. }
            )
    }));
}

#[test]
fn finish_rejects_invalid_fields_without_losing_the_draft() {
    let mut builder = RaceBuilderState::new();
    *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
    builder.draft_mut().growth_rate = 0;
    builder.draft_mut().name.clear();

    let errors = builder.finish().expect_err("invalid fields");
    let keys: std::collections::BTreeSet<_> =
        errors.iter().map(|error| error.field_key()).collect();
    assert!(keys.contains("growth_rate"));
    assert!(keys.contains("name"));
    assert_eq!(builder.draft().growth_rate, 0);
    assert!(builder.draft().primary.is_some());
}

#[test]
fn editing_after_finish_clears_stale_validation() {
    let context = egui::Context::default();
    let mut builder = RaceBuilderState::new();
    *builder.draft_mut() = RaceDraft::from_spec(&RaceSpec::humanoids_anchor());
    builder.finish().expect("valid");
    assert!(builder.validated_race().is_some());

    builder.draft_mut().name = "Changed".to_owned();
    render(&mut builder, &context, egui::vec2(900.0, 700.0));
    assert!(builder.validated_race().is_none());
}

#[test]
fn special_traits_are_visible_in_the_editor_model() {
    let mut builder = RaceBuilderState::new();
    builder.draft_mut().primary = Some(stars_core::race::PrimaryTrait::SpaceDemolition);
    builder.draft_mut().lesser.extend([
        LesserTrait::TotalTerraforming,
        LesserTrait::UltimateRecycling,
        LesserTrait::MineralAlchemy,
    ]);
    for trait_value in [
        LesserTrait::TotalTerraforming,
        LesserTrait::UltimateRecycling,
        LesserTrait::MineralAlchemy,
    ] {
        assert!(builder.draft().lesser.contains(&trait_value));
    }
    let evaluation = builder.evaluation().expect("evaluation");
    assert!(evaluation.term("LRT.TT").is_some());
    assert!(evaluation.term("LRT.UR").is_some());
    assert!(evaluation.term("LRT.MA").is_some());
}
